use crate::{
    models::{alias::Context, error::CommandError},
    utils::queue::{QueueType, enqueue},
};

#[poise::command(slash_command, guild_only)]
pub async fn queue(ctx: Context<'_>, #[rename = "url"] query: String) -> Result<(), CommandError> {
    enqueue(ctx, query, QueueType::Back).await
}
