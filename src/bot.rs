use crate::{
    commands::create_commands,
    handlers::{
        SerenityEventHandler,
        before::{command_check, pre_command},
        error::on_error,
    },
    models::{data::Data, guild::Guilds},
};
use poise::{Framework, FrameworkOptions};
use serenity::{Client, all::ClientBuilder, prelude::GatewayIntents};
use songbird::SerenityInit;
use std::{
    path::{Path, PathBuf},
    pin::Pin,
    sync::Arc,
};
use tokio_cron_scheduler::JobScheduler;
use tracing::{error, info};
use uuid::Uuid;

pub struct Turto {
    client: Client,
    guild_data: Arc<Guilds>,
    data_path: PathBuf,
}

impl Turto {
    pub async fn new(
        token: impl AsRef<str>,
        data: Data,
        data_path: impl AsRef<Path>,
    ) -> Result<Self, serenity::Error> {
        let options = FrameworkOptions {
            commands: create_commands(&data.config, &data.help),
            command_check: Some(command_check),
            on_error: on_error,
            pre_command: pre_command,
            ..Default::default()
        };

        let guild_data = data.guilds.clone();

        let serenity_event_handler = SerenityEventHandler {
            playing: data.playing.clone(),
            guild_data: data.guilds.clone(),
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

        let data_path = data_path.as_ref().to_path_buf();
        Ok(Self {
            client,
            guild_data,
            data_path,
        })
    }

    pub async fn start(&mut self) -> Result<(), serenity::Error> {
        self.client.start().await
    }

    pub async fn shutdown(&mut self) {
        self.client.shard_manager.shutdown_all().await;
        match self.guild_data.save(&self.data_path) {
            Ok(bytes) => info!(bytes, path = %self.data_path.display(), "data saved"),
            Err(err) => {
                error!(error = ?err, path = %self.data_path.display(), "failed to save data")
            }
        }
    }

    pub fn auto_save_job(
        &self,
    ) -> impl FnMut(Uuid, JobScheduler) -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync + 'static
    {
        let data_path = self.data_path.clone();
        let guilds = self.guild_data.clone();
        move |_uuid, _job_scheduler| {
            let data_path = data_path.clone();
            let guilds = guilds.clone();
            Box::pin(async move {
                match guilds.save(&data_path) {
                    Ok(bytes) => info!(bytes, path = %data_path.display(), "data auto saved"),
                    Err(err) => {
                        error!(error = ?err, path = %data_path.display(), "failed to auto save data")
                    }
                }
            })
        }
    }
}
