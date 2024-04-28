use crate::{
    models::alias::{Context, Error},
    utils::queue::{enqueue, QueueType},
};

#[poise::command(slash_command, guild_only)]
pub async fn queue(ctx: Context<'_>, #[rename = "url"] query: String) -> Result<(), Error> {
    enqueue(ctx, query, QueueType::Back).await
}
