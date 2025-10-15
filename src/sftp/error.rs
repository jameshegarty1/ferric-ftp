use std::fmt;

#[derive(Debug)]
pub enum SftpError {
    IoError(std::io::Error),
    ServerError {
        code: u32,
        request_id: u32,
        message: String,
    },
    ClientError(Box<dyn std::error::Error>),
    NotADirectory(String),
    UnexpectedPacket(&'static str),
    UnexpectedResponse(&'static str),
    UnknownError,
    UnexpectedCommand,
    InvalidCommand(&'static str),
}

// Implement Display for SftpError
impl fmt::Display for SftpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SftpError::IoError(e) => write!(f, "IO error: {}", e),
            SftpError::ServerError {
                code,
                request_id,
                message,
            } => write!(
                f,
                "Server error (code: {}, request_id: {}): {}",
                code, request_id, message
            ),
            SftpError::ClientError(e) => write!(f, "Client error: {}", e),
            SftpError::NotADirectory(path) => write!(f, "Not a directory: {}", path),
            SftpError::UnexpectedPacket(msg) => write!(f, "Unexpected packet: {}", msg),
            SftpError::UnexpectedResponse(msg) => write!(f, "Unexpected response: {}", msg),
            SftpError::UnknownError => write!(f, "Unknown error"),
            SftpError::UnexpectedCommand => write!(f, "Unexpected command"),
            SftpError::InvalidCommand(msg) => write!(f, "Invalid command: {}", msg),
        }
    }
}

impl std::error::Error for SftpError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            SftpError::IoError(e) => Some(e),
            SftpError::ClientError(e) => Some(e.as_ref()),
            _ => None,
        }
    }
}

impl From<std::io::Error> for SftpError {
    fn from(error: std::io::Error) -> Self {
        SftpError::IoError(error)
    }
}
