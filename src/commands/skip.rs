use serenity::{
    framework::standard::{macros::command, CommandResult},
    model::prelude::{Mentionable, Message},
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
#[bucket = "music"]
async fn skip(ctx: &Context, msg: &Message) -> CommandResult {
    let guild = msg.guild(ctx).unwrap();

    match guild.cmp_voice_channel(&ctx.cache.current_user_id(), &msg.author.id) {
        VoiceChannelState::Different(bot_vc, _) | VoiceChannelState::OnlyFirst(bot_vc) => {
            msg.reply(ctx, format!("You are not in {}", bot_vc.mention())).await?;
            return Ok(());
        }
        VoiceChannelState::OnlySecond(_) | VoiceChannelState::None => {
            msg.reply(ctx, "Currently not in a voice channel").await?;
            return Ok(());
        }
        VoiceChannelState::Same(_) => (),
    }

    let handler_lock = match songbird::get(ctx)
        .await
        .expect("Songbird Voice client placed in at initialization.")
        .get(guild.id)
    {
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
        msg.reply(ctx, format!("⏭️ {}", meta.title.clone().unwrap()))
            .await?;
    }
    Ok(())
}
