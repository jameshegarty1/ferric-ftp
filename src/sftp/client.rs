use super::commands;
use super::error::SftpError;
use super::packet::ClientPacket;
use super::packet::ServerPacket;
use super::session::SftpSession;
use super::types::{CommandOutput, SftpCommand};
use log::info;
use ssh2::Channel;
use std::collections::HashMap;
use std::io::{Read, Write};
use std::path::PathBuf;

pub struct SftpClient {
    pub session: SftpSession,
    pub working_dir: PathBuf,
}

impl SftpClient {
    pub fn new(mut channel: Channel, version: u32) -> Result<Self, SftpError> {
        let init_packet = ClientPacket::Init { version };
        channel
            .write_all(&init_packet.to_bytes())
            .map_err(|e| SftpError::ClientError(e.into()))?;

        let mut session = SftpSession {
            channel: channel,
            version: version,
            next_request_id: 0,
            handles: HashMap::new(),
        };

        match ServerPacket::from_session(&mut session)? {
            ServerPacket::Version { version: _ } => {
                // Initialize working dir with a RealPath request
                let mut working_dir: PathBuf = PathBuf::new();
                let realpath_packet = ClientPacket::RealPath {
                    request_id: session.next_request_id,
                    path: ".".to_string(),
                };
                session.next_request_id += 1;
                session
                    .send_packet(realpath_packet)
                    .map_err(|e| SftpError::ClientError(e.into()))?;

                match ServerPacket::from_session(&mut session)? {
                    ServerPacket::Name { request_id, files } => {
                        if files.len() == 1 {
                            // The first (and only) entry in the response is the absolute path
                            working_dir = PathBuf::from(&files[0].short_name);
                            info!("Initialized working directory: {}", working_dir.display());
                        } else {
                            return Err(SftpError::ClientError(
                                std::io::Error::new(
                                    std::io::ErrorKind::Other,
                                    "Unexpected number of paths in realpath response",
                                )
                                .into(),
                            ));
                        }
                    }
                    ServerPacket::Status {
                        status_code,
                        message,
                        ..
                    } => {
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
                            )
                            .into(),
                        ));
                    }
                }
                Ok(SftpClient {
                    session,
                    working_dir,
                })
            }
            _ => Err(SftpError::ClientError(
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "SFTP error when creating SFTP session",
                )
                .into(),
            )),
        }
    }

    pub fn execute_command(&mut self, cmd: &SftpCommand) -> Result<CommandOutput, SftpError> {
        match cmd {
            SftpCommand::Ls { path } => commands::list_directory(self, path),
            SftpCommand::Cd { path } => commands::change_directory(self, path),
            _ => {
                todo!()
            } // etc.
        }
    }
}
