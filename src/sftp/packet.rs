use super::constants::*;
use super::error::SftpError;
use super::session::SftpSession;
use super::types::{FileAttributes, FileInfo};
use log::info;

pub trait SftpPacketInfo {
    fn packet_type(&self) -> u8;
    fn packet_name(&self) -> &'static str;
}

#[derive(Debug)]
pub enum ClientPacket {
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
    RealPath {
        request_id: u32,
        path: String,
    },
    Stat {
        request_id: u32,
        path: String,
    },
    Open {
        request_id: u32,
        path: String,
        pflags: u32,
        attrs: FileAttributes,
    },
}

#[derive(Debug)]
pub enum ServerPacket {
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
        status_code: u32,
        message: String,
    },
    Attrs {
        request_id: u32,
        attrs: FileAttributes,
    },
}

impl SftpPacketInfo for ClientPacket {
    fn packet_type(&self) -> u8 {
        match self {
            ClientPacket::Init { .. } => SSH_FXP_INIT,
            ClientPacket::OpenDir { .. } => SSH_FXP_OPENDIR,
            ClientPacket::ReadDir { .. } => SSH_FXP_READDIR,
            ClientPacket::Close { .. } => SSH_FXP_CLOSE,
            ClientPacket::RealPath { .. } => SSH_FXP_REALPATH,
            ClientPacket::Stat { .. } => SSH_FXP_STAT,
            ClientPacket::Open { .. } => SSH_FXP_OPEN,
        }
    }

    fn packet_name(&self) -> &'static str {
        match self {
            ClientPacket::Init { .. } => "SSH_FXP_INIT",
            ClientPacket::OpenDir { .. } => "SSH_FXP_OPENDIR",
            ClientPacket::ReadDir { .. } => "SSH_FXP_READDIR",
            ClientPacket::Close { .. } => "SSH_FXP_CLOSE",
            ClientPacket::RealPath { .. } => "SSH_FXP_REALPATH",
            ClientPacket::Stat { .. } => "SSH_FXP_STAT",
            ClientPacket::Open { .. } => "SSH_FXP_OPEN",
        }
    }
}

impl SftpPacketInfo for ServerPacket {
    fn packet_type(&self) -> u8 {
        match self {
            ServerPacket::Version { .. } => SSH_FXP_VERSION,
            ServerPacket::Handle { .. } => SSH_FXP_HANDLE,
            ServerPacket::Name { .. } => SSH_FXP_NAME,
            ServerPacket::Status { .. } => SSH_FXP_STATUS,
            ServerPacket::Attrs { .. } => SSH_FXP_ATTRS,
        }
    }

    fn packet_name(&self) -> &'static str {
        match self {
            ServerPacket::Version { .. } => "SSH_FXP_VERSION",
            ServerPacket::Handle { .. } => "SSH_FXP_HANDLE",
            ServerPacket::Name { .. } => "SSH_FXP_NAME",
            ServerPacket::Status { .. } => "SSH_FXP_STATUS",
            ServerPacket::Attrs { .. } => "SSH_FXP_ATTRS",
        }
    }
}

impl ClientPacket {
    fn add_header(&self, payload: Vec<u8>) -> Vec<u8> {
        let mut packet: Vec<u8> = Vec::new();
        let length = payload.len() as u32;
        packet.extend_from_slice(&length.to_be_bytes());
        packet.extend(payload);
        packet
    }

    fn add_u32(&self, payload: &mut Vec<u8>, num: &u32) {
        payload.extend_from_slice(&num.to_be_bytes());
    }

    fn add_string(&self, payload: &mut Vec<u8>, string: &str) {
        payload.extend_from_slice(&(string.len() as u32).to_be_bytes());
        payload.extend_from_slice(string.as_bytes());
    }

    fn add_bytes(&self, payload: &mut Vec<u8>, bytes: &[u8]) {
        payload.extend_from_slice(&(bytes.len() as u32).to_be_bytes());
        payload.extend_from_slice(bytes);
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut payload: Vec<u8> = Vec::new();

        payload.push(self.packet_type());

        match self {
            ClientPacket::Init { version } => {
                payload.extend_from_slice(&version.to_be_bytes());
            }
            ClientPacket::OpenDir { request_id, path } => {
                self.add_u32(&mut payload, request_id);
                self.add_string(&mut payload, path);
            }
            ClientPacket::ReadDir { request_id, handle } => {
                self.add_u32(&mut payload, request_id);
                self.add_bytes(&mut payload, handle);
            }
            ClientPacket::Close { request_id, handle } => {
                self.add_u32(&mut payload, request_id);
                self.add_bytes(&mut payload, handle);
            }
            ClientPacket::RealPath { request_id, path } => {
                self.add_u32(&mut payload, request_id);
                self.add_string(&mut payload, path);
            }
            ClientPacket::Stat { request_id, path } => {
                self.add_u32(&mut payload, request_id);
                self.add_string(&mut payload, path);
            }
            ClientPacket::Open {
                request_id,
                path,
                pflags,
                attrs,
            } => {
                self.add_u32(&mut payload, request_id);
                self.add_string(&mut payload, path);
                self.add_u32(&mut payload, pflags);
                //Implement attrs here
                //
                //
            }
        }
        self.add_header(payload)
    }
}

impl ServerPacket {
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

                Ok(ServerPacket::Version { version })
            }
            SSH_FXP_HANDLE => {
                let request_id = session.read_u32()?;
                remaining_bytes -= 4;
                let handle = session.read_string()?;
                remaining_bytes -= 4 + handle.len();

                Ok(ServerPacket::Handle { request_id, handle })
            }
            SSH_FXP_NAME => {
                let request_id = session.read_u32()?;
                remaining_bytes -= 4;

                let count = session.read_u32()?;
                remaining_bytes -= 4;

                let mut files: Vec<FileInfo> = Vec::new();
                for _ in 0..count {
                    let name = session.read_string()?;
                    let display_name = session.read_string()?;

                    remaining_bytes -= 8 + name.len() + display_name.len();

                    let attr_flags = session.read_u32()?;
                    remaining_bytes -= 4;

                    let (attrs_length, attrs): (usize, FileAttributes) =
                        session.parse_file_attributes(&attr_flags)?;
                    remaining_bytes -= attrs_length;

                    let file = FileInfo {
                        name: String::from_utf8(name)
                            .map_err(|e| SftpError::ClientError(e.into()))?,
                        display_name: String::from_utf8(display_name)
                            .map_err(|e| SftpError::ClientError(e.into()))?,
                        attrs,
                    };

                    files.push(file);
                }

                if remaining_bytes > 0 {
                    session.discard(&remaining_bytes)?;
                }

                Ok(ServerPacket::Name { request_id, files })
            }
            SSH_FXP_STATUS => {
                let request_id = session.read_u32()?;
                remaining_bytes -= 4;

                let status_code = session.read_u32()?;

                remaining_bytes -= 4;

                let message = String::from_utf8(session.read_string()?)
                    .map_err(|e| SftpError::ClientError(e.into()))?;

                info!(
                    "Status Response to request_id: {} with code: {} and message: {}",
                    request_id, status_code, message
                );

                remaining_bytes -= 1 + message.len();

                let lang = session.read_string()?;

                remaining_bytes -= 1 + lang.len();

                Ok(ServerPacket::Status {
                    request_id,
                    status_code,
                    message,
                })
            }
            SSH_FXP_ATTRS => {
                let request_id = session.read_u32()?;
                remaining_bytes -= 4;

                let attr_flags = session.read_u32()?;
                remaining_bytes -= 4;

                let (attrs_length, attrs): (usize, FileAttributes) =
                    session.parse_file_attributes(&attr_flags)?;
                remaining_bytes -= attrs_length;

                Ok(ServerPacket::Attrs { request_id, attrs })
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

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_packet_length(bytes: &[u8], expected_payload_length: usize) {
        let length = u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        assert_eq!(length as usize, expected_payload_length);
        assert_eq!(length as usize, bytes[4..].len());
    }

    fn assert_packet_type(bytes: &[u8], expected_type: u8) {
        assert_eq!(bytes[4], expected_type);
    }

    fn assert_request_id(bytes: &[u8], expected_id: u32) {
        let request_id = u32::from_be_bytes([bytes[5], bytes[6], bytes[7], bytes[8]]);
        assert_eq!(request_id, expected_id);
    }

    fn assert_string_field(bytes: &[u8], start_index: usize, expected_value: &str) {
        let len_bytes = start_index;
        let data_start = start_index + 4;

        let field_len = u32::from_be_bytes([
            bytes[len_bytes],
            bytes[len_bytes + 1],
            bytes[len_bytes + 2],
            bytes[len_bytes + 3],
        ]) as usize;

        let field_data = &bytes[data_start..data_start + field_len];
        let field_string = String::from_utf8(field_data.to_vec()).unwrap();
        assert_eq!(field_string, expected_value);
    }

    fn assert_bytes_field(bytes: &[u8], start_index: usize, expected_value: &[u8]) {
        let len_bytes = start_index;
        let data_start = start_index + 4;

        let field_len = u32::from_be_bytes([
            bytes[len_bytes],
            bytes[len_bytes + 1],
            bytes[len_bytes + 2],
            bytes[len_bytes + 3],
        ]) as usize;

        let field_data = &bytes[data_start..data_start + field_len];
        assert_eq!(field_data, expected_value);
    }

    #[test]
    fn test_client_packet_info() {
        let init = ClientPacket::Init { version: 3 };
        assert_eq!(init.packet_type(), SSH_FXP_INIT);
        assert_eq!(init.packet_name(), "SSH_FXP_INIT");

        let opendir = ClientPacket::OpenDir {
            request_id: 1,
            path: "/".to_string(),
        };
        assert_eq!(opendir.packet_type(), SSH_FXP_OPENDIR);
        assert_eq!(opendir.packet_name(), "SSH_FXP_OPENDIR");
    }

    #[test]
    fn test_server_packet_info() {
        let version = ServerPacket::Version { version: 3 };
        assert_eq!(version.packet_type(), SSH_FXP_VERSION);
        assert_eq!(version.packet_name(), "SSH_FXP_VERSION");

        let handle = ServerPacket::Handle {
            request_id: 1,
            handle: vec![1, 2, 3],
        };
        assert_eq!(handle.packet_type(), SSH_FXP_HANDLE);
        assert_eq!(handle.packet_name(), "SSH_FXP_HANDLE");
    }

    #[test]
    fn test_client_packet_init() {
        let init = ClientPacket::Init { version: 3 };
        let bytes = init.to_bytes();

        assert_packet_length(&bytes, 5);
        assert_packet_type(&bytes, SSH_FXP_INIT);
    }

    #[test]
    fn test_client_packet_opendir() {
        let opendir = ClientPacket::OpenDir {
            request_id: 100,
            path: "/home".to_string(),
        };
        let bytes = opendir.to_bytes();

        assert_packet_length(&bytes, 14);
        assert_packet_type(&bytes, SSH_FXP_OPENDIR);
        assert_request_id(&bytes, 100);
        assert_string_field(&bytes, 9, "/home");
    }

    #[test]
    fn test_client_packet_readdir() {
        let handle = vec![0x01, 0x02, 0x03];
        let readdir = ClientPacket::ReadDir {
            request_id: 100,
            handle: handle.clone(),
        };
        let bytes = readdir.to_bytes();

        assert_packet_length(&bytes, 12); // 1 + 4 + 4 + 3
        assert_packet_type(&bytes, SSH_FXP_READDIR);
        assert_request_id(&bytes, 100);
        assert_bytes_field(&bytes, 9, &handle);
    }

    #[test]
    fn test_client_packet_close() {
        let handle = vec![0x01, 0x02, 0x03];
        let close = ClientPacket::Close {
            request_id: 100,
            handle: handle.clone(),
        };
        let bytes = close.to_bytes();

        assert_packet_length(&bytes, 12);
        assert_packet_type(&bytes, SSH_FXP_CLOSE);
        assert_request_id(&bytes, 100);
        assert_bytes_field(&bytes, 9, &handle);
    }

    #[test]
    fn test_client_packet_realpath() {
        let realpath = ClientPacket::RealPath {
            request_id: 100,
            path: "/home".to_string(),
        };
        let bytes = realpath.to_bytes();

        assert_packet_length(&bytes, 14); // 1 + 4 + 4 + 5
        assert_packet_type(&bytes, SSH_FXP_REALPATH);
        assert_request_id(&bytes, 100);
        assert_string_field(&bytes, 9, "/home");
    }

    #[test]
    fn test_client_packet_stat() {
        let opendir = ClientPacket::Stat {
            request_id: 100,
            path: "/home".to_string(),
        };
        let bytes = opendir.to_bytes();

        assert_packet_length(&bytes, 14);
        assert_packet_type(&bytes, SSH_FXP_STAT);
        assert_request_id(&bytes, 100);
        assert_string_field(&bytes, 9, "/home");
    }

    //#[test]
    //fn test_client_packet_open() {
    //    todo!();
    //}
}
