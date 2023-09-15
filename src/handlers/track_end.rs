use serenity::{async_trait, model::prelude::GuildId, prelude::Context};
use songbird::events::{Event, EventContext, EventHandler};
use tracing::error;

use crate::{
    guild::setting::GuildSettings,
    models::guild_setting::GuildSetting,
    utils::play::{play_next, PlayError},
};

pub struct TrackEndHandler {
    pub ctx: Context,
    pub guild_id: GuildId,
}

#[async_trait]
impl EventHandler for TrackEndHandler {
    async fn act(&self, _ctx: &EventContext<'_>) -> Option<Event> {
        if let Err(PlayError::EmptyPlaylist(_guild)) = play_next(&self.ctx, self.guild_id).await {
            let settings_lock = {
                let data_lock = self.ctx.data.read().await;
                let data = data_lock
                    .get::<GuildSettings>()
                    .expect("Expected Settings in TypeMap")
                    .clone();
                data
            };
            let auto_leave = {
                let mut settings = settings_lock.lock().await;
                let setting = settings
                    .entry(self.guild_id)
                    .or_insert_with(GuildSetting::default);
                setting.auto_leave
            };
            if auto_leave {
                let manager = songbird::get(&self.ctx)
                    .await
                    .expect("Songbird Voice client placing in Resource failed.")
                    .clone();

                if let Err(e) = manager.remove(self.guild_id).await {
                    error!("Error leave voice channel: {:?}", e);
                }
            }
        }
        None
    }
}
