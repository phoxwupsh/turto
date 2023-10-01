use tokio::sync::{Mutex, RwLock};
use turto::{
    commands::{
        autoleave::AUTOLEAVE_COMMAND, ban::BAN_COMMAND, help::HELP_COMMAND, join::JOIN_COMMAND,
        leave::LEAVE_COMMAND, pause::PAUSE_COMMAND, play::PLAY_COMMAND, playlist::PLAYLIST_COMMAND,
        playwhat::PLAYWHAT_COMMAND, queue::QUEUE_COMMAND, remove::REMOVE_COMMAND,
        seek::SEEK_COMMAND, skip::SKIP_COMMAND, stop::STOP_COMMAND, unban::UNBAN_COMMAND,
        volume::VOLUME_COMMAND,
    },
    config::TurtoConfigProvider,
    typemap::{config::GuildConfigs, playing::Playing, playlist::Playlists},
    handlers::before::before_hook,
    models::{guild::config::GuildConfig, playlist::Playlist},
};

use serenity::{
    async_trait,
    client::{Client, EventHandler},
    framework::standard::{buckets::LimitedFor, macros::group, StandardFramework},
    model::{gateway::Ready, prelude::GuildId},
    prelude::{Context, GatewayIntents},
};
use std::{collections::HashMap, env, fs, sync::Arc};

use songbird::SerenityInit;

use tracing::{error, info, Level};

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, _: Context, ready: Ready) {
        let name = &ready.user.name;
        let id = &ready.user.id;
        info!("{} is connected with ID {}.", name, id);
    }
}

#[group]
#[commands(
    play, pause, playwhat, stop, volume, playlist, queue, remove, join, leave, skip, seek, help,
    autoleave, ban, unban
)]
#[only_in(guilds)]
struct Music;

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

    let framework = StandardFramework::new()
        .configure(|c| c.prefix(config.command_prefix.clone()))
        .bucket("music", |bucket| {
            bucket
                .delay(config.command_delay)
                .await_ratelimits(1)
                .limit_for(LimitedFor::Guild)
        })
        .await
        .group(&MUSIC_GROUP)
        .before(before_hook);

    let intents = GatewayIntents::non_privileged() | GatewayIntents::MESSAGE_CONTENT;
    let mut client = Client::builder(&token, intents)
        .event_handler(Handler)
        .framework(framework)
        .intents(intents)
        .register_songbird()
        .await
        .unwrap_or_else(|err| panic!("Error creating client: {}", err));

    let playlists_json = fs::read_to_string("playlists.json").unwrap_or("{}".to_owned());
    let playlists: HashMap<GuildId, Playlist> =
        serde_json::from_str(&playlists_json).unwrap_or_default();

    let guild_configs_json = fs::read_to_string("guilds.json").unwrap_or("{}".to_owned());
    let guild_configs: HashMap<GuildId, GuildConfig> =
        serde_json::from_str(&guild_configs_json).unwrap_or_default();

    {
        let mut data = client.data.write().await;
        data.insert::<Playing>(Arc::new(RwLock::new(HashMap::default())));
        data.insert::<Playlists>(Arc::new(Mutex::new(playlists)));
        data.insert::<GuildConfigs>(Arc::new(Mutex::new(guild_configs)));
    }

    let shard_manager = client.shard_manager.clone();
    let data = client.data.clone();

    tokio::spawn(async move {
        if let Err(why) = tokio::signal::ctrl_c().await {
            error!("Client error: {:?}", why);
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
