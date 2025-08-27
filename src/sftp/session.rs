use log::info;
use ssh2::Channel;
use std::collections::HashMap;
use std::io::{Read, Write};

use super::client::SftpClient;
use super::constants::*;
use super::error::SftpError;
use super::packet::ClientPacket;
use super::packet::ServerPacket;
use super::types::FileAttributes;

pub struct SftpSession {
    pub channel: Channel,
    pub version: u32,
    pub working_dir: String,
    pub next_request_id: u32,
    pub handles: HashMap<String, Vec<u8>>,
}

impl SftpSession {
    pub fn generate_client(mut channel: Channel, version: u32) -> Result<SftpClient, SftpError> {
        let init_packet = ClientPacket::Init { version };
        channel
            .write_all(&init_packet.to_bytes())
            .map_err(|e| SftpError::ClientError(e.into()))?;

        let mut session = SftpSession {
            channel: channel,
            version: version,
            working_dir: String::new(),
            next_request_id: 0,
            handles: HashMap::new(),
        };

        match ServerPacket::from_session(&mut session)? {
            ServerPacket::Version { version: _ } => {
                // Initialize working dir with a RealPath request
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
                            session.working_dir = files[0].short_name.clone();
                            info!("Initialized working directory: {}", session.working_dir);
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
                Ok(SftpClient { session })
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

    pub fn send_packet(&mut self, packet: ClientPacket) -> Result<(), std::io::Error> {
        self.channel.write_all(&packet.to_bytes())?;
        self.channel.flush()?;
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
            info!("  Size: {:?}", attrs.size);
        }

        if flags & SSH_FILEXFER_ATTR_UIDGID != 0 {
            let uid = self.read_u32()?;
            len += 4;
            info!("  UID: {}", uid);
            let gid = self.read_u32()?;
            len += 4;
            info!("  GID: {}", gid);
        }

        if flags & SSH_FILEXFER_ATTR_PERMISSIONS != 0 {
            attrs.permissions = Some(self.read_u32()?);
            len += 4;
            info!(
                "  Permissions: 0o{:o} (0x{:x})",
                attrs.permissions.unwrap(),
                attrs.permissions.unwrap()
            );
        }

        if flags & SSH_FILEXFER_ATTR_ACMODTIME != 0 {
            let atime = self.read_u32()?;
            len += 4;
            info!("  Access time: {}", atime);
            attrs.modify_time = Some(self.read_u32()?);
            len += 4;
            info!("  Modify time: {:?}", attrs.modify_time);
        }

        if flags & SSH_FILEXFER_ATTR_EXTENDED != 0 {
            let extended_count = self.read_u32()?;
            len += 4;
            info!("  Extended attributes count: {}", extended_count);

            for i in 0..extended_count {
                let name = self.read_string()?;
                let value = self.read_string()?;
                len += 8 + name.len() + value.len();
                info!(
                    "    Extended[{}]: {} = {}",
                    i,
                    String::from_utf8_lossy(&name),
                    String::from_utf8_lossy(&value)
                );
            }
        }

        info!("Total attributes length: {}", len);
        Ok((len, attrs))
    }

    pub fn debug_peek_bytes(&mut self, count: usize) -> Result<Vec<u8>, SftpError> {
        let mut buffer = vec![0u8; count];
        self.channel
            .read_exact(&mut buffer)
            .map_err(|e| SftpError::ClientError(e.into()))?;
        info!("Raw bytes: {:02x?}", buffer);
        Ok(buffer)
    }
}
