use crate::{
    models::{alias::Context, error::CommandError},
    turto_command,
    utils::queue::{QueueType, enqueue},
};
use tracing::{Span, instrument};

#[turto_command(
    short = "Add new item to the end of playlist.",
    long = "Add new item to the end of playlist, the parameter `url` can be any URL. Supported various platforms, as long as it's supported by yt-dlp. You can also directly add entire YouTube playlists, and playlist URLs will be prioritized."
)]
#[poise::command(slash_command, guild_only)]
#[instrument(
    name = "queue",
    skip_all,
    parent = ctx.invocation_data::<Span>().await.as_deref().unwrap_or(&Span::none())
    fields(query)
)]
pub async fn queue(
    ctx: Context<'_>,
    #[description = "The link to what you want to enqueue"]
    #[rename = "url"]
    query: String,
) -> Result<(), CommandError> {
    tracing::info!("invoked");
    enqueue(ctx, query, QueueType::Back).await
}
