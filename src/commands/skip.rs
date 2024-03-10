use poise::CreateReply;

use crate::{
    messages::{
        TurtoMessage,
        TurtoMessageKind::{
            BotNotInVoiceChannel, DifferentVoiceChannel, Loading, NotPlaying, Skip,
        },
    },
    models::alias::{Context, Error},
    utils::{
        guild::{GuildUtil, VoiceChannelState},
        play::play_next,
    },
};

#[poise::command(slash_command, guild_only)]
pub async fn skip(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap();
    let bot_id = ctx.cache().current_user().id;
    let user_id = ctx.author().id;
    let vc_stat = ctx.guild().unwrap().cmp_voice_channel(&bot_id, &user_id);
    let locale = ctx.locale();

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

    let call = match songbird::get(ctx.serenity_context())
        .await
        .unwrap()
        .get(guild_id)
    {
        Some(handler_lock) => handler_lock,
        None => {
            ctx.say(TurtoMessage {
                locale,
                kind: NotPlaying,
            })
            .await?;
            return Ok(());
        }
    };
    {
        let mut call = call.lock().await;
        call.stop();
    }
    let reply = ctx
        .say(TurtoMessage {
            locale,
            kind: Loading,
        })
        .await?;
    if let Some(Ok(meta)) = play_next(
        call,
        ctx.data().guilds.clone(),
        ctx.data().playing.clone(),
        guild_id,
    )
    .await
    {
        reply
            .edit(
                ctx,
                CreateReply::default().content(TurtoMessage {
                    locale,
                    kind: Skip {
                        title: meta.title.as_ref().unwrap(),
                    },
                }),
            )
            .await?;
    } else {
        reply.delete(ctx).await?;
    }
    Ok(())
}
