use crate::{
    commands::create_commands,
    config::get_config,
    handlers::{before::before, SerenityEventHandler},
    models::{data::Data, guild::data::GuildData},
    utils::json::{read_json, write_json},
};
use dashmap::DashMap;
use poise::{Framework, FrameworkOptions};
use serenity::{all::ClientBuilder, model::prelude::GuildId, prelude::GatewayIntents, Client};
use songbird::SerenityInit;
use std::{
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};
use tokio::sync::oneshot::{self, Receiver, Sender};
use tracing::{error, info, warn};

pub struct Turto {
    client: Client,
    guild_data: Arc<DashMap<GuildId, GuildData>>,
    data_path: PathBuf,
    auto_save_tx: Option<Sender<()>>,
}

impl Turto {
    pub async fn new(
        token: impl AsRef<str>,
        data_path: impl Into<PathBuf>,
    ) -> Result<Self, serenity::Error> {
        let options = FrameworkOptions {
            commands: create_commands(),
            command_check: Some(before),
            ..Default::default()
        };

        let data_path = data_path.into();

        let guild_data = Arc::new(load_data(&data_path));
        let data = Data {
            guilds: guild_data.clone(),
            ..Default::default()
        };

        let serenity_event_handler = SerenityEventHandler {
            playing: data.playing.clone(),
            guild_data: guild_data.clone(),
            voice_channel_counts: Default::default(),
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
        Ok(Self {
            client,
            guild_data,
            data_path,
            auto_save_tx: None,
        })
    }

    pub async fn start(&mut self) -> Result<(), serenity::Error> {
        if get_config().auto_save {
            let (tx, rx) = oneshot::channel::<()>();
            self.auto_save_tx = Some(tx);
            tokio::spawn(auto_save(
                self.guild_data.clone(),
                self.data_path.clone(),
                rx,
            ));
        }
        self.client.start().await
    }

    pub async fn shutdown(&mut self) {
        if let Some(tx) = self.auto_save_tx.take() {
            let _ = tx.send(());
        }
        self.client.shard_manager.shutdown_all().await;
        save_data(self.guild_data.clone(), &self.data_path);
    }
}

fn load_data(data_path: impl AsRef<Path>) -> DashMap<GuildId, GuildData> {
    match read_json(data_path.as_ref()) {
        Ok(data) => data,
        Err(err) => {
            warn!(
                "Failed to load data from {}: {}, will initialize new guilds data",
                data_path.as_ref().display(),
                err
            );
            Default::default()
        }
    }
}

async fn auto_save(
    data: Arc<DashMap<GuildId, GuildData>>,
    data_path: PathBuf,
    mut rx: Receiver<()>,
) {
    let sleep_interval = Duration::from_secs(get_config().auto_save_interval);
    let sleep = tokio::time::sleep(sleep_interval);
    tokio::pin!(sleep);

    loop {
        tokio::select! {
            _ = &mut sleep => {
                save_data(data.clone(), &data_path);
                let next = sleep.deadline() + sleep_interval;
                sleep.as_mut().reset(next);
            },
            _ = &mut rx => break
        }
    }
}

fn save_data(data: Arc<DashMap<GuildId, GuildData>>, data_path: impl AsRef<Path>) {
    let data_path = data_path.as_ref();
    match write_json(&*data, data_path) {
        Ok(bytes) => info!(
            "Data saved, {} bytes has been written to {}",
            bytes,
            data_path.display()
        ),
        Err(err) => error!("Failed to write data to {}: {:#}", data_path.display(), err),
    }
}
