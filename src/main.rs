use tokio::sync::{Mutex, RwLock};
use turto_rs::{
    commands::{
        autoleave::AUTOLEAVE_COMMAND, help::HELP_COMMAND, join::JOIN_COMMAND, leave::LEAVE_COMMAND,
        pause::PAUSE_COMMAND, play::PLAY_COMMAND, playlist::PLAYLIST_COMMAND,
        playwhat::PLAYWHAT_COMMAND, queue::QUEUE_COMMAND, remove::REMOVE_COMMAND,
        seek::SEEK_COMMAND, skip::SKIP_COMMAND, stop::STOP_COMMAND, volume::VOLUME_COMMAND,
    },
    guild::{playing::Playing, playlist::Playlists, setting::GuildSettings},
    models::{playlist::Playlist, guild_setting::GuildSetting},
};

use serenity::{
    async_trait,
    client::{Client, EventHandler},
    framework::standard::{macros::group, StandardFramework, buckets::LimitedFor},
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
        let name = ready.user.name.clone();
        let id = ready.user.id.to_string();
        info!("{} is connected with ID {}.", name, id);
    }
}

#[group]
#[commands(
    play, pause, playwhat, stop, volume, playlist, queue, remove, join, leave, skip, seek, help,
    autoleave
)]
#[only_in(guilds)]
struct Music;

#[tokio::main]
async fn main() {
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("Tracing initialization failed.");

    dotenv::dotenv().expect("Failed to load .env file");

    let token = env::var("DISCORD_TOKEN").expect("Expected a token in the environment");

    let framework = StandardFramework::new()
        .configure(|c| c.prefix("!")) // Set the command prefix to "!"
        .bucket("music", |bucket| {
            bucket.delay(1)
                .await_ratelimits(1)
                .limit_for(LimitedFor::Guild)
        })
        .await
        .group(&MUSIC_GROUP);

    let intents = GatewayIntents::non_privileged() | GatewayIntents::MESSAGE_CONTENT;
    let mut client = Client::builder(&token, intents)
        .event_handler(Handler)
        .framework(framework)
        .intents(intents)
        .register_songbird()
        .await
        .expect("Error creating client");

    // Load the data from playlists.json and settings.json
    let playlists_json = fs::read_to_string("playlists.json").unwrap_or_else(|_| "{}".to_string());
    let playlists: HashMap<GuildId, Playlist> =
        serde_json::from_str(&playlists_json).unwrap_or_default();

    let settings_json = fs::read_to_string("settings.json").unwrap_or_else(|_| "{}".to_string());
    let settings: HashMap<GuildId, GuildSetting> =
        serde_json::from_str(&settings_json).unwrap_or_default();

    {
        let mut data = client.data.write().await;
        data.insert::<Playing>(Arc::new(RwLock::new(HashMap::default())));
        data.insert::<Playlists>(Arc::new(Mutex::new(playlists)));
        data.insert::<GuildSettings>(Arc::new(Mutex::new(settings)));
    }

    let shard_manager = client.shard_manager.clone();
    let data = client.data.clone();

    tokio::spawn(async move {
        if let Err(why) = tokio::signal::ctrl_c().await {
            error!("Client error: {:?}", why);
        } else {
            {
                // Shutdown the client first
                shard_manager.lock().await.shutdown_all().await;
            }

            // Write Playlists and Settings into json files
            let playlists_json: String;
            let settings_json: String;
            {
                let data_read = data.read().await;
                let playlists = data_read
                    .get::<Playlists>()
                    .expect("Expected Playlists in TypeMap.")
                    .lock()
                    .await;
                let settings = data_read
                    .get::<GuildSettings>()
                    .expect("Expected Settings in TypeMap.")
                    .lock()
                    .await;
                playlists_json =
                    serde_json::to_string(&*playlists).unwrap_or_else(|_| "{}".to_string());
                settings_json =
                    serde_json::to_string(&*settings).unwrap_or_else(|_| "{}".to_string());
            }
            let playlists_json_size = playlists_json.len();
            let settings_json_size = settings_json.len();
            if let Err(why) = fs::write("playlists.json", playlists_json) {
                error!("Error occured while writing playlists.json: {:?}", why);
            } else {
                info!("Written {} bytes into playlists.json", playlists_json_size);
            }
            if let Err(why) = fs::write("settings.json", settings_json) {
                error!("Error occured while writing settings.json: {:?}", why);
            } else {
                info!("Written {} bytes into settings.json", settings_json_size);
            }
        }
    });

    if let Err(why) = client.start().await {
        error!("Client error: {:?}", why);
    }
}
