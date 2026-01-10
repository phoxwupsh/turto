use crate::{
    message::TurtoMessageKind::{AdministratorOnly, Unban},
    models::{alias::Context, error::CommandError},
    utils::turto_say,
};
use serenity::all::UserId;
use tracing::{Span, instrument};

#[poise::command(slash_command, guild_only)]
#[instrument(
    name = "unban",
    skip_all,
    parent = ctx.invocation_data::<Span>().await.as_deref().unwrap_or(&Span::none())
    fields(target = %user)
)]
pub async fn unban(ctx: Context<'_>, user: UserId) -> Result<(), CommandError> {
    tracing::info!(target = %user, "command invoked");

    let guild_id = ctx.guild_id().unwrap();
    let user_id = ctx.author().id;

    // Since this is a guild only command interaction
    let is_admin = ctx
        .author_member()
        .await
        .unwrap()
        .permissions
        .unwrap()
        .administrator();

    if !(is_admin || ctx.data().config.is_owner(&user_id)) {
        turto_say(ctx, AdministratorOnly).await?;
        return Ok(());
    }

    let mut guild_data = ctx.data().guilds.entry(guild_id).or_default();
    let success = guild_data.config.banned.remove(&user);
    drop(guild_data);

    tracing::info!("unban success");

    turto_say(ctx, Unban { success, user }).await?;
    Ok(())
}
