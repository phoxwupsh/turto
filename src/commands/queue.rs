use tracing::instrument;
use crate::{
    models::{alias::Context, error::CommandError},
    utils::queue::{QueueType, enqueue},
};

#[poise::command(slash_command, guild_only)]
#[instrument(
    name = "queue",
    skip_all,
    fields(
        user = %ctx.author().id,
        guild = %ctx.guild_id().unwrap()
    )
)]
pub async fn queue(ctx: Context<'_>, #[rename = "url"] query: String) -> Result<(), CommandError> {
    tracing::info!("invoked");
    enqueue(ctx, query, QueueType::Back).await
}
