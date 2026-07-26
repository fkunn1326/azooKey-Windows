mod pipe_stream;

use shared::conversion::conversion_service_server::{ConversionService, ConversionServiceServer};
use shared::conversion::{ConvertRequest, ConvertResponse};
use tonic::{transport::Server, Request, Response, Status};

struct ConversionServiceImpl;

#[tonic::async_trait]
impl ConversionService for ConversionServiceImpl {
    async fn convert(
        &self,
        request: Request<ConvertRequest>,
    ) -> Result<Response<ConvertResponse>, Status> {
        let text = request.into_inner().text;
        tracing::info!("Received: {}", text);

        Ok(Response::new(ConvertResponse { text }))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    let pipe_name = "azookey-conversion";
    tracing::info!(
        "Starting conversion server on \\\\.\\pipe\\{}",
        pipe_name
    );

    let stream = pipe_stream::create_pipe_stream(pipe_name);
    let svc = ConversionServiceServer::new(ConversionServiceImpl);

    Server::builder()
        .add_service(svc)
        .serve_with_incoming(stream)
        .await?;

    Ok(())
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;
    use shared::conversion::conversion_service_client::ConversionServiceClient;
    use shared::conversion::ConvertRequest;
    use tokio::net::windows::named_pipe::ClientOptions;
    use tokio::time::Duration;
    use tonic::transport::Endpoint;
    use hyper_util::rt::TokioIo;
    use tower::service_fn;

    #[tokio::test]
    async fn test_grpc_roundtrip() {
        let pipe_name = format!("azookey-test-grpc-{}", std::process::id());

        let stream = pipe_stream::create_pipe_stream(&pipe_name);
        let server = ConversionServiceServer::new(ConversionServiceImpl);
        let server_handle = tokio::spawn(async move {
            Server::builder()
                .add_service(server)
                .serve_with_incoming(stream)
                .await
                .unwrap();
        });

        tokio::time::sleep(Duration::from_millis(50)).await;

        let pipe_path = format!(r"\\.\pipe\{}", pipe_name);
        let channel = Endpoint::try_from("http://[::]:50051")
            .unwrap()
            .connect_with_connector(service_fn(move |_| {
                let pipe_path = pipe_path.clone();
                async move {
                    let client = ClientOptions::new().open(&pipe_path)?;
                    Ok::<_, std::io::Error>(TokioIo::new(client))
                }
            }))
            .await
            .unwrap();

        let mut client = ConversionServiceClient::new(channel);
        let response = client
            .convert(ConvertRequest {
                text: "hello".into(),
            })
            .await
            .unwrap();
        assert_eq!(response.into_inner().text, "hello");

        server_handle.abort();
    }
}
