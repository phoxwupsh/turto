use serenity::{
    framework::standard::{macros::command, CommandResult},
    model::prelude::Message,
    prelude::Context,
};

use crate::{
    messages::TurtoMessage,
    utils::{
        guild::{GuildUtil, VoiceChannelState},
        play::play_next,
    },
};

#[command]
#[bucket = "turto"]
async fn skip(ctx: &Context, msg: &Message) -> CommandResult {
    let guild = msg.guild(ctx).unwrap();

    match guild.cmp_voice_channel(&ctx.cache.current_user_id(), &msg.author.id) {
        VoiceChannelState::Different(bot_vc, _) | VoiceChannelState::OnlyFirst(bot_vc) => {
            msg.reply(ctx, TurtoMessage::DifferentVoiceChannel { bot: &bot_vc })
                .await?;
            return Ok(());
        }
        VoiceChannelState::OnlySecond(_) | VoiceChannelState::None => {
            msg.reply(ctx, TurtoMessage::BotNotInVoiceChannel).await?;
            return Ok(());
        }
        VoiceChannelState::Same(_) => (),
    }

    let handler_lock = match songbird::get(ctx).await.unwrap().get(guild.id) {
        Some(handler_lock) => handler_lock,
        None => {
            msg.reply(ctx, TurtoMessage::NotPlaying).await?;
            return Ok(());
        }
    };
    {
        let mut handler = handler_lock.lock().await;
        handler.stop();
    }
    if let Ok(meta) = play_next(ctx, msg.guild_id.unwrap()).await {
        msg.reply(
            ctx,
            TurtoMessage::Skip {
                title: meta.title.as_ref().unwrap(),
            },
        )
        .await?;
    }
    Ok(())
}
