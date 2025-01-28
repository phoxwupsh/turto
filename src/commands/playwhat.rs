use crate::{
    messages::{
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
    let locale = ctx.locale();

    let playing_map = ctx.data().playing.read().await;
    let Some(playing) = playing_map.get(&guild_id) else {
        turto_say(ctx, NotPlaying).await?;
        return Ok(());
    };

    let title = playing.metadata.title.as_deref().unwrap_or_default();
    let embed_title = match playing.track_handle.get_info().await {
        Ok(track_state) => match track_state.playing {
            PlayMode::Play => TurtoMessage {
                locale,
                kind: Play { title },
            },
            PlayMode::Pause => TurtoMessage {
                locale,
                kind: Pause { title },
            },
            _ => {
                turto_say(ctx, NotPlaying).await?;
                return Ok(());
            }
        },
        Err(err) => {
            error!("Error getting track: {err}");
            turto_say(ctx, NotPlaying).await?;
            return Ok(());
        }
    };

    let mut embed = CreateEmbed::new().title(embed_title);
    if let Some(url) = &playing.metadata.source_url {
        embed = embed.url(url);
    }
    if let Some(channel) = playing
        .metadata
        .artist
        .as_deref()
        .or(playing.metadata.channel.as_deref())
    {
        embed = embed.description(channel);
    }
    if let Some(thumbnail) = playing.metadata.thumbnail.as_deref() {
        embed = embed.image(thumbnail);
    }
    drop(playing_map);

    let response = CreateReply::default().embed(embed);
    ctx.send(response).await?;

    Ok(())
}
