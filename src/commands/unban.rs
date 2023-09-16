use serenity::{
    framework::standard::{Args, CommandResult, macros::command},
    model::prelude::{Message, UserId},
    prelude::Context,
};
use crate::{messages::TurtoMessage, guild::setting::GuildSettings, models::guild_setting::GuildSetting};

#[command]
#[bucket = "music"]
async fn unban(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let guild = msg.guild(ctx).unwrap();
    let member = guild.member(ctx, &msg.author).await?;
    if  !member.permissions(ctx).unwrap().administrator() {
        msg.reply(ctx, TurtoMessage::AdministratorOnly).await?;
        return Ok(())
    }
    let unbanned = {
        let Ok(unbanned) = args.parse::<UserId>() else {
            msg.reply(ctx, TurtoMessage::InvalidUser).await?;
            return Ok(())
        };
        if let Ok(unbanned_member) = guild.member(ctx, unbanned).await {
            unbanned_member
        } else {
            msg.reply(ctx, TurtoMessage::InvalidUser).await?;
            return Ok(())            
        }
    };

    let guild_settings_lock = ctx
        .data
        .read()
        .await
        .get::<GuildSettings>()
        .expect("Expected GuildSettings in TypeMap")
        .clone();
    
    let unban_result = {
        let mut guild_settings = guild_settings_lock.lock().await;
        let guild_setting = guild_settings.entry(guild.id).or_insert_with(GuildSetting::default);
        if guild_setting.banned.remove(&unbanned.user.id) {
            Ok(unbanned.user.id)
        } else {
            Err(unbanned.user.id)
        }
    };

    msg.reply(ctx, TurtoMessage::UserGotUnbanned(unban_result)).await?;

    Ok(())
}
