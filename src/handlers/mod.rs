use dashmap::DashMap;
use serenity::{
    all::{ChannelId, GuildId},
    async_trait,
    model::{prelude::Ready, voice::VoiceState},
    prelude::{Context, EventHandler},
};
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};
use tokio::sync::RwLock;
use tracing::{error, info};

use crate::models::{autoleave::AutoleaveType, guild::data::GuildData, playing::Playing};

pub mod before;
pub mod track_end;

#[derive(Default)]
pub struct SerenityEventHandler {
    pub playing: Arc<RwLock<HashMap<GuildId, Playing>>>,
    pub guild_data: Arc<DashMap<GuildId, GuildData>>,
    pub voice_channel_counts: DashMap<ChannelId, AtomicUsize>,
}

#[async_trait]
impl EventHandler for SerenityEventHandler {
    async fn ready(&self, _: Context, ready: Ready) {
        let name = &ready.user.name;
        let user_id = &ready.user.id;
        let session = &ready.session_id;
        info!(
            "{} is connected with user id {}, session id {}",
            name, user_id, session
        );
    }

    async fn cache_ready(&self, ctx: Context, guilds: Vec<GuildId>) {
        // calculate the user counts of every voice channel
        let bot_id = ctx.cache.current_user().id;
        for guild_id in guilds {
            let Some(guild) = guild_id.to_guild_cached(&ctx) else {
                continue;
            };
            for voice_state in guild
                .voice_states
                .values()
                .filter(|voice_state| voice_state.user_id != bot_id)
            {
                if let Some(channel_id) = voice_state.channel_id {
                    self.voice_channel_add(channel_id);
                }
            }
        }
    }

    async fn voice_state_update(&self, ctx: Context, old: Option<VoiceState>, new: VoiceState) {
        if new.user_id == ctx.cache.current_user().id {
            // if the bot is manually disconnected by the user instead using command,
            // then remove the current track handle (if there is one)
            self.playing.write().await.remove(&new.guild_id.unwrap());
        } else {
            // update the user count in both old and new voice channels
            if let Some(new_channel) = new.channel_id {
                self.voice_channel_add(new_channel);
            }
            if let Some(old_channel) = old.and_then(|old| old.channel_id) {
                self.voice_channel_sub(old_channel);
            }
            // check if there are other users in the channel that the bot currently in,
            // and leave if autoleave if enabled
            let Some(guild_id) = new.guild_id else {
                return;
            };
            let autoleave = self
                .guild_data
                .entry(guild_id)
                .or_default()
                .config
                .auto_leave;
            if autoleave == AutoleaveType::Empty || autoleave == AutoleaveType::On {
                let Some(call) = songbird::get(&ctx).await.unwrap().get(guild_id) else {
                    return;
                };
                let mut call = call.lock().await;
                let Some(bot_channel_id) = call.current_channel() else {
                    return;
                };
                if self
                    .voice_channel_counts
                    .entry(bot_channel_id.0.into())
                    .or_default()
                    .load(Ordering::Acquire)
                    == 0
                {
                    if let Err(err) = call.leave().await {
                        error!(
                            "Error occured while leaving voice channel {}: {}",
                            bot_channel_id, err
                        );
                    }
                }
            }
        }
    }
}

impl SerenityEventHandler {
    fn voice_channel_add(&self, channel_id: ChannelId) {
        self.voice_channel_counts
            .entry(channel_id)
            .or_default()
            .fetch_add(1, Ordering::Relaxed);
    }

    fn voice_channel_sub(&self, channel_id: ChannelId) {
        self.voice_channel_counts
            .entry(channel_id)
            .or_default()
            .fetch_sub(1, Ordering::Relaxed);
    }
}
