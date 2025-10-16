use super::constants::*;
use super::error::SftpError;
use super::packet::{ClientPacket, ServerPacket};
use super::types::{FileAttributes, FileType};
use log::info;
use ssh2::Channel;
use std::io::{Read, Write};

pub struct SftpSession {
    pub channel: Channel,
    //pub version: u32,
    pub next_request_id: u32,
}

pub trait TransportLayer: Send {
    fn send_packet(&mut self, packet: ClientPacket) -> Result<(), SftpError>;
    fn receive_packet(&mut self) -> Result<ServerPacket, SftpError>;
    fn next_request_id(&mut self) -> u32;
}

impl TransportLayer for SftpSession {
    fn send_packet(&mut self, packet: ClientPacket) -> Result<(), SftpError> {
        self.send_packet(packet)
    }

    fn receive_packet(&mut self) -> Result<ServerPacket, SftpError> {
        ServerPacket::from_session(self)
    }

    fn next_request_id(&mut self) -> u32 {
        let id = self.next_request_id;
        self.next_request_id += 1;
        id
    }
}

impl SftpSession {
    pub fn new(mut channel: Channel, version: u32) -> Result<Self, SftpError> {
        let init_packet = ClientPacket::Init { version };
        channel
            .write_all(&init_packet.to_bytes())
            .map_err(|e| SftpError::ClientError(e.into()))?;

        let mut session = Self {
            channel,
            //version,
            next_request_id: 0,
        };
        match ServerPacket::from_session(&mut session)? {
            ServerPacket::Version { version: _ } => Ok(session),
            _ => Err(SftpError::ClientError(
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "Error when creating SFTP session",
                )
                .into(),
            )),
        }
    }

    pub fn send_packet(&mut self, packet: ClientPacket) -> Result<(), SftpError> {
        self.channel
            .write_all(&packet.to_bytes())
            .map_err(|e| SftpError::IoError(e))?;
        self.channel.flush().map_err(|e| SftpError::IoError(e))?;
        Ok(())
    }

    pub fn read_u32(&mut self) -> Result<u32, SftpError> {
        let mut buffer: [u8; 4] = [0; 4];
        self.channel
            .read_exact(&mut buffer)
            .map_err(|e| SftpError::ClientError(e.into()))?;
        Ok(u32::from_be_bytes(buffer))
    }

    pub fn read_u8(&mut self) -> Result<u8, SftpError> {
        let mut buffer: [u8; 1] = [0; 1];
        self.channel
            .read_exact(&mut buffer)
            .map_err(|e| SftpError::ClientError(e.into()))?;
        Ok(buffer[0])
    }

    pub fn read_string(&mut self) -> Result<Vec<u8>, SftpError> {
        let buffer_length = self.read_u32()? as usize;
        let mut buffer: Vec<u8> = vec![0; buffer_length];
        self.channel
            .read_exact(&mut buffer)
            .map_err(|e| SftpError::ClientError(e.into()))?;
        Ok(buffer)
    }

    pub fn read_i64(&mut self) -> Result<i64, SftpError> {
        let mut buffer: [u8; 8] = [0; 8];
        self.channel
            .read_exact(&mut buffer)
            .map_err(|e| SftpError::ClientError(e.into()))?;
        Ok(i64::from_be_bytes(buffer))
    }

    pub fn read_u64(&mut self) -> Result<u64, SftpError> {
        let mut buffer: [u8; 8] = [0; 8];
        self.channel
            .read_exact(&mut buffer)
            .map_err(|e| SftpError::ClientError(e.into()))?;
        Ok(u64::from_be_bytes(buffer))
    }

    pub fn discard(&mut self, bytes: &usize) -> Result<(), SftpError> {
        let mut buffer = vec![0; *bytes];
        self.channel
            .read_exact(&mut buffer)
            .map_err(|e| SftpError::ClientError(e.into()))?;
        Ok(())
    }

    pub fn parse_file_attributes(
        &mut self,
        flags: &u32,
    ) -> Result<(usize, FileAttributes), SftpError> {
        let mut attrs = FileAttributes::default();

        let mut len: usize = 0;

        if flags & SSH_FILEXFER_ATTR_SIZE != 0 {
            attrs.size = Some(self.read_u64()?);
            len += 8;
        }

        if flags & SSH_FILEXFER_ATTR_UIDGID != 0 {
            self.read_u32()?; // uid
            len += 4;
            self.read_u32()?; // gid
            len += 4;
        }

        if flags & SSH_FILEXFER_ATTR_PERMISSIONS != 0 {
            let perms = self.read_u32()?;

            attrs.permissions = Some(perms);
            len += 4;

            attrs.file_type = Self::file_type_from_permissions(perms);
            attrs.is_directory = attrs.file_type == FileType::Directory;
            attrs.is_regular_file = attrs.file_type == FileType::RegularFile;
            attrs.is_symlink = attrs.file_type == FileType::Symlink;
        }

        if flags & SSH_FILEXFER_ATTR_ACMODTIME != 0 {
            self.read_u32()?; // atime
            len += 4;
            attrs.modify_time = Some(self.read_u32()?);
            len += 4;
        }

        if flags & SSH_FILEXFER_ATTR_EXTENDED != 0 {
            let extended_count = self.read_u32()?;
            len += 4;

            for _ in 0..extended_count {
                let name = self.read_string()?;
                let value = self.read_string()?;
                len += 8 + name.len() + value.len();
            }
        }

        Ok((len, attrs))
    }

    fn file_type_from_permissions(perms: u32) -> FileType {
        match perms & S_IFMT {
            S_IFDIR => FileType::Directory,
            S_IFREG => FileType::RegularFile,
            S_IFLNK => FileType::Symlink,
            S_IFCHR => FileType::CharacterDevice,
            S_IFBLK => FileType::BlockDevice,
            S_IFIFO => FileType::Fifo,
            S_IFSOCK => FileType::Socket,
            _ => FileType::Unknown,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::sftp::{
        types::{FileInfo, SftpStatus},
        SftpClient, SftpCommand,
    };

    use super::*;
    use std::{collections::VecDeque, path::PathBuf};

    struct MockTransport {
        expected_requests: VecDeque<ClientPacket>,
        responses: VecDeque<ServerPacket>,
        request_id_counter: u32,
    }

    impl MockTransport {
        fn new() -> Self {
            Self {
                expected_requests: VecDeque::new(),
                responses: VecDeque::new(),
                request_id_counter: 0,
            }
        }

        fn expect_request(mut self, packet: ClientPacket) -> Self {
            self.expected_requests.push_back(packet);
            self
        }

        fn respond_with(mut self, response: ServerPacket) -> Self {
            self.responses.push_back(response);
            self
        }
    }

    impl TransportLayer for MockTransport {
        fn send_packet(&mut self, packet: ClientPacket) -> Result<(), SftpError> {
            if let Some(expected) = self.expected_requests.pop_front() {
                assert_eq!(
                    std::mem::discriminant(&expected),
                    std::mem::discriminant(&packet)
                );
            }
            Ok(())
        }

        fn receive_packet(&mut self) -> Result<ServerPacket, SftpError> {
            self.responses
                .pop_front()
                .ok_or_else(|| SftpError::ClientError("No more responses".into()))
        }

        fn next_request_id(&mut self) -> u32 {
            let id = self.request_id_counter;
            self.request_id_counter += 1;
            id
        }
    }

    #[test]
    fn test_list_directory() {
        let mock_transport = MockTransport::new()
            .expect_request(ClientPacket::RealPath {
                request_id: 0,
                path: "/".to_string(),
            })
            .respond_with(ServerPacket::Name {
                request_id: 0,
                files: vec![FileInfo {
                    name: "/".to_string(),
                    display_name: "/".to_string(),
                    attrs: FileAttributes::default(),
                }],
            })
            .expect_request(ClientPacket::OpenDir {
                request_id: 1,
                path: "/test".to_string(),
            })
            .respond_with(ServerPacket::Handle {
                request_id: 1,
                handle: vec![1, 2, 3],
            })
            .expect_request(ClientPacket::ReadDir {
                request_id: 2,
                handle: vec![1, 2, 3],
            })
            .respond_with(ServerPacket::Name {
                request_id: 2,
                files: vec![FileInfo {
                    name: "test.txt".to_string(),
                    display_name: "-rw-r--r-- 1 user user 0 Jan 1 00:00 test.txt".to_string(),
                    attrs: FileAttributes::default(),
                }],
            })
            .expect_request(ClientPacket::ReadDir {
                request_id: 2,
                handle: vec![1, 2, 3],
            })
            .respond_with(ServerPacket::Status {
                request_id: 2,
                status_code: 1, // EOF
                message: "".to_string(),
            })
            .expect_request(ClientPacket::Close {
                request_id: 3,
                handle: vec![1, 2, 3],
            })
            .respond_with(ServerPacket::Status {
                request_id: 3,
                status_code: SftpStatus::Ok as u32,
                message: "OK".to_string(),
            });

        let mut client = SftpClient::new(mock_transport, Some("/")).unwrap();

        let cmd = SftpCommand::Ls {
            path: Some(PathBuf::from("test")),
        };
        let result = client.execute_command(&cmd);
        assert!(result.is_ok());
    }
}
