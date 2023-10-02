use serenity::{
    framework::standard::{macros::command, Args, CommandResult},
    model::prelude::Message,
    prelude::Context,
};

use songbird::input::ytdl;

use crate::{
    messages::TurtoMessage,
    models::{playlist::Playlist, playlist_item::PlaylistItem, queue::Queueing, url::ParsedUrl},
    typemap::playlist::Playlists,
};

#[command]
#[bucket = "music"]
async fn queue(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    while !args.is_empty() {
        let arg = args.single::<String>().unwrap();
        let Ok(parsed) = arg.parse::<ParsedUrl>() else {
            msg.reply(ctx, TurtoMessage::InvalidUrl(None)).await?;
            continue;
        };

        let queueing = match &parsed {
            ParsedUrl::Youtube(yt_url) => {
                if yt_url.is_playlist() {
                    let (Some(res), Some(info)) = Playlist::ytdl_playlist(&yt_url.to_string()) else {
                        msg.reply(ctx, TurtoMessage::InvalidUrl(Some(&parsed))).await?;
                        continue;
                    };
                    Queueing::Multiple {
                        playlist: res,
                        playlist_info: info,
                    }
                } else {
                    let Ok(source) = ytdl(&yt_url.to_string()).await else {
                        msg.reply(ctx, TurtoMessage::InvalidUrl(Some(&parsed))).await?;
                        continue;
                    };
                    let metadata = source.metadata.clone();
                    Queueing::Single(PlaylistItem::from(*metadata))
                }
            }
            ParsedUrl::Other(url) => {
                let Ok(source) = ytdl(url).await else {
                    msg.reply(ctx, TurtoMessage::InvalidUrl(Some(&parsed))).await?;
                    continue;
                };
                let metadata = source.metadata.clone();
                Queueing::Single(PlaylistItem::from(*metadata))
            }
        };

        let playlists_lock = ctx.data.read().await.get::<Playlists>().unwrap().clone();
        {
            let mut playlists = playlists_lock.lock().await;
            let playlist = playlists.entry(msg.guild_id.unwrap()).or_default();

            match queueing {
                Queueing::Single(playlist_item) => {
                    msg.reply(
                        ctx,
                        TurtoMessage::Queue {
                            title: &playlist_item.title,
                        },
                    )
                    .await?;
                    playlist.push_back(playlist_item); // Add song to playlist
                }
                Queueing::Multiple {
                    playlist: queueing_pl,
                    playlist_info,
                } => {
                    playlist.extend(queueing_pl.into_iter());
                    msg.reply(
                        ctx,
                        TurtoMessage::Queue {
                            title: &playlist_info.playlist_title,
                        },
                    )
                    .await?;
                }
            }
        }
    }
    Ok(())
}
