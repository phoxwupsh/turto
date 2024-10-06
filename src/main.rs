use anyhow::{Context, Result};
use chrono::Local;
use std::env;
use tracing::{error, level_filters::LevelFilter, warn};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{fmt::layer, layer::SubscriberExt, EnvFilter};
use turto::{
    bot::Turto,
    config::{help::load_help, load_config, message_template::load_templates},
    signal::wait_shutdown_signal,
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
        return error!("{:#}", err);
    }

    let token = match env::var("DISCORD_TOKEN") {
        Ok(token) => {
            if token.is_empty() {
                return error!("DISCORD_TOKEN is not set in the environment");
            }
            token
        }
        Err(err) => return error!("Failed to load DISCORD_TOKEN from the environment: {}", err),
    };

    let data_path = "guilds.json".to_string();
    let bot = match Turto::new(token, data_path).await {
        Ok(bot) => bot,
        Err(err) => return error!("Turto client initialization failed: {}", err),
    };

    bot_process(bot).await;
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

async fn bot_process(mut bot: Turto) {
    tokio::select! {
        _ = wait_shutdown_signal() => {
            bot.shutdown().await;
        }
        _ = bot.start() => ()
    }
}
