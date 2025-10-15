// tests/integration/test_utils.rs
use ferric_ftp::sftp::client::SftpClient;
use ferric_ftp::sftp::constants::*;
use ferric_ftp::sftp::session::SftpSession;
use ssh2::{Channel, Session, Sftp};
use std::net::TcpStream;
use std::time::Duration;

fn connect_to_test_server() -> Result<Session, Box<dyn std::error::Error>> {
    let tcp = TcpStream::connect("test.rebex.net:22")?;
    let mut session = Session::new()?;
    session.set_tcp_stream(tcp);
    session.handshake()?;
    Ok(session)
}
pub fn connect_and_auth() -> Result<Channel, Box<dyn std::error::Error>> {
    let session = connect_to_test_server()?;
    session.userauth_password("demo", "password")?;

    if !session.authenticated() {
        return Err("Authentication failed".into());
    }

    let mut channel = session.channel_session()?;
    channel.subsystem("sftp").unwrap();

    Ok(channel)
}

pub fn create_test_client() -> Result<SftpClient<SftpSession>, Box<dyn std::error::Error>> {
    let channel = connect_and_auth()?;
    let sftp_session = SftpSession::new(channel, SFTP_SUPPORTED_VERSION)?;
    let client = SftpClient::new(sftp_session, None)?;
    Ok(client)
}

/*
pub fn ensure_test_server() -> Result<(), Box<dyn std::error::Error>> {
    let _session = connect_to_test_server()?;
    Ok(())
}
*/
