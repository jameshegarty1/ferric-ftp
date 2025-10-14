use super::constants::*;
use super::error::SftpError;
use super::packet::ClientPacket;
use super::types::{FileAttributes, FileType};
use log::info;
use ssh2::Channel;
use std::io::{Read, Write};

pub struct SftpSession {
    pub channel: Channel,
    pub version: u32,
    pub next_request_id: u32,
}

impl SftpSession {
    pub fn send_packet(&mut self, packet: ClientPacket) -> Result<(), SftpError> {
        self.channel
            .write_all(&packet.to_bytes())
            .map_err(|e| SftpError::IoError(e));
        self.channel.flush().map_err(|e| SftpError::IoError(e));
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
            let uid = self.read_u32()?;
            len += 4;
            let gid = self.read_u32()?;
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
            let atime = self.read_u32()?;
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

    pub fn debug_peek_bytes(&mut self, count: usize) -> Result<Vec<u8>, SftpError> {
        let mut buffer = vec![0u8; count];
        self.channel
            .read_exact(&mut buffer)
            .map_err(|e| SftpError::ClientError(e.into()))?;
        info!("Raw bytes: {:02x?}", buffer);
        Ok(buffer)
    }
}
