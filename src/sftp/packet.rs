use super::constants::*;
use super::error::SftpError;
use super::session::SftpSession;
use super::types::{FileAttributes, FileInfo, FileType};
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

pub trait SftpReader {
    fn read_u32(&mut self) -> Result<u32, SftpError>;
    fn read_u8(&mut self) -> Result<u8, SftpError>;
    fn read_string(&mut self) -> Result<Vec<u8>, SftpError>;
    fn read_i64(&mut self) -> Result<i64, SftpError>;
    fn read_u64(&mut self) -> Result<u64, SftpError>;
    fn discard(&mut self, bytes: &usize) -> Result<(), SftpError>;
    fn parse_file_attributes(&mut self, flags: &u32) -> Result<(usize, FileAttributes), SftpError>;
}

impl SftpReader for SftpSession {
    fn read_u32(&mut self) -> Result<u32, SftpError> {
        self.read_u32()
    }

    fn read_u8(&mut self) -> Result<u8, SftpError> {
        self.read_u8()
    }

    fn read_string(&mut self) -> Result<Vec<u8>, SftpError> {
        self.read_string()
    }

    fn read_i64(&mut self) -> Result<i64, SftpError> {
        self.read_i64()
    }

    fn read_u64(&mut self) -> Result<u64, SftpError> {
        self.read_u64()
    }

    fn discard(&mut self, bytes: &usize) -> Result<(), SftpError> {
        self.discard(bytes)
    }

    fn parse_file_attributes(&mut self, flags: &u32) -> Result<(usize, FileAttributes), SftpError> {
        self.parse_file_attributes(flags)
    }
}

pub struct BufferReader<'a> {
    data: &'a [u8],
    position: usize,
}

impl<'a> BufferReader<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self { data, position: 0 }
    }
}

impl<'a> SftpReader for BufferReader<'a> {
    fn read_u32(&mut self) -> Result<u32, SftpError> {
        if self.position + 4 > self.data.len() {
            return Err(SftpError::ClientError(
                std::io::Error::new(std::io::ErrorKind::UnexpectedEof, "Not enough data for u32")
                    .into(),
            ));
        }
        let bytes = [
            self.data[self.position],
            self.data[self.position + 1],
            self.data[self.position + 2],
            self.data[self.position + 3],
        ];
        self.position += 4;
        Ok(u32::from_be_bytes(bytes))
    }

    fn read_u8(&mut self) -> Result<u8, SftpError> {
        if self.position >= self.data.len() {
            return Err(SftpError::ClientError(
                std::io::Error::new(std::io::ErrorKind::UnexpectedEof, "Not enough data for u8")
                    .into(),
            ));
        }
        let byte = self.data[self.position];
        self.position += 1;
        Ok(byte)
    }

    fn read_string(&mut self) -> Result<Vec<u8>, SftpError> {
        let len = self.read_u32()? as usize;
        if self.position + len > self.data.len() {
            return Err(SftpError::ClientError(
                std::io::Error::new(
                    std::io::ErrorKind::UnexpectedEof,
                    "Not enough data for string",
                )
                .into(),
            ));
        }
        let result = self.data[self.position..self.position + len].to_vec();
        self.position += len;
        Ok(result)
    }

    fn read_i64(&mut self) -> Result<i64, SftpError> {
        if self.position + 8 > self.data.len() {
            return Err(SftpError::ClientError(
                std::io::Error::new(std::io::ErrorKind::UnexpectedEof, "Not enough data for i64")
                    .into(),
            ));
        }
        let bytes = [
            self.data[self.position],
            self.data[self.position + 1],
            self.data[self.position + 2],
            self.data[self.position + 3],
            self.data[self.position + 4],
            self.data[self.position + 5],
            self.data[self.position + 6],
            self.data[self.position + 7],
        ];
        self.position += 8;
        Ok(i64::from_be_bytes(bytes))
    }

    fn read_u64(&mut self) -> Result<u64, SftpError> {
        if self.position + 8 > self.data.len() {
            return Err(SftpError::ClientError(
                std::io::Error::new(std::io::ErrorKind::UnexpectedEof, "Not enough data for u64")
                    .into(),
            ));
        }
        let bytes = [
            self.data[self.position],
            self.data[self.position + 1],
            self.data[self.position + 2],
            self.data[self.position + 3],
            self.data[self.position + 4],
            self.data[self.position + 5],
            self.data[self.position + 6],
            self.data[self.position + 7],
        ];
        self.position += 8;
        Ok(u64::from_be_bytes(bytes))
    }

    fn discard(&mut self, bytes: &usize) -> Result<(), SftpError> {
        if self.position + bytes > self.data.len() {
            return Err(SftpError::ClientError(
                std::io::Error::new(
                    std::io::ErrorKind::UnexpectedEof,
                    "Not enough data to discard",
                )
                .into(),
            ));
        }
        self.position += bytes;
        Ok(())
    }

    fn parse_file_attributes(&mut self, flags: &u32) -> Result<(usize, FileAttributes), SftpError> {
        let mut attrs = FileAttributes::default();
        let mut len: usize = 0;

        if flags & SSH_FILEXFER_ATTR_SIZE != 0 {
            attrs.size = Some(self.read_u64()?);
            len += 8;
        }

        if flags & SSH_FILEXFER_ATTR_UIDGID != 0 {
            let _uid = self.read_u32()?;
            let _gid = self.read_u32()?;
            len += 8;
        }

        if flags & SSH_FILEXFER_ATTR_PERMISSIONS != 0 {
            attrs.permissions = Some(self.read_u32()?);
            len += 4;
        }

        if flags & SSH_FILEXFER_ATTR_ACMODTIME != 0 {
            let _atime = self.read_u32()?;
            attrs.modify_time = Some(self.read_u32()?);
            len += 8;
        }

        if flags & SSH_FILEXFER_ATTR_EXTENDED != 0 {
            let extended_count = self.read_u32()?;
            len += 4;
            for _ in 0..extended_count {
                let _name = self.read_string()?;
                let _value = self.read_string()?;
                len += 8 + _name.len() + _value.len();
            }
        }

        Ok((len, attrs))
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
    pub fn from_bytes(data: &[u8]) -> Result<Self, SftpError> {
        let mut reader = BufferReader::new(data);
        Self::from_reader(&mut reader)
    }

    pub fn from_reader<R: SftpReader>(reader: &mut R) -> Result<Self, SftpError> {
        let message_length = reader.read_u32()? as usize;
        let message_type = reader.read_u8()?;
        let mut remaining_bytes = message_length - 1;

        match message_type {
            SSH_FXP_VERSION => {
                let version = reader.read_u32()?;
                remaining_bytes -= 4;
                reader.discard(&remaining_bytes)?;
                Ok(ServerPacket::Version { version })
            }
            SSH_FXP_HANDLE => {
                let request_id = reader.read_u32()?;
                remaining_bytes -= 4;
                let handle = reader.read_string()?;
                remaining_bytes -= 4 + handle.len();
                Ok(ServerPacket::Handle { request_id, handle })
            }
            SSH_FXP_NAME => {
                let request_id = reader.read_u32()?;
                remaining_bytes -= 4;

                let count = reader.read_u32()?;
                remaining_bytes -= 4;

                let mut files: Vec<FileInfo> = Vec::new();
                for _ in 0..count {
                    let name = reader.read_string()?;
                    let display_name = reader.read_string()?;
                    remaining_bytes -= 8 + name.len() + display_name.len();

                    let attr_flags = reader.read_u32()?;
                    remaining_bytes -= 4;

                    let (attrs_length, attrs) = reader.parse_file_attributes(&attr_flags)?;
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
                    reader.discard(&remaining_bytes)?;
                }

                Ok(ServerPacket::Name { request_id, files })
            }

            SSH_FXP_STATUS => {
                let request_id = reader.read_u32()?;
                remaining_bytes -= 4;

                let status_code = reader.read_u32()?;

                info!(
                    "Status Response to request_id: {} with code: {}",
                    request_id, status_code
                );
                remaining_bytes -= 4;

                let message = String::from_utf8(reader.read_string()?)
                    .map_err(|e| SftpError::ClientError(e.into()))?;

                remaining_bytes -= 1 + message.len();

                let lang = reader.read_string()?;

                remaining_bytes -= 1 + lang.len();

                Ok(ServerPacket::Status {
                    request_id,
                    status_code,
                    message,
                })
            }
            SSH_FXP_ATTRS => {
                let request_id = reader.read_u32()?;
                remaining_bytes -= 4;

                let attr_flags = reader.read_u32()?;
                remaining_bytes -= 4;

                let (attrs_length, attrs): (usize, FileAttributes) =
                    reader.parse_file_attributes(&attr_flags)?;
                remaining_bytes -= attrs_length;

                Ok(ServerPacket::Attrs { request_id, attrs })
            }
            // ... other packet types (copy from your existing from_session)
            _ => Err(SftpError::ClientError(
                std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("Unknown message type: {}", message_type),
                )
                .into(),
            )),
        }
    }
    pub fn from_session(session: &mut SftpSession) -> Result<Self, SftpError> {
        Self::from_reader(session)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sftp::test_utils::MockSession;

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

    fn create_test_attrs() -> FileAttributes {
        FileAttributes {
            size: Some(1024),
            permissions: Some(0o755),
            modify_time: Some(1234567890),
            file_type: FileType::RegularFile,
            is_directory: false,
            is_regular_file: true,
            is_symlink: false,
        }
    }

    fn create_test_file_info() -> FileInfo {
        FileInfo {
            name: "test.txt".to_string(),
            display_name: "test.txt".to_string(),
            attrs: create_test_attrs(),
        }
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

    #[test]
    fn test_server_packet_version() {
        let data = vec![
            0,
            0,
            0,
            5,               // length = 5
            SSH_FXP_VERSION, // packet type
            0,
            0,
            0,
            3, // version = 3
        ];

        let packet = ServerPacket::from_bytes(&data).unwrap();
        assert!(matches!(packet, ServerPacket::Version { version: 3 }));
    }
    #[test]
    fn test_server_packet_handle() {
        let data = vec![
            0,
            0,
            0,
            13, // length = 13
            SSH_FXP_HANDLE,
            0,
            0,
            0,
            1, // request id = 1
            0,
            0,
            0,
            3, // handle length
            0x01,
            0x02,
            0x03, // handle
        ];

        let packet = ServerPacket::from_bytes(&data).unwrap();
        if let ServerPacket::Handle { request_id, handle } = packet {
            assert_eq!(request_id, 1);
            assert_eq!(handle, vec![0x01, 0x02, 0x03]);
        } else {
            panic!("Expected Handle packet");
        }
    }
}
