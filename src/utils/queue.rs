use super::turto_say;
use crate::{
    message::TurtoMessageKind::{InvalidUrl, Queue},
    models::{
        alias::Context, error::CommandError, queue_item::{QueueItem, QueueItemKind}
    },
};
use std::mem::replace;
use url::Url;

pub enum QueueType {
    Front,
    Back,
}

pub async fn enqueue(ctx: Context<'_>, query: String, queue_type: QueueType) -> Result<(), CommandError> {
    let Ok(parsed) = Url::parse(&query) else {
        turto_say(ctx, InvalidUrl(None)).await?;
        return Ok(());
    };
    ctx.defer().await?;

    let queue_item = QueueItem::new(parsed);

    let Ok(queue_item_kind) = queue_item.query().await else {
        turto_say(ctx, InvalidUrl(Some(&query))).await?;
        return Ok(());
    };

    let title = match queue_item_kind {
        QueueItemKind::Single(playlist_item) => {
            let ytdlp_config = ctx.data().config.ytdlp.clone();
            let title = playlist_item
                .fetch_metadata(ytdlp_config.clone())
                .await?
                .title
                .clone()
                .unwrap_or_default();

            let guild_id = ctx.guild_id().unwrap();
            let mut guild_data = ctx.data().guilds.entry(guild_id).or_default();

            match queue_type {
                QueueType::Front => guild_data
                    .playlist
                    .push_front_prefetch(playlist_item, ytdlp_config),
                QueueType::Back => guild_data
                    .playlist
                    .push_back_prefetch(playlist_item, ytdlp_config),
            }
            drop(guild_data);

            tracing::info!("enqueue single success");

            title
        }
        QueueItemKind::Playlist(mut yt_playlist) => {
            let title = yt_playlist.title.take().unwrap_or_default();

            let guild_id = ctx.guild_id().unwrap();
            let mut guild_data = ctx.data().guilds.entry(guild_id).or_default();
            let ytdlp_config = ctx.data().config.ytdlp.clone();

            match queue_type {
                QueueType::Front => {
                    let new_playlist = yt_playlist.to_playlist();
                    let tail = replace(&mut guild_data.playlist, new_playlist);
                    guild_data.playlist.extend_prefetch(tail, ytdlp_config);
                }
                QueueType::Back => guild_data
                    .playlist
                    .extend_prefetch(yt_playlist, ytdlp_config),
            }
            drop(guild_data);

            tracing::info!("enqueue playlist success");

            title
        }
    };

    turto_say(ctx, Queue { title: &title }).await?;
    Ok(())
}
