// tests/integration/test_utils.rs
use ferric_ftp::sftp::client::SftpClient;
use ssh2::Session;
use std::net::TcpStream;
use std::time::Duration;

pub fn connect_to_test_server() -> Result<Session, Box<dyn std::error::Error>> {
    let tcp = TcpStream::connect("localhost:2222")?;
    let mut session = Session::new()?;
    session.set_tcp_stream(tcp);
    session.handshake()?;
    Ok(session)
}

pub fn connect_and_auth() -> Result<Session, Box<dyn std::error::Error>> {
    let mut session = connect_to_test_server()?;
    session.userauth_password("sftptest", "pass")?;
    
    if !session.authenticated() {
        return Err("Authentication failed".into());
    }
    
    Ok(session)
}

pub fn create_test_client() -> Result<SftpClient, Box<dyn std::error::Error>> {
    let session = connect_and_auth()?;
    let client = SftpClient::new(session)?;
    Ok(client)
}

pub fn ensure_test_server() -> Result<(), Box<dyn std::error::Error>> {
    let _session = connect_to_test_server()?;
    Ok(())
}
