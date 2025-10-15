use ferric_ftp::sftp::client::SftpClient;
use ferric_ftp::sftp::constants::*;
use ferric_ftp::sftp::session::SftpSession;
use ferric_ftp::sftp::types::SftpCommand;
use std::path::PathBuf;

use super::test_utils;

#[test]
fn test_sftp_session_initialization() {
    let channel = test_utils::connect_and_auth().unwrap();
    let session = SftpSession::new(channel, SFTP_SUPPORTED_VERSION).unwrap();
    let client = SftpClient::new(session, None);
    assert!(!client.is_err());
}
#[test]
fn test_list_directory() {
    let mut client = test_utils::create_test_client().unwrap();
    let command = SftpCommand::Ls {
        path: Some(PathBuf::from(".")),
    };

    let _ = client.execute_command(&command).unwrap();

    assert!(!client.current_listing.is_empty());

    for file in client.current_listing {
        assert!(!file.name.is_empty());
        assert!(!file.display_name.is_empty());
    }
}
#[test]
fn test_change_directory() {
    let mut client = test_utils::create_test_client().unwrap();
    let mut command = SftpCommand::Cd {
        path: Some(PathBuf::from("pub")),
    };

    let original_dir = client.working_dir.clone();

    client.execute_command(&command).unwrap();

    let next_dir = client.working_dir.clone();

    command = SftpCommand::Cd {
        path: Some(PathBuf::from("..")),
    };

    client.execute_command(&command).unwrap();

    let final_dir = client.working_dir.clone();

    assert_eq!(original_dir, final_dir);
    assert_ne!(original_dir, next_dir);
    assert_eq!(next_dir, PathBuf::from("/pub"));
}

#[test]
fn test_get_file() {
    let mut client = test_utils::create_test_client().unwrap();
    let test_filename = "readme.txt";

    let command = SftpCommand::Get {
        remote_path: PathBuf::from("readme.txt"),
        local_path: Some(PathBuf::from("test_readme.txt")),
    };

    client.execute_command(&command).unwrap();
}

/*
#[test]
fn test_error_handling() {
    let mut client = test_utils::create_test_client().unwrap();

    // Test error cases
    let result = client.list_directory("/non_existent_directory");
    assert!(result.is_err());

    let result = client.change_directory("/path/that/does/not/exist");
    assert!(result.is_err());
}
*/
