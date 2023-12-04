use crate::{
    commands::TURTOCOMMANDS_GROUP,
    config::get_config,
    handlers::{before::before_hook, SerenityEventHandler},
    models::guild::data::GuildData,
    typemap::{guild_data::GuildDataMap, playing::PlayingMap},
    utils::json::{read_json, write_json},
};
use dashmap::DashMap;
use serenity::{
    framework::{
        standard::{buckets::LimitedFor, BucketBuilder, Configuration},
        StandardFramework,
    },
    model::prelude::GuildId,
    prelude::GatewayIntents,
    Client,
};
use songbird::SerenityInit;
use std::{collections::HashMap, io, path::Path, sync::Arc};
use tokio::{signal::ctrl_c, spawn, sync::RwLock};
use tracing::error;

pub struct Turto {
    client: Client,
}

impl Turto {
    pub async fn new(token: impl AsRef<str>) -> Result<Self, serenity::Error> {
        let config = get_config();
        let framework = StandardFramework::new()
            .bucket(
                "turto",
                BucketBuilder::new_guild()
                    .delay(config.command_delay)
                    .await_ratelimits(1)
                    .limit_for(LimitedFor::Guild),
            )
            .await
            .group(&TURTOCOMMANDS_GROUP)
            .before(before_hook);
        framework.configure(Configuration::new().prefix(config.command_prefix.clone()));
        let intents = GatewayIntents::non_privileged() | GatewayIntents::MESSAGE_CONTENT;
        let client = Client::builder(token, intents)
            .event_handler(SerenityEventHandler)
            .framework(framework)
            .intents(intents)
            .register_songbird()
            .type_map_insert::<PlayingMap>(Arc::new(RwLock::new(HashMap::default())))
            .type_map_insert::<GuildDataMap>(Arc::new(DashMap::<GuildId, GuildData>::default()))
            .await?;
        Ok(Self { client })
    }

    pub async fn start(&mut self) -> Result<(), serenity::Error> {
        let shard_manager = self.client.shard_manager.clone();
        let shard_manager_panic = shard_manager.clone();

        let default_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |panic_info| {
            default_hook(panic_info);
            let shard_manager_panic_ = shard_manager_panic.clone();
            spawn(async move {
                shard_manager_panic_.shutdown_all().await;
            });
        }));

        spawn(async move {
            match ctrl_c().await {
                Ok(_) => shard_manager.shutdown_all().await,
                Err(err) => error!("Error occured while shutdown: {}", err),
            }
        });
        self.client.start().await
    }

    pub async fn load_data(&mut self, path: impl AsRef<Path>) -> Result<(), io::Error> {
        let guild_data_map: DashMap<GuildId, GuildData> = read_json(path)?;
        let mut data = self.client.data.write().await;
        data.insert::<GuildDataMap>(Arc::new(guild_data_map));
        Ok(())
    }

    pub async fn save_data(&self, path: impl AsRef<Path>) -> Result<usize, io::Error> {
        let data = self.client.data.read().await;
        let guild_data_map = data.get::<GuildDataMap>().unwrap().clone();
        write_json(&*guild_data_map, path)
    }
}
