use crate::{
    models::{alias::Context, error::CommandError},
    utils::queue::{QueueType, enqueue},
};
use tracing::{Span, instrument};

#[poise::command(slash_command, guild_only)]
#[instrument(
    name = "queue",
    skip_all,
    parent = ctx.invocation_data::<Span>().await.as_deref().unwrap_or(&Span::none())
    fields(query)
)]
pub async fn queue(ctx: Context<'_>, #[rename = "url"] query: String) -> Result<(), CommandError> {
    tracing::info!("invoked");
    enqueue(ctx, query, QueueType::Back).await
}
