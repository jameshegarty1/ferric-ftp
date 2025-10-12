#[cfg(test)]
mod tests {
    use super::*;
    use crate::sftp::session::MockSftpSession;

    struct TestSession {
        data: Vec<u8>,
        position: usize,
    }

    impl TestSession {
        fn new(data: Vec<u8>) -> Self {
            Self { data, position: 0 }
        }
    }

    impl SftpSession for TestSession {
        fn read_u32(&mut self) -> Result<u32, SftpError> {
            let bytes: [u8; 4] = self.data[self.position..self.position + 4]
                .try_into()
                .unwrap();
            self.position += 4;
            Ok(u32::from_be_bytes(bytes))
        }

        fn read_u8(&mut self) -> Result<u8, SftpError> {
            let byte = self.data[self.position];
            self.position += 1;
            Ok(byte)
        }

        fn read_string(&mut self) -> Result<Vec<u8>, SftpError> {
            let len = self.read_u32()? as usize;
            let result = self.data[self.position..self.position + len].to_vec();
            self.position += len;
            Ok(result)
        }

        fn discard(&mut self, bytes: &usize) -> Result<(), SftpError> {
            self.position += bytes;
            Ok(())
        }

        // Implement other required methods with default behavior
        fn parse_file_attributes(
            &mut self,
            _flags: &u32,
        ) -> Result<(usize, FileAttributes), SftpError> {
            Ok((0, FileAttributes::default()))
        }
    }

    #[test]
    fn test_client_packet_init_serialization() {
        let packet = ClientPacket::Init { version: 3 };
        let bytes = packet.to_bytes();

        // Verify structure: [length: u32, type: u8, version: u32]
        assert_eq!(bytes.len(), 9); // 4 (length) + 1 (type) + 4 (version)
        assert_eq!(bytes[4], SSH_FXP_INIT);

        let version_bytes = &bytes[5..9];
        assert_eq!(u32::from_be_bytes(version_bytes.try_into().unwrap()), 3);
    }

    #[test]
    fn test_client_packet_opendir_serialization() {
        let packet = ClientPacket::OpenDir {
            request_id: 42,
            path: "/home".to_string(),
        };
        let bytes = packet.to_bytes();

        // Should be: [length, SSH_FXP_OPENDIR, request_id, path_length, path]
        assert_eq!(bytes[4], SSH_FXP_OPENDIR);

        let request_id_bytes = &bytes[5..9];
        assert_eq!(u32::from_be_bytes(request_id_bytes.try_into().unwrap()), 42);

        let path_len_bytes = &bytes[9..13];
        let path_len = u32::from_be_bytes(path_len_bytes.try_into().unwrap());
        assert_eq!(path_len, 5);

        let path = String::from_utf8(bytes[13..13 + 5].to_vec()).unwrap();
        assert_eq!(path, "/home");
    }

    #[test]
    fn test_server_packet_version_deserialization() {
        // Create test data: [length=5, type=SSH_FXP_VERSION, version=3]
        let mut data = vec![0, 0, 0, 5]; // length
        data.push(SSH_FXP_VERSION); // type
        data.extend_from_slice(&3u32.to_be_bytes()); // version

        let mut session = TestSession::new(data);
        let packet = ServerPacket::from_session(&mut session).unwrap();

        match packet {
            ServerPacket::Version { version } => assert_eq!(version, 3),
            _ => panic!("Expected Version packet"),
        }
    }

    #[test]
    fn test_server_packet_handle_deserialization() {
        let handle_data = b"test_handle".to_vec();

        let mut data = vec![0, 0, 0, 20]; // length: 4 (req_id) + 4 (handle_len) + 11 (handle) + 1 (type) = 20
        data.push(SSH_FXP_HANDLE);
        data.extend_from_slice(&42u32.to_be_bytes()); // request_id
        data.extend_from_slice(&(handle_data.len() as u32).to_be_bytes());
        data.extend_from_slice(&handle_data);

        let mut session = TestSession::new(data);
        let packet = ServerPacket::from_session(&mut session).unwrap();

        match packet {
            ServerPacket::Handle { request_id, handle } => {
                assert_eq!(request_id, 42);
                assert_eq!(handle, handle_data);
            }
            _ => panic!("Expected Handle packet"),
        }
    }
}

