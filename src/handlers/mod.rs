use serenity::{
    async_trait,
    model::prelude::Ready,
    prelude::{Context, EventHandler},
};
use tracing::info;

pub mod before;
pub mod track_end;

pub struct SerenityEventHandler;

#[async_trait]
impl EventHandler for SerenityEventHandler {
    async fn ready(&self, _: Context, ready: Ready) {
        let name = &ready.user.name;
        let user_id = &ready.user.id;
        let session = &ready.session_id;
        info!("{} is connected with user id {}, session id {}", name, user_id, session);
    }
}
