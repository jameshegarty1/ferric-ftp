use std::process::{Child, Command};
use std::thread;
use std::time::Duration;

pub struct TestSftpServer {
    process: Option<Child>,
}

impl TestSftpServer {
    pub fn start() -> Result<Self, Box<dyn std::error::Error>> {
        println!("Make sure test SFTP server is running on localhost:2222");

        // Or start one programmatically:
        // let process = Command::new("docker")
        //     .args(&["compose", "-f", "test_server/docker-compose.yml", "up"])
        //     .spawn()?;

        thread::sleep(Duration::from_secs(2)); // Wait for server to start

        Ok(Self { process: None })
    }
}

impl Drop for TestSftpServer {
    fn drop(&mut self) {
        if let Some(mut process) = self.process.take() {
            let _ = process.kill();
        }
    }
}
