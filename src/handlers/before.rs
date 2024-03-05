use crate::{
    messages::{TurtoMessage, TurtoMessageKind::BannedUserResponse},
    models::alias::{Context, Error},
};
use std::{future::Future, pin::Pin};

pub fn before(ctx: Context<'_>) -> Pin<Box<dyn Future<Output = Result<bool, Error>> + Send + '_>> {
    Box::pin(async move {
        if let Some(guild_id) = ctx.guild_id() {
            let user_id = ctx.author().id;
            let is_banned = ctx
                .data()
                .guilds
                .entry(guild_id)
                .or_default()
                .config
                .banned
                .contains(&user_id);

            if is_banned {
                ctx.say(TurtoMessage {
                    locale: ctx.locale(),
                    kind: BannedUserResponse,
                })
                .await?;
            }

            return Ok(!is_banned);
        }
        Ok(true)
    })
}
