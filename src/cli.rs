use std::{path::PathBuf, sync::Arc, time::Duration};
use clap::ArgAction;
use tokio_cron_scheduler::{Job, JobScheduler, JobSchedulerError};
use tracing::{error, warn};

use crate::{
    bot::Turto,
    deps::setup_ext_deps,
    message::template::Templates,
    models::{config::TurtoConfig, data::Data, guild::Guilds, help::HelpConfig},
    sched::auto_update_ytdlp,
    signal::wait_shutdown_signal,
};

#[derive(Debug, clap::Parser)]
#[command(disable_help_flag = true)]
pub struct Cli {
    #[arg(short = '?', long = "usage", action = ArgAction::Help, help = "show the usage")]
    usage: bool,

    #[arg(long, value_name = "FILE", value_hint = clap::ValueHint::FilePath, default_value = "config.toml", help = "path to config file")]
    config: PathBuf,

    #[arg(long, value_name = "FILE", value_hint = clap::ValueHint::FilePath, default_value = "guilds.json", help = "path to guilds data file")]
    guilds: PathBuf,

    #[arg(long, value_name = "FILE", value_hint = clap::ValueHint::FilePath, default_value = "help.toml", help = "path to help messages file")]
    help: PathBuf,

    #[arg(long, value_name = "FILE", value_hint = clap::ValueHint::FilePath, default_value = "templates.toml", help = "path to message templates file")]
    tempaltes: PathBuf,
}

impl Cli {
    pub fn run(&self) {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(self.main());
    }

    async fn main(&self) {
        if let Err(err) = dotenvy::dotenv() {
            warn!(error = ?err, "Failed to load .env file");
        }

        let config = match TurtoConfig::load(&self.config) {
            Ok(config) => config,
            Err(err) => {
                error!(error = ?err, path = %self.config.display(), "failed to load config");
                return;
            }
        };

        let help = match HelpConfig::load(&self.help) {
            Ok(help) => help,
            Err(err) => {
                error!(error = ?err, path = %self.help.display(), "failed to load help");
                return;
            }
        };

        let guild_data = match Guilds::load(&self.guilds) {
            Ok(data) => data,
            Err(err) => {
                warn!(error = ?err, "failed to load guilds data, will initialize new");
                Default::default()
            }
        };

        let templates = match Templates::load(&self.tempaltes) {
            Ok(templates) => templates,
            Err(err) => {
                error!(error = ?err, "failed to load templates");
                return;
            }
        };

        if let Err(err) = setup_ext_deps(&config.ytdlp).await {
            error!(error = ?err, "failed to setup yt-dlp");
            return;
        }

        let token = match std::env::var("DISCORD_TOKEN") {
            Ok(token) => {
                if token.is_empty() {
                    error!("DISCORD_TOKEN is not set in the environment");
                    return;
                }
                token
            }
            Err(err) => {
                error!(error = ?err, "failed to load DISCORD_TOKEN from the environment");
                return;
            }
        };

        let data = Data {
            guilds: Arc::new(guild_data),
            config: Arc::new(config),
            help,
            templates,
            playing: Default::default(),
        };

        let config = data.config.clone();

        let mut bot = match Turto::new(token, data, &self.guilds).await {
            Ok(bot) => bot,
            Err(err) => {
                error!(error = ?err, "turto client failed to initialize");
                return;
            }
        };

        let auto_save_job_factory = bot.auto_save_job();
        let auto_update_job_factory = auto_update_ytdlp("yt-dlp", config.ytdlp.clone());
        let scheduler = async move {
            let scheduler = JobScheduler::new().await?;
            scheduler
                .add(Job::new_repeated_async(
                    Duration::from_secs(86400),
                    auto_update_job_factory,
                )?)
                .await?;

            scheduler
                .add(Job::new_repeated_async(
                    Duration::from_secs(config.auto_save_interval),
                    auto_save_job_factory,
                )?)
                .await?;
            Result::<JobScheduler, JobSchedulerError>::Ok(scheduler)
        }
        .await;

        let mut scheduler = match scheduler {
            Ok(sched) => {
                if let Err(err) = sched.start().await {
                    error!(error = ?err, "scheduler error");
                    return;
                }
                sched
            }
            Err(err) => {
                error!(error = ?err, "scheduler error");
                return;
            }
        };

        tokio::select! {
            _ = wait_shutdown_signal() => {
                bot.shutdown().await;
                let _ = scheduler.shutdown().await;
            }
            _ = bot.start() => ()
        }
    }
}
