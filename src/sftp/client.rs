use super::session::SftpSession;
use super::types::{SftpCommand, CommandOutput};
use super::error::SftpError;
use super::commands;

pub struct SftpClient {
    pub session: SftpSession,
}

impl SftpClient {
    pub fn execute_command(&mut self, cmd: &SftpCommand) -> Result<CommandOutput, SftpError> {
        match cmd {
            SftpCommand::Ls { path } => commands::list_directory(self, path),
            SftpCommand::Cd { path } => commands::change_directory(self, path),
            _ => {
                todo!()
            } // etc.
        }
    }
}