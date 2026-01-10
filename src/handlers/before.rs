use crate::{
    message::TurtoMessageKind::BannedUserResponse,
    models::{alias::Context, error::CommandError},
    utils::turto_say,
};
use std::{future::Future, pin::Pin};
use tracing::{Instrument, info_span};

pub fn command_check(
    ctx: Context<'_>,
) -> Pin<Box<dyn Future<Output = Result<bool, CommandError>> + Send + '_>> {
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
                turto_say(ctx, BannedUserResponse).await?;
            }

            return Ok(!is_banned);
        }
        Ok(true)
    })
}

pub fn pre_command(ctx: Context<'_>) -> Pin<Box<dyn Future<Output = ()> + Send + '_>> {
    let command_span = info_span!(
        "command",
        command = ctx.invoked_command_name(),
        user = %ctx.author().id,
        guild = %ctx.guild_id().unwrap(),
        id = ctx.id()
    );

    let command_span_inner = command_span.clone();

    let fut = async move {
        ctx.set_invocation_data(command_span_inner).await;
    };

    Box::pin(fut.instrument(command_span))
}
