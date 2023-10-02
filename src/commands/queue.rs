use crate::{
    messages::TurtoMessage,
    models::{playlist_item::PlaylistItem, url::ParsedUrl, youtube_playlist::YouTubePlaylist},
    typemap::playlist::Playlists,
    utils::ytdl::ytdl_playlist,
};
use serenity::{
    framework::standard::{macros::command, Args, CommandResult},
    model::prelude::Message,
    prelude::Context,
};
use songbird::input::ytdl;

enum QueueType {
    Single(PlaylistItem),
    Multiple(YouTubePlaylist),
}

#[command]
#[bucket = "turto"]
async fn queue(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    while !args.is_empty() {
        let arg = args.single::<String>().unwrap();
        let Ok(parsed) = arg.parse::<ParsedUrl>() else {
            msg.reply(ctx, TurtoMessage::InvalidUrl(None)).await?;
            continue;
        };

        let queue_item = match &parsed {
            ParsedUrl::Youtube(yt_url) => {
                if yt_url.is_playlist() {
                    let Some(yt_playlist) = ytdl_playlist(&yt_url.to_string()) else {
                        msg.reply(ctx, TurtoMessage::InvalidUrl(Some(&parsed))).await?;
                        continue;
                    };
                    QueueType::Multiple(yt_playlist)
                } else {
                    let Ok(source) = ytdl(&yt_url.to_string()).await else {
                        msg.reply(ctx, TurtoMessage::InvalidUrl(Some(&parsed))).await?;
                        continue;
                    };
                    let metadata = source.metadata.clone();
                    QueueType::Single(PlaylistItem::from(*metadata))
                }
            }
            ParsedUrl::Other(url) => {
                let Ok(source) = ytdl(url).await else {
                    msg.reply(ctx, TurtoMessage::InvalidUrl(Some(&parsed))).await?;
                    continue;
                };
                let metadata = source.metadata.clone();
                QueueType::Single(PlaylistItem::from(*metadata))
            }
        };

        let playlists_lock = ctx.data.read().await.get::<Playlists>().unwrap().clone();
        {
            let mut playlists = playlists_lock.lock().await;
            let playlist = playlists.entry(msg.guild_id.unwrap()).or_default();

            match queue_item {
                QueueType::Single(playlist_item) => {
                    msg.reply(
                        ctx,
                        TurtoMessage::Queue {
                            title: &playlist_item.title,
                        },
                    )
                    .await?;
                    playlist.push_back(playlist_item); // Add song to playlist
                }
                QueueType::Multiple(mut yt_playlist) => {
                    let title = yt_playlist.title.take().unwrap_or_default();
                    playlist.extend(yt_playlist.into_iter());
                    msg.reply(ctx, TurtoMessage::Queue { title: &title })
                        .await?;
                }
            }
        }
    }
    Ok(())
}
