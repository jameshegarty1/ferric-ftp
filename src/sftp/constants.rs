pub const SFTP_SUPPORTED_VERSION: u32 = 3;

// SFTP Protocol message types
pub const SSH_FXP_INIT: u8 = 1;
pub const SSH_FXP_VERSION: u8 = 2;
pub const SSH_FXP_CLOSE: u8 = 4;
pub const SSH_FXP_OPENDIR: u8 = 11;
pub const SSH_FXP_READDIR: u8 = 12;
pub const SSH_FXP_HANDLE: u8 = 102;
pub const SSH_FXP_NAME: u8 = 104;
pub const SSH_FXP_STATUS: u8 = 101;

// File attribute flags
pub const SSH_FILEXFER_ATTR_SIZE: u32 = 0x00000001;
pub const SSH_FILEXFER_ATTR_UIDGID: u32 = 0x00000002;
pub const SSH_FILEXFER_ATTR_PERMISSIONS: u32 = 0x00000004;
pub const SSH_FILEXFER_ATTR_ACMODTIME: u32 = 0x00000008;
pub const SSH_FILEXFER_ATTR_EXTENDED: u32 = 0x80000000;

pub const ATTR_FLAGS: &[u32] = &[
    0x00000001, // Size
    0x00000002, // UIDGID
    0x00000004, // Permissions
    0x00000008, // ModifyTime
    0x80000000, // Extended
];