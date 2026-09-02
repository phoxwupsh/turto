use crate::{
    models::{guild::Guilds, playing::Playing},
    player,
};
use serenity::{
    all::{ChannelId, GuildId},
    async_trait,
    model::{prelude::Ready, voice::VoiceState},
    prelude::{Context, EventHandler},
};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::RwLock;
use tracing::{Instrument, error, info, info_span};

pub mod before;
pub mod error;
pub mod track_end;
pub mod track_error;

#[derive(Default)]
pub struct SerenityEventHandler {
    pub playing: Arc<RwLock<HashMap<GuildId, Playing>>>,
    pub guild_data: Arc<Guilds>,
}

#[async_trait]
impl EventHandler for SerenityEventHandler {
    async fn ready(&self, _: Context, ready: Ready) {
        let name = &ready.user.name;
        let user_id = &ready.user.id;
        let session = &ready.session_id;
        info!(
            session_id = %session,
            %user_id,
            bot_name = name,
            "connected"
        );
    }

    async fn voice_state_update(&self, ctx: Context, _old: Option<VoiceState>, new: VoiceState) {
        let Some(guild_id) = new.guild_id else {
            return;
        };
        self.on_voice_state(ctx, new, guild_id)
            .instrument(info_span!("voice_update", guild = %guild_id))
            .await;
    }
}

impl SerenityEventHandler {
    /// The body of [`EventHandler::voice_state_update`], under the `voice_update`
    /// span. Handles a manual bot disconnect (tear down the current track) and,
    /// for another user's move, auto-leaves a channel they left the bot alone in.
    async fn on_voice_state(&self, ctx: Context, new: VoiceState, guild_id: GuildId) {
        let bot_id = ctx.cache.current_user().id;
        if new.user_id == bot_id {
            // A disconnect only. The same event also carries mutes, suppression and
            // channel moves, which leave the track playing.
            if new.channel_id.is_none()
                && let Some(removed) = self.playing.write().await.remove(&guild_id)
            {
                if let Err(error) = removed.track_handle.stop() {
                    error!(?error, "failed to stop track");
                }
                drop(removed);
            }
            return;
        }

        let autoleave = self
            .guild_data
            .entry(guild_id)
            .or_default()
            .config
            .auto_leave;
        if !autoleave.leaves_on_empty_channel() {
            return;
        }
        let Some(call) = songbird::get(&ctx).await.unwrap().get(guild_id) else {
            return;
        };
        let mut call = call.lock().await;
        let Some(bot_channel) = call.current_channel() else {
            return;
        };
        let bot_channel = ChannelId::from(bot_channel.0);

        // Serenity applies the cache update before dispatching the event, so the
        // guild's voice states already reflect `new`: reading them is authoritative
        // where a maintained counter drifts (a guild joined after startup was never
        // counted, a gateway resume replays or drops updates) and can never
        // underflow into "never empty".
        let others_present = {
            let Some(guild) = guild_id.to_guild_cached(&ctx) else {
                return;
            };
            guild
                .voice_states
                .values()
                .any(|state| state.user_id != bot_id && state.channel_id == Some(bot_channel))
            // the cache guard must drop before the await below
        };
        if !others_present {
            player::leave(&mut call, guild_id).await;
        }
    }
}
