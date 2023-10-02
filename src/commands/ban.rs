use serenity::{
    framework::standard::{Args, CommandResult, macros::command},
    model::prelude::{Message, UserId},
    prelude::Context,
};
use crate::{messages::TurtoMessage, typemap::config::GuildConfigs};

#[command]
#[bucket = "music"]
async fn ban(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let guild = msg.guild(ctx).unwrap();
    let member = guild.member(ctx, &msg.author).await?;
    if  !member.permissions(ctx).unwrap().administrator() {
        msg.reply(ctx, TurtoMessage::AdministratorOnly).await?;
        return Ok(())
    }
    let banned = {
        let Ok(banned) = args.parse::<UserId>() else {
            msg.reply(ctx, TurtoMessage::InvalidUser).await?;
            return Ok(())  
        };
        if let Ok(banned_member) = guild.member(ctx, banned).await {
            banned_member
        } else {
            msg.reply(ctx, TurtoMessage::InvalidUser).await?;
            return Ok(())            
        }
    };

    let guild_configs_lock = ctx
        .data
        .read()
        .await
        .get::<GuildConfigs>()
        .unwrap()
        .clone();
    
    let ban_result = {
        let mut guild_configs = guild_configs_lock.lock().await;
        let guild_config = guild_configs.entry(guild.id).or_default();
        if guild_config.banned.insert(banned.user.id) {
            Ok(banned.user.id)
        } else {
            Err(banned.user.id)
        }
    };

    msg.reply(ctx, TurtoMessage::UserGotBanned(ban_result)).await?;

    Ok(())
}
