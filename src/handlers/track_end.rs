use crate::{
    typemap::guild_data::GuildDataMap,
    utils::play::{play_next, PlayError},
};
use serenity::{async_trait, model::prelude::GuildId, prelude::Context};
use songbird::events::{Event, EventContext, EventHandler};
use tracing::error;

pub struct TrackEndHandler {
    pub ctx: Context,
    pub guild_id: GuildId,
}

#[async_trait]
impl EventHandler for TrackEndHandler {
    async fn act(&self, _ctx: &EventContext<'_>) -> Option<Event> {
        if let Err(PlayError::EmptyPlaylist(_guild)) = play_next(&self.ctx, self.guild_id).await {
            let guild_data_map = self
                .ctx
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
                let manager = songbird::get(&self.ctx).await.unwrap().clone();
                if let Err(err) = manager.remove(self.guild_id).await {
                    error!("Error leave voice channel: {}", err);
                }
            }
        }
        None
    }
}
