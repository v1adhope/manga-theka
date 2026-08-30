use tracing_log::LogTracer;
use tracing_subscriber::{EnvFilter, Registry, fmt::time::UtcTime, layer::SubscriberExt};

pub fn init_subscriber(env_filter: &str) {
    LogTracer::init().expect("failed to set global logger");

    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(env_filter));
    let fmt_layer = tracing_subscriber::fmt::layer()
        .json()
        .with_timer(UtcTime::rfc_3339());
    let subscriber = Registry::default().with(filter).with(fmt_layer);

    tracing::subscriber::set_global_default(subscriber).expect("failed to set global subscriber");
}
