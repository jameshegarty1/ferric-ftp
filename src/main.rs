use interface::CommandInterface;
use ssh2::Session;
use std::net::TcpStream;
use std::process::exit;
use log::{LevelFilter, error, info, debug};
use env_logger::Builder;
use crate::sftp::{SftpCommand, SftpSession};
use crate::sftp::constants::*;

mod interface;
mod util;
mod sftp;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut builder = Builder::from_default_env();
    builder
        .default_format()
        .filter(None, LevelFilter::Debug)
        .target(env_logger::Target::Pipe(Box::new(std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open("ferric_ftp.log")
            .unwrap())))
        .init();

    let tcp = TcpStream::connect("test.rebex.net:22")?;

    let mut session = Session::new()?;
    session.set_tcp_stream(tcp);
    session.handshake()?;
    session.userauth_password("demo", "password")?;

    info!("SSH connection successful!");

    let mut channel = session.channel_session()?;
    channel.subsystem("sftp")?;

    let mut sftp_client = if let Ok(client) = SftpSession::generate_client(channel, SFTP_SUPPORTED_VERSION) {
        client
    } else {
        error!("SFTP error: failed to create client");
        exit(1);
    };

    CommandInterface::greet();

    let mut running = true;

    while running {
        match CommandInterface::parse_next_input() {
            Ok(ref command) => match command {
                SftpCommand::Ls { path } => {
                    info!("Got ls command with path {:?}", path);
                    match sftp_client.execute_command( &command ) {
                        Ok(CommandOutput) => {
                            info!("{}", CommandOutput.message);
                            continue
                        },
                        Err(e) => {
                            error!("Failed to execute command: {:?}", e);
                        }
                    }
                },
                SftpCommand::Cd { path } => {
                    info!("Got cd command with path {:?}", path);
                },
                SftpCommand::Exit => {
                    info!("Got exit command");
                    running = false;
                }
                _ => {}
            },
            Err(_) => {
                println!("Error parsing command!");
            }
        }
    }
    Ok(())
}
