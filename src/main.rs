use crate::sftp::constants::*;
use crate::sftp::session::SftpSession;
use crate::sftp::{SftpClient, SftpCommand};
use env_logger::Builder;
use interface::CommandInterface;
use log::{error, info, LevelFilter};
use ssh2::Session;
use std::net::TcpStream;
use std::process::exit;

mod interface;
mod sftp;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Why is it so confusing to initialise a logger??
    let mut builder = Builder::from_default_env();
    builder
        .default_format()
        .filter(None, LevelFilter::Debug)
        .target(env_logger::Target::Pipe(Box::new(
            std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open("ferric_ftp.log")
                .unwrap(),
        )))
        .init();

    //let tcp = TcpStream::connect("localhost:2222")?;

    let tcp = TcpStream::connect("test.rebex.net:22")?;

    let mut session = Session::new()?;
    session.set_tcp_stream(tcp);
    session.handshake()?;
    session.userauth_password("demo", "password")?;

    //session.userauth_password("sftptest", "pass")?;

    info!("SSH connection successful!");

    let mut channel = session.channel_session()?;
    channel.subsystem("sftp")?;
    let sftp_session = SftpSession::new(channel, SFTP_SUPPORTED_VERSION)?;

    let mut sftp_client = SftpClient::new(sftp_session, None)?;

    CommandInterface::greet();

    loop {
        match CommandInterface::parse_next_input() {
            Ok(ref cmd) => {
                info!("Got command: {:?}", cmd);

                match sftp_client.execute_command(cmd) {
                    Ok(success) => {
                        if !success {
                            break;
                        }
                        continue;
                    }
                    Err(e) => {
                        error!("Failed to execute command: {:?}", e);
                    }
                }
            }
            Err(_) => {
                println!("Error parsing command!");
            }
        }
    }
    Ok(())
}
