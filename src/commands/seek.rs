use crate::{
    config::get_config,
    messages::{
        TurtoMessage,
        TurtoMessageKind::{
            BotNotInVoiceChannel, DifferentVoiceChannel, InvalidSeek, NotPlaying,
            SeekNotAllow, SeekNotLongEnough, SeekSuccess,
        },
    },
    models::alias::{Context, Error},
    utils::guild::{GuildUtil, VoiceChannelState},
};
use songbird::tracks::PlayMode;
use std::time::Duration;
use tracing::error;

#[poise::command(slash_command, guild_only)]
pub async fn seek(ctx: Context<'_>, #[min = 0] time: u64) -> Result<(), Error> {
    let config = get_config();
    let locale = ctx.locale();

    if !config.allow_seek {
        ctx.say(TurtoMessage {
            locale,
            kind: SeekNotAllow { backward: false },
        })
        .await?;
        return Ok(());
    }

    let guild_id = ctx.guild_id().unwrap();
    let bot_id = ctx.cache().current_user().id;
    let user_id = ctx.author().id;
    let vc_stat = ctx.guild().unwrap().cmp_voice_channel(&bot_id, &user_id);

    match vc_stat {
        VoiceChannelState::Different(bot_vc, _) | VoiceChannelState::OnlyFirst(bot_vc) => {
            ctx.say(TurtoMessage {
                locale,
                kind: DifferentVoiceChannel { bot: bot_vc },
            })
            .await?;
            return Ok(());
        }
        VoiceChannelState::OnlySecond(_) | VoiceChannelState::None => {
            ctx.say(TurtoMessage {
                locale,
                kind: BotNotInVoiceChannel,
            })
            .await?;
            return Ok(());
        }
        VoiceChannelState::Same(_) => (),
    }

    {
        let playing_map = ctx.data().playing.read().await;
        if let Some(playing) = playing_map.get(&guild_id) {
            if let Ok(track_state) = playing.track_handle.get_info().await {
                if track_state.playing == PlayMode::Stop || track_state.playing == PlayMode::End {
                    ctx.say(TurtoMessage {
                        locale,
                        kind: NotPlaying,
                    })
                    .await?;
                    return Ok(());
                }
                if track_state.position.as_secs() + config.seek_limit <= time {
                    ctx.say(TurtoMessage {
                        locale,
                        kind: InvalidSeek {
                            seek_limit: config.seek_limit,
                        },
                    })
                    .await?;
                    return Ok(());
                }
                if !config.allow_backward_seek && track_state.position.as_secs() > time {
                    ctx.say(TurtoMessage {
                        locale,
                        kind: SeekNotAllow { backward: true },
                    })
                    .await?;
                    return Ok(());
                }
            }

            let length = playing.metadata.duration.unwrap().as_secs();
            let title = playing.metadata.title.as_ref().unwrap();
            if length < time {
                ctx.say(TurtoMessage {
                    locale,
                    kind: SeekNotLongEnough { title, length },
                })
                .await?;
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
                ctx.say(TurtoMessage {
                    locale,
                    kind: SeekSuccess,
                })
                .await?;
            }
        }
    }

    Ok(())
}
