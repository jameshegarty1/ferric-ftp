use super::constants::*;
use super::error::SftpError;
use super::packet::ClientPacket;
use super::packet::ServerPacket;
use super::session::TransportLayer;
use super::types::{DirectoryCache, FileAttributes, FileInfo, SftpCommand, SftpStatus};
use crate::sftp::protocol::SftpProtocol;
use log::info;
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::SystemTime;

pub struct SftpClient<T: TransportLayer> {
    protocol: SftpProtocol<T>,
    pub working_dir: PathBuf,
    pub directory_cache: HashMap<PathBuf, DirectoryCache>,
    pub current_listing: Vec<FileInfo>,
    pub handles: HashMap<String, Vec<u8>>,
}

impl<T: TransportLayer> SftpClient<T> {
    pub fn new(transport: T, initial_path: Option<&str>) -> Result<Self, SftpError> {
        let mut protocol = SftpProtocol::new(transport);
        let working_dir = PathBuf::from(protocol.realpath(initial_path.unwrap_or("/"))?);

        Ok(Self {
            protocol,
            working_dir,
            directory_cache: HashMap::new(),
            current_listing: Vec::new(),
            handles: HashMap::new(),
        })
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
        info!("Executing command: {:?}", cmd);
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

        let handle = self.protocol.open_dir(path_str)?;
        let files = self.read_entire_directory(&handle)?;
        self.protocol.close(handle)?;
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

    fn read_entire_directory(&mut self, handle: &[u8]) -> Result<Vec<FileInfo>, SftpError> {
        let mut all_files = Vec::new();

        loop {
            let files = self.protocol.read_dir(handle)?;
            if files.is_empty() {
                break;
            }
            all_files.extend(files);
        }

        Ok(all_files)
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

        let resolved_path = self.protocol.realpath(path_str)?;

        let attrs = self.protocol.stat(&resolved_path)?;
        if !attrs.is_directory {
            return Err(SftpError::NotADirectory(resolved_path));
        }

        self.working_dir = PathBuf::from(resolved_path);
        self.current_listing.clear();
        Ok(())
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

    /*
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
    */
}
