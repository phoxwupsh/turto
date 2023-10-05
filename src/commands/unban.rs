use serenity::{
    framework::standard::{Args, CommandResult, macros::command},
    model::prelude::{Message, UserId},
    prelude::Context,
};
use crate::{messages::TurtoMessage, typemap::guild_data::GuildDataMap};

#[command]
#[bucket = "turto"]
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

    let guild_data_map_lock = ctx
        .data
        .read()
        .await
        .get::<GuildDataMap>()
        .unwrap()
        .clone();
    
    let unban_result = {
        let mut guild_data_map = guild_data_map_lock.lock().await;
        let guild_data = guild_data_map.entry(guild.id).or_default();
        if guild_data.config.banned.remove(&unbanned.user.id) {
            Ok(unbanned.user.id)
        } else {
            Err(unbanned.user.id)
        }
    };

    msg.reply(ctx, TurtoMessage::UserGotUnbanned(unban_result)).await?;

    Ok(())
}
