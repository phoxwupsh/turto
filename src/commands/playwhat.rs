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

    let guild_id = ctx.guild_id().unwrap();

    let playing_map = ctx.data().playing.read().await;
    let Some(playing) = playing_map.get(&guild_id) else {
        turto_say(ctx, NotPlaying).await?;
        return Ok(());
    };

    let meta = playing
        .ytdlfile
        .fetch_metadata(ctx.data().config.ytdlp.clone())
        .await?;
    let play_state = match playing.track_handle.get_info().await?.playing {
        PlayMode::Stop | PlayMode::End | PlayMode::Errored(_) => {
            turto_say(ctx, NotPlaying).await?;
            return Ok(());
        }
        PlayMode::Play => PlayState::Play,
        PlayMode::Pause => PlayState::Pause,
        _ => unreachable!()
    };
    drop(playing_map);

    let response = create_playing_embed(ctx, Some(play_state), &meta);
    ctx.send(CreateReply::default().embed(response)).await?;

    Ok(())
}
