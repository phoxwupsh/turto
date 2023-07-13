use crate::{
    messages::NOT_IN_ANY_VOICE_CHANNEL,
    utils::{bot_in_voice_channel, same_voice_channel, user_in_voice_channel},
};

use tracing::error;

use serenity::{
    client::Context,
    framework::standard::{macros::command, CommandResult},
    model::channel::Message,
    prelude::Mentionable,
};

#[command]
async fn join(ctx: &Context, msg: &Message) -> CommandResult {
    let guild = msg.guild(&ctx.cache).unwrap();

    let connect_to = match user_in_voice_channel(ctx, msg).await {
        Some(channel) => channel,
        None => {
            msg.reply(ctx, NOT_IN_ANY_VOICE_CHANNEL).await?;
            return Ok(());
        }
    };

    let manager = songbird::get(ctx)
        .await
        .expect("Songbird Voice client placing in Resource failed.")
        .clone();

    // Check if the bot is already in another voice channel or not
    if let Some(current_bot_voice_channel) = bot_in_voice_channel(ctx, msg).await {
        if !same_voice_channel(ctx, msg).await {
            // Notify th user if they are in different voice channel
            msg.reply(
                ctx,
                format!("I'm currently in {}.", current_bot_voice_channel.mention()),
            )
            .await?;
            return Ok(());
        }
    }

    msg.channel_id
        .say(ctx, format!("🐢 {}", connect_to.mention()))
        .await?;
    if let (_, Err(why)) = manager.join(guild.id, connect_to).await {
        error!("Error join voice channel {}: {:?}", connect_to, why);
    }
    Ok(())
}
