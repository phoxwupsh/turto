use anyhow::{anyhow, Context, Result};
use chrono::Local;
use std::{env, time::Duration};
use tokio_graceful_shutdown::{SubsystemBuilder, SubsystemHandle, Toplevel};
use tracing::{error, info, level_filters::LevelFilter, warn};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{fmt::layer, layer::SubscriberExt, EnvFilter};
use turto::{
    bot::Turto,
    config::{help::load_help, load_config, message_template::load_templates},
};
use which::which_global;

#[tokio::main]
async fn main() {
    let _log_guard = match setup_log() {
        Ok(guard) => guard,
        Err(err) => {
            println!("{:#}", err);
            return;
        }
    };

    if let Err(err) = setup_env() {
        error!("{:#}", err);
        return;
    }

    let res = Toplevel::new(|subsystem| async move {
        subsystem.start(SubsystemBuilder::new("bot", bot_process));
    })
    .catch_signals()
    .handle_shutdown_requests(Duration::from_secs(10))
    .await;

    if let Err(err) = res {
        error!("Error occured while shutdown: {}", err);
    }
}

fn setup_env() -> Result<()> {
    if let Err(err) = dotenv::dotenv() {
        warn!("Failed to load .env file: {}", err);
    }
    which_global("yt-dlp").context("yt-dlp is not installed")?;
    load_config("config.toml")?;
    load_help("help.toml")?;
    load_templates("templates.toml")?;
    Ok(())
}

fn setup_log() -> Result<WorkerGuard> {
    let time = Local::now().format("%Y-%m-%d_%H-%M-%S").to_string();
    let log_filename = format!("{}.log", time);

    let file_appender = tracing_appender::rolling::never(env::current_dir()?, log_filename);
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

async fn bot_process(subsys: SubsystemHandle) -> Result<()> {
    let token = env::var("DISCORD_TOKEN").context("DISCORD_TOKEN is not set in the environment")?;
    if token.is_empty() {
        return Err(anyhow!("DISCORD_TOKEN is not set in the environment"));
    }
    let data_path = "guilds.json";
    let mut bot = Turto::new(token, data_path)
        .await
        .context("Turto client initialization failed: {}")?;

    let shard_manager = bot.shard_manager();

    tokio::select! {
        _ = subsys.on_shutdown_requested() => {
            shard_manager.shutdown_all().await;
        }
        _ = bot.start() => ()
    }

    let bytes = bot
        .save_data(data_path)
        .await
        .context(format!("Failed to write data to {}", data_path))?;
    info!("Write {} bytes to {}", bytes, data_path);
    Ok(())
}
