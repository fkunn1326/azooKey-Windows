use std::fmt::Write as _;
use tracing::field::{Field, Visit};
use tracing_core::LevelFilter;
use tracing_subscriber::filter::Targets;
use tracing_subscriber::{layer::SubscriberExt as _, util::SubscriberInitExt};
use windows::{core::PCWSTR, Win32::System::Diagnostics::Debug::OutputDebugStringW};

use crate::extension::StringExt as _;
use crate::tracing_chrome::{ChromeLayerBuilder, EventOrSpan};

const LOG_FOLDER: &str = "D:/azookey-windows/logs";

pub struct StringVisitor<'a> {
    string: &'a mut String,
}

impl<'a> Visit for StringVisitor<'a> {
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        // do nothing
        if field.name() == "message" {
            write!(self.string, "{:?}", value).unwrap();
        }
    }
}

pub fn setup_logger() -> anyhow::Result<()> {
    let timestamp = chrono::Local::now().format("%Y-%m-%d-%H.%M.%S");
    let path = format!("{}/{}.json", LOG_FOLDER, timestamp);

    let writer = {
        if let Ok(file) = std::fs::File::create(&path) {
            file
        } else {
            return Ok(());
        }
    };

    let builder = ChromeLayerBuilder::new()
        .file(writer)
        .include_locations(true)
        .include_args(true)
        .name_fn(Box::new(|event_or_span| match event_or_span {
            EventOrSpan::Event(event) => {
                let message = {
                    let mut message = String::new();
                    event.record(&mut StringVisitor {
                        string: &mut message,
                    });
                    message
                };

                let (level, file, line) = {
                    let metadeta = event.metadata();
                    let level = metadeta.level().as_str();
                    let file = metadeta.file().unwrap_or_default();
                    let line = metadeta.line().unwrap_or_default();

                    (level, file, line)
                };

                let str = format!("[{}: {}:{}] {}", level, file, line, message);
                let wide: Vec<u16> = str.as_str().to_wide_16();
                unsafe { OutputDebugStringW(PCWSTR(wide.as_ptr())) };

                message
            }
            EventOrSpan::Span(span) => span.metadata().name().to_string(),
        }));

    let chrome_layer = builder.build();

    // ignore traces from other crates
    let filter = Targets::new()
        .with_target("azookey_windows", LevelFilter::DEBUG)
        .with_default(LevelFilter::OFF);

    tracing_subscriber::registry()
        .with(filter)
        .with(chrome_layer)
        .init();

    Ok(())
}
