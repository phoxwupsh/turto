use tracing::{Span, instrument};
use crate::{
    message::TurtoMessageKind::{BotNotInVoiceChannel, DifferentVoiceChannel, NotPlaying, Stop},
    models::{alias::Context, error::CommandError},
    utils::{
        guild::{GuildUtil, VoiceChannelState},
        turto_say,
    },
};

#[poise::command(slash_command, guild_only)]
#[instrument(
    name = "stop",
    skip_all,
    parent = ctx.invocation_data::<Span>().await.as_deref().unwrap_or(&Span::none())
)]
pub async fn stop(ctx: Context<'_>) -> Result<(), CommandError> {
    tracing::info!("invoked");
    
    let guild_id = ctx.guild_id().unwrap();
    let bot_id = ctx.cache().current_user().id;
    let user_id = ctx.author().id;
    let vc_stat = ctx.guild().unwrap().cmp_voice_channel(&bot_id, &user_id);

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

    let mut playing_map = ctx.data().playing.write().await;
    let Some(playing) = playing_map.remove(&guild_id) else {
        turto_say(ctx, NotPlaying).await?;
        return Ok(());
    };
    drop(playing_map);

    playing.track_handle.stop()?;

    tracing::info!(stopped = playing.ytdlfile.url(), "stop success");

    let meta = playing
        .ytdlfile
        .fetch_metadata(ctx.data().config.ytdlp.clone())
        .await?;
    let title = meta.title.as_deref().unwrap();
    turto_say(ctx, Stop { title }).await?;
    Ok(())
}
