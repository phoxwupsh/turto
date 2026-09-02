use crate::{
    message::TurtoMessageKind::{EmptyPlaylist, Shuffle},
    models::{alias::Context, error::CommandError},
    turto_command,
    utils::turto_say,
};
use tracing::{Span, instrument};

#[turto_command(short = "Shuffle the playlist.", long = "Shuffle the playlist.")]
#[poise::command(slash_command, guild_only)]
#[instrument(
    name = "shuffle",
    skip_all,
    parent = ctx.invocation_data::<Span>().await.as_deref().unwrap_or(&Span::none())
)]
pub async fn shuffle(ctx: Context<'_>) -> Result<(), CommandError> {
    tracing::info!("invoked");

    let guild = ctx.guild_id().ok_or(CommandError::GuildOnly)?;
    let mut guild_data = ctx.data().guilds.entry(guild).or_default();
    if guild_data.playlist.is_empty() {
        drop(guild_data);
        turto_say(ctx, EmptyPlaylist).await?;
        return Ok(());
    }
    guild_data.playlist.shuffle();
    drop(guild_data);

    tracing::info!("shuffle success");

    turto_say(ctx, Shuffle).await?;
    Ok(())
}
