use crate::{
    config::TurtoConfigProvider,
    messages::TurtoMessage,
    typemap::playing::Playing,
    utils::guild::{GuildUtil, VoiceChannelState},
};
use serenity::{
    framework::standard::{macros::command, Args, CommandResult},
    model::prelude::Message,
    prelude::Context,
};
use songbird::tracks::PlayMode;
use std::time::Duration;
use tracing::error;

#[command]
#[bucket = "turto"]
async fn seek(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let guild = msg.guild(ctx).unwrap();
    let config = TurtoConfigProvider::get();

    if !config.allow_seek {
        msg.reply(ctx, TurtoMessage::SeekNotAllow { backward: false })
            .await?;
        return Ok(());
    }

    match guild.cmp_voice_channel(&ctx.cache.current_user_id(), &msg.author.id) {
        VoiceChannelState::Different(bot_vc, _) | VoiceChannelState::OnlyFirst(bot_vc) => {
            msg.reply(ctx, TurtoMessage::DifferentVoiceChannel { bot: bot_vc })
                .await?;
            return Ok(());
        }
        VoiceChannelState::OnlySecond(_) | VoiceChannelState::None => {
            msg.reply(ctx, TurtoMessage::BotNotInVoiceChannel).await?;
            return Ok(());
        }
        VoiceChannelState::Same(_) => (),
    }

    let sec = match args.parse::<u64>() {
        Ok(s) => s,
        Err(_) => {
            msg.reply(
                ctx,
                TurtoMessage::InvalidSeek {
                    seek_limit: config.seek_limit,
                },
            )
            .await?;
            return Ok(());
        }
    };

    // Update the volume if there is a currently playing TrackHandle
    let playing_lock = ctx.data.read().await.get::<Playing>().unwrap().clone();
    {
        let playing = playing_lock.read().await;
        if let Some(current_track) = playing.get(&msg.guild_id.unwrap()) {
            if let Ok(track_state) = current_track.get_info().await {
                if track_state.playing == PlayMode::Stop || track_state.playing == PlayMode::End {
                    msg.reply(ctx, TurtoMessage::NotPlaying).await?;
                    return Ok(());
                }
                if track_state.position.as_secs() + config.seek_limit <= sec {
                    msg.reply(
                        ctx,
                        TurtoMessage::InvalidSeek {
                            seek_limit: config.seek_limit,
                        },
                    )
                    .await?;
                    return Ok(());
                }
                if !config.allow_backward_seek && track_state.position.as_secs() > sec {
                    msg.reply(ctx, TurtoMessage::SeekNotAllow { backward: true })
                        .await?;
                    return Ok(());
                }
            }

            let track_sec = current_track.metadata().duration.unwrap().as_secs();
            if track_sec < sec {
                msg.reply(
                    ctx,
                    TurtoMessage::SeekNotLongEnough {
                        title: current_track.metadata().title.as_ref().unwrap(),
                        length: track_sec,
                    },
                )
                .await?;
                return Ok(());
            }

            if let Err(why) = current_track.seek_time(Duration::from_secs(sec)) {
                error!("Failed to seek track {}: {}", current_track.uuid(), why);
            }
        }
    }

    Ok(())
}
