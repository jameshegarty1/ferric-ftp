#[derive(Debug)]
pub enum SftpError {
    ServerError { code: u32, message: String },
    ClientError(Box<dyn std::error::Error>),
    UnknownError { message: String },
}