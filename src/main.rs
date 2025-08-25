use crate::sftp::{SftpCommand, SftpSession};
use interface::CommandInterface;
use ssh2::Session;
use std::fmt::Debug;
use std::io::prelude::*;
use std::net::TcpStream;
use std::process::exit;
use log::{info, warn, error};

mod interface;
mod sftp;
mod util;
const SFTP_SUPPORTED_VERSION: u32 = 3;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
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
        println!("SFTP error: failed to create client");
        exit(1);
    };

    CommandInterface::greet();

    let mut running = true;

    while running {
        match CommandInterface::parse_next_input() {
            Ok(ref command) => match command {
                SftpCommand::Ls { path } => {
                    println!("Got ls command with path {:?}", path);
                    match sftp_client.execute_command( &command ) {
                        Ok(CommandOutput) => {
                            continue
                        },
                        Err(e) => {
                            println!("Failed to execute command: {:?}", e);

                        }
                    }
                },
                SftpCommand::Cd { path } => {
                    println!("Got cd command with path {:?}", path);
                },
                SftpCommand::Exit => {
                    println!("Got exit command");
                    running = false;
                }
                _ => {}
            },
            Err(_) => {
                running = false;
            }
        }
    }
    Ok(())
}
