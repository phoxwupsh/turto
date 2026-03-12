use crate::{
    message::TurtoMessageKind::{BotNotInVoiceChannel, DifferentVoiceChannel, Leave},
    models::{alias::Context, error::CommandError},
    utils::{
        guild::{GuildUtil, VoiceChannelState},
        turto_say,
    },
};
use tracing::{Span, instrument};

#[poise::command(slash_command, guild_only)]
#[instrument(
    name = "leave",
    skip_all,
    parent = ctx.invocation_data::<Span>().await.as_deref().unwrap_or(&Span::none())
)]
pub async fn leave(ctx: Context<'_>) -> Result<(), CommandError> {
    tracing::info!("invoked");

    let guild_id = ctx.guild_id().ok_or(CommandError::GuildOnly)?;
    let bot_id = ctx.cache().current_user().id;
    let user_id = ctx.author().id;
    let vc_stat = ctx
        .guild()
        .ok_or(CommandError::GuildOnly)?
        .cmp_voice_channel(&bot_id, &user_id);

    let channel = match vc_stat {
        VoiceChannelState::None | VoiceChannelState::OnlySecond(_) => {
            turto_say(ctx, BotNotInVoiceChannel).await?;
            return Ok(());
        }
        VoiceChannelState::Different(bot, _) | VoiceChannelState::OnlyFirst(bot) => {
            turto_say(ctx, DifferentVoiceChannel { bot }).await?;
            return Ok(());
        }
        VoiceChannelState::Same(vc) => vc,
    };

    let manager = songbird::get(ctx.serenity_context()).await.unwrap();
    manager.remove(guild_id).await?;

    // explicitly stop to avoid continue playing after re-join
    if let Some(removed) = ctx.data().playing.write().await.remove(&guild_id)
        && let Err(error) = removed.track_handle.stop()
    {
        tracing::error!(?error, "failed to stop track");
        drop(removed);
    }

    tracing::info!(%channel, "leave success");

    turto_say(ctx, Leave(channel)).await?;
    Ok(())
}
