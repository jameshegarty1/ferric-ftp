use super::constants::*;
use super::error::SftpError;
use super::packet::ClientPacket;
use super::packet::ServerPacket;
use super::session::SftpSession;
use super::types::{DirectoryCache, FileAttributes, FileInfo, SftpCommand, SftpStatus};
use log::info;
use ssh2::Channel;
use std::collections::HashMap;
use std::io::Write;
use std::path::PathBuf;
use std::time::SystemTime;

pub struct SftpClient {
    pub session: SftpSession,
    pub working_dir: PathBuf,
    pub directory_cache: HashMap<PathBuf, DirectoryCache>,
    pub current_listing: Vec<FileInfo>,
    pub handles: HashMap<String, Vec<u8>>,
}

impl SftpClient {
    pub fn new(mut channel: Channel, version: u32) -> Result<Self, SftpError> {
        // Initialise connection
        let init_packet = ClientPacket::Init { version };
        channel
            .write_all(&init_packet.to_bytes())
            .map_err(|e| SftpError::ClientError(e.into()))?;

        let mut session = SftpSession {
            channel,
            version,
            next_request_id: 0,
        };

        // Handle Init reponse
        match ServerPacket::from_session(&mut session)? {
            // Successful Init
            ServerPacket::Version { version: _ } => {
                // Initialize working dir with a RealPath request
                let mut working_dir: PathBuf = PathBuf::new();
                let realpath_packet = ClientPacket::RealPath {
                    request_id: session.next_request_id,
                    path: ".".to_string(),
                };
                session.next_request_id += 1;
                session.send_packet(realpath_packet)?;

                match ServerPacket::from_session(&mut session)? {
                    ServerPacket::Name { request_id, files } => {
                        if files.len() == 1 {
                            working_dir = PathBuf::from(&files[0].name);
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
                        request_id,
                    } => {
                        return Err(SftpError::ServerError {
                            code: status_code,
                            request_id,
                            message,
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
                    directory_cache: HashMap::new(),
                    current_listing: Vec::new(),
                    handles: HashMap::new(),
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

    pub fn resolve_path(&self, path: &PathBuf) -> PathBuf {
        if path.is_absolute() {
            return path.clone();
        }

        let path_str = path.to_string_lossy();

        match path_str.as_ref() {
            "." => self.working_dir.clone(),
            ".." => self.get_parent_directory(),
            _ => self.working_dir.join(path),
        }
    }

    fn get_parent_directory(&self) -> PathBuf {
        let components: Vec<String> = self
            .working_dir
            .to_string_lossy()
            .split('/')
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect();

        if components.is_empty() {
            PathBuf::from("/")
        } else {
            let mut new_components = components;
            new_components.pop();
            if new_components.is_empty() {
                PathBuf::from("/")
            } else {
                PathBuf::from(format!("/{}", new_components.join("/")))
            }
        }
    }

    pub fn execute_command(&mut self, cmd: &SftpCommand) -> Result<bool, SftpError> {
        match cmd {
            SftpCommand::Ls { path } => {
                self.list_directory(path.as_ref())?;
                Ok(true)
            }
            SftpCommand::Cd { path } => {
                self.change_directory(path.as_ref())?;
                Ok(true)
            }
            SftpCommand::Pwd => {
                self.print_working_directory()?;
                Ok(true)
            }
            SftpCommand::Get {
                remote_path,
                local_path,
            } => {
                self.get_file(remote_path, local_path.as_ref())?;
                Ok(true)
            }
            SftpCommand::Put {
                local_path,
                remote_path,
            } => {
                self.put_file(remote_path, local_path.as_ref())?;
                Ok(true)
            }
            SftpCommand::Help => {
                self.show_help()?;
                Ok(true)
            }
            SftpCommand::Bye => Ok(false),
        }
    }

    fn list_directory(&mut self, path: Option<&PathBuf>) -> Result<(), SftpError> {
        let target_path = match path {
            Some(p) => self.resolve_path(p),
            None => self.working_dir.clone(),
        };

        if let Some(cache) = self.directory_cache.get(&target_path) {
            self.current_listing = cache.files.clone();
            self.display_current_listing();
            return Ok(());
        }

        let path_str = target_path
            .to_str()
            .ok_or_else(|| SftpError::ClientError("Invalid UTF-8 in path".into()))?;

        let opendir_packet = ClientPacket::OpenDir {
            request_id: self.session.next_request_id,
            path: path_str.to_string(),
        };

        self.session.send_packet(opendir_packet)?;
        self.session.next_request_id += 1;

        let opendir_response_packet = ServerPacket::from_session(&mut self.session)?;

        let handle = match opendir_response_packet {
            ServerPacket::Handle { request_id, handle } => {
                self.handles.insert(path_str.to_string(), handle.clone());
                handle
            }
            ServerPacket::Status {
                status_code,
                message,
                request_id,
            } => {
                return Err(SftpError::ServerError {
                    code: status_code,
                    request_id,
                    message,
                });
            }
            _ => {
                return Err(SftpError::UnexpectedPacket(
                    "Unexpected packet while getting directory handle.",
                ))
            }
        };

        // Parse response into a vector of FileInfos
        let mut files = Vec::new();
        loop {
            let readdir_packet = ClientPacket::ReadDir {
                request_id: self.session.next_request_id,
                handle: handle.clone(),
            };

            self.session.send_packet(readdir_packet)?;
            self.session.next_request_id += 1;

            let readdir_response_packet = ServerPacket::from_session(&mut self.session)?;

            match readdir_response_packet {
                ServerPacket::Name {
                    request_id,
                    files: names,
                } => {
                    files.extend(names);
                }
                ServerPacket::Status {
                    request_id,
                    status_code,
                    message,
                } => {
                    if status_code == SftpStatus::Eof as u32 {
                        break;
                    } else {
                        return Err(SftpError::ServerError {
                            code: status_code,
                            request_id,
                            message,
                        });
                    }
                }
                _ => {
                    return Err(SftpError::UnexpectedPacket(
                        "Unexpected packet when reading directory.",
                    ))
                }
            }
        }

        let close_packet = ClientPacket::Close {
            request_id: self.session.next_request_id,
            handle,
        };

        self.session.send_packet(close_packet)?;
        self.session.next_request_id += 1;

        let close_response = ServerPacket::from_session(&mut self.session)?;
        if let ServerPacket::Status {
            status_code,
            message,
            request_id,
        } = close_response
        {
            if status_code != SftpStatus::Ok as u32 {
                return Err(SftpError::ServerError {
                    code: status_code,
                    request_id,
                    message,
                });
            }
        }

        self.current_listing = files.clone();
        self.directory_cache.insert(
            target_path,
            DirectoryCache {
                files,
                timestamp: SystemTime::now(),
            },
        );

        self.display_current_listing();

        Ok(())
    }

    fn display_current_listing(&self) {
        for file in self.current_listing.clone() {
            println!("{}", file.display_name);
        }
    }

    fn change_directory(&mut self, path: Option<&PathBuf>) -> Result<(), SftpError> {
        let path_str = match path {
            Some(p) => p.to_str(),
            None => Some("/"),
        }
        .ok_or_else(|| SftpError::ClientError("Invalid UTF-8 in path".into()))?;

        let realpath_packet = ClientPacket::RealPath {
            request_id: self.session.next_request_id,
            path: path_str.to_string(),
        };
        self.session.send_packet(realpath_packet)?;

        self.session.next_request_id += 1;

        let response = ServerPacket::from_session(&mut self.session)?;
        match response {
            ServerPacket::Name { files, .. } => {
                if files.len() == 1 {
                    let resolved_path = PathBuf::from(&files[0].name);

                    let stat = self.stat(&resolved_path)?;
                    if !stat.is_directory {
                        return Err(SftpError::NotADirectory(
                            resolved_path.display().to_string(),
                        ));
                    }

                    self.working_dir = resolved_path;
                    self.current_listing.clear();
                    Ok(())
                } else {
                    Err(SftpError::UnexpectedResponse(
                        "Expected exactly one path from RealPath",
                    ))
                }
            }
            ServerPacket::Status {
                status_code,
                request_id,
                message,
            } => Err(SftpError::ServerError {
                code: status_code,
                request_id,
                message,
            }),
            _ => Err(SftpError::UnexpectedPacket("RealPath response")),
        }
    }

    fn print_working_directory(&self) -> Result<(), SftpError> {
        print!("{}\n", self.working_dir.display());
        Ok(())
    }

    fn show_help(&self) -> Result<(), SftpError> {
        println!("Available commands:\nls - list files in current directory\ncd - change current directory\nget - download file\nput - upload file\nbye - exit");
        Ok(())
    }

    fn put_file(
        &mut self,
        remote_path: &PathBuf,
        local_path: Option<&PathBuf>,
    ) -> Result<(), SftpError> {
        todo!()
    }
    fn get_file(
        &mut self,
        remote_path: &PathBuf,
        local_path: Option<&PathBuf>,
    ) -> Result<(), SftpError> {
        todo!()
    }

    fn get_file_handle(&mut self, path: &PathBuf) -> Result<Vec<u8>, SftpError> {
        let path_str = path
            .to_str()
            .ok_or_else(|| SftpError::ClientError("Invalid UTF-8 in path".into()))?;

        if let Some(handle) = self.handles.get(path_str) {
            return Ok(handle.clone());
        }

        let open_packet = ClientPacket::Open {
            request_id: self.session.next_request_id,
            path: path_str.to_string(),
            pflags: SSH_FXF_READ,
            attrs: FileAttributes::default(),
        };
        self.session.send_packet(open_packet)?;
        self.session.next_request_id += 1;

        let response = ServerPacket::from_session(&mut self.session)?;
        match response {
            ServerPacket::Handle { handle, .. } => {
                self.handles.insert(path_str.to_string(), handle.clone());
                Ok(handle)
            }
            ServerPacket::Status {
                status_code,
                request_id,
                message,
            } => Err(SftpError::ServerError {
                code: status_code,
                request_id,
                message,
            }),
            _ => Err(SftpError::UnexpectedPacket("Stat response")),
        }
    }

    fn close_handle(&mut self, path: &str) -> Result<(), SftpError> {
        if let Some(handle) = self.handles.remove(path) {
            let close_packet = ClientPacket::Close {
                request_id: self.session.next_request_id,
                handle,
            };
            self.session.send_packet(close_packet)?;
            self.session.next_request_id += 1;

            let _ = ServerPacket::from_session(&mut self.session);
        }
        Ok(())
    }

    fn stat(&mut self, path: &PathBuf) -> Result<FileAttributes, SftpError> {
        let path_str = path
            .to_str()
            .ok_or_else(|| SftpError::ClientError("Invalid UTF-8 in path".into()))?;

        let stat_packet = ClientPacket::Stat {
            request_id: self.session.next_request_id,
            path: path_str.to_string(),
        };
        self.session.send_packet(stat_packet)?;
        self.session.next_request_id += 1;

        let response = ServerPacket::from_session(&mut self.session)?;
        match response {
            ServerPacket::Attrs { attrs, .. } => Ok(attrs),
            ServerPacket::Status {
                status_code,
                request_id,
                message,
            } => Err(SftpError::ServerError {
                code: status_code,
                request_id,
                message,
            }),
            _ => Err(SftpError::UnexpectedPacket("Stat response")),
        }
    }
}
