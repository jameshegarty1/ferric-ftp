use std::os::unix::prelude::OsStrExt;
use std::path::PathBuf;
use log::info;

use super::client::SftpClient;
use super::error::SftpError;
use super::packet::ClientPacket;
use super::packet::ServerPacket;
use super::types::{CommandOutput, SftpStatus};

pub fn list_directory(client: &mut SftpClient, path: &PathBuf) -> Result<CommandOutput, SftpError> {
    let path_str = path
        .to_str()
        .ok_or_else(|| SftpError::ClientError("Invalid UTF-8 in path".into()))?;

    let opendir_packet = ClientPacket::OpenDir {
        request_id: client.session.next_request_id,
        path: path_str.to_string(),
    };

    client
        .session
        .send_packet(opendir_packet)
        .map_err(|e| SftpError::ClientError(Box::new(e)))?;

    client.session.next_request_id += 1;

    let opendir_response_packet = ServerPacket::from_session(&mut client.session)?;

    match opendir_response_packet {
        ServerPacket::Status {
            request_id,
            status_code,
            message,
        } => {
            return Err(SftpError::UnknownError {
                message: format!(
                    "SFTP StatusMessage Request {} Error Code: {} Message: {}",
                    request_id, status_code, message
                ),
            });
        }
        ServerPacket::Handle { request_id, handle } => {
            info!("Got a handle {:?} for path {}", handle, path.display());
            client
                .session
                .handles
                .insert(path_str.to_string(), handle.clone());
        }
        _ => {
            return Err(SftpError::UnknownError {
                message: "Unexpected packet type".to_string(),
            });
        }
    }

    let handle = client
        .session
        .handles
        .get(path_str)
        .ok_or_else(|| SftpError::UnknownError {
            message: "No handle found for path".to_string(),
        })?
        .clone();

    let mut files = Vec::new();
    loop {
        let readdir_packet = ClientPacket::ReadDir {
            request_id: client.session.next_request_id,
            handle: handle.clone(),
        };

        client
            .session
            .send_packet(readdir_packet)
            .map_err(|e| SftpError::ClientError(Box::new(e)))?;

        client.session.next_request_id += 1;

        let readdir_response_packet = ServerPacket::from_session(&mut client.session)?;

        match readdir_response_packet {
            ServerPacket::Status {
                request_id,
                status_code,
                message,
            } => {
                if status_code == SftpStatus::Eof as u32 {
                    break;
                } else {
                    return Err(SftpError::UnknownError {
                        message: format!(
                            "Unexpected Status message: {} - {}",
                            status_code, message
                        ),
                    });
                }
            }
            ServerPacket::Name {
                request_id,
                files: names,
            } => {
                files.extend(names);
            }

            _ => {
                return Err(SftpError::UnknownError {
                    message: "Unexpected packet type".to_string(),
                });
            }
        }
    }

    let close_packet = ClientPacket::Close {
        request_id: client.session.next_request_id,
        handle,
    };

    client
        .session
        .send_packet(close_packet)
        .map_err(|e| SftpError::ClientError(Box::new(e)))?;

    let close_response_packet = ServerPacket::from_session(&mut client.session)?;

    match close_response_packet {
        ServerPacket::Status {
            request_id,
            status_code,
            message,
        } => {
            if status_code == SftpStatus::Ok as u32 {
                {}
            } else {
                return Err(SftpError::UnknownError {
                    message: format!(
                        "Unexpected Status message: {} - {}",
                        status_code, message
                    ),
                });
            }
        }
        _ => {
            return Err(SftpError::UnknownError {
                message: "Unexpected response to close packet".to_string(),
            });
        }
    }

    client.session.next_request_id += 1;

    client.session.handles.remove(path_str);

    for file in &files {
        println!("{}", file.long_name);
    }

    Ok(CommandOutput {
        result: true,
        message: format!("Listed {} entries from {}", files.len(), path.display()),
    })
}

pub fn change_directory(client: &mut SftpClient, path: &PathBuf) -> Result<CommandOutput, SftpError> {
    let new_path = if path.starts_with("/") {
        PathBuf::from(path)
    } else {
        client.session.working_dir.join(path)
    };

    let path_str = new_path.to_string_lossy();
    // Check its a real path
    let stat_packet = ClientPacket::Stat {
        request_id: client.session.next_request_id,
        path: path_str.to_string(),
    };

    client.session.send_packet(stat_packet).map_err(|e| {
        SftpError::ClientError(Box::new(e))
    })?;

    client.session.next_request_id += 1;

    match ServerPacket::from_session(&mut client.session)? {
        ServerPacket::Name { request_id, files } => {
            if files.len() == 1 {
                client.session.working_dir = PathBuf::from(files[0].short_name.clone());
            } else {
                return Err(SftpError::ClientError(
                    std::io::Error::new(
                        std::io::ErrorKind::Other,
                        "Unexpected number of paths in realpath response",
                    ).into(),
                ));
            }
        }
        ServerPacket::Status {  status_code, message, .. } => {
            return Err(SftpError::ServerError {
                code: status_code,
                message: message,
            });
        }
        _ => {
            return Err(SftpError::ClientError(
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "Unexpected response type for realpath",
                ).into(),
            ));
        }
    }

    Ok(CommandOutput {
        result: true,
        message: format!("Set working directory to {}", path.display()),
    })
}