use std::path::PathBuf;
use std::time::SystemTime;

use crate::sftp::constants::{
    SSH_FILEXFER_ATTR_ACMODTIME, SSH_FILEXFER_ATTR_PERMISSIONS, SSH_FILEXFER_ATTR_SIZE,
};

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

impl FileAttributes {
    pub fn exists(&self) -> bool {
        self.size.is_some() && self.permissions.is_some() && self.modify_time.is_some()
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        let mut flags = 0u32;

        if self.size.is_some() {
            flags |= SSH_FILEXFER_ATTR_SIZE;
        }
        if self.permissions.is_some() {
            flags |= SSH_FILEXFER_ATTR_PERMISSIONS;
        }
        if self.modify_time.is_some() {
            flags |= SSH_FILEXFER_ATTR_ACMODTIME;
        }

        bytes.extend_from_slice(&flags.to_be_bytes());

        if let Some(size) = self.size {
            bytes.extend_from_slice(&size.to_be_bytes());
        }
        if let Some(perms) = self.permissions {
            bytes.extend_from_slice(&perms.to_be_bytes());
        }
        if let Some(mtime) = self.modify_time {
            bytes.extend_from_slice(&mtime.to_be_bytes());
        }

        bytes
    }
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
    //pub timestamp: SystemTime,
}

#[repr(u8)]
#[derive(Debug)]
pub enum SftpStatus {
    Ok = 0,            // SSH_FX_OK
    Eof = 1,           // SSH_FX_EOF
    InvalidHandle = 4, // SSH_FX_INVALID_HANDLE
}
