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
}
