use tracing::level_filters::LevelFilter;
use tracing_subscriber::{layer::SubscriberExt as _, util::SubscriberInitExt as _, EnvFilter};

use super::{config, lib::common_error::CommonError};
pub(crate) fn init() -> Result<(), CommonError> {
    let crate_name = config::get_env::<String>("CRATE_NAME");
    let crate_log = config::get_env::<String>("CRATE_LOG");
    let directive = format!("{}={}", crate_name, crate_log);
    let filter = EnvFilter::builder()
        .with_default_directive(LevelFilter::INFO.into())
        .from_env()
        .map_err(|e| CommonError::from(e.to_string()))?
        .add_directive(
            directive
                .parse().unwrap()
        );
    tracing_subscriber::registry()
        .with(filter)
        .with(tracing_subscriber::fmt::layer())
        .init();
    Ok(())
}
/// Log a debug message at the INFO level.
pub(crate) fn info(message: &str) {
    tracing::info!("{}", message);
}

/// Log a debug message at the ERROR level.
pub fn error(message: &str) {
    tracing::error!("{}", message);
}

/// Log a debug message at the WARN level.
pub fn warn(message: &str) {
    tracing::warn!("{}", message);
}

/// Log a debug message at the DEBUG level.
pub fn debug(message: &str) {
    tracing::debug!("{}", message);
}
