use crate::{
    models::alias::{Context, Error},
    utils::queue::{QueueType, enqueue},
};

#[poise::command(slash_command, guild_only)]
pub async fn insert(ctx: Context<'_>, #[rename = "url"] query: String) -> Result<(), Error> {
    enqueue(ctx, query, QueueType::Front).await
}
