use crate::{
    message::TurtoMessageKind::{EmptyPlaylist, Shuffle},
    models::{alias::Context, error::CommandError},
    utils::turto_say,
};
use rand::{seq::SliceRandom, thread_rng};
use tracing::{Span, instrument};

#[poise::command(slash_command, guild_only)]
#[instrument(
    name = "shuffle",
    skip_all,
    parent = ctx.invocation_data::<Span>().await.as_deref().unwrap_or(&Span::none())
)]
pub async fn shuffle(ctx: Context<'_>) -> Result<(), CommandError> {
    tracing::info!("invoked");

    let guild = ctx.guild_id().unwrap();
    let mut guild_data = ctx.data().guilds.entry(guild).or_default();
    if guild_data.playlist.is_empty() {
        drop(guild_data);
        turto_say(ctx, EmptyPlaylist).await?;
        return Ok(());
    }
    let playlist = guild_data.playlist.make_contiguous();
    playlist.shuffle(&mut thread_rng());
    drop(guild_data);

    tracing::info!("shuffle success");

    turto_say(ctx, Shuffle).await?;
    Ok(())
}
