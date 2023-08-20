use crate::{
    messages::NOT_IN_ANY_VOICE_CHANNEL,
    utils::guild::GuildUtil,
};

use tracing::error;

use serenity::{
    client::Context,
    framework::standard::{macros::command, CommandResult},
    model::channel::Message,
    prelude::Mentionable,
};

#[command]
#[bucket = "music"]
async fn join(ctx: &Context, msg: &Message) -> CommandResult {
    let guild = msg.guild(ctx).unwrap();

    let connect_to = match guild.get_user_voice_channel(&msg.author.id) {
        Some(channel) => channel,
        None => {
            msg.reply(ctx, NOT_IN_ANY_VOICE_CHANNEL).await?;
            return Ok(());
        }
    };

    // Check if the bot is already in another voice channel or not
    if let Some(bot_voice_channel) = guild.get_user_voice_channel(&ctx.cache.current_user_id()) {
        if Some(bot_voice_channel) != guild.get_user_voice_channel(&msg.author.id) {
            // Notify th user if they are in different voice channel
            msg.reply(
                ctx,
                format!("I'm currently in {}.", bot_voice_channel.mention()),
            )
            .await?;
            return Ok(());
        }
    }
    
    let manager = songbird::get(ctx)
        .await
        .expect("Songbird Voice client placing in Resource failed.")
        .clone();


    msg.channel_id
        .say(ctx, format!("🐢 {}", connect_to.mention()))
        .await?;
    if let (_, Err(why)) = manager.join(guild.id, connect_to).await {
        error!("Error join voice channel {}: {:?}", connect_to, why);
    }
    Ok(())
}
