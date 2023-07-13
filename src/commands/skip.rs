use serenity::{
    prelude::Context, 
    model::prelude::{
        Message, 
        Mentionable
    }, 
    framework::standard::{
        CommandResult, 
        macros::command
    }
};

use crate::{utils::{user_in_voice_channel, bot_in_voice_channel, same_voice_channel, play_next}, messages::NOT_PLAYING};

#[command]
#[bucket = "music"]
async fn skip(ctx: &Context, msg: &Message) -> CommandResult {
    let guild = msg.guild(&ctx.cache).unwrap();
    let guild_id = guild.id;

    let user_voice_channel = match user_in_voice_channel(ctx, msg).await {
        Some(channel) => channel,
        None => {
            msg.reply(ctx, "Not in a voice channel").await?;
            return Ok(());
        }
    };

    // Get the Songbird instance
    let manager = songbird::get(ctx).await
        .expect("Songbird Voice client placed in at initialization.")
        .clone();

    if let Some(current_channel_id) = bot_in_voice_channel(ctx, msg).await {
        if !same_voice_channel(ctx, msg).await {
            msg.reply(ctx, format!("You are not in {}.", current_channel_id.mention())).await?;
            return Ok(())
        }
    }
    else {
        msg.reply(ctx, NOT_PLAYING).await?;
        return Ok(())
    }

    let handler_lock = match manager.get(guild_id) {
        Some(handler_lock) => { // If the bot is already in a voice channel
            let current_channel_id = guild.voice_states.get(&ctx.cache.current_user_id())
                .and_then(|voice_state| voice_state.channel_id)
                .unwrap();

            if current_channel_id != user_voice_channel {
            // The bot is in another channel
                msg.reply(ctx, format!("You are not in {}.", current_channel_id.mention())).await?;
                return Ok(());
            }
            handler_lock
        },
        None => {
            msg.reply(ctx, NOT_PLAYING).await?;
            return Ok(())
        }
    };
    {
        let mut handler = handler_lock.lock().await;
        handler.stop();
    }
    
    if let Ok(meta) = play_next(ctx, msg.guild_id.unwrap()).await {
        msg.reply(ctx, format!("⏭️ {}", meta.title.clone().unwrap())).await?;
    }
    Ok(())
}