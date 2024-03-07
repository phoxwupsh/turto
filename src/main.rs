use anyhow::{Context, Result};
use std::{env, path::Path};
use tracing::{error, info, warn, Level};
use turto::{bot::Turto, config::{help::load_help, load_config, message_template::load_templates}};
use which::which_global;

#[tokio::main]
async fn main() {
    if let Err(err) = setup_env() {
        panic!("{:#}", err);
    }
    let token = env::var("DISCORD_TOKEN")
        .unwrap_or_else(|err| panic!("DISCORD_TOKEN is not set in the environment: {}", err));
    if token.is_empty() {
        panic!("DISCORD_TOKEN is not set in the environment");
    }

    let data_path = Path::new("guilds.json");

    let mut bot = Turto::new(token)
        .await
        .unwrap_or_else(|err| panic!("Turto client initialization failed: {}", err));
    bot.load_data(data_path).await.unwrap_or_else(|err| {
        warn!(
            "Failed to load data from {}: {}, will initialize new guilds data",
            data_path.display(),
            err
        )
    });

    if let Err(why) = bot.start().await {
        error!("Error occured while starting bot client: {}", why);
    } else {
        match bot.save_data(data_path).await {
            Ok(size) => info!("Write {} bytes to {}", size, data_path.display()),
            Err(err) => error!(
                "Error occured while writing data to {}: {}",
                data_path.display(),
                err
            ),
        }
    }
}

fn setup_env() -> Result<()> {
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber).unwrap();
    std::panic::set_hook(Box::new(|panic_info| {
        error!("{}", panic_info.payload().downcast_ref::<String>().unwrap());
    }));
    dotenv::dotenv().context("Failed to load .env file")?;
    which_global("yt-dlp").context("yt-dlp is not installed")?;
    load_config("config.toml")?;
    load_help("help.toml")?;
    load_templates("templates.toml")?;
    Ok(())
}
