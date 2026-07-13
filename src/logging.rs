use tracing::{Event, Level, Subscriber};
use tracing_subscriber::layer::{Context, Layer};

use gwl_logger::Logger;

pub struct TracingToGwlLayer;

impl<S> Layer<S> for TracingToGwlLayer
where
    S: Subscriber,
{
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        let mut visitor = Visitor::default();
        event.record(&mut visitor);

        let msg = visitor.message.unwrap_or_else(|| "event".to_string());

        let logger = logger();

        match *event.metadata().level() {
            Level::ERROR => logger.error(&msg),
            Level::WARN => logger.warn(&msg),
            Level::INFO => logger.info(&msg),
            Level::DEBUG => logger.debug(&msg),
            Level::TRACE => logger.trace(&msg),
        }
    }
}

fn logger() -> &'static Logger {
    Logger::global()
}

#[derive(Default)]
struct Visitor {
    message: Option<String>,
}

impl tracing::field::Visit for Visitor {
    fn record_str(&mut self, _field: &tracing::field::Field, value: &str) {
        self.message = Some(value.to_string());
    }

    fn record_debug(&mut self, _field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        self.message = Some(format!("{:?}", value));
    }
}
