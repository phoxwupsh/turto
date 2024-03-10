use crate::{
    commands::create_commands,
    handlers::{before::before, SerenityEventHandler},
    models::{data::Data, guild::data::GuildData},
    utils::json::{read_json, write_json},
};
use dashmap::DashMap;
use poise::{Framework, FrameworkOptions};
use serenity::{
    all::{ClientBuilder, ShardManager},
    model::prelude::GuildId,
    prelude::GatewayIntents,
    Client,
};
use songbird::SerenityInit;
use std::{io, path::Path, sync::Arc};

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
            guild_data: guild_data.clone(),
            voice_channel_counts: Default::default()
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

    pub fn shard_manager(&self) -> Arc<ShardManager> {
        self.client.shard_manager.clone()
    }
}
