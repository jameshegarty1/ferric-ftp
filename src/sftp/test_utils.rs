// src/sftp/test_utils.rs
use super::error::SftpError;
use super::types::FileAttributes;
use std::collections::VecDeque;

#[derive(Default)]
pub struct MockSession {
    pub read_data: VecDeque<u8>,
    pub written_data: Vec<u8>,
    pub should_fail: bool,
}

impl MockSession {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_read_data(mut self, data: Vec<u8>) -> Self {
        self.read_data = VecDeque::from(data);
        self
    }

    pub fn with_failure(mut self) -> Self {
        self.should_fail = true;
        self
    }

    pub fn push_read_data(&mut self, data: &[u8]) {
        self.read_data.extend(data);
    }

    pub fn push_u32(&mut self, value: u32) {
        self.push_read_data(&value.to_be_bytes());
    }

    pub fn push_string(&mut self, s: &str) {
        self.push_u32(s.len() as u32);
        self.push_read_data(s.as_bytes());
    }

    pub fn read_u8(&mut self) -> Result<u8, SftpError> {
        if self.should_fail {
            return Err(SftpError::ClientError(
                std::io::Error::new(std::io::ErrorKind::Other, "Mock failure").into(),
            ));
        }

        self.read_data
            .pop_front()
            .ok_or_else(|| SftpError::ClientError(
                std::io::Error::new(std::io::ErrorKind::UnexpectedEof, "No more data").into(),
            ))
    }

    pub fn read_u32(&mut self) -> Result<u32, SftpError> {
        if self.should_fail {
            return Err(SftpError::ClientError(
                std::io::Error::new(std::io::ErrorKind::Other, "Mock failure").into(),
            ));
        }

        if self.read_data.len() < 4 {
            return Err(SftpError::ClientError(
                std::io::Error::new(std::io::ErrorKind::UnexpectedEof, "Not enough data for u32").into(),
            ));
        }

        let bytes: Vec<u8> = self.read_data.drain(0..4).collect();
        Ok(u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    pub fn read_string(&mut self) -> Result<Vec<u8>, SftpError> {
        let len = self.read_u32()? as usize;
        
        if self.read_data.len() < len {
            return Err(SftpError::ClientError(
                std::io::Error::new(std::io::ErrorKind::UnexpectedEof, "Not enough data for string").into(),
            ));
        }

        let bytes: Vec<u8> = self.read_data.drain(0..len).collect();
        Ok(bytes)
    }

    pub fn read_i64(&mut self) -> Result<i64, SftpError> {
        if self.read_data.len() < 8 {
            return Err(SftpError::ClientError(
                std::io::Error::new(std::io::ErrorKind::UnexpectedEof, "Not enough data for i64").into(),
            ));
        }

        let bytes: Vec<u8> = self.read_data.drain(0..8).collect();
        Ok(i64::from_be_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3],
            bytes[4], bytes[5], bytes[6], bytes[7],
        ]))
    }

    pub fn read_u64(&mut self) -> Result<u64, SftpError> {
        if self.read_data.len() < 8 {
            return Err(SftpError::ClientError(
                std::io::Error::new(std::io::ErrorKind::UnexpectedEof, "Not enough data for u64").into(),
            ));
        }

        let bytes: Vec<u8> = self.read_data.drain(0..8).collect();
        Ok(u64::from_be_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3],
            bytes[4], bytes[5], bytes[6], bytes[7],
        ]))
    }

    pub fn discard(&mut self, bytes: &usize) -> Result<(), SftpError> {
        if self.read_data.len() < *bytes {
            return Err(SftpError::ClientError(
                std::io::Error::new(std::io::ErrorKind::UnexpectedEof, "Not enough data to discard").into(),
            ));
        }

        self.read_data.drain(0..*bytes);
        Ok(())
    }

    pub fn parse_file_attributes(&mut self, flags: &u32) -> Result<(usize, FileAttributes), SftpError> {
        let mut attrs = FileAttributes::default();
        let mut len = 0;

        if flags & super::constants::SSH_FILEXFER_ATTR_SIZE != 0 {
            attrs.size = Some(self.read_u64()?);
            len += 8;
        }

        if flags & super::constants::SSH_FILEXFER_ATTR_PERMISSIONS != 0 {
            attrs.permissions = Some(self.read_u32()?);
            len += 4;
        }

        Ok((len, attrs))
    }

    pub fn write_all(&mut self, data: &[u8]) -> Result<(), SftpError> {
        if self.should_fail {
            return Err(SftpError::ClientError(
                std::io::Error::new(std::io::ErrorKind::Other, "Mock write failure").into(),
            ));
        }

        self.written_data.extend(data);
        Ok(())
    }

    pub fn flush(&mut self) -> Result<(), SftpError> {
        Ok(())
    }

    pub fn get_written_data(&self) -> &[u8] {
        &self.written_data
    }

    pub fn clear_written_data(&mut self) {
        self.written_data.clear();
    }
}
