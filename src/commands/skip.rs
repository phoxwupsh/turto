use crate::{
    message::TurtoMessageKind::{BotNotInVoiceChannel, DifferentVoiceChannel, NotPlaying, Skip},
    models::{
        alias::{Context, Error},
        autoleave::AutoleaveType,
    },
    utils::{
        guild::{GuildUtil, VoiceChannelState},
        play::{PlayContext, play_ytdlfile_meta},
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

    ctx.defer().await?;

    let mut guild_data = ctx.data().guilds.entry(guild_id).or_default();
    let next = guild_data
        .playlist
        .pop_front_prefetch(ctx.data().config.ytdlp.clone());
    drop(guild_data);

    if let Some(next) = next {
        let meta_fut = play_ytdlfile_meta(PlayContext::from_ctx(ctx).unwrap(), call, next).await?;
        let metadata = meta_fut.await?;

        turto_say(
            ctx,
            Skip {
                title: metadata.title.as_deref(),
            },
        )
        .await?;
    } else {
        let auto_leave = ctx
            .data()
            .guilds
            .entry(guild_id)
            .or_default()
            .config
            .auto_leave;
        let should_leave = auto_leave == AutoleaveType::On || auto_leave == AutoleaveType::Silent;
        if should_leave {
            let mut call = call.lock().await;
            call.leave().await?;
        }
        turto_say(ctx, Skip { title: None }).await?;
    }

    Ok(())
}
