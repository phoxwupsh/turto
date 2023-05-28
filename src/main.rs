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
        help::MY_HELP
    },
    guild::{
        playing::Playing,
        playlist::Playlists,
        volume::Volume
    },
};

use serenity::{
    async_trait,
    client::{Client, EventHandler},
    framework::standard::{
        macros::group, 
        StandardFramework,
    },
    model::gateway::Ready,
    prelude::{GatewayIntents, Context},
};
use std::{collections::HashMap, env, sync::Arc};

use songbird::SerenityInit;

use tracing::{info, error, Level};

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, _: Context, ready: Ready) {
        let log = format!("{} is connected with ID {}.", ready.user.name.clone(), ready.user.id.to_string());
        info!(log);
    }
}

#[group]
#[commands(play, pause, playwhat, stop, volume, playlist, queue, remove, join, leave, skip, seek)]
#[only_in(guilds)]
struct Music;

#[tokio::main]
async fn main() {
    let subscriber = tracing_subscriber::fmt().with_max_level(Level::INFO).finish();
    tracing::subscriber::set_global_default(subscriber)
        .expect("Tracing initialization failed.");

    dotenv::dotenv().expect("Failed to load .env file");

    let token = env::var("DISCORD_TOKEN").expect("Expected a token in the environment");

    let framework = StandardFramework::new()
        .configure(|c| c.prefix("!")) // Set the command prefix to "!"
        .group(&MUSIC_GROUP)
        .help(&MY_HELP);

    let intents = GatewayIntents::non_privileged() | GatewayIntents::MESSAGE_CONTENT;
    let mut client = Client::builder(&token, intents)
        .event_handler(Handler)
        .framework(framework)
        .intents(intents)
        .register_songbird()
        .await
        .expect("Error creating client");

    {
        let mut data = client.data.write().await;
        data.insert::<Playing>(Arc::new(RwLock::new(HashMap::default())));
        data.insert::<Playlists>(Arc::new(Mutex::new(HashMap::default())));
        data.insert::<Volume>(Arc::new(Mutex::new(HashMap::default())));
    }

    if let Err(why) = client.start().await {
        let log = format!("Client error: {:?}", why);
        error!(log);
    }
}

