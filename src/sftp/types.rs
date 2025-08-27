use std::path::PathBuf;

#[derive(Debug)]
pub struct FileInfo {
    pub short_name: String,
    pub long_name: String,
    pub attrs: FileAttributes,
}

#[derive(Debug, Default)]
pub struct FileAttributes {
    pub file_type: u8,
    pub size: Option<u64>,
    pub permissions: Option<u32>,
    pub modify_time: Option<u32>,
}

#[derive(Debug)]
pub enum SftpCommand {
    Ls {
        path: PathBuf,
    },
    Cd {
        path: PathBuf,
    },
    Get {
        remote_path: PathBuf,
        local_path: Option<PathBuf>,
    },
    Put {
        remote_path: PathBuf,
        local_path: Option<PathBuf>,
    },
    Pwd,
    Help,
    Bye,
}

pub struct CommandOutput {
    pub result: bool,
    pub message: String,
}

#[repr(u8)]
#[derive(Debug)]
pub enum SftpStatus {
    Ok = 0,            // SSH_FX_OK
    Eof = 1,           // SSH_FX_EOF
    InvalidHandle = 4, // SSH_FX_INVALID_HANDLE
}