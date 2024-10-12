use crate::{
    models::{autoleave::AutoleaveType, guild::data::GuildData, playing::Playing},
    utils::play::{play_next, play_url},
};
use dashmap::DashMap;
use serenity::{async_trait, model::prelude::GuildId};
use songbird::{
    events::{Event, EventContext, EventHandler},
    tracks::PlayMode,
    Call,
};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::{Mutex, RwLock};
use tracing::error;

pub struct TrackEndHandler {
    pub guild_data: Arc<DashMap<GuildId, GuildData>>,
    pub guild_playing: Arc<RwLock<HashMap<GuildId, Playing>>>,
    pub call: Arc<Mutex<Call>>,
    pub url: Arc<str>,
    pub guild_id: GuildId,
}

#[async_trait]
impl EventHandler for TrackEndHandler {
    async fn act(&self, ctx: &EventContext<'_>) -> Option<Event> {
        let data = self.guild_data.entry(self.guild_id).or_default();
        let repeat = data.config.repeat;
        let auto_leave = data.config.auto_leave;
        drop(data);

        let EventContext::Track(ctx) = ctx else {
            return None;
        };
        let (state, _handle) = ctx[0];

        match &state.playing {
            PlayMode::End => {
                if repeat {
                    let _meta = play_url(
                        self.call.clone(),
                        self.guild_data.clone(),
                        self.guild_playing.clone(),
                        self.guild_id,
                        self.url.clone(),
                    )
                    .await;
                    None
                } else {
                    if play_next(
                        self.call.clone(),
                        self.guild_data.clone(),
                        self.guild_playing.clone(),
                        self.guild_id,
                    )
                    .await
                    .is_none()
                        && (auto_leave == AutoleaveType::Silent || auto_leave == AutoleaveType::On)
                    {
                        let mut call = self.call.lock().await;
                        if let Err(err) = call.leave().await {
                            error!("Failed to leave voice channel: {}", err);
                        }
                    }
                    None
                }
            }
            PlayMode::Stop => return None,
            _ => return None,
        }
    }
}
