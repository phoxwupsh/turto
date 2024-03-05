use crate::{
    messages::{
        TurtoMessage,
        TurtoMessageKind::{BotNotInVoiceChannel, DifferentVoiceChannel, NotPlaying, Pause},
    },
    models::alias::{Context, Error},
    utils::guild::{GuildUtil, VoiceChannelState},
};
use tracing::error;

#[poise::command(slash_command, guild_only)]
pub async fn pause(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap();
    let bot_id = ctx.cache().current_user().id;
    let user_id = ctx.author().id;
    let vc_stat = ctx.guild().unwrap().cmp_voice_channel(&bot_id, &user_id);
    let locale = ctx.locale();

    match vc_stat {
        VoiceChannelState::None | VoiceChannelState::OnlySecond(_) => {
            ctx.say(TurtoMessage {
                locale,
                kind: BotNotInVoiceChannel,
            })
            .await?;
            return Ok(());
        }
        VoiceChannelState::Different(bot_vc, _) | VoiceChannelState::OnlyFirst(bot_vc) => {
            ctx.say(TurtoMessage {
                locale,
                kind: DifferentVoiceChannel { bot: bot_vc },
            })
            .await?;
            return Ok(());
        }
        VoiceChannelState::Same(_) => (),
    }

    let playing_map = ctx.data().playing.read().await;
    let Some(playing) = playing_map.get(&guild_id) else {
        ctx.say(TurtoMessage {
            locale,
            kind: NotPlaying,
        })
        .await?;
        return Ok(());
    };

    if let Err(why) = playing.track_handle.pause() {
        let uuid = playing.track_handle.uuid();
        error!("Failed to pause track {uuid}: {why}");
    }
    let title = playing.metadata.title.as_ref().unwrap();
    ctx.say(TurtoMessage {
        locale,
        kind: Pause { title },
    })
    .await?;

    Ok(())
}
