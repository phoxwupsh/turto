use std::sync::Arc;

use crate::{
    typemap::guild_data::GuildDataMap,
    utils::play::{play_next, play_url},
};
use serenity::{async_trait, model::prelude::GuildId, prelude::TypeMap};
use songbird::{
    events::{Event, EventContext, EventHandler},
    tracks::PlayMode,
    Call,
};
use tokio::sync::{Mutex, RwLock};
use tracing::error;

pub struct TrackEndHandler {
    pub data: Arc<RwLock<TypeMap>>,
    pub call: Arc<Mutex<Call>>,
    pub guild_id: GuildId,
}

#[async_trait]
impl EventHandler for TrackEndHandler {
    async fn act(&self, ctx: &EventContext<'_>) -> Option<Event> {
        let guild_data_map = self
            .data
            .read()
            .await
            .get::<GuildDataMap>()
            .unwrap()
            .clone();
        let guild_data = guild_data_map.entry(self.guild_id).or_default();

        if guild_data.config.repeat {
            drop(guild_data);
            let EventContext::Track(ctx) = ctx else {
                return None;
            };
            let (state, handle) = ctx[0];
            if state.playing != PlayMode::Stop {
                let _ = play_url(
                    self.call.clone(),
                    self.data.clone(),
                    self.guild_id,
                    handle.metadata().source_url.clone().unwrap(),
                )
                .await;
            }
            None
        } else if guild_data.playlist.is_empty() && guild_data.config.auto_leave {
            drop(guild_data);
            let mut call = self.call.lock().await;
            if let Err(err) = call.leave().await {
                error!("Failed to leave voice channel: {}", err);
            }
            None
        } else {
            drop(guild_data);
            let _ = play_next(self.call.clone(), self.data.clone(), self.guild_id).await;
            None
        }
    }
}
