// modifying from https://github.com/thoren-d/tracing-chrome

use serde::Serialize;
use tracing::field::Visit;
use tracing_core::{field::Field, span, Event, Subscriber};
use tracing_subscriber::{
    layer::Context,
    registry::{LookupSpan, SpanRef},
    Layer,
};

use serde_json::Value as JsonValue;
use std::{
    fmt::Write as _,
    fs::File,
    io::{Seek, Write},
    marker::PhantomData,
    sync::{Arc, Mutex},
};

type NameFn<S> = Box<dyn Fn(&EventOrSpan<'_, '_, S>) -> String + Send + Sync>;
type Object = serde_json::Map<String, JsonValue>;

#[derive(Serialize)]
struct ChromeTrace {
    #[serde(skip)]
    entries: Mutex<Vec<TraceEntry>>,
}

#[derive(Serialize)]
struct TraceEntry {
    // Chrome Trace Event format fields
    ph: String,   // Phase type
    pid: u32,     // Process ID
    tid: u32,     // Thread ID
    ts: f64,      // Timestamp
    name: String, // Event name
    cat: String,  // Event category
    #[serde(skip_serializing_if = "Option::is_none")]
    s: Option<String>, // Scope - only for instant events
    #[serde(skip_serializing_if = "Option::is_none")]
    args: Option<serde_json::Map<String, serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = ".file")]
    file: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = ".line")]
    line: Option<u32>,
}

impl ChromeTrace {
    fn new() -> Self {
        ChromeTrace {
            entries: Mutex::new(Vec::new()),
        }
    }

    fn add_entry(&self, entry: TraceEntry) {
        let mut entries = self.entries.lock().unwrap();
        entries.push(entry);
    }

    fn write_entries(&self, writer: &mut File) -> std::io::Result<()> {
        let entries = self.entries.lock().unwrap();
        // clear file
        writer.set_len(0)?;
        writer.seek(std::io::SeekFrom::Start(0))?;

        let json = serde_json::to_string_pretty(&*entries)?;
        writer.write_all(json.as_bytes())?;

        Ok(())
    }
}

pub struct ChromeLayer<S>
where
    S: Subscriber + for<'span> LookupSpan<'span>,
{
    writer: Mutex<File>,
    trace: ChromeTrace,
    start: std::time::Instant,
    include_args: bool,
    include_locations: bool,
    name_fn: Option<NameFn<S>>,
    cat_fn: Option<NameFn<S>>,
    _inner: PhantomData<S>,
}

#[derive(Default)]
pub struct ChromeLayerBuilder<S>
where
    S: Subscriber + for<'span> LookupSpan<'span>,
{
    out_writer: Option<File>,
    name_fn: Option<NameFn<S>>,
    cat_fn: Option<NameFn<S>>,
    include_args: bool,
    include_locations: bool,
    _inner: PhantomData<S>,
}

impl<S> ChromeLayerBuilder<S>
where
    S: Subscriber + for<'span> LookupSpan<'span>,
{
    pub fn new() -> Self {
        ChromeLayerBuilder {
            out_writer: None,
            name_fn: None,
            cat_fn: None,
            include_args: false,
            include_locations: true,
            _inner: PhantomData,
        }
    }

    /// Set the file to which to output the trace.
    ///
    /// Defaults to `./trace-{unix epoch in micros}.json`.
    ///
    /// # Panics
    ///
    /// If `file` could not be opened/created. To handle errors,
    /// open a file and pass it to [`writer`](crate::ChromeLayerBuilder::writer) instead.
    pub fn file(mut self, file: File) -> Self {
        self.out_writer = Some(file);
        self
    }

    /// Supply an arbitrary writer to which to write trace contents.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use tracing_chrome::ChromeLayerBuilder;
    /// # use tracing_subscriber::prelude::*;
    /// let (layer, guard) = ChromeLayerBuilder::new().writer(std::io::sink()).build();
    /// # tracing_subscriber::registry().with(layer).init();
    /// ```
    // pub fn writer<W: Write + Send + 'static>(mut self, writer: W) -> Self {
    //     self.out_writer = Some();
    //     self
    // }

    /// Include arguments in each trace entry.
    ///
    /// Defaults to `false`.
    ///
    /// Includes the arguments used when creating a span/event
    /// in the "args" section of the trace entry.
    pub fn include_args(mut self, include: bool) -> Self {
        self.include_args = include;
        self
    }

    /// Include file+line with each trace entry.
    ///
    /// Defaults to `true`.
    ///
    /// This can add quite a bit of data to the output so turning
    /// it off might be helpful when collecting larger traces.
    pub fn include_locations(mut self, include: bool) -> Self {
        self.include_locations = include;
        self
    }

    pub fn name_fn(mut self, name_fn: NameFn<S>) -> Self {
        self.name_fn = Some(name_fn);
        self
    }

    pub fn build(self) -> ChromeLayer<S> {
        let writer = self.out_writer.unwrap();

        ChromeLayer {
            writer: Mutex::new(writer),
            trace: ChromeTrace::new(),
            start: std::time::Instant::now(),
            name_fn: self.name_fn,
            cat_fn: self.cat_fn,
            include_args: self.include_args,
            include_locations: self.include_locations,
            _inner: PhantomData,
        }
    }
}

pub enum EventOrSpan<'a, 'b, S>
where
    S: Subscriber + for<'span> LookupSpan<'span>,
{
    Event(&'a Event<'b>),
    Span(&'a SpanRef<'b, S>),
}

impl<S> ChromeLayer<S>
where
    S: Subscriber + for<'span> LookupSpan<'span>,
{
    fn write_event(&self, ph: &str, ts: f64, data: EventOrSpan<S>) -> anyhow::Result<()> {
        let name = self.name_fn.as_ref().map(|name_fn| name_fn(&data));
        let target = self.cat_fn.as_ref().map(|cat_fn| cat_fn(&data));
        let meta = match data {
            EventOrSpan::Event(e) => e.metadata(),
            EventOrSpan::Span(s) => s.metadata(),
        };

        let args = if self.include_args {
            match data {
                EventOrSpan::Event(e) => {
                    let mut args_map = Object::new();
                    e.record(&mut JsonVisitor {
                        object: &mut args_map,
                    });
                    if !args_map.is_empty() {
                        Some(args_map)
                    } else {
                        None
                    }
                }
                EventOrSpan::Span(s) => s
                    .extensions()
                    .get::<ArgsWrapper>()
                    .filter(|w| !w.args.is_empty())
                    .map(|w| (*w.args).clone()),
            }
        } else {
            None
        };

        let (file, line) = if self.include_locations {
            (meta.file().map(String::from), meta.line())
        } else {
            (None, None)
        };

        let entry = TraceEntry {
            ph: ph.to_string(),
            pid: 1,
            tid: 1,
            ts,
            name: name.unwrap_or_else(|| meta.name().into()),
            cat: target.unwrap_or_else(|| meta.target().into()),
            s: if ph == "i" {
                Some("t".to_string())
            } else {
                None
            },
            args,
            file,
            line,
        };

        self.trace.add_entry(entry);

        let mut writer = self.writer.lock().unwrap();
        self.trace.write_entries(&mut *writer)?;

        Ok(())
    }

    fn write_record(
        &self,
        ts: f64,
        span: &SpanRef<'_, S>,
        values: &span::Record<'_>,
    ) -> anyhow::Result<()> {
        let mut args = Object::new();
        values.record(&mut JsonVisitor { object: &mut args });
        if args.is_empty() {
            return Ok(());
        }

        let entry = TraceEntry {
            ph: "N".to_string(),
            pid: 1,
            tid: 1,
            ts,
            name: self
                .name_fn
                .as_ref()
                .map(|f| f(&EventOrSpan::Span(span)))
                .unwrap_or_else(|| span.metadata().name().into()),
            cat: self
                .cat_fn
                .as_ref()
                .map(|f| f(&EventOrSpan::Span(span)))
                .unwrap_or_else(|| span.metadata().target().into()),
            s: None,
            args: Some(args),
            file: self
                .include_locations
                .then(|| span.metadata().file().map(String::from))
                .flatten(),
            line: self
                .include_locations
                .then(|| span.metadata().line())
                .flatten(),
        };

        self.trace.add_entry(entry);

        let mut writer = self.writer.lock().unwrap();
        self.trace.write_entries(&mut *writer)?;

        Ok(())
    }

    fn get_ts(&self) -> f64 {
        self.start.elapsed().as_nanos() as f64 / 1000.0
    }
}

impl<S> Layer<S> for ChromeLayer<S>
where
    S: Subscriber + for<'span> LookupSpan<'span>,
{
    fn on_enter(&self, id: &span::Id, ctx: Context<'_, S>) {
        let ts = self.get_ts();
        if let Some(span) = ctx.span(id) {
            self.write_event("B", ts, EventOrSpan::Span(&span));
        }
    }

    fn on_record(&self, id: &span::Id, values: &span::Record<'_>, ctx: Context<'_, S>) {
        if self.include_args {
            let ts = self.get_ts();
            if let Some(span) = ctx.span(id) {
                if let Some(args_wrapper) = span.extensions_mut().get_mut::<ArgsWrapper>() {
                    let args = Arc::make_mut(&mut args_wrapper.args);
                    values.record(&mut JsonVisitor { object: args });
                } else {
                    let mut args = Object::new();
                    values.record(&mut JsonVisitor { object: &mut args });
                    span.extensions_mut().insert(ArgsWrapper {
                        args: Arc::new(args),
                    });
                }

                // レコードイベントを書き込む
                self.write_record(ts, &span, values);
            }
        }
    }

    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        let ts = self.get_ts();
        self.write_event("i", ts, EventOrSpan::Event(event));
    }

    fn on_exit(&self, id: &span::Id, ctx: Context<'_, S>) {
        let ts = self.get_ts();
        if let Some(span) = ctx.span(id) {
            self.write_event("E", ts, EventOrSpan::Span(&span));
        }
    }

    fn on_new_span(&self, attrs: &span::Attributes<'_>, id: &span::Id, ctx: Context<'_, S>) {
        if self.include_args {
            let mut args = Object::new();
            attrs.record(&mut JsonVisitor { object: &mut args });
            if let Some(span) = ctx.span(id) {
                span.extensions_mut().insert(ArgsWrapper {
                    args: Arc::new(args),
                });
            }
        }
    }

    fn on_close(&self, id: span::Id, ctx: Context<'_, S>) {
        let ts = self.get_ts();
        if let Some(span) = ctx.span(&id) {
            self.write_event("E", ts, EventOrSpan::Span(&span));
        }
    }
}

struct JsonVisitor<'a> {
    object: &'a mut Object,
}

impl<'a> Visit for JsonVisitor<'a> {
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        self.object
            .insert(field.name().to_owned(), format!("{value:?}").into());
    }
}

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

struct ArgsWrapper {
    args: Arc<Object>,
}

// Send + Sync実装を明示的に追加
unsafe impl<S> Send for ChromeLayer<S> where S: Subscriber + for<'span> LookupSpan<'span> {}
unsafe impl<S> Sync for ChromeLayer<S> where S: Subscriber + for<'span> LookupSpan<'span> {}
