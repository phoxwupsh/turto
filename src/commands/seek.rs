use crate::{
    config::get_config,
    messages::TurtoMessage,
    typemap::playing::PlayingMap,
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
    let guild = msg.guild(&ctx.cache).unwrap().clone();
    let config = get_config();

    if !config.allow_seek {
        msg.reply(ctx, TurtoMessage::SeekNotAllow { backward: false })
            .await?;
        return Ok(());
    }

    let bot_id = ctx.cache.current_user().id;
    match guild.cmp_voice_channel(&bot_id, &msg.author.id) {
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
    let playing_lock = ctx.data.read().await.get::<PlayingMap>().unwrap().clone();
    {
        let playing_map = playing_lock.read().await;
        if let Some(playing) = playing_map.get(&msg.guild_id.unwrap()) {
            if let Ok(track_state) = playing.track_handle.get_info().await {
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

            let length = playing.metadata.duration.unwrap().as_secs();
            let title = playing.metadata.title.as_ref().unwrap();
            if length < sec {
                msg.reply(ctx, TurtoMessage::SeekNotLongEnough { title, length })
                    .await?;
                return Ok(());
            }

            if let Err(why) = playing
                .track_handle
                .seek_async(Duration::from_secs(sec))
                .await
            {
                let uuid = playing.track_handle.uuid();
                error!("Failed to seek track {uuid}: {why}");
            }
        }
    }

    Ok(())
}
