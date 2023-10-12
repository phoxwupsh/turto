use std::{env, path::Path};
use tracing::{error, info, warn, Level};
use turto::bot::Turto;

#[tokio::main]
async fn main() {
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)
        .unwrap_or_else(|err| panic!("Tracing initialization failed: {}", err));
    std::panic::set_hook(Box::new(|panic_info| {
        error!("{}", panic_info.payload().downcast_ref::<String>().unwrap());
    }));
    if let Err(err) = dotenv::dotenv() {
        warn!("Failed to load .env file: {}", err);
    }
    let token = env::var("DISCORD_TOKEN")
        .unwrap_or_else(|err| panic!("Failed to load DISCORD_TOKEN in the environment: {}", err));
    if token.is_empty() {
        error!("You need to set DISCORD_TOKEN in the enviroment!");
    }
    let data_path = Path::new("guilds.json");

    let mut bot = Turto::new(token)
        .await
        .unwrap_or_else(|err| panic!("Turto client initialization failed: {}", err));
    bot.load_data(data_path)
        .await
        .unwrap_or_else(|err| error!("Failed to load data from {}: {}", data_path.display(), err));

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
