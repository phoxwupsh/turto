use tokio::sync::{RwLock, Mutex};
use turto_rs::{
    commands::{
        join::JOIN_COMMAND, 
        leave::LEAVE_COMMAND, 
        pause::PAUSE_COMMAND, 
        play::PLAY_COMMAND,
        playlist::PLAYLIST_COMMAND, 
        skip::SKIP_COMMAND, 
        stop::STOP_COMMAND,
        playwhat::PLAYWHAT_COMMAND,
        volume::VOLUME_COMMAND,
        queue::QUEUE_COMMAND,
        remove::REMOVE_COMMAND,
        seek::SEEK_COMMAND,
        help::HELP_COMMAND
    },
    guild::{
        playing::Playing,
        playlist::{Playlists, Playlist},
        volume::{Volume, GuildVolume}
    },
};

use serenity::{
    async_trait,
    client::{Client, EventHandler},
    framework::standard::{
        macros::group, 
        StandardFramework,
    },
    model::{gateway::Ready, prelude::GuildId},
    prelude::{GatewayIntents, Context},
};
use std::{collections::HashMap, env, sync::Arc, fs};

use songbird::SerenityInit;

use tracing::{info, error, Level};

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
#[commands(play, pause, playwhat, stop, volume, playlist, queue, remove, join, leave, skip, seek, help)]
#[only_in(guilds)]
struct Music;

#[tokio::main]
async fn main() {
    let subscriber = tracing_subscriber::fmt().with_max_level(Level::INFO).finish();
    tracing::subscriber::set_global_default(subscriber).expect("Tracing initialization failed.");

    dotenv::dotenv().expect("Failed to load .env file");

    let token = env::var("DISCORD_TOKEN").expect("Expected a token in the environment");

    let framework = StandardFramework::new()
        .configure(|c| c.prefix("!")) // Set the command prefix to "!"
        .group(&MUSIC_GROUP);

    let intents = GatewayIntents::non_privileged() | GatewayIntents::MESSAGE_CONTENT;
    let mut client = Client::builder(&token, intents)
        .event_handler(Handler)
        .framework(framework)
        .intents(intents)
        .register_songbird()
        .await
        .expect("Error creating client");

    // Load the data from playlists.json and volume.json
    let playlists_json = fs::read_to_string("playlists.json").unwrap_or_else(|_| "{}".to_string());
    let playlists: HashMap<GuildId, Playlist> = serde_json::from_str(&playlists_json).unwrap_or_default();

    let volume_json = fs::read_to_string("volume.json").unwrap_or_else(|_| "{}".to_string());
    let volume: HashMap<GuildId, GuildVolume> = serde_json::from_str(&volume_json).unwrap_or_default();

    {
        let mut data = client.data.write().await;
        data.insert::<Playing>(Arc::new(RwLock::new(HashMap::default())));
        data.insert::<Playlists>(Arc::new(Mutex::new(playlists)));
        data.insert::<Volume>(Arc::new(Mutex::new(volume)));
    }

    let shard_manager = client.shard_manager.clone();
    let data = client.data.clone();

    tokio::spawn(async move {
        if let Err(why) = tokio::signal::ctrl_c().await {
            error!("Client error: {:?}", why);
        }
        else {
            { // Shutdown the client first
                shard_manager.lock().await.shutdown_all().await;
            }

            // Write Playlists and Volume into json files
            let playlists_json: String;
            let volume_json: String;
            {
                let data_read = data.read().await;
                let playlists = data_read.get::<Playlists>().expect("Expected Playlists in TypeMap.").lock().await;
                let volume = data_read.get::<Volume>().expect("Expected Volume in TypeMap.").lock().await;
                playlists_json = serde_json::to_string(&*playlists).unwrap_or_else(|_|"{}".to_string());
                volume_json = serde_json::to_string(&*volume).unwrap_or_else(|_|"{}".to_string());
            }
            let playlists_json_size = playlists_json.len();
            let volume_json_size = volume_json.len();
            if let Err(why) = fs::write("playlists.json", playlists_json) {
                error!("Error occured while writing playlists.json: {:?}", why);
            } else {
                info!("Written {} bytes into playlists.json", playlists_json_size);
            }
            if let Err(why) = fs::write("volume.json", volume_json) {
                error!("Error occured while writing volume.json: {:?}", why);
            } else {
                info!("Written {} bytes into volume.json", volume_json_size);
            }
        }
    });

    if let Err(why) = client.start().await {
        error!("Client error: {:?}", why);
    }
}

