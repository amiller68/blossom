mod chat;
#[allow(dead_code)]
mod config;
#[allow(dead_code)]
mod database;
mod engine;
mod state;
mod tool_call;
mod version;
pub use chat::Chat;
pub use config::Config;
pub use state::State;
pub use tool_call::ToolCall;

/// Sets up system panics to use the tracing infrastructure to log reported issues. This doesn't
/// prevent the panic from taking out the service but ensures that it and any available information
/// is properly reported using the standard logging mechanism.
pub fn register_panic_logger() {
    std::panic::set_hook(Box::new(|panic| match panic.location() {
        Some(loc) => {
            tracing::error!(
                message = %panic,
                panic.file = loc.file(),
                panic.line = loc.line(),
                panic.column = loc.column(),
            );
        }
        None => tracing::error!(message = %panic),
    }));
}

pub fn report_version() {
    let version = version::Version::new();
    tracing::info!(
        build_profile = ?version.build_profile(),
        features = ?version.build_features(),
        version = ?version.version(),
        repo_version = ?version.repo_version(),
        "service starting up"
    );
}

#[cfg(test)]
mod tests;
