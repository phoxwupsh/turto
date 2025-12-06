use crate::{
    message::TurtoMessageKind::{DifferentVoiceChannel, UserNotInVoiceChannel},
    models::alias::{Context, Error},
    utils::{
        guild::{GuildUtil, VoiceChannelState},
        join_voice_channel, turto_say,
    },
};
use tracing::error;

#[poise::command(slash_command, guild_only)]
pub async fn join(ctx: Context<'_>) -> Result<(), Error> {
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
            if let Err(err) = join_voice_channel(ctx, guild_id, user_vc).await {
                error!("Failed to join voice channel {user_vc}: {err}");
            }
        }
        VoiceChannelState::Same(_) => (),
    }
    Ok(())
}
