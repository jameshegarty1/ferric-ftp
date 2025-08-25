use super::constants::*;
use super::error::SftpError;
use super::types::{FileAttributes, FileInfo, SftpStatus};
use super::session::SftpSession;
use log::info;

#[derive(Debug)]
pub enum SftpPacket {
    Init {
        version: u32,
    },
    OpenDir {
        request_id: u32,
        path: String,
    },
    ReadDir {
        request_id: u32,
        handle: Vec<u8>,
    },
    Close {
        request_id: u32,
        handle: Vec<u8>,
    },
    Version {
        version: u32,
    },
    Handle {
        request_id: u32,
        handle: Vec<u8>,
    },
    Name {
        request_id: u32,
        files: Vec<FileInfo>,
    },
    Status {
        request_id: u32,
        error_code: u32,
        message: String,
    },
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
            }
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
            }
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
            }
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
            _ => packet,
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
                info!("Handle Response to request_id: {}", request_id);
                let handle = session.read_string()?;
                remaining_bytes -= 4 + handle.len();

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
                    info!("Short name: {}", String::from_utf8_lossy(&short_name));

                    remaining_bytes -= 8 + short_name.len() + long_name.len();

                    let attr_flags = session.read_u32()?;
                    remaining_bytes -= 4;

                    let (attrs_length, attrs): (usize, FileAttributes) =
                        session.parse_file_attributes(&attr_flags)?;
                    remaining_bytes -= attrs_length;

                    let file = FileInfo {
                        short_name: String::from_utf8(short_name)
                            .map_err(|e| SftpError::ClientError(e.into()))?,
                        long_name: String::from_utf8(long_name)
                            .map_err(|e| SftpError::ClientError(e.into()))?,
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

                info!("Status Response to request_id: {}", request_id);
                let error_code = session.read_u32()?;
                remaining_bytes -= 4;

                let message = String::from_utf8(session.read_string()?)
                    .map_err(|e| SftpError::ClientError(e.into()))?;

                remaining_bytes -= 1 + message.len();

                let lang = session.read_string()?;

                remaining_bytes -= 1 + lang.len();

                Ok(SftpPacket::Status {
                    request_id,
                    error_code,
                    message,
                })
            }
            _ => Err(SftpError::ClientError(
                std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("Unknown message type: {}", message_type),
                )
                    .into(),
            )),
        }
    }
}