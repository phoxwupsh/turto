use crate::{
    messages::{
        TurtoMessage,
        TurtoMessageKind::{DifferentVoiceChannel, UserNotInVoiceChannel},
    },
    models::alias::{Context, Error},
    utils::{guild::{GuildUtil, VoiceChannelState}, join_voice_channel},
};
use tracing::error;

#[poise::command(slash_command, guild_only)]
pub async fn join(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap();
    let bot_id = ctx.cache().current_user().id;
    let user_id = ctx.author().id;
    let vc_stat = ctx.guild().unwrap().cmp_voice_channel(&bot_id, &user_id);
    let locale = ctx.locale();

    match vc_stat {
        VoiceChannelState::Different(bot_vc, _) => {
            ctx.say(TurtoMessage {
                locale,
                kind: DifferentVoiceChannel { bot: bot_vc },
            })
            .await?;
            return Ok(());
        }
        VoiceChannelState::None | VoiceChannelState::OnlyFirst(_) => {
            ctx.say(TurtoMessage {
                locale,
                kind: UserNotInVoiceChannel,
            })
            .await?;
            return Ok(());
        }
        VoiceChannelState::OnlySecond(user_vc) => {
            if let Err (err) = join_voice_channel(ctx, locale, guild_id, user_vc).await {
                error!("Failed to join voice channel {user_vc}: {err}");
            }
        }
        VoiceChannelState::Same(_) => (),
    }
    Ok(())
}
