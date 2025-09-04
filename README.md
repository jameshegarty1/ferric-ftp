# ferric-ftp
A fast, safe, and lightweight command-line SFTP (SSH File Transfer Protocol) client written in Rust. This tool implements the client-side of the [SFTP protocol version 3](https://datatracker.ietf.org/doc/html/draft-ietf-secsh-filexfer-02#section-6.2),

![license](https://img.shields.io/badge/License-MIT-yellow.svg  "MIT License")

## Features

* SFTP v3 Protocol: Client-side implementation of the widely supported SFTP version 3 protocol.
* CLI Interface: Simple and intuitive command-line interface similar to familiar tools like OpenSSH.
* Cross-Platform: Runs on any platform supported by Rust and libssh2 (Linux, macOS, Windows*).
 
## Installation

### From Source (requires Cargo)
1. Ensure you have Rust and Cargo installed.
2. Clone the repo and build:

```
git clone https://github.com/jameshegarty1/ferric-ftp.git
cd ferric-ftp
cargo build --release
```
3. The compiled binary will be available at `./target/release/ferric-ftp`. You can move it to a directory in your `PATH` for easy access.


## Usage
Basic syntax:
```
ferric-ftp [USER@]HOST[:PORT] [ -p password ]
```
If connection successful and authenticated, interactive mode will show:
```
🦀sftp >
```
Type `help` to see the available commands:
```
🦀sftp > help
Available commands:
ls - list files in current directory
cd - change current directory
get - download file
put - upload file
bye - exit
```

### Commands
| Command                | Description                        |
| -----------------------|:----------------------------------:|
| ls [path]              | List contents of remote directory. |
| get <remote> [local]   | Download a file or directory       |
| put <local> [remote]   | Upload a file or directory         |
| cd [path]              | Change working directory           |
| pwd                    | Print working directory            |


## Dependencies
This project stands on the shoulders of giants:
* ssh2: Rust bindings for libssh2, providing the core SSH2 protocol functionality.
* libssh2-sys: Raw Rust bindings to the C libssh2 library.

## License

Distributed under the MIT License. See LICENSE file for more information.
