use std::fmt;

use clap::ValueEnum;
use tracing::field::{Field, Visit};
use tracing::{Event, Subscriber};
use tracing_subscriber::EnvFilter;
use tracing_subscriber::fmt::format::Writer;
use tracing_subscriber::fmt::{FmtContext, FormatEvent, FormatFields};
use tracing_subscriber::prelude::*;
use tracing_subscriber::registry::LookupSpan;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, ValueEnum)]
pub enum LogFormat {
    #[default]
    Human,
    Json,
}

pub fn init_logging(verbose: bool, format: LogFormat) {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        EnvFilter::new(if verbose {
            "ferrflow=debug"
        } else {
            "ferrflow=info"
        })
    });

    let registry = tracing_subscriber::registry().with(filter);
    match format {
        LogFormat::Human => registry
            .with(
                tracing_subscriber::fmt::layer()
                    .event_format(MessageOnly)
                    .with_writer(std::io::stderr),
            )
            .init(),
        LogFormat::Json => registry
            .with(
                tracing_subscriber::fmt::layer()
                    .json()
                    .with_writer(std::io::stderr),
            )
            .init(),
    }
}

struct MessageOnly;

impl<S, N> FormatEvent<S, N> for MessageOnly
where
    S: Subscriber + for<'a> LookupSpan<'a>,
    N: for<'a> FormatFields<'a> + 'static,
{
    fn format_event(
        &self,
        _ctx: &FmtContext<'_, S, N>,
        mut writer: Writer<'_>,
        event: &Event<'_>,
    ) -> fmt::Result {
        let mut visitor = MessageVisitor {
            writer: &mut writer,
            result: Ok(()),
        };
        event.record(&mut visitor);
        visitor.result?;
        writeln!(writer)
    }
}

struct MessageVisitor<'a, 'b> {
    writer: &'a mut Writer<'b>,
    result: fmt::Result,
}

impl Visit for MessageVisitor<'_, '_> {
    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        if self.result.is_ok() && field.name() == "message" {
            self.result = write!(self.writer, "{value:?}");
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use tracing_subscriber::fmt::MakeWriter;
    use tracing_subscriber::prelude::*;

    use super::MessageOnly;

    #[derive(Clone, Default)]
    struct BufWriter(Arc<Mutex<Vec<u8>>>);

    impl std::io::Write for BufWriter {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(buf);
            Ok(buf.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    impl<'a> MakeWriter<'a> for BufWriter {
        type Writer = BufWriter;
        fn make_writer(&'a self) -> Self::Writer {
            self.clone()
        }
    }

    #[test]
    fn human_layer_emits_only_the_message_with_fields_hidden() {
        let buf = BufWriter::default();
        let subscriber = tracing_subscriber::registry().with(
            tracing_subscriber::fmt::layer()
                .event_format(MessageOnly)
                .with_writer(buf.clone()),
        );

        tracing::subscriber::with_default(subscriber, || {
            tracing::info!(remote = "origin", branch = "main", "✓ pushed and verified");
        });

        let out = String::from_utf8(buf.0.lock().unwrap().clone()).unwrap();
        // Message text only — no timestamp, level, target, or key=value fields.
        assert_eq!(out, "✓ pushed and verified\n");
    }
}
