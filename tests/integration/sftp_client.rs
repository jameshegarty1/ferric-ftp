use ferric_ftp::sftp::client::SftpClient;
use ferric_ftp::sftp::session::SftpSession;
use ssh2::Session;
use std::net::TcpStream;
use std::path::Path;

mod test_utils;

#[test]
#[ignore] // Ignore by default since it needs a real server
fn test_connect_and_authenticate() {
    let mut session = test_utils::connect_to_test_server().unwrap();

    // Test that we can authenticate
    session.userauth_password("testuser", "testpass").unwrap();
    assert!(session.authenticated());
}

#[test]
#[ignore]
fn test_sftp_session_initialization() {
    let ssh_session = test_utils::connect_and_auth().unwrap();
    let sftp_session = SftpSession::new(ssh_session).unwrap();

    assert_eq!(sftp_session.version, 3);
}

#[test]
#[ignore]
fn test_list_directory() {
    let mut client = test_utils::create_test_client().unwrap();

    // This tests the full flow:
    // 1. OPENDIR packet
    // 2. HANDLE response
    // 3. READDIR packet
    // 4. NAME response with FileInfo list
    let files = client.list_directory("/").unwrap();

    // Should at least see . and ..
    assert!(files.len() >= 2);

    // Verify we got proper FileInfo structures
    for file in files {
        assert!(!file.name.is_empty());
        assert!(!file.display_name.is_empty());
        // Attributes might be present depending on server
    }
}

#[test]
#[ignore]
fn test_get_current_directory() {
    let mut client = test_utils::create_test_client().unwrap();

    let current_dir = client.get_current_directory().unwrap();
    assert!(!current_dir.is_empty());
}

#[test]
#[ignore]
fn test_change_directory() {
    let mut client = test_utils::create_test_client().unwrap();

    let original_dir = client.get_current_directory().unwrap();

    // Change to root and back
    client.change_directory("/").unwrap();
    let root_dir = client.get_current_directory().unwrap();

    client.change_directory(&original_dir).unwrap();
    let final_dir = client.get_current_directory().unwrap();

    assert_eq!(original_dir, final_dir);
    assert_ne!(original_dir, root_dir);
}

#[test]
#[ignore]
fn test_file_operations() {
    let mut client = test_utils::create_test_client().unwrap();
    let test_filename = "integration_test_file.txt";

    // Test file stat
    let result = client.get_file_attributes(test_filename);
    // Might not exist, that's OK - we're testing the protocol

    // Test realpath
    let real_path = client.get_real_path(".").unwrap();
    assert!(!real_path.is_empty());
}

#[test]
#[ignore]
fn test_error_handling() {
    let mut client = test_utils::create_test_client().unwrap();

    // Test error cases
    let result = client.list_directory("/non_existent_directory");
    assert!(result.is_err());

    let result = client.change_directory("/path/that/does/not/exist");
    assert!(result.is_err());
}
