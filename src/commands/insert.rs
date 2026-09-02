use tracing::{Span, instrument};
use crate::{
    models::{alias::Context, error::CommandError},
    turto_command,
    utils::queue::{QueueType, enqueue},
};

#[turto_command(
    short = "Add new item to the beginning of playlist.",
    long = "Add new item to the beginning of playlist, the parameter `url` can be any URL. Supported various platforms, as long as it's supported by yt-dlp. You can also directly add entire YouTube playlists, and playlist URLs will be prioritized."
)]
#[poise::command(slash_command, guild_only)]
#[instrument(
    name = "insert",
    skip_all,
    parent = ctx.invocation_data::<Span>().await.as_deref().unwrap_or(&Span::none())
    fields(query)
)]
pub async fn insert(
    ctx: Context<'_>,
    #[description = "The link to what you want to insert"]
    #[rename = "url"]
    query: String,
) -> Result<(), CommandError> {
    tracing::info!("invoked");
    enqueue(ctx, query, QueueType::Front).await
}
