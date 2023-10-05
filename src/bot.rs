use crate::{
    commands::TURTOCOMMANDS_GROUP,
    config::TurtoConfigProvider,
    handlers::{before::before_hook, SerenityEventHandler},
    models::{guild::config::GuildConfig, playlist::Playlist},
    typemap::{config::GuildConfigs, playing::Playing, playlist::Playlists},
    utils::json::{read_json, write_json},
};
use serenity::{
    framework::{standard::buckets::LimitedFor, StandardFramework},
    model::prelude::GuildId,
    prelude::GatewayIntents,
    Client,
};
use songbird::SerenityInit;
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc,
};
use tokio::{
    signal::ctrl_c,
    spawn,
    sync::{Mutex, RwLock},
};
use tracing::{error, info};

pub struct Turto {
    client: Client,
    playlists_path: PathBuf,
    guilds_path: PathBuf,
}

impl Turto {
    pub async fn new(
        token: impl AsRef<str>,
        playlists_path: impl AsRef<Path>,
        guilds_path: impl AsRef<Path>,
    ) -> Result<Self, serenity::Error> {
        let config = TurtoConfigProvider::get();
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
        let playlists: HashMap<GuildId, Playlist> =
            read_json(playlists_path.as_ref()).unwrap_or_default();
        let guild_configs: HashMap<GuildId, GuildConfig> =
            read_json(guilds_path.as_ref()).unwrap_or_default();
        let intents = GatewayIntents::non_privileged() | GatewayIntents::MESSAGE_CONTENT;
        let client = Client::builder(token, intents)
            .event_handler(SerenityEventHandler)
            .framework(framework)
            .intents(intents)
            .register_songbird()
            .type_map_insert::<Playing>(Arc::new(RwLock::new(HashMap::default())))
            .type_map_insert::<Playlists>(Arc::new(Mutex::new(playlists)))
            .type_map_insert::<GuildConfigs>(Arc::new(Mutex::new(guild_configs)))
            .await?;
        Ok(Self {
            client,
            playlists_path: playlists_path.as_ref().to_path_buf(),
            guilds_path: guilds_path.as_ref().to_path_buf(),
        })
    }

    pub async fn start(&mut self) -> Result<(), serenity::Error> {
        let shard_manager = self.client.shard_manager.clone();
        let shard_manager_panic = shard_manager.clone();

        let default_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |panic_info| {
            default_hook(panic_info);
            let shard_manager_panic_ = shard_manager_panic.clone();
            spawn(async move {
                shard_manager_panic_.lock().await.shutdown_all().await;
            });
        }));

        spawn(async move {
            match ctrl_c().await {
                Ok(_) => shard_manager.lock().await.shutdown_all().await,
                Err(err) => error!("Error occured while shutdown: {}", err),
            }
        });
        self.client.start().await
    }

    pub async fn save_data(&self) {
        let data = self.client.data.read().await;
        let playlists = data.get::<Playlists>().unwrap().lock().await;
        let guild_configs = data.get::<GuildConfigs>().unwrap().lock().await;
        match write_json(&*playlists, self.playlists_path.as_path()) {
            Ok(size) => info!("Write {} bytes to playlists.json", size),
            Err(err) => error!("Error occured while writing playlists.json: {}", err),
        }
        match write_json(&*guild_configs, self.guilds_path.as_path()) {
            Ok(size) => info!("Write {} bytes to guilds.json", size),
            Err(err) => error!("Error occured while writing guilds.json: {}", err),
        }
    }
}