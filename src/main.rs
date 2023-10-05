use std::env;
use tracing::{error, Level};
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
    dotenv::dotenv().unwrap_or_else(|err| panic!("Error loading .env file: {}", err));
    let token = env::var("DISCORD_TOKEN")
        .unwrap_or_else(|err| panic!("Error loading DISCORD_TOKEN in the environment: {}", err));

    let mut bot = Turto::new(token, "guilds.json")
        .await
        .unwrap_or_else(|err| panic!("Turto client initialization failed: {}", err));

    if let Err(why) = bot.start().await {
        error!("Error occured while start bot client: {}", why);
    }
}
