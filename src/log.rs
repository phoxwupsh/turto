use chrono::Local;
use std::env;
use tracing::level_filters::LevelFilter;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{EnvFilter, fmt::layer, layer::SubscriberExt};

pub fn setup_log() -> std::io::Result<WorkerGuard> {
    let time = Local::now().format("%Y-%m-%d_%H-%M-%S").to_string();
    let log_filename = format!("{}.log", time);

    let file_appender =
        tracing_appender::rolling::never(env::current_dir()?.join("log"), log_filename);
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    let file_layer = layer().with_writer(non_blocking).with_ansi(false);
    let console_layer = layer().with_writer(std::io::stdout);
    let subscriber = tracing_subscriber::registry()
        .with(file_layer)
        .with(console_layer)
        .with(
            EnvFilter::builder()
                .with_default_directive(LevelFilter::INFO.into())
                .with_env_var("TURTO_LOG")
                .from_env_lossy(),
        );

    tracing::subscriber::set_global_default(subscriber).unwrap();
    Ok(guard)
}
