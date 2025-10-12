use crate::sftp::SftpCommand;
use std::io;
use std::io::*;
use std::path::PathBuf;

const PROMPT: &str = "🦀sftp > ";
const DEFAULT_LS_PATH: &str = ".";
const DEFAULT_CD_PATH: &str = "/";

pub struct CommandInterface;

impl CommandInterface {
    pub fn greet() {
        println!("Welcome to Rust SFTP Client! 🦀");
    }

    pub fn parse_next_input() -> Result<SftpCommand> {
        print!("{}", PROMPT);
        io::stdout().flush()?;

        let mut input_buffer = String::new();
        io::stdin().read_line(&mut input_buffer)?;
        Self::parse_input(&input_buffer)
    }

    pub fn parse_input(input: &str) -> Result<SftpCommand> {
        let mut tokens = input.trim().split_whitespace();

        match tokens.next() {
            Some("ls") => {
                let path = PathBuf::from(tokens.next().unwrap_or("."));
                Ok(SftpCommand::Ls { path: Some(path) })
            }
            Some("cd") => {
                let path = PathBuf::from(tokens.next().unwrap_or("/"));
                Ok(SftpCommand::Cd { path: Some(path) })
            }
            Some("pwd") => Ok(SftpCommand::Pwd),
            Some("bye") => Ok(SftpCommand::Bye),
            Some("help") => Ok(SftpCommand::Help),
            Some(&_) => Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Unknown command!",
            )),
            None => Err(io::Error::new(io::ErrorKind::InvalidInput, "No command")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_ls() {
        let command = CommandInterface::parse_input("ls").unwrap();
        if let SftpCommand::Ls { path } = command {
            assert_eq!(path, Some(PathBuf::from(".")));
        } else {
            panic!("Expected Ls command");
        }
    }

    #[test]
    fn test_parse_ls_path() {
        let command = CommandInterface::parse_input("ls test").unwrap();
        if let SftpCommand::Ls { path } = command {
            assert_eq!(path, Some(PathBuf::from("test")));
        } else {
            panic!("Expected Ls command");
        }
    }
}
