use crate::{
    message::TurtoMessageKind::{BotNotInVoiceChannel, DifferentVoiceChannel, NotPlaying},
    models::{alias::Context, error::CommandError, playing::PlayState},
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
    name = "stop",
    skip_all,
    parent = ctx.invocation_data::<Span>().await.as_deref().unwrap_or(&Span::none())
)]
pub async fn stop(ctx: Context<'_>) -> Result<(), CommandError> {
    tracing::info!("invoked");

    let guild_id = ctx.guild_id().ok_or(CommandError::GuildOnly)?;
    let bot_id = ctx.cache().current_user().id;
    let user_id = ctx.author().id;
    let vc_stat = ctx
        .guild()
        .ok_or(CommandError::GuildOnly)?
        .cmp_voice_channel(&bot_id, &user_id);

    match vc_stat {
        VoiceChannelState::None | VoiceChannelState::OnlySecond(_) => {
            turto_say(ctx, BotNotInVoiceChannel).await?;
            return Ok(());
        }
        VoiceChannelState::Different(bot, _) | VoiceChannelState::OnlyFirst(bot) => {
            turto_say(ctx, DifferentVoiceChannel { bot }).await?;
            return Ok(());
        }
        VoiceChannelState::Same(_) => (),
    }

    // Take it out and release the guard in the one statement: `playing` is one map
    // shared by every guild, and the `NotPlaying` reply below is a Discord round-trip
    // that must not be made holding its *write* lock.
    let removed = ctx.data().playing.write().await.remove(&guild_id);
    let Some(playing) = removed else {
        turto_say(ctx, NotPlaying).await?;
        return Ok(());
    };

    playing.track_handle.stop()?;

    tracing::info!(stopped = playing.ytdlfile.url(), "stop success");

    let meta = playing.ytdlfile.fetch_metadata().await?;

    let resp = create_playing_embed(ctx, Some(PlayState::Stop), &meta);
    ctx.send(CreateReply::default().embed(resp)).await?;

    Ok(())
}
