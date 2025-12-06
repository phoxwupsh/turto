use crate::{
    message::TurtoMessageKind::{BotNotInVoiceChannel, DifferentVoiceChannel, NotPlaying, Stop},
    models::alias::{Context, Error},
    utils::{
        guild::{GuildUtil, VoiceChannelState},
        turto_say,
    },
};
use tracing::error;

#[poise::command(slash_command, guild_only)]
pub async fn stop(ctx: Context<'_>) -> Result<(), Error> {
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

    if let Err(why) = playing.track_handle.stop() {
        error!(error = ?why, ?playing, "failed to stop track");
    }

    let meta = playing
        .ytdlfile
        .fetch_metadata(ctx.data().config.ytdlp.clone())
        .await?;
    let title = meta.title.as_deref().unwrap();
    turto_say(ctx, Stop { title }).await?;
    Ok(())
}
