use tracing::{Span, instrument};
use crate::{
    message::TurtoMessageKind::RemoveAll,
    models::{alias::Context, error::CommandError},
    utils::turto_say,
};

#[poise::command(slash_command, prefix_command, guild_only)]
#[instrument(
    name = "clear",
    skip_all,
    parent = ctx.invocation_data::<Span>().await.as_deref().unwrap_or(&Span::none())
)]
pub async fn clear(ctx: Context<'_>) -> Result<(), CommandError> {
    tracing::info!("invoked");

    let guild_id = ctx.guild_id().unwrap();
    let mut guild_data = ctx.data().guilds.entry(guild_id).or_default();

    guild_data.playlist.clear();
    drop(guild_data);

    turto_say(ctx, RemoveAll).await?;
    Ok(())
}
