use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;

use async_stream::stream;
use futures_core::stream::Stream;
use tokio::{
    io::{AsyncRead, AsyncWrite},
    net::windows::named_pipe::{NamedPipeServer, PipeMode, ServerOptions},
};
use tonic::transport::server::Connected;
use tracing::{info, error};
use windows::Win32::Foundation::{LocalFree, HLOCAL};
use windows::Win32::Security::Authorization::ConvertStringSecurityDescriptorToSecurityDescriptorW;
use windows::Win32::Security::{PSECURITY_DESCRIPTOR, SECURITY_ATTRIBUTES};
use windows::core::w;

// ACL設定: サンドボックス内のアプリ(AppContainer, Restricted Token等)からの接続を許可
// 参考: https://nathancorvussolis.blogspot.com/2018/05/windows-ime-security.html
fn create_pipe(name: &str, is_first_instance: bool) -> std::io::Result<NamedPipeServer> {
    info!("Creating pipe: {} (first={})", name, is_first_instance);

    let mut descripter = PSECURITY_DESCRIPTOR(std::ptr::null_mut());
    unsafe {
        ConvertStringSecurityDescriptorToSecurityDescriptorW(
            w!("D:(A;;GA;;;AC)(A;;GA;;;RC)(A;;GA;;;SY)(A;;GA;;;BA)(A;;GA;;;BU)S:(ML;;NW;;;LW)"),
            1,
            &mut descripter,
            None,
        )
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, format!("{:?}", e)))?;
    }

    let mut attributes = SECURITY_ATTRIBUTES {
        nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
        lpSecurityDescriptor: descripter.0,
        bInheritHandle: false.into(),
    };

    let mut opts = ServerOptions::new();
    opts.first_pipe_instance(is_first_instance)
        .pipe_mode(PipeMode::Byte)
        .max_instances(2)
        .in_buffer_size(4096)
        .out_buffer_size(4096)
        .reject_remote_clients(false);

    let pipe_path = format!("\\\\.\\pipe\\{}", name);

    let server = unsafe {
        opts.create_with_security_attributes_raw(
            &pipe_path,
            &mut attributes as *mut SECURITY_ATTRIBUTES as *mut _,
        )
    }?;

    unsafe {
        let _ = LocalFree(HLOCAL(descripter.0));
    }

    info!("Pipe created: {}", pipe_path);
    Ok(server)
}

pub struct TonicNamedPipeServer {
    inner: NamedPipeServer,
}

impl TonicNamedPipeServer {
    pub fn new(inner: NamedPipeServer) -> Self {
        Self { inner }
    }
}

impl Connected for TonicNamedPipeServer {
    type ConnectInfo = ();
    fn connect_info(&self) -> Self::ConnectInfo {
        ()
    }
}

impl AsyncRead for TonicNamedPipeServer {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.inner).poll_read(cx, buf)
    }
}

impl AsyncWrite for TonicNamedPipeServer {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, std::io::Error>> {
        Pin::new(&mut self.inner).poll_write(cx, buf)
    }

    fn poll_flush(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Result<(), std::io::Error>> {
        Pin::new(&mut self.inner).poll_flush(cx)
    }

    fn poll_shutdown(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Result<(), std::io::Error>> {
        Pin::new(&mut self.inner).poll_shutdown(cx)
    }
}

async fn create_pipe_with_retry(name: &str) -> std::io::Result<NamedPipeServer> {
    loop {
        match create_pipe(name, false) {
            Ok(s) => return Ok(s),
            Err(e) => {
                error!("Failed to create pipe: {}, retrying...", e);
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        }
    }
}

pub fn create_pipe_stream(
    name: &str,
) -> impl Stream<Item = std::io::Result<TonicNamedPipeServer>> {
    let name = name.to_string();
    stream! {
        let mut server = create_pipe(&name, true)?;
        loop {
            info!("Waiting for client...");
            server.connect().await?;
            info!("Client connected");

            let next_server = create_pipe_with_retry(&name).await;

            let connected = TonicNamedPipeServer::new(server);
            yield Ok(connected);

            server = next_server?;
        }
    }
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::windows::named_pipe::ClientOptions;
    use tokio_stream::StreamExt;
    use windows::Win32::Foundation::{ERROR_FILE_NOT_FOUND, ERROR_PIPE_BUSY};

    fn unique_pipe_name(test_name: &str) -> String {
        format!(
            "azookey-test-{}-{}-{}",
            std::process::id(),
            test_name,
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        )
    }

    async fn connect_to_pipe(
        pipe_path: &str,
    ) -> std::io::Result<tokio::net::windows::named_pipe::NamedPipeClient> {
        loop {
            match ClientOptions::new().open(pipe_path) {
                Ok(client) => return Ok(client),
                Err(e)
                    if e.raw_os_error() == Some(ERROR_PIPE_BUSY.0 as i32)
                        || e.raw_os_error() == Some(ERROR_FILE_NOT_FOUND.0 as i32) =>
                {
                    tokio::time::sleep(Duration::from_millis(10)).await;
                }
                Err(e) => return Err(e),
            }
        }
    }

    #[tokio::test]
    async fn test_create_pipe_success() {
        let name = unique_pipe_name("success");
        let result = create_pipe(&name, true);
        assert!(
            result.is_ok(),
            "Expected pipe creation to succeed, got: {:?}",
            result.err()
        );
    }

    #[tokio::test]
    async fn test_create_pipe_empty_name() {
        let result = create_pipe("", true);
        assert!(result.is_err(), "Expected pipe creation with empty name to fail");
    }

    #[tokio::test]
    async fn test_create_pipe_first_instance_conflict() {
        let name = unique_pipe_name("first_instance");

        let first = create_pipe(&name, true);
        assert!(first.is_ok(), "First instance creation should succeed");

        let second = create_pipe(&name, true);
        assert!(second.is_err(), "Second first-instance creation should fail");

        drop(first);

        let third = create_pipe(&name, false);
        assert!(
            third.is_ok(),
            "Non-first instance creation should succeed after first is dropped"
        );
    }

    #[tokio::test]
    async fn test_tonic_named_pipe_server_new() {
        let name = unique_pipe_name("tonic_new");
        let server = create_pipe(&name, true).expect("Failed to create pipe");
        let tonic_server = TonicNamedPipeServer::new(server);
        drop(tonic_server);
    }

    #[tokio::test]
    async fn test_pipe_stream_yields_server_on_connect() {
        let name = unique_pipe_name("stream_yield");
        let pipe_path = format!(r"\\.\pipe\{}", name);

        let stream = create_pipe_stream(&name);
        tokio::pin!(stream);

        let client_path = pipe_path.clone();
        let client_handle = tokio::spawn(async move { connect_to_pipe(&client_path).await });

        let result = tokio::time::timeout(Duration::from_secs(5), stream.as_mut().next()).await;

        assert!(result.is_ok(), "Stream should yield a server within timeout");
        let server_result = result.unwrap();
        assert!(server_result.is_some(), "Stream should yield a non-None item");
        let server = server_result.unwrap();
        assert!(server.is_ok(), "Stream item should be Ok");

        let _ = client_handle.await;
    }

    #[tokio::test]
    async fn test_pipe_stream_continuity() {
        let name = unique_pipe_name("stream_continuity");
        let pipe_path = format!(r"\\.\pipe\{}", name);

        let stream = create_pipe_stream(&name);
        tokio::pin!(stream);

        // First connection
        let client_path1 = pipe_path.clone();
        let client1_handle = tokio::spawn(async move { connect_to_pipe(&client_path1).await });

        let server1 = tokio::time::timeout(Duration::from_secs(5), stream.as_mut().next())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        let client1 = client1_handle.await.unwrap().unwrap();

        drop(client1);
        drop(server1);

        // Second connection - stream should still be alive
        let client_path2 = pipe_path.clone();
        let client2_handle = tokio::spawn(async move { connect_to_pipe(&client_path2).await });

        let server2 = tokio::time::timeout(Duration::from_secs(5), stream.as_mut().next())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        let _client2 = client2_handle.await.unwrap().unwrap();

        drop(_client2);
        drop(server2);
    }

    #[tokio::test]
    async fn test_pipe_data_roundtrip() {
        let name = unique_pipe_name("data_roundtrip");
        let pipe_path = format!(r"\\.\pipe\{}", name);

        let stream = create_pipe_stream(&name);
        tokio::pin!(stream);

        let client_path = pipe_path.clone();
        let client_handle = tokio::spawn(async move { connect_to_pipe(&client_path).await });

        let mut server = tokio::time::timeout(Duration::from_secs(5), stream.as_mut().next())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        let mut client = client_handle.await.unwrap().unwrap();

        // Client -> Server
        let test_data = b"Hello, pipe!";
        client.write_all(test_data).await.unwrap();
        let mut buf = vec![0u8; test_data.len()];
        server.read_exact(&mut buf).await.unwrap();
        assert_eq!(&buf, test_data);

        // Server -> Client
        let test_data2 = b"Hello, client!";
        server.write_all(test_data2).await.unwrap();
        let mut buf2 = vec![0u8; test_data2.len()];
        client.read_exact(&mut buf2).await.unwrap();
        assert_eq!(&buf2, test_data2);
    }
}
