use serenity::async_trait;
use songbird::{Event, EventContext, EventHandler, tracks::PlayMode};
use tracing::error;

pub struct TrackErrorHandler;

#[async_trait]
impl EventHandler for TrackErrorHandler {
    async fn act(&self, ctx: &EventContext<'_>) -> Option<Event> {
        let EventContext::Track(ctx) = ctx else {
            return None;
        };
        let (state, handle) = ctx[0];
        let PlayMode::Errored(err) = &state.playing else {
            return None;
        };
        error!(error = ?err, ?handle, "track error occured");
        None
    }
}
