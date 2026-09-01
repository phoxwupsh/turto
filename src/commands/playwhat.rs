use crate::{
    message::TurtoMessageKind::NotPlaying,
    models::{alias::Context, error::CommandError, playing::PlayState},
    utils::{create_playing_embed, turto_say},
};
use poise::CreateReply;
use songbird::tracks::PlayMode;
use tracing::{Span, instrument};

#[poise::command(slash_command, guild_only)]
#[instrument(
    name = "playwhat",
    skip_all,
    parent = ctx.invocation_data::<Span>().await.as_deref().unwrap_or(&Span::none())
)]
pub async fn playwhat(ctx: Context<'_>) -> Result<(), CommandError> {
    tracing::info!("invoked");

    let guild_id = ctx.guild_id().ok_or(CommandError::GuildOnly)?;

    // Clone out and release the guard before awaiting: `playing` is one map shared by
    // every guild, and holding it read-locked across an extract and a Discord
    // round-trip blocks the write `player::spawn_playback` needs to start a track
    // anywhere.
    let current = ctx
        .data()
        .playing
        .read()
        .await
        .get(&guild_id)
        .map(|playing| (playing.ytdlfile.clone(), playing.track_handle.clone()));
    let Some((ytdlfile, track_handle)) = current else {
        turto_say(ctx, NotPlaying).await?;
        return Ok(());
    };

    let meta = ytdlfile.fetch_metadata().await?;
    // `PlayMode` is `#[non_exhaustive]`: anything songbird adds later is treated as
    // "not playing" rather than panicking (the release profile aborts on panic).
    let play_state = match track_handle.get_info().await?.playing {
        PlayMode::Play => PlayState::Play,
        PlayMode::Pause => PlayState::Pause,
        PlayMode::Stop | PlayMode::End | PlayMode::Errored(_) => {
            turto_say(ctx, NotPlaying).await?;
            return Ok(());
        }
        other => {
            tracing::warn!(?other, "unrecognized play mode; reporting as not playing");
            turto_say(ctx, NotPlaying).await?;
            return Ok(());
        }
    };

    let response = create_playing_embed(ctx, Some(play_state), &meta);
    ctx.send(CreateReply::default().embed(response)).await?;

    Ok(())
}
