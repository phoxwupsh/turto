use crate::{
    message::{
        TurtoMessage,
        TurtoMessageKind::{NotPlaying, Pause, Play},
    },
    models::alias::{Context, Error},
    utils::turto_say,
};
use poise::CreateReply;
use serenity::builder::CreateEmbed;
use songbird::tracks::PlayMode;
use tracing::error;

#[poise::command(slash_command, guild_only)]
pub async fn playwhat(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap();

    let playing_map = ctx.data().playing.read().await;
    let Some(playing) = playing_map.get(&guild_id) else {
        turto_say(ctx, NotPlaying).await?;
        return Ok(());
    };

    let meta = playing
        .ytdlfile
        .fetch_metadata(ctx.data().config.ytdlp.clone())
        .await?;
    let title = meta.title.as_deref().unwrap_or_default();
    let embed_title = match playing.track_handle.get_info().await {
        Ok(track_state) => match track_state.playing {
            PlayMode::Play => TurtoMessage::new(ctx, Play { title }),
            PlayMode::Pause => TurtoMessage::new(ctx, Pause { title }),
            _ => {
                turto_say(ctx, NotPlaying).await?;
                return Ok(());
            }
        },
        Err(err) => {
            error!(error = ?err, "failed to get track");
            turto_say(ctx, NotPlaying).await?;
            return Ok(());
        }
    };

    let mut embed = CreateEmbed::new().title(embed_title);
    if let Some(url) = meta.webpage_url.as_deref() {
        embed = embed.url(url);
    }
    if let Some(channel) = meta.artist.as_deref().or(meta.channel.as_deref()) {
        embed = embed.description(channel);
    }
    if let Some(thumbnail) = meta.thumbnail.as_deref() {
        embed = embed.image(thumbnail);
    }
    drop(playing_map);

    let response = CreateReply::default().embed(embed);
    ctx.send(response).await?;

    Ok(())
}
