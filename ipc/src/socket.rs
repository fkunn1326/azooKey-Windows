use std::{env::temp_dir, ffi::CString};

use anyhow::{bail, ensure, Context, Result};
use windows::Win32::Networking::WinSock::{
    closesocket, connect, recv, send, socket, WSACleanup, WSAStartup, ADDRESS_FAMILY, AF_UNIX,
    SEND_RECV_FLAGS, SOCKADDR_UN, SOCKET, SOCKET_ERROR, SOCK_STREAM,
};

#[derive(serde::Serialize)]
pub struct KeyEvent {
    pub r#type: String,
    pub message: String,
}

#[derive(Clone, Debug)]
pub struct SocketManager {
    socket: SOCKET,
}

impl SocketManager {
    pub fn new() -> Result<Self> {
        unsafe {
            let mut wsa_data = std::mem::zeroed();
            WSAStartup(0x202, &mut wsa_data);

            let temp_path = temp_dir();
            let sock_path = temp_path.join("azookey.sock");

            // Create socket
            let sock = socket(AF_UNIX as i32, SOCK_STREAM, 0)?;
            if sock == SOCKET::default() {
                WSACleanup();
                panic!("Failed to create socket");
            }

            // Prepare the sockaddr_un structure
            let mut sock_addr: SOCKADDR_UN = std::mem::zeroed();
            sock_addr.sun_family = ADDRESS_FAMILY(AF_UNIX);
            let path = CString::new(sock_path.to_str().context("failed to convert str")?)
                .context("Failed to convert path")?;
            let path_bytes = path.as_bytes_with_nul();
            let max_len = sock_addr.sun_path.len().min(path_bytes.len());
            sock_addr.sun_path[..max_len].copy_from_slice(
                &path_bytes[..max_len]
                    .iter()
                    .map(|&b| b as i8)
                    .collect::<Vec<i8>>(),
            );

            // Connect to the server
            let result = connect(
                sock,
                &sock_addr as *const SOCKADDR_UN as *const _,
                std::mem::size_of::<SOCKADDR_UN>() as i32,
            );
            if result == SOCKET_ERROR {
                closesocket(sock);
                WSACleanup();
                panic!("Failed to connect to the server");
            }

            Ok(Self { socket: sock })
        }
    }

    pub fn get(&self, message: String) -> Result<String> {
        // send message
        let bytes_sent = unsafe { send(self.socket, message.as_bytes(), SEND_RECV_FLAGS(0)) };

        ensure!(bytes_sent == SOCKET_ERROR, "Failed to send message");

        // Receive a response (uint32 max)
        let mut buffer = [0u8; 4096];
        let bytes_received = unsafe { recv(self.socket, &mut buffer, SEND_RECV_FLAGS(0)) };
        if bytes_received > 0 {
            let response = String::from_utf8_lossy(&buffer[..bytes_received as usize]);

            Ok(response.to_string())
        } else {
            bail!("Failed to receive message");
        }
    }

    pub fn send(&self, message: String) -> Result<()> {
        // send message
        let bytes_sent = unsafe { send(self.socket, message.as_bytes(), SEND_RECV_FLAGS(0)) };
        ensure!(bytes_sent == SOCKET_ERROR, "Failed to send message");
        Ok(())
    }

    pub fn recv(&self) -> Result<String> {
        // Receive a response (uint32 max)
        let mut buffer = [0u8; 4096];
        let bytes_received = unsafe { recv(self.socket, &mut buffer, SEND_RECV_FLAGS(0)) };
        if bytes_received > 0 {
            let response = String::from_utf8_lossy(&buffer[..bytes_received as usize]);

            Ok(response.to_string())
        } else {
            bail!("Failed to receive message");
        }
    }

    pub fn debug(&self, message: String) -> Result<()> {
        let message = serde_json::to_string(&KeyEvent {
            r#type: "debug".to_string(),
            message: message.to_string(),
        })
        .context("Failed to serialize message")?;

        self.send(message)?;
        Ok(())
    }
}
