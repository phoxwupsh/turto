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
        let id = &ready.user.id;
        info!("{} is connected with ID {}.", name, id);
    }
}
