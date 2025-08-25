use std::path::PathBuf;
use log::info;

use super::client::SftpClient;
use super::error::SftpError;
use super::packet::SftpPacket;
use super::types::{CommandOutput, SftpStatus};

pub fn list_directory(client: &mut SftpClient, path: &PathBuf) -> Result<CommandOutput, SftpError> {
    let path_str = path
        .to_str()
        .ok_or_else(|| SftpError::ClientError("Invalid UTF-8 in path".into()))?;

    let opendir_packet = SftpPacket::OpenDir {
        request_id: client.session.next_request_id,
        path: path_str.to_string(),
    };

    client
        .session
        .send_packet(opendir_packet)
        .map_err(|e| SftpError::ClientError(Box::new(e)))?;

    client.session.next_request_id += 1;

    let opendir_response_packet = SftpPacket::from_session(&mut client.session)?;

    match opendir_response_packet {
        SftpPacket::Status {
            request_id,
            error_code,
            message,
        } => {
            return Err(SftpError::UnknownError {
                message: format!(
                    "SFTP StatusMessage Request {} Error Code: {} Message: {}",
                    request_id, error_code, message
                ),
            });
        }
        SftpPacket::Handle { request_id, handle } => {
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
        let readdir_packet = SftpPacket::ReadDir {
            request_id: client.session.next_request_id,
            handle: handle.clone(),
        };

        client
            .session
            .send_packet(readdir_packet)
            .map_err(|e| SftpError::ClientError(Box::new(e)))?;

        client.session.next_request_id += 1;

        let readdir_response_packet = SftpPacket::from_session(&mut client.session)?;

        match readdir_response_packet {
            SftpPacket::Status {
                request_id,
                error_code,
                message,
            } => {
                if error_code == SftpStatus::Eof as u32 {
                    break;
                } else {
                    return Err(SftpError::UnknownError {
                        message: format!(
                            "Unexpected Status message: {} - {}",
                            error_code, message
                        ),
                    });
                }
            }
            SftpPacket::Name {
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

    let close_packet = SftpPacket::Close {
        request_id: client.session.next_request_id,
        handle,
    };

    client
        .session
        .send_packet(close_packet)
        .map_err(|e| SftpError::ClientError(Box::new(e)))?;

    let close_response_packet = SftpPacket::from_session(&mut client.session)?;

    match close_response_packet {
        SftpPacket::Status {
            request_id,
            error_code,
            message,
        } => {
            if error_code == SftpStatus::Ok as u32 {
                {}
            } else {
                return Err(SftpError::UnknownError {
                    message: format!(
                        "Unexpected Status message: {} - {}",
                        error_code, message
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
    todo!("Implement change_directory")
}