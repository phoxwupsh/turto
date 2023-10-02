use serenity::{
    client::Client,
    framework::standard::{buckets::LimitedFor, StandardFramework},
    model::prelude::GuildId,
    prelude::GatewayIntents,
};
use songbird::SerenityInit;
use std::{collections::HashMap, env, fs, sync::Arc};
use tokio::sync::{Mutex, RwLock};
use tracing::{error, info, Level};
use turto::{
    commands::TURTOCOMMANDS_GROUP,
    config::TurtoConfigProvider,
    handlers::{before::before_hook, SerenityEventHandler},
    models::{guild::config::GuildConfig, playlist::Playlist},
    typemap::{config::GuildConfigs, playing::Playing, playlist::Playlists},
};

#[tokio::main]
async fn main() {
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("Tracing initialization failed.");
    dotenv::dotenv().unwrap_or_else(|err| panic!("Error loading .env file: {}", err));
    let token = env::var("DISCORD_TOKEN")
        .unwrap_or_else(|err| panic!("Error loading DISCORD_TOKEN in the environment: {}", err));
    let config = TurtoConfigProvider::get();

    let playlists_json = fs::read_to_string("playlists.json").unwrap_or("{}".to_owned());
    let playlists: HashMap<GuildId, Playlist> =
        serde_json::from_str(&playlists_json).unwrap_or_default();

    let guild_configs_json = fs::read_to_string("guilds.json").unwrap_or("{}".to_owned());
    let guild_configs: HashMap<GuildId, GuildConfig> =
        serde_json::from_str(&guild_configs_json).unwrap_or_default();

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
    let data = client.data.clone();

    tokio::spawn(async move {
        if let Err(why) = tokio::signal::ctrl_c().await {
            error!("Client error: {}", why);
        } else {
            {
                shard_manager.lock().await.shutdown_all().await;
            }

            let playlists_json: String;
            let guild_configs_json: String;
            {
                let data_read = data.read().await;
                let playlists = data_read.get::<Playlists>().unwrap().lock().await;
                let guild_configs = data_read.get::<GuildConfigs>().unwrap().lock().await;
                playlists_json = serde_json::to_string(&*playlists).unwrap_or("{}".to_owned());
                guild_configs_json =
                    serde_json::to_string(&*guild_configs).unwrap_or("{}".to_owned());
            }
            let playlists_json_size = playlists_json.len();
            let guild_configs_json_size = guild_configs_json.len();
            if let Err(why) = fs::write("playlists.json", playlists_json) {
                error!("Error occured while writing playlists.json: {}", why);
            } else {
                info!("Written {} bytes into playlists.json", playlists_json_size);
            }
            if let Err(why) = fs::write("guilds.json", guild_configs_json) {
                error!("Error occured while writing guilds.json: {}", why);
            } else {
                info!("Written {} bytes into guilds.json", guild_configs_json_size);
            }
        }
    });

    if let Err(why) = client.start().await {
        error!("Client error: {}", why);
    }
}
