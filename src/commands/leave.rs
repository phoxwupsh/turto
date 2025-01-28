use crate::{
    messages::TurtoMessageKind::{BotNotInVoiceChannel, DifferentVoiceChannel, Leave},
    models::alias::{Context, Error},
    utils::{
        guild::{GuildUtil, VoiceChannelState},
        turto_say,
    },
};

#[poise::command(slash_command, guild_only)]
pub async fn leave(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap();
    let bot_id = ctx.cache().current_user().id;
    let user_id = ctx.author().id;
    let vc_stat = ctx.guild().unwrap().cmp_voice_channel(&bot_id, &user_id);

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
    ctx.data().playing.write().await.remove(&guild_id);

    turto_say(ctx, Leave(channel)).await?;
    Ok(())
}
