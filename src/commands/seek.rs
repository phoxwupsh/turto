use crate::{
    config::get_config,
    messages::TurtoMessageKind::{
        BotNotInVoiceChannel, DifferentVoiceChannel, InvalidSeek, NotPlaying, SeekNotAllow,
        SeekNotLongEnough, SeekSuccess,
    },
    models::alias::{Context, Error},
    utils::{
        guild::{GuildUtil, VoiceChannelState},
        turto_say,
    },
};
use songbird::tracks::PlayMode;
use std::time::Duration;
use tracing::error;

#[poise::command(slash_command, guild_only)]
pub async fn seek(ctx: Context<'_>, #[min = 0] time: u64) -> Result<(), Error> {
    let config = get_config();

    if !config.allow_seek {
        turto_say(ctx, SeekNotAllow { backward: false }).await?;
        return Ok(());
    }

    let guild_id = ctx.guild_id().unwrap();
    let bot_id = ctx.cache().current_user().id;
    let user_id = ctx.author().id;
    let vc_stat = ctx.guild().unwrap().cmp_voice_channel(&bot_id, &user_id);

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

    {
        let playing_map = ctx.data().playing.read().await;
        if let Some(playing) = playing_map.get(&guild_id) {
            if let Ok(track_state) = playing.track_handle.get_info().await {
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

            let length = playing.metadata.duration.unwrap().as_secs();
            let title = playing.metadata.title.as_ref().unwrap();
            if length < time {
                turto_say(ctx, SeekNotLongEnough { title, length }).await?;
                return Ok(());
            }

            ctx.defer().await?;
            if let Err(why) = playing
                .track_handle
                .seek_async(Duration::from_secs(time))
                .await
            {
                let uuid = playing.track_handle.uuid();
                error!("Failed to seek track {uuid}: {why}");
            } else {
                turto_say(ctx, SeekSuccess).await?;
            }
        }
    }

    Ok(())
}
