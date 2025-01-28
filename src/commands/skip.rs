use crate::{
    messages::TurtoMessageKind::{BotNotInVoiceChannel, DifferentVoiceChannel, NotPlaying, Skip},
    models::{
        alias::{Context, Error},
        autoleave::AutoleaveType,
    },
    utils::{
        guild::{GuildUtil, VoiceChannelState},
        play::play_next,
        turto_say,
    },
};

#[poise::command(slash_command, guild_only)]
pub async fn skip(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap();
    let bot_id = ctx.cache().current_user().id;
    let user_id = ctx.author().id;
    let vc_stat = ctx.guild().unwrap().cmp_voice_channel(&bot_id, &user_id);

    match vc_stat {
        VoiceChannelState::Different(bot, _) | VoiceChannelState::OnlyFirst(bot) => {
            turto_say(ctx, DifferentVoiceChannel { bot }).await?;
            return Ok(());
        }
        VoiceChannelState::OnlySecond(_) | VoiceChannelState::None => {
            turto_say(ctx, BotNotInVoiceChannel).await?;
            return Ok(());
        }
        VoiceChannelState::Same(_) => (),
    }

    let Some(call) = songbird::get(ctx.serenity_context())
        .await
        .unwrap()
        .get(guild_id)
    else {
        turto_say(ctx, NotPlaying).await?;
        return Ok(());
    };
    {
        let mut call = call.lock().await;
        call.stop();
    }

    let data = ctx.data();
    ctx.defer().await?;
    let meta = play_next(
        call.clone(),
        data.guilds.clone(),
        data.playing.clone(),
        guild_id,
    )
    .await
    .and_then(Result::ok);

    // Leave when there is no next track and autoleave is on or in silent mode
    let auto_leave = data.guilds.entry(guild_id).or_default().config.auto_leave;
    let should_leave =
        meta.is_none() && (auto_leave == AutoleaveType::On || auto_leave == AutoleaveType::Silent);

    let title = meta.as_ref().and_then(|meta| meta.title.as_deref());
    turto_say(ctx, Skip { title }).await?;

    if should_leave {
        let mut call = call.lock().await;
        call.leave().await?;
    }

    Ok(())
}
