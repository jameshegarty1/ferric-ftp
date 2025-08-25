use std::collections::HashMap;
use std::io::{Error, Read, Write};
use std::path::PathBuf;

use ssh2::{Channel, File};
use crate::sftp::SftpError::{ClientError, UnknownError};
use crate::util;

const SSH_FXP_INIT: u8 = 1;
const SSH_FXP_VERSION: u8 = 2;

const SSH_FXP_CLOSE: u8 = 4;
const SSH_FXP_OPENDIR: u8 = 11;
const SSH_FXP_READDIR: u8 = 12;
const SSH_FXP_HANDLE: u8 = 102;
const SSH_FXP_NAME: u8 = 104;
const SSH_FXP_STATUS: u8 = 101;

const SSH_FILEXFER_ATTR_SIZE: u32 = 0x00000001;

const SSH_FILEXFER_ATTR_UIDGID: u32 = 0x00000002;
const SSH_FILEXFER_ATTR_PERMISSIONS: u32 = 0x00000004;

const SSH_FILEXFER_ATTR_ACMODTIME: u32 = 0x00000008;
const SSH_FILEXFER_ATTR_EXTENDED: u32 = 0x80000000;

const ATTR_FLAGS: &[u32] = &[
    0x00000001, // Size
    0x00000002, // UIDGID
    0x00000004, // Permissions
    0x00000008, // ModifyTime
    0x80000000, // Extended
];

pub struct SftpClient {
    session: SftpSession,
}

pub struct SftpSession {
    channel: Channel,
    version: u32,
    working_dir: String,
    next_request_id: u32,
    handles: HashMap<String, Vec<u8>>,
}

#[derive(Debug)]
pub enum SftpError {
    ServerError { code: u32, message: String },
    ClientError(Box<dyn std::error::Error>),
    UnknownError { message: String },
}

#[derive(Debug)]
struct FileInfo {
    short_name: String,
    long_name: String,
    attrs: FileAttributes
}

#[derive(Debug, Default)]
pub struct FileAttributes {
    pub file_type: u8,
    pub size: Option<u64>,
    pub permissions: Option<u32>,
    pub modify_time: Option<u32>,
}

#[derive(Debug)]
pub enum SftpCommand {
    Ls { path: PathBuf },
    Cd { path: PathBuf },
    Get { remote_path: PathBuf, local_path: Option<PathBuf> },
    Put { remote_path: PathBuf, local_path: Option<PathBuf> },
    Pwd {},
    Exit,
}

pub struct CommandOutput {
    result: bool,
    message: String,
}

#[derive(Debug)]
pub enum SftpPacket {
    Init { version: u32 },
    OpenDir { request_id: u32, path: String },
    ReadDir { request_id: u32, handle: Vec<u8> },
    Close { request_id: u32, handle: Vec<u8> },

    Version { version: u32 },
    Handle { request_id: u32, handle: Vec<u8> },
    Name { request_id: u32, files: Vec<FileInfo> },
    Status { request_id: u32, error_code: u32, message: String },
}

#[repr(u8)]
#[derive(Debug)]
pub enum SftpStatus {
    Ok = 0, // SSH_FX_OK
    Eof = 1, // SSH_FX_EOF
    InvalidHandle = 4, // SSH_FX_INVALID_HANDLE
}

impl SftpSession {
    pub fn generate_client(mut channel: Channel, version: u32) -> Result<SftpClient, SftpError> {
        let init_packet = SftpPacket::Init { version };
        channel.write_all(&init_packet.to_bytes()).map_err(|e| SftpError::ClientError(e.into()))?;

        let mut session = SftpSession { channel: channel, version: version, working_dir: String::new(), next_request_id: 0, handles: HashMap::new() };

        match SftpPacket::from_session(&mut session)? {
            SftpPacket::Version { version } => Ok(SftpClient { session }),
            _ => Err(ClientError(std::io::Error::new(
                std::io::ErrorKind::Other,
                "SFTP error when creating SFTP session",
            ).into())),
        }
    }

    pub fn send_packet(&mut self, packet: SftpPacket) -> Result<(), std::io::Error> {
        self.channel.write_all(&packet.to_bytes())?;
        Ok(())
    }

    pub fn read_u32(&mut self) -> Result<u32, SftpError> {
        let mut buffer: [u8; 4] = [0; 4];
        self.channel.read_exact(&mut buffer).map_err(|e| ClientError(e.into()))?;
        Ok(u32::from_be_bytes(buffer))
    }

    pub fn read_u8(&mut self) -> Result<u8, SftpError> {
        let mut buffer: [u8; 1] = [0; 1];
        self.channel.read_exact(&mut buffer).map_err(|e| ClientError(e.into()))?;
        Ok(buffer[0])
    }

    pub fn read_string(&mut self) -> Result<Vec<u8>, SftpError> {
        let buffer_length = self.read_u32()? as usize;
        let mut buffer: Vec<u8> = vec![0; buffer_length];
        self.channel.read_exact(&mut buffer).map_err(|e| ClientError(e.into()))?;
        Ok(buffer)
    }

    pub fn read_i64(&mut self) -> Result<i64, SftpError> {
        let mut buffer: [u8; 8] = [0; 8];
        self.channel.read_exact(&mut buffer).map_err(|e| ClientError(e.into()))?;
        Ok(i64::from_be_bytes(buffer))
    }

    pub fn read_u64(&mut self) -> Result<u64, SftpError> {
        let mut buffer: [u8; 8] = [0; 8];
        self.channel.read_exact(&mut buffer).map_err(|e| ClientError(e.into()))?;
        Ok(u64::from_be_bytes(buffer))
    }

    pub fn discard(&mut self, bytes: &usize) -> Result<(), SftpError> {
        let mut buffer = vec![0; *bytes];
        self.channel.read_exact(&mut buffer).map_err(|e| ClientError(e.into()))?;
        Ok(())
    }

    pub fn parse_file_attributes(&mut self, flags: &u32) -> Result<(usize, FileAttributes), SftpError> {
        let mut attrs = FileAttributes::default();

        let mut len: usize = 0;

        if flags & SSH_FILEXFER_ATTR_SIZE != 0 {
            attrs.size = Some(self.read_u64()?);
            len += 8;
            println!("  Size: {:?}", attrs.size);
        }

        if flags & SSH_FILEXFER_ATTR_UIDGID != 0 {
            let uid = self.read_u32()?;
            len += 4;
            println!("  UID: {}", uid);
            let gid = self.read_u32()?;
            len += 4;
            println!("  GID: {}", gid);
        }

        if flags & SSH_FILEXFER_ATTR_PERMISSIONS != 0 {
            attrs.permissions = Some(self.read_u32()?);
            len += 4;
            println!("  Permissions: 0o{:o} (0x{:x})",
                     attrs.permissions.unwrap(), attrs.permissions.unwrap());
        }

        if flags & SSH_FILEXFER_ATTR_ACMODTIME != 0 {
            let atime = self.read_u32()?;
            len += 4;
            println!("  Access time: {}", atime);
            attrs.modify_time = Some(self.read_u32()?);
            len += 4;
            println!("  Modify time: {:?}", attrs.modify_time);
        }

        if flags & SSH_FILEXFER_ATTR_EXTENDED != 0 {
            let extended_count = self.read_u32()?;
            len += 4;
            println!("  Extended attributes count: {}", extended_count);

            for i in 0..extended_count {
                let name = self.read_string()?;
                let value = self.read_string()?;
                len += (8 + name.len() + value.len());
                println!("    Extended[{}]: {} = {}", i,
                         String::from_utf8_lossy(&name),
                         String::from_utf8_lossy(&value));
            }
        }

        println!("Total attributes length: {}", len);
        Ok((len, attrs))
    }

    // Add a debug method to see raw bytes
    pub fn debug_peek_bytes(&mut self, count: usize) -> Result<Vec<u8>, SftpError> {
        let mut buffer = vec![0u8; count];
        self.channel.read_exact(&mut buffer).map_err(|e| ClientError(e.into()))?;
        println!("Raw bytes: {:02x?}", buffer);
        Ok(buffer)
    }
}


impl SftpClient {
    pub fn execute_command(&mut self, cmd: &SftpCommand) -> Result<CommandOutput, SftpError> {
        match cmd {
            SftpCommand::Ls { path } => self.list_directory(&path),
            SftpCommand::Cd { path } => self.change_directory(&path),
            _ => { todo!() }
            // etc.
        }
    }

    fn list_directory(&mut self, path: &PathBuf) -> Result<(CommandOutput), SftpError> {
        let path_str = path.to_str()
            .ok_or_else(|| SftpError::ClientError("Invalid UTF-8 in path".into()))?;

        let opendir_packet = SftpPacket::OpenDir {
            request_id: self.session.next_request_id,
            path: path_str.to_string(),
        };

        self.session.send_packet(opendir_packet)
            .map_err(|e| SftpError::ClientError(Box::new(e)))?;

        self.session.next_request_id += 1;

        let opendir_response_packet = SftpPacket::from_session(&mut self.session)?;

        match opendir_response_packet {
            SftpPacket::Status { request_id, error_code, message } => {
                return Err(SftpError::UnknownError{message: format!(
                    "SFTP StatusMessage Error Code: {} Message: {}",
                    error_code, message
                )});
            }
            SftpPacket::Handle { request_id, handle } => {
                println!("Got a handle {:?} for path {}", handle, path.display());
                self.session.handles.insert(path_str.to_string(), handle.clone());
            }
            _ => {
                return Err(SftpError::UnknownError{ message: "Unexpected packet type".to_string()});
            }
        }

        let handle = self.session.handles.get(path_str)
            .ok_or_else(|| SftpError::UnknownError{ message: "No handle found for path".to_string()} )?.clone();

        let mut files = Vec::new();
        loop {
            let readdir_packet = SftpPacket::ReadDir {
                request_id: self.session.next_request_id,
                handle: handle.clone(),
            };

            self.session.send_packet(readdir_packet)
                .map_err(|e| SftpError::ClientError(Box::new(e)))?;

            self.session.next_request_id += 1;

            let readdir_response_packet = SftpPacket::from_session(&mut self.session)?;

            match readdir_response_packet {
                SftpPacket::Status { request_id, error_code, message } => {
                    if error_code == SftpStatus::Eof as u32 {
                        break;
                    } else {
                        return Err(SftpError::UnknownError{ message: format!(
                            "Unexpected Status message: {} - {}",
                            error_code, message
                        )});
                    }
                }
                SftpPacket::Name { request_id, files: names } => {
                    // This is likely what you want for directory listings
                    files.extend(names);
                }

                _ => {
                    return Err(SftpError::UnknownError { message: "Unexpected packet type".to_string() });
                }
            }
        }

        let close_packet = SftpPacket::Close {
            request_id: self.session.next_request_id,
            handle,
        };

        if let Err(e) = self.session.send_packet(close_packet) {
            println!("Warning: Failed to close handle: {:?}", e);
        }
        self.session.next_request_id += 1;

        self.session.handles.remove(path_str);

        Ok(CommandOutput {
            result: true,
            message: format!("Listed {} entries from {}", files.len(), path.display()),
        })
    }


    fn change_directory(&mut self, path: &PathBuf) -> Result<CommandOutput, SftpError> {
            todo!()
        }
    }

    impl SftpPacket {
        pub fn packet_type(&self) -> u8 {
            match self {
                SftpPacket::Init { .. } => SSH_FXP_INIT,
                SftpPacket::OpenDir { .. } => SSH_FXP_OPENDIR,
                SftpPacket::Version { .. } => SSH_FXP_VERSION,
                SftpPacket::Handle { .. } => SSH_FXP_HANDLE,
                SftpPacket::Name { .. } => SSH_FXP_NAME,
                SftpPacket::Status { .. } => SSH_FXP_STATUS,
                SftpPacket::ReadDir { .. } => SSH_FXP_READDIR,
                SftpPacket::Close { .. } => SSH_FXP_CLOSE,
            }
        }

        pub fn packet_name(&self) -> &'static str {
            match self {
                SftpPacket::Init { .. } => "SSH_FXP_INIT",
                SftpPacket::OpenDir { .. } => "SSH_FXP_OPENDIR",
                SftpPacket::Version { .. } => "SSH_FXP_VERSION",
                SftpPacket::Handle { .. } => "SSH_FXP_HANDLE",
                SftpPacket::Name { .. } => "SSH_FXP_NAME",
                SftpPacket::Status { .. } => "SSH_FXP_STATUS",
                SftpPacket::ReadDir { .. } => "SSH_FXP_READDIR",
                SftpPacket::Close { .. } => "SSH_FXP_CLOSE",
            }
        }

        pub fn to_bytes(&self) -> Vec<u8> {
            let mut payload: Vec<u8> = Vec::new();
            let mut packet: Vec<u8> = Vec::new();
            payload.push(self.packet_type());

            match self {
                SftpPacket::Init { version } => {
                    payload.extend_from_slice(&version.to_be_bytes());
                    let length = payload.len() as u32;
                    packet.extend_from_slice(&length.to_be_bytes());
                    packet.extend(payload);
                    packet
                },
                SftpPacket::OpenDir { request_id, path } => {
                    let mut payload: Vec<u8> = Vec::new();
                    let mut packet: Vec<u8> = Vec::new();
                    payload.push(SSH_FXP_OPENDIR);
                    payload.extend_from_slice(&request_id.to_be_bytes());

                    payload.extend_from_slice(&path.len().to_be_bytes());
                    payload.extend_from_slice(path.as_bytes());

                    let length = payload.len() as u32;
                    packet.extend_from_slice(&length.to_be_bytes());
                    packet.extend(payload);

                    packet
                },
                SftpPacket::ReadDir { request_id, handle } => {
                    let mut payload: Vec<u8> = Vec::new();
                    let mut packet: Vec<u8> = Vec::new();
                    payload.push(SSH_FXP_READDIR);
                    payload.extend_from_slice(&request_id.to_be_bytes());

                    payload.extend_from_slice(&(handle.len() as u32).to_be_bytes());
                    payload.extend_from_slice(handle);

                    let length = payload.len() as u32;
                    packet.extend_from_slice(&length.to_be_bytes());
                    packet.extend(payload);

                    packet
                },
                SftpPacket::Close { request_id, handle } => {
                    let mut payload: Vec<u8> = Vec::new();
                    let mut packet: Vec<u8> = Vec::new();
                    payload.push(SSH_FXP_CLOSE);
                    payload.extend_from_slice(&request_id.to_be_bytes());
                    payload.extend_from_slice(&(handle.len() as u32).to_be_bytes());
                    payload.extend_from_slice(handle);

                    let length = payload.len() as u32;
                    packet.extend_from_slice(&length.to_be_bytes());
                    packet.extend(payload);
                    packet
                }
                _ => {
                    todo!();
                    packet
                }
            }
        }

        pub fn from_session(session: &mut SftpSession) -> Result<Self, SftpError> {
            let message_length = session.read_u32()?;
            let mut remaining_bytes = message_length as usize;

            let message_type = session.read_u8()?;
            remaining_bytes -= 1;

            match message_type {
                SSH_FXP_VERSION => {
                    let version = session.read_u32()?;
                    remaining_bytes -= 4;
                    session.discard(&remaining_bytes)?;

                    Ok(SftpPacket::Version { version })
                }
                SSH_FXP_HANDLE => {
                    let request_id = session.read_u32()?;
                    remaining_bytes -= 4;
                    println!("Handle Response to request_id: {}", request_id);
                    let handle = session.read_string()?;
                    remaining_bytes -= ( 4 + handle.len());

                    Ok(SftpPacket::Handle { request_id, handle })
                }
                SSH_FXP_NAME => {
                    let request_id = session.read_u32()?;
                    remaining_bytes -= 4;

                    let count = session.read_u32()?;
                    remaining_bytes -= 4;

                    let mut files: Vec<FileInfo> = Vec::new();
                    for _ in 0..count {
                        let short_name = session.read_string()?;
                        let long_name = session.read_string()?;
                        println!("Short name: {}", String::from_utf8_lossy(&short_name));

                        remaining_bytes -= ( 8 + short_name.len() + long_name.len());

                        let attr_flags = session.read_u32()?;
                        remaining_bytes -= 4;

                        let (attrs_length, attrs): (usize, FileAttributes) = session.parse_file_attributes(&attr_flags)?;
                        remaining_bytes -= attrs_length;

                        let file = FileInfo {
                            short_name: String::from_utf8(short_name).map_err(|e| SftpError::ClientError(e.into()))?,
                            long_name: String::from_utf8(long_name).map_err(|e| SftpError::ClientError(e.into()))?,
                            attrs,
                        };

                        files.push(file);
                    }

                    if remaining_bytes > 0 {
                        session.discard(&remaining_bytes)?;
                    }

                    Ok(SftpPacket::Name { request_id, files })
                }
                SSH_FXP_STATUS => {
                    let request_id = session.read_u32()?;
                    remaining_bytes -= 4;

                    println!("Status Response to request_id: {}", request_id);
                    let error_code = session.read_u32()?;
                    remaining_bytes -= 4;

                    let message = String::from_utf8(session.read_string()?).map_err(|e| SftpError::ClientError(e.into()))?;

                    remaining_bytes -= (1 + message.len());

                    let lang = session.read_string()?;

                    remaining_bytes -= (1 + lang.len());

                    Ok(SftpPacket::Status { request_id, error_code, message  })
                }
                _ => {
                    Err(SftpError::ClientError(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        format!("Unknown message type: {}", message_type),
                    ).into()))
                }
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn test_init_packet_format() {
            let packet = create_init_packet();
            assert_eq!(packet.len(), 9); // 4 bytes length + 1 byte type + 4 bytes version
            assert_eq!(packet[4], 1); // SSH_FXP_INIT type
        }
    }