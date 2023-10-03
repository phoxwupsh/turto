use serenity::{
    client::Client,
    framework::standard::{buckets::LimitedFor, StandardFramework},
    model::prelude::GuildId,
    prelude::GatewayIntents,
};
use songbird::SerenityInit;
use std::{collections::HashMap, env, sync::Arc};
use tokio::sync::{Mutex, RwLock};
use tracing::{error, info, Level};
use turto::{
    commands::TURTOCOMMANDS_GROUP,
    config::TurtoConfigProvider,
    handlers::{before::before_hook, SerenityEventHandler},
    models::{guild::config::GuildConfig, playlist::Playlist},
    typemap::{config::GuildConfigs, playing::Playing, playlist::Playlists},
    utils::json::{read_json, write_json},
};

#[tokio::main]
async fn main() {
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)
        .unwrap_or_else(|err| panic!("Tracing initialization failed: {}", err));
    dotenv::dotenv().unwrap_or_else(|err| panic!("Error loading .env file: {}", err));
    let token = env::var("DISCORD_TOKEN")
        .unwrap_or_else(|err| panic!("Error loading DISCORD_TOKEN in the environment: {}", err));
    let config = TurtoConfigProvider::get();

    let playlists: HashMap<GuildId, Playlist> = read_json("playlists.json").unwrap_or_default();
    let guild_configs: HashMap<GuildId, GuildConfig> = read_json("guilds.json").unwrap_or_default();

    let framework = StandardFramework::new()
        .configure(|c| c.prefix(config.command_prefix.clone()))
        .bucket("turto", |bucket| {
            bucket
                .delay(config.command_delay)
                .await_ratelimits(1)
                .limit_for(LimitedFor::Guild)
        })
        .await
        .group(&TURTOCOMMANDS_GROUP)
        .before(before_hook);

    let intents = GatewayIntents::non_privileged() | GatewayIntents::MESSAGE_CONTENT;
    let mut client = Client::builder(&token, intents)
        .event_handler(SerenityEventHandler)
        .framework(framework)
        .intents(intents)
        .register_songbird()
        .type_map_insert::<Playing>(Arc::new(RwLock::new(HashMap::default())))
        .type_map_insert::<Playlists>(Arc::new(Mutex::new(playlists)))
        .type_map_insert::<GuildConfigs>(Arc::new(Mutex::new(guild_configs)))
        .await
        .unwrap_or_else(|err| panic!("Error creating client: {}", err));
    
    let shard_manager = client.shard_manager.clone();
    let shard_manager_panic = shard_manager.clone();

    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        default_hook(panic_info);
        let shard_manager_panic_ = shard_manager_panic.clone();
        tokio::spawn(async move {
            shard_manager_panic_.lock().await.shutdown_all().await;
        });
    }));

    tokio::spawn(async move {
        match tokio::signal::ctrl_c().await {
            Ok(_) => shard_manager.lock().await.shutdown_all().await,
            Err(err) => error!("Error occured while shutdown: {}", err)
            
        }
    });

    if let Err(why) = client.start().await {
        error!("Error occured while start bot client: {}", why);
    } else {
        let data_read = client.data.read().await;
        let playlists = data_read.get::<Playlists>().unwrap().lock().await;
        let guild_configs = data_read.get::<GuildConfigs>().unwrap().lock().await;
        match write_json(&*playlists, "playlists.json") {
            Ok(size) => info!("Written {} bytes into playlists.json", size),
            Err(err) => error!("Error occured while writing playlists.json: {}", err),
        }
        match write_json(&*guild_configs, "guilds.json") {
            Ok(size) => info!("Written {} bytes into guilds.json", size),
            Err(err) => error!("Error occured while writing guilds.json: {}", err),
        }
    }
}
