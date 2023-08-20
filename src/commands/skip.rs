use serenity::{
    framework::standard::{macros::command, CommandResult},
    model::prelude::{Mentionable, Message},
    prelude::Context,
};

use crate::{
    messages::NOT_PLAYING,
    utils::{
        guild::GuildUtil,
        play::play_next,
    },
};

#[command]
#[bucket = "music"]
async fn skip(ctx: &Context, msg: &Message) -> CommandResult {
    let guild = msg.guild(ctx).unwrap();

    let user_voice_channel = match guild.get_user_voice_channel(&msg.author.id) {
        Some(channel) => channel,
        None => {
            msg.reply(ctx, "Not in a voice channel").await?;
            return Ok(());
        }
    };

    // Get the Songbird instance
    let manager = songbird::get(ctx)
        .await
        .expect("Songbird Voice client placed in at initialization.")
        .clone();

    match guild.get_user_voice_channel(&ctx.cache.current_user_id()) {
        Some(bot_voice_channel) => {
            if user_voice_channel != bot_voice_channel { // If the bot is in another channel
                msg.reply(
                    ctx,
                    format!("You are not in {}.", bot_voice_channel.mention()),
                )
                .await?;
                return Ok(());
            }
        },
        None => {
            msg.reply(ctx, NOT_PLAYING).await?;
            return Ok(());
        }
    }

    let handler_lock = match manager.get(guild.id) {
        Some(handler_lock) => handler_lock,
        None => {
            msg.reply(ctx, NOT_PLAYING).await?;
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
