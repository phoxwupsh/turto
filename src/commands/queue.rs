use serenity::{
    framework::standard::{macros::command, Args, CommandResult},
    model::prelude::Message,
    prelude::Context,
};

use crate::{
    guild::playlist::Playlists,
    models::{playlist_item::PlaylistItem, playlist::Playlist, url::ParsedUrl, queue::Queueing}, messages::TurtoMessage,
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
                if let Some(playlist_url) = yt_url.playlist_url() {
                    let (Some(res), Some(info)) = Playlist::ytdl_playlist(&playlist_url) else {
                        msg.reply(ctx, TurtoMessage::InvalidUrl(Some(&parsed))).await?;
                        continue;
                    };
                    Queueing::Multiple{playlist: res, playlist_info: info}
                } else if let Some(video_url) = yt_url.video_url().build() {
                    let Ok(source) = songbird::input::ytdl(&video_url).await else {
                        msg.reply(ctx, TurtoMessage::InvalidUrl(Some(&parsed))).await?;
                        continue;
                    };
                    let metadata = source.metadata.clone();
                    Queueing::Single(PlaylistItem::from(*metadata))
                } else {
                    msg.reply(ctx, TurtoMessage::InvalidUrl(Some(&parsed))).await?;
                    continue;
                }
            },
            ParsedUrl::Other(url) => {
                let Ok(source) = songbird::input::ytdl(url).await else {
                    msg.reply(ctx, TurtoMessage::InvalidUrl(Some(&parsed))).await?;
                    continue;
                };
                let metadata = source.metadata.clone();
                Queueing::Single(PlaylistItem::from(*metadata))
            },
        };
        

        let playlists_lock = ctx
            .data
            .read()
            .await
            .get::<Playlists>()
            .expect("Expected Playlists in TypeMap.")
            .clone();
        {
            let mut playlists = playlists_lock.lock().await;
            let playlist = playlists
                .entry(msg.guild_id.expect("Expected guild_id"))
                .or_insert_with(Playlist::new);

            match queueing {
                Queueing::Single(playlist_item) => {
                    msg.reply(ctx, TurtoMessage::Queue { title: &playlist_item.title }).await?;
                    playlist.push_back(playlist_item); // Add song to playlist
                },
                Queueing::Multiple{ playlist: queueing_pl, playlist_info } => {
                    playlist.extend(queueing_pl.into_iter());
                    msg.reply(ctx, TurtoMessage::Queue { title: &playlist_info.playlist_title }).await?;
                }
            }
            
        }   
    }
    Ok(())
}
