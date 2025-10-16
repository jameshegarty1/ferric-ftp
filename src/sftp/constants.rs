pub const SFTP_SUPPORTED_VERSION: u32 = 3;

// SFTP Protocol message types
pub const SSH_FXP_INIT: u8 = 1;
pub const SSH_FXP_VERSION: u8 = 2;
pub const SSH_FXP_OPEN: u8 = 3;
pub const SSH_FXP_READ: u8 = 5;
pub const SSH_FXP_CLOSE: u8 = 4;
pub const SSH_FXP_OPENDIR: u8 = 11;
pub const SSH_FXP_READDIR: u8 = 12;
pub const SSH_FXP_REALPATH: u8 = 16;
pub const SSH_FXP_STAT: u8 = 17;
pub const SSH_FXP_STATUS: u8 = 101;
pub const SSH_FXP_HANDLE: u8 = 102;
pub const SSH_FXP_DATA: u8 = 103;
pub const SSH_FXP_NAME: u8 = 104;
pub const SSH_FXP_ATTRS: u8 = 105;

// File attribute flags
pub const SSH_FILEXFER_ATTR_SIZE: u32 = 0x00000001;
pub const SSH_FILEXFER_ATTR_UIDGID: u32 = 0x00000002;
pub const SSH_FILEXFER_ATTR_PERMISSIONS: u32 = 0x00000004;
pub const SSH_FILEXFER_ATTR_ACMODTIME: u32 = 0x00000008;
pub const SSH_FILEXFER_ATTR_EXTENDED: u32 = 0x80000000;

// Unix file permissions
pub const S_IFMT: u32 = 0o170000; // bit mask for the file type bit field
pub const S_IFDIR: u32 = 0o040000; // directory
pub const S_IFREG: u32 = 0o100000; // regular file
pub const S_IFLNK: u32 = 0o120000; // symbolic link
pub const S_IFCHR: u32 = 0o020000; // character device
pub const S_IFBLK: u32 = 0o060000; // block device
pub const S_IFIFO: u32 = 0o010000; // FIFO
pub const S_IFSOCK: u32 = 0o140000; // socket

// File pflags
pub const SSH_FXF_READ: u32 = 0x00000001;
pub const SSH_FXF_WRITE: u32 = 0x00000002;
//pub const SSH_FXF_APPEND: u32 = 0x00000004;
//pub const SSH_FXF_CREAT: u32 = 0x00000008;
//pub const SSH_FXF_TRUNC: u32 = 0x00000010;
//pub const SSH_FXF_EXCL: u32 = 0x00000020;
