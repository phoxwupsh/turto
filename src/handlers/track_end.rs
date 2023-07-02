use serenity::{prelude::Context, model::prelude::GuildId, async_trait};
use songbird::events::{
    Event,
    EventContext,
    EventHandler
};
use tracing::error;

use crate::{utils::play_next, error::TurtoError, guild::setting::Settings, models::setting::GuildSetting};

pub struct TrackEndHandler {
    pub ctx: Context,
    pub guild_id: GuildId,
}

#[async_trait]
impl EventHandler for TrackEndHandler {
    async fn act(&self, _ctx: &EventContext<'_>) -> Option<Event> {
        if let Err(e) = play_next(&self.ctx, self.guild_id).await {
            if let TurtoError::EmptyPlaylist = e {
                let settings_lock = {
                    let data_lock = self.ctx.data.read().await;
                    let data = data_lock.get::<Settings>().expect("Expected Settings in TypeMap").clone();
                    data
                };
                let auto_leave = {
                    let mut settings = settings_lock.lock().await;
                    let setting = settings.entry(self.guild_id).or_insert_with(GuildSetting::default);
                    setting.auto_leave.clone()
                };
                if auto_leave {
                    let manager = songbird::get(&self.ctx).await
                        .expect("Songbird Voice client placing in Resource failed.")
                        .clone();
    
                    if let Err(e) = manager.remove(self.guild_id).await {
                        error!("Error leave voice channel: {:?}", e);
                    }
                }
            }
        }
        None
    }
}