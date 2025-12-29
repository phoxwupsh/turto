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
    name = "pause",
    skip_all,
    parent = ctx.invocation_data::<Span>().await.as_deref().unwrap_or(&Span::none())
)]
pub async fn pause(ctx: Context<'_>) -> Result<(), CommandError> {
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

    let playing_map = ctx.data().playing.read().await;
    let Some(playing) = playing_map.get(&guild_id) else {
        turto_say(ctx, NotPlaying).await?;
        return Ok(());
    };

    playing.track_handle.pause()?;
    tracing::info!(paused = playing.ytdlfile.url(), "pause success");

    let meta = playing
        .ytdlfile
        .fetch_metadata(ctx.data().config.ytdlp.clone())
        .await?;

    let resp = create_playing_embed(ctx, Some(PlayState::Pause), &meta);
    ctx.send(CreateReply::default().embed(resp)).await?;

    Ok(())
}
