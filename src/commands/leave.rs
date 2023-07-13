use crate::utils::{bot_in_voice_channel, same_voice_channel};
use serenity::{
    framework::standard::{macros::command, CommandResult},
    model::prelude::Message,
    prelude::{Context, Mentionable},
};

#[command]
#[bucket = "music"]
async fn leave(ctx: &Context, msg: &Message) -> CommandResult {
    if let Some(bot_current_voice_channel) = bot_in_voice_channel(ctx, msg).await {
        if !same_voice_channel(ctx, msg).await {
            msg.reply(
                ctx,
                format!("You are not in {}", bot_current_voice_channel.mention()),
            )
            .await?;
            return Ok(());
        }

        let guild = msg.guild(ctx).unwrap();

        let manager = songbird::get(ctx)
            .await
            .expect("Songbird Voice client placing in Resource failed.")
            .clone();

        manager.remove(guild.id).await?;
    }
    Ok(())
}
