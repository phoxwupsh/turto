use crate::{
    models::autoleave::AutoleaveType,
    utils::play::{PlayContext, play_ytdlfile},
    ytdl::YouTubeDl,
};
use serenity::async_trait;
use songbird::{
    Call,
    events::{Event, EventContext, EventHandler},
    tracks::PlayMode,
};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::error;

pub struct TrackEndHandler {
    pub call: Arc<Mutex<Call>>,
    pub ytdl_file: YouTubeDl,
    pub ctx: PlayContext,
}

#[async_trait]
impl EventHandler for TrackEndHandler {
    async fn act(&self, ctx: &EventContext<'_>) -> Option<Event> {
        let EventContext::Track(ctx) = ctx else {
            return None;
        };

        let mut guild_data = self.ctx.data.entry(self.ctx.guild_id).or_default();
        let repeat = guild_data.config.repeat;
        let auto_leave = guild_data.config.auto_leave;
        let next = if repeat {
            None
        } else {
            guild_data.playlist.pop_front()
        };
        drop(guild_data);

        let (state, _handle) = ctx[0];

        let PlayMode::End = state.playing else {
            return None;
        };

        if repeat {
            play_ytdlfile(self.ctx.clone(), self.call.clone(), self.ytdl_file.clone())
                .await
                .ok()?;
            return None;
        }

        if let Some(next) = next {
            play_ytdlfile(self.ctx.clone(), self.call.clone(), next)
                .await
                .ok()?;
        } else if auto_leave == AutoleaveType::Silent || auto_leave == AutoleaveType::On {
            let mut call = self.call.lock().await;
            if let Err(err) = call.leave().await {
                error!(error = ?err, channel = ?call.current_channel(), "failed to leave voice channel");
            }
        }
        None
    }
}
