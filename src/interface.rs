use std::io;
use std::io::*;
use std::path::PathBuf;
use crate::sftp::SftpCommand;

const PROMPT: &'static str  = "🦀sftp > ";

pub struct CommandInterface;

impl CommandInterface {
    pub fn greet() {
        println!("Welcome to Rust SFTP Client! 🦀");
    }

    pub fn parse_next_input() -> Result<SftpCommand> {
        print!("{}", PROMPT);
        io::stdout().flush()?;

        let mut input_buffer = String::new();
        let stdin = io::stdin();
        stdin.read_line(&mut input_buffer).expect("panic: unable to read user input!");

        let mut tokens = input_buffer.trim().split_whitespace();

        match tokens.next() {
            Some("ls") => {
                let path = PathBuf::from(tokens.next().unwrap_or("."));
                Ok(SftpCommand::Ls { path })
            },
            Some("cd") => {
                let path = PathBuf::from(tokens.next().unwrap_or("~"));
                Ok(SftpCommand::Cd { path })
            },
            Some("exit") => {
                Ok(SftpCommand::Exit)
            }
            Some("help") => {
                Self::print_help();
                Ok(SftpCommand::Help)
            }
            Some(&_) => {
                {Err(io::Error::new(io::ErrorKind::InvalidInput, "Unknown command!"))}
            },
            None => {Err(io::Error::new(io::ErrorKind::InvalidInput, "No command"))}
        }
    }

    fn print_help() {
        println!("Available commands:\nls\ncd\nget\nput\nexit");
    }
}