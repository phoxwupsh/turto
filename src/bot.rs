use crate::{
    commands::create_commands,
    handlers::{before::before, SerenityEventHandler},
    models::{data::Data, guild::data::GuildData},
    utils::json::{read_json, write_json},
};
use dashmap::DashMap;
use poise::{Framework, FrameworkOptions};
use serenity::{all::ClientBuilder, model::prelude::GuildId, prelude::GatewayIntents, Client};
use songbird::SerenityInit;
use std::{io, path::Path, sync::Arc};
use tokio::{signal::ctrl_c, spawn};
use tracing::error;

pub struct Turto {
    client: Client,
    guild_data: Arc<DashMap<GuildId, GuildData>>,
}

impl Turto {
    pub async fn new(token: impl AsRef<str>) -> Result<Self, serenity::Error> {
        let options = FrameworkOptions {
            commands: create_commands(),
            command_check: Some(before),
            ..Default::default()
        };

        let data = Data::default();
        let guild_data = data.guilds.clone();
        let serenity_event_handler = SerenityEventHandler {
            playing: data.playing.clone(),
        };
        let framework = Framework::builder()
            .setup(|ctx, _ready, framework| {
                Box::pin(async move {
                    poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                    Ok(data)
                })
            })
            .options(options)
            .build();
        let intents = GatewayIntents::non_privileged();
        let client = ClientBuilder::new(token, intents)
            .framework(framework)
            .event_handler(serenity_event_handler)
            .register_songbird()
            .await?;
        Ok(Self { client, guild_data })
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
        self.guild_data = Arc::new(guild_data_map);
        Ok(())
    }

    pub async fn save_data(&self, path: impl AsRef<Path>) -> Result<usize, io::Error> {
        let guild_data_map = self.guild_data.clone();
        write_json(&*guild_data_map, path)
    }
}
