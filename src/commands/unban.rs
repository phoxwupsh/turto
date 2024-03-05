use crate::{
    config::get_config,
    messages::{
        TurtoMessage,
        TurtoMessageKind::{AdministratorOnly, Unban},
    },
    models::alias::{Context, Error},
};
use serenity::all::UserId;

#[poise::command(slash_command, guild_only)]
pub async fn unban(ctx: Context<'_>, user: UserId) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap();
    let user_id = ctx.author().id;
    let member = ctx.author_member().await.unwrap();
    let locale = ctx.locale();

    if !(member.permissions(ctx).unwrap().administrator() || get_config().is_owner(&user_id)) {
        ctx.say(TurtoMessage {
            locale,
            kind: AdministratorOnly,
        })
        .await?;
        return Ok(());
    }

    let mut guild_data = ctx.data().guilds.entry(guild_id).or_default();
    let success = guild_data.config.banned.remove(&user);
    drop(guild_data);

    ctx.say(TurtoMessage {
        locale,
        kind: Unban { success, user },
    })
    .await?;

    Ok(())
}
