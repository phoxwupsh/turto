use serenity::{
    framework::standard::{
        macros::command, 
        CommandResult
    }, 
    prelude::{Context, Mentionable}, 
    model::prelude::Message
};
use crate::utils::{bot_in_voice_channel, same_voice_channel};

#[command]
#[description = "讓turto離開目前所在的語音頻道。"]
#[usage = ""]
async fn leave(ctx: &Context, msg: &Message) -> CommandResult {
    if let Some (bot_current_voice_channel) = bot_in_voice_channel(ctx, msg).await {
        if !same_voice_channel(ctx, msg).await {
            msg.reply(ctx, format!("You are not in {}", bot_current_voice_channel.mention())).await?;
            return  Ok(())
        }

        let guild = msg.guild(&ctx.cache).unwrap();

        let manager = songbird::get(&ctx).await
            .expect("Songbird Voice client placing in Resource failed.")
            .clone();
    
        manager.remove(guild.id).await?;
    }
    Ok(())
}