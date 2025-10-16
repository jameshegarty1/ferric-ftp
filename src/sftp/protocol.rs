use super::error::SftpError;
use super::packet::{ClientPacket, ServerPacket};
use super::session::TransportLayer;
use super::types::FileAttributes;
use super::types::{FileInfo, SftpStatus};
use log::info;

pub struct SftpProtocol<T: TransportLayer> {
    transport: T,
}

impl<T: TransportLayer> SftpProtocol<T> {
    pub fn new(transport: T) -> Self {
        Self { transport }
    }

    pub fn realpath(&mut self, path: &str) -> Result<String, SftpError> {
        let request_id = self.transport.next_request_id();
        let packet = ClientPacket::RealPath {
            request_id,
            path: path.to_string(),
        };

        self.transport.send_packet(packet)?;

        match self.transport.receive_packet()? {
            ServerPacket::Name { files, .. } if files.len() == 1 => {
                Ok(String::from(&files[0].name))
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
            _ => Err(SftpError::UnexpectedResponse("RealPath response")),
        }
    }

    pub fn open_dir(&mut self, path: &str) -> Result<Vec<u8>, SftpError> {
        let request_id = self.transport.next_request_id();
        let packet = ClientPacket::OpenDir {
            request_id,
            path: path.to_string(),
        };

        self.transport.send_packet(packet)?;

        match self.transport.receive_packet()? {
            ServerPacket::Handle { handle, .. } => Ok(handle),
            ServerPacket::Status {
                status_code,
                request_id,
                message,
            } => Err(SftpError::ServerError {
                code: status_code,
                request_id,
                message,
            }),
            _ => Err(SftpError::UnexpectedPacket("OpenDir response")),
        }
    }

    pub fn read_dir(&mut self, handle: &[u8]) -> Result<Vec<FileInfo>, SftpError> {
        let request_id = self.transport.next_request_id();
        let packet = ClientPacket::ReadDir {
            request_id,
            handle: handle.to_vec(),
        };

        self.transport.send_packet(packet)?;

        match self.transport.receive_packet()? {
            ServerPacket::Name { files, .. } => Ok(files),
            ServerPacket::Status {
                status_code,
                request_id,
                message,
            } => {
                if status_code == SftpStatus::Eof as u32 {
                    Ok(Vec::new())
                } else {
                    Err(SftpError::ServerError {
                        code: status_code,
                        request_id,
                        message,
                    })
                }
            }
            _ => Err(SftpError::UnexpectedPacket("ReadDir response")),
        }
    }

    pub fn close(&mut self, handle: Vec<u8>) -> Result<(), SftpError> {
        let request_id = self.transport.next_request_id();
        let packet = ClientPacket::Close { request_id, handle };

        self.transport.send_packet(packet)?;

        match self.transport.receive_packet()? {
            ServerPacket::Status { status_code, .. } if status_code == SftpStatus::Ok as u32 => {
                Ok(())
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
            _ => Ok(()),
        }
    }

    pub fn stat(&mut self, path: &str) -> Result<FileAttributes, SftpError> {
        let request_id = self.transport.next_request_id();
        let packet = ClientPacket::Stat {
            request_id,
            path: path.to_string(),
        };

        self.transport.send_packet(packet)?;

        match self.transport.receive_packet()? {
            ServerPacket::Attrs { attrs, .. } => Ok(attrs),
            ServerPacket::Status {
                request_id,
                status_code,
                message,
            } => Err(SftpError::ServerError {
                code: status_code,
                request_id,
                message,
            }),
            _ => Err(SftpError::UnexpectedPacket("Unexpected Stat response")),
        }
    }

    pub fn open(&mut self, path: &str, pflags: u32) -> Result<Vec<u8>, SftpError> {
        let request_id = self.transport.next_request_id();
        let packet = ClientPacket::Open {
            request_id,
            path: path.to_string(),
            pflags,
            attrs: FileAttributes::default(),
        };

        self.transport.send_packet(packet)?;

        match self.transport.receive_packet()? {
            ServerPacket::Handle { handle, .. } => Ok(handle),
            ServerPacket::Status {
                status_code,
                request_id,
                message,
            } => Err(SftpError::ServerError {
                code: status_code,
                request_id,
                message,
            }),
            _ => Err(SftpError::UnexpectedPacket("OpenDir response")),
        }
    }

    pub fn read(&mut self, handle: &[u8]) -> Result<Vec<u8>, SftpError> {
        let mut offset: u64 = 0;
        let chunk_size: u32 = 32768;
        let mut result: Vec<u8> = Vec::new();
        loop {
            let request_id = self.transport.next_request_id();
            let packet = ClientPacket::Read {
                request_id,
                handle: handle.to_vec(),
                offset,
                len: chunk_size,
            };

            self.transport.send_packet(packet)?;

            match self.transport.receive_packet()? {
                ServerPacket::Data { data, .. } => {
                    let data_len = data.len() as u64;
                    result.extend_from_slice(&data);

                    if data_len < chunk_size as u64 {
                        break;
                    }
                    offset += data_len;
                }
                ServerPacket::Status {
                    status_code,
                    request_id,
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
                    return Err(SftpError::UnexpectedPacket("Read response"));
                }
            }
        }
        Ok(result)
    }
}
