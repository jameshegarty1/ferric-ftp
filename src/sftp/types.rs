use std::path::PathBuf;
use std::time::{Duration, SystemTime};

#[derive(Debug)]
pub enum SftpCommand {
    Ls {
        path: Option<PathBuf>,
    },
    Cd {
        path: Option<PathBuf>,
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
#[derive(Debug, Clone)]
pub struct FileInfo {
    pub name: String,
    pub display_name: String,
    pub attrs: FileAttributes,
}

#[derive(Debug, Default, Clone)]
pub struct FileAttributes {
    pub size: Option<u64>,
    pub permissions: Option<u32>,
    pub modify_time: Option<u32>,
    pub file_type: FileType,
    pub is_directory: bool,
    pub is_regular_file: bool,
    pub is_symlink: bool,
}

#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub enum FileType {
    #[default]
    Unknown,
    RegularFile,
    Directory,
    Symlink,
    CharacterDevice,
    BlockDevice,
    Fifo,
    Socket,
}

#[derive(Debug, Clone)]
pub struct DirectoryCache {
    pub files: Vec<FileInfo>,
    pub timestamp: SystemTime,
}

#[repr(u8)]
#[derive(Debug)]
pub enum SftpStatus {
    Ok = 0,            // SSH_FX_OK
    Eof = 1,           // SSH_FX_EOF
    InvalidHandle = 4, // SSH_FX_INVALID_HANDLE
}
