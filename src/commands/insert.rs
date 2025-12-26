use tracing::{Span, instrument};
use crate::{
    models::{alias::Context, error::CommandError},
    utils::queue::{QueueType, enqueue},
};

#[poise::command(slash_command, guild_only)]
#[instrument(
    name = "insert",
    skip_all,
    parent = ctx.invocation_data::<Span>().await.as_deref().unwrap_or(&Span::none())
    fields(query)
)]
pub async fn insert(ctx: Context<'_>, #[rename = "url"] query: String) -> Result<(), CommandError> {
    tracing::info!("invoked");
    enqueue(ctx, query, QueueType::Front).await
}
