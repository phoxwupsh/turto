use crate::utils::guild::GuildUtil;
use serenity::{
    framework::standard::{macros::command, CommandResult},
    model::prelude::Message,
    prelude::{Context, Mentionable},
};

#[command]
#[bucket = "music"]
async fn leave(ctx: &Context, msg: &Message) -> CommandResult {
    let guild = msg.guild(ctx).unwrap();
    if let Some(bot_voice_channel) = guild.get_user_voice_channel(&ctx.cache.current_user_id()) {
        if  Some(bot_voice_channel) != guild.get_user_voice_channel(&msg.author.id) {
            msg.reply(
                ctx,
                format!("You are not in {}", bot_voice_channel.mention()),
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
