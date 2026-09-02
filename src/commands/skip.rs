use crate::{
    message::TurtoMessageKind::{BotNotInVoiceChannel, DifferentVoiceChannel, NotPlaying, Skip},
    models::{alias::Context, error::CommandError, playing::PlayState},
    player::{self, PlayContext},
    utils::{
        create_playing_embed,
        guild::{GuildUtil, VoiceChannelState},
        turto_say,
    },
};
use poise::CreateReply;
use tracing::{Span, instrument};

#[poise::command(slash_command, guild_only)]
#[instrument(
    name = "skip",
    skip_all,
    parent = ctx.invocation_data::<Span>().await.as_deref().unwrap_or(&Span::none())
)]
pub async fn skip(ctx: Context<'_>) -> Result<(), CommandError> {
    tracing::info!("invoked");

    let guild_id = ctx.guild_id().ok_or(CommandError::GuildOnly)?;
    let bot_id = ctx.cache().current_user().id;
    let user_id = ctx.author().id;
    let vc_stat = ctx
        .guild()
        .ok_or(CommandError::GuildOnly)?
        .cmp_voice_channel(&bot_id, &user_id);

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

    tracing::info!("skip success");

    ctx.defer().await?;

    let mut guild_data = ctx.data().guilds.entry(guild_id).or_default();
    let next = guild_data.playlist.pop_front();
    drop(guild_data);

    if let Some(next) = next {
        tracing::info!(next = next.url(), "play next");

        let metadata = player::play_track(PlayContext::try_from(ctx)?, call, next).await?;

        let resp = create_playing_embed(ctx, Some(PlayState::Skip), &metadata);
        ctx.send(CreateReply::default().embed(resp)).await?;
    } else {
        let auto_leave = ctx
            .data()
            .guilds
            .entry(guild_id)
            .or_default()
            .config
            .auto_leave;
        if auto_leave.leaves_on_empty_queue() {
            let mut call = call.lock().await;
            player::leave(&mut call, guild_id).await;
        }
        turto_say(ctx, Skip { title: None }).await?;
    }

    Ok(())
}
