use crate::{
    commands::TURTOCOMMANDS_GROUP,
    config::TurtoConfigProvider,
    handlers::{before::before_hook, SerenityEventHandler},
    models::guild::data::GuildData,
    typemap::{guild_data::GuildDataMap, playing::Playing},
    utils::json::{read_json, write_json},
};
use dashmap::DashMap;
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
    sync::RwLock,
};
use tracing::{error, info};

pub struct Turto {
    inner: Option<TurtoInner>,
}

struct TurtoInner {
    client: Client,
    guilds_path: PathBuf,
}

impl Turto {
    pub async fn new(
        token: impl AsRef<str>,
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
        let guild_data_map: DashMap<GuildId, GuildData> =
            read_json(guilds_path.as_ref()).unwrap_or_default();
        let intents = GatewayIntents::non_privileged() | GatewayIntents::MESSAGE_CONTENT;
        let client = Client::builder(token, intents)
            .event_handler(SerenityEventHandler)
            .framework(framework)
            .intents(intents)
            .register_songbird()
            .type_map_insert::<Playing>(Arc::new(RwLock::new(HashMap::default())))
            .type_map_insert::<GuildDataMap>(Arc::new(guild_data_map))
            .await?;
        Ok(Self {
            inner: Some(TurtoInner {
                client,
                guilds_path: guilds_path.as_ref().to_path_buf(),
            }),
        })
    }

    pub async fn start(&mut self) -> Result<(), serenity::Error> {
        let inner = self.inner.as_mut().unwrap(); // It's impossible to be None
        let shard_manager = inner.client.shard_manager.clone();
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
        inner.client.start().await
    }
}

impl TurtoInner {
    async fn save_data(&self) {
        let data = self.client.data.read().await;
        let guild_data_map = data.get::<GuildDataMap>().unwrap().clone();
        match write_json(&*guild_data_map, self.guilds_path.as_path()) {
            Ok(size) => info!("Write {} bytes to guilds.json", size),
            Err(err) => error!("Error occured while writing guilds.json: {}", err),
        }
    }
}

impl Drop for Turto {
    fn drop(&mut self) {
        if let Some(this) = self.inner.take() {
            spawn(async move {
                this.save_data().await;
            });
        }
    }
}
