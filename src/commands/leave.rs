use crate::{
    messages::{
        TurtoMessage,
        TurtoMessageKind::{BotNotInVoiceChannel, DifferentVoiceChannel, Leave},
    },
    models::alias::{Context, Error},
    utils::guild::{GuildUtil, VoiceChannelState},
};

#[poise::command(slash_command, guild_only)]
pub async fn leave(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap();
    let bot_id = ctx.cache().current_user().id;
    let user_id = ctx.author().id;
    let vc_stat = ctx.guild().unwrap().cmp_voice_channel(&bot_id, &user_id);
    let locale = ctx.locale();

    let channel = match vc_stat {
        VoiceChannelState::None | VoiceChannelState::OnlySecond(_) => {
            ctx.say(TurtoMessage{locale,kind:BotNotInVoiceChannel}).await?;
            return Ok(());
        }
        VoiceChannelState::Different(bot_vc, _) | VoiceChannelState::OnlyFirst(bot_vc) => {
            ctx.say(TurtoMessage{locale,kind:DifferentVoiceChannel { bot: bot_vc }})
                .await?;
            return Ok(());
        }
        VoiceChannelState::Same(vc) => vc,
    };

    let manager = songbird::get(ctx.serenity_context()).await.unwrap().clone();
    manager.remove(guild_id).await?;
    ctx.data().playing.write().await.remove(&guild_id);

    ctx.say(TurtoMessage{locale,kind:Leave(channel)}).await?;
    Ok(())
}
