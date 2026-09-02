use crate::{
    message::TurtoMessageKind::{
        BotNotInVoiceChannel, DifferentVoiceChannel, InvalidSeek, NotPlaying, SeekNotAllow,
        SeekNotLongEnough, SeekSuccess,
    },
    models::{alias::Context, error::CommandError},
    turto_command,
    utils::{
        guild::{GuildUtil, VoiceChannelState},
        turto_say,
    },
};
use songbird::tracks::PlayMode;
use std::time::Duration;
use tracing::{Span, instrument};

#[turto_command(
    short = "Seek the currently playing item to certain time",
    long = "If there is a currently playing or paused item, jump to the specified `time` in seconds."
)]
#[poise::command(slash_command, guild_only)]
#[instrument(
    name = "seek",
    skip_all,
    parent = ctx.invocation_data::<Span>().await.as_deref().unwrap_or(&Span::none())
    fields(time)
)]
pub async fn seek(
    ctx: Context<'_>,
    #[description = "The time to seek, denoted in second"]
    #[min = 0]
    time: u64,
) -> Result<(), CommandError> {
    tracing::info!("invoked");

    let config = ctx.data().config.clone();

    if !config.allow_seek {
        turto_say(ctx, SeekNotAllow { backward: false }).await?;
        return Ok(());
    }

    let guild_id = ctx.guild_id().ok_or(CommandError::GuildOnly)?;
    let bot_id = ctx.cache().current_user().id;
    let user_id = ctx.author().id;
    let vc_stat = ctx
        .guild()
        .ok_or(CommandError::GuildOnly)?
        .cmp_voice_channel(&bot_id, &user_id);

    match vc_stat {
        VoiceChannelState::Different(bot, _) | VoiceChannelState::OnlyFirst(bot) => {
            turto_say(ctx, DifferentVoiceChannel { bot }).await?;
            return Ok(());
        }
        VoiceChannelState::OnlySecond(_) | VoiceChannelState::None => {
            turto_say(ctx, BotNotInVoiceChannel).await?;
            return Ok(());
        }
        VoiceChannelState::Same(_) => (),
    }

    // Clone out and release the guard before awaiting: `playing` is one map shared
    // by every guild, and a seek round-trip held under its read lock blocks the
    // write `player::spawn_playback` needs to start a track anywhere.
    let current = ctx
        .data()
        .playing
        .read()
        .await
        .get(&guild_id)
        .map(|playing| (playing.ytdlfile.clone(), playing.track_handle.clone()));
    let Some((ytdlfile, track_handle)) = current else {
        // A slash command that returns without replying leaves the user staring at
        // "the application did not respond"; every other command answers `NotPlaying`.
        turto_say(ctx, NotPlaying).await?;
        return Ok(());
    };

    if let Ok(track_state) = track_handle.get_info().await {
        if track_state.playing == PlayMode::Stop || track_state.playing == PlayMode::End {
            turto_say(ctx, NotPlaying).await?;
            return Ok(());
        }
        if track_state.position.as_secs() + config.seek_limit <= time {
            let seek_limit = config.seek_limit;
            turto_say(ctx, InvalidSeek { seek_limit }).await?;
            return Ok(());
        }
        if !config.allow_backward_seek && track_state.position.as_secs() > time {
            turto_say(ctx, SeekNotAllow { backward: true }).await?;
            return Ok(());
        }
    }

    let meta = ytdlfile.fetch_metadata().await?;
    let length = meta.duration.map(|t| t as u64).unwrap_or(0);
    let title = meta.title.as_deref().unwrap_or_default();
    if length < time {
        turto_say(ctx, SeekNotLongEnough { title, length }).await?;
        return Ok(());
    }

    ctx.defer().await?;
    track_handle.seek_async(Duration::from_secs(time)).await?;

    tracing::info!("seek success");

    turto_say(ctx, SeekSuccess).await?;

    Ok(())
}
