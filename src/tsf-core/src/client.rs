use std::sync::OnceLock;
use std::time::Duration;

use tokio::net::windows::named_pipe::ClientOptions;
use tokio::runtime::Runtime;
use tokio::time;
use hyper_util::rt::TokioIo;
use tonic::transport::Endpoint;
use tower::service_fn;
use windows::Win32::Foundation::ERROR_PIPE_BUSY;

use shared::conversion::conversion_service_client::ConversionServiceClient;
use shared::conversion::ConvertRequest;

const PIPE_NAME: &str = r"\\.\pipe\azookey-conversion";
const DUMMY_URI: &str = "http://[::]:50051";

static RUNTIME: OnceLock<Runtime> = OnceLock::new();

fn get_runtime() -> &'static Runtime {
    RUNTIME.get_or_init(|| Runtime::new().expect("Failed to create tokio runtime"))
}

pub fn convert(text: &str) -> String {
    let text = text.to_string();
    get_runtime().block_on(async move {
        match do_convert(&text).await {
            Ok(result) => result,
            Err(e) => {
                tracing::warn!("Conversion server unavailable: {:?}, using fallback", e);
                text
            }
        }
    })
}

async fn do_convert(
    text: &str,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let channel = Endpoint::try_from(DUMMY_URI)?
        .connect_with_connector(service_fn(|_| async {
            let client = loop {
                match ClientOptions::new().open(PIPE_NAME) {
                    Ok(c) => break c,
                    Err(e) if e.raw_os_error() == Some(ERROR_PIPE_BUSY.0 as i32) => {
                        time::sleep(Duration::from_millis(10)).await;
                        continue;
                    }
                    Err(e) => return Err(e),
                }
            };
            Ok::<_, std::io::Error>(TokioIo::new(client))
        }))
        .await?;

    let mut client = ConversionServiceClient::new(channel);
    let response = client
        .convert(ConvertRequest {
            text: text.to_string(),
        })
        .await?;
    Ok(response.into_inner().text)
}
