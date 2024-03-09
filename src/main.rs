use anyhow::{anyhow, Context, Result};
use std::{env, time::Duration};
use tokio_graceful_shutdown::{SubsystemBuilder, SubsystemHandle, Toplevel};
use tracing::{error, info, warn, Level};
use turto::{
    bot::Turto,
    config::{help::load_help, load_config, message_template::load_templates},
};
use which::which_global;

#[tokio::main]
async fn main() {
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
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber).unwrap();

    dotenv::dotenv().context("Failed to load .env file")?;
    which_global("yt-dlp").context("yt-dlp is not installed")?;
    load_config("config.toml")?;
    load_help("help.toml")?;
    load_templates("templates.toml")?;
    Ok(())
}

async fn bot_process(subsys: SubsystemHandle) -> Result<()> {
    let token = env::var("DISCORD_TOKEN").context("DISCORD_TOKEN is not set in the environment")?;
    if token.is_empty() {
        return Err(anyhow!("DISCORD_TOKEN is not set in the environment"));
    }
    let mut bot = Turto::new(token)
        .await
        .context("Turto client initialization failed: {}")?;

    let data_path = "guilds.json";
    bot.load_data(data_path).await.unwrap_or_else(|err| {
        warn!(
            "Failed to load data from {}: {}, will initialize new guilds data",
            data_path, err
        )
    });
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
