use std::sync::Arc;

use crate::{
    typemap::guild_data::GuildDataMap,
    utils::play::{play_next, PlayError},
};
use serenity::{async_trait, model::prelude::GuildId, prelude::TypeMap};
use songbird::{
    events::{Event, EventContext, EventHandler},
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
    async fn act(&self, _ctx: &EventContext<'_>) -> Option<Event> {
        if let Err(PlayError::EmptyPlaylist(_guild)) =
            play_next(self.call.clone(), self.data.clone(), self.guild_id).await
        {
            let guild_data_map = self
                .data
                .read()
                .await
                .get::<GuildDataMap>()
                .unwrap()
                .clone();
            let guild_data = guild_data_map.entry(self.guild_id).or_default();
            let auto_leave = guild_data.config.auto_leave;
            drop(guild_data);

            if auto_leave {
                let mut call = self.call.lock().await;
                if let Err(err) = call.leave().await {
                    error!("Failed to leave voice channel: {}", err);
                }
            }
        }
        None
    }
}
