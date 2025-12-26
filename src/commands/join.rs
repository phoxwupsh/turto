use tracing::{Span, instrument};
use crate::{
    message::TurtoMessageKind::{DifferentVoiceChannel, UserNotInVoiceChannel},
    models::{alias::Context, error::CommandError},
    utils::{
        guild::{GuildUtil, VoiceChannelState},
        join_voice_channel, turto_say,
    },
};

#[poise::command(slash_command, guild_only)]
#[instrument(
    name = "join",
    skip_all,
    parent = ctx.invocation_data::<Span>().await.as_deref().unwrap_or(&Span::none())
)]
pub async fn join(ctx: Context<'_>) -> Result<(), CommandError> {
    tracing::info!("invoked");

    let guild_id = ctx.guild_id().unwrap();
    let bot_id = ctx.cache().current_user().id;
    let user_id = ctx.author().id;
    let vc_stat = ctx.guild().unwrap().cmp_voice_channel(&bot_id, &user_id);

    match vc_stat {
        VoiceChannelState::Different(bot, _) => {
            turto_say(ctx, DifferentVoiceChannel { bot }).await?;
            return Ok(());
        }
        VoiceChannelState::None | VoiceChannelState::OnlyFirst(_) => {
            turto_say(ctx, UserNotInVoiceChannel).await?;
            return Ok(());
        }
        VoiceChannelState::OnlySecond(user_vc) => {
            join_voice_channel(ctx, guild_id, user_vc).await?;
        }
        VoiceChannelState::Same(_) => (),
    }
    Ok(())
}
