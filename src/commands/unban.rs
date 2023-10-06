use crate::{
    config::TurtoConfigProvider, messages::TurtoMessage, typemap::guild_data::GuildDataMap,
};
use serenity::{
    framework::standard::{macros::command, Args, CommandResult},
    model::prelude::{Message, UserId},
    prelude::Context,
};

#[command]
#[bucket = "turto"]
async fn unban(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let guild = msg.guild(ctx).unwrap();
    let member = guild.member(ctx, &msg.author).await?;
    if !(member.permissions(ctx).unwrap().administrator()
        || TurtoConfigProvider::get().is_owner(&msg.author.id))
    {
        msg.reply(ctx, TurtoMessage::AdministratorOnly).await?;
        return Ok(());
    }
    let unbanned = {
        let Ok(unbanned) = args.parse::<UserId>() else {
            msg.reply(ctx, TurtoMessage::InvalidUser).await?;
            return Ok(());
        };
        if let Ok(unbanned_member) = guild.member(ctx, unbanned).await {
            unbanned_member
        } else {
            msg.reply(ctx, TurtoMessage::InvalidUser).await?;
            return Ok(());
        }
    };

    let guild_data_map = ctx.data.read().await.get::<GuildDataMap>().unwrap().clone();
    let mut guild_data = guild_data_map.entry(guild.id).or_default();
    let unban_result = if guild_data.config.banned.remove(&unbanned.user.id) {
        Ok(unbanned.user.id)
    } else {
        Err(unbanned.user.id)
    };
    drop(guild_data);

    msg.reply(ctx, TurtoMessage::UserGotUnbanned(unban_result))
        .await?;

    Ok(())
}
