use crate::{config::get_config, messages::TurtoMessage, typemap::guild_data::GuildDataMap};
use serenity::{
    framework::standard::{macros::command, Args, CommandResult},
    model::prelude::Message,
    prelude::Context,
    utils::parse_user_mention,
};

#[command]
#[bucket = "turto"]
async fn ban(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let guild = msg.guild_id.unwrap();
    let member = guild.member(ctx, &msg.author).await?;
    if !(member.permissions(ctx).unwrap().administrator() || get_config().is_owner(&msg.author.id))
    {
        msg.reply(ctx, TurtoMessage::AdministratorOnly).await?;
        return Ok(());
    }
    let banned = {
        let Some(banned) = parse_user_mention(args.rest()) else {
            msg.reply(ctx, TurtoMessage::InvalidUser).await?;
            return Ok(());
        };
        if let Ok(banned_member) = guild.member(ctx, banned).await {
            banned_member
        } else {
            msg.reply(ctx, TurtoMessage::InvalidUser).await?;
            return Ok(());
        }
    };

    let guild_data_map = ctx.data.read().await.get::<GuildDataMap>().unwrap().clone();
    let mut guild_data = guild_data_map.entry(guild).or_default();
    let success = guild_data.config.banned.insert(banned.user.id);
    drop(guild_data);

    msg.reply(
        ctx,
        TurtoMessage::Ban {
            success,
            user: banned.user.id,
        },
    )
    .await?;

    Ok(())
}
