use serenity::{
    framework::standard::{macros::command, Args, CommandResult},
    model::prelude::Message,
    prelude::Context,
};

use crate::{
    guild::playlist::Playlists,
    models::{playlist_item::PlaylistItem, playlist::Playlist, url_type::UrlType},
};

enum Queueing {
    Single(PlaylistItem),
    Multiple(Playlist)
}

#[command]
#[bucket = "music"]
async fn queue(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    while !args.is_empty() {
        let arg = args.single::<String>().unwrap();
        let Ok(parsed) = arg.parse::<UrlType>() else {
            msg.reply(ctx, format!("`{}` is not a valid url", arg)).await?;
            continue;
        };

        let queueing = match &parsed {
            UrlType::Youtube { id: _, time: _ } => {
                let url = parsed.to_string();
                let Ok(source) = songbird::input::ytdl(&url).await else {
                    msg.reply(ctx, format!("Cannot find `{}`", &url)).await?;
                    continue;
                };
                let metadata = source.metadata.clone();
                Queueing::Single(PlaylistItem::from(*metadata))
            },
            UrlType::YoutubePlaylist { playlist_id: _ } => {
                let url = parsed.to_string();
                let Some(res) = Playlist::ytdl_playlist(&parsed.to_string()) else {
                    msg.reply(ctx, format!("Cannot find playlist `{}`", url)).await?;
                    continue;
                };
                Queueing::Multiple(res)
            },
            UrlType::Other(url) => {
                let Ok(source) = songbird::input::ytdl(parsed.to_string()).await else {
                    msg.reply(ctx, format!("Cannot find `{}`", url)).await?;
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
                Queueing::Single(queueing_playlist_item) => {
                    msg.reply(ctx, format!("✅ {}", queueing_playlist_item.title)).await?;
                    playlist.push_back(queueing_playlist_item); // Add song to playlist
                },
                Queueing::Multiple(queueing_playlist) => {
                    let response = queueing_playlist.iter()
                    .map(|p| format!("✅ {}", p.title))
                    .collect::<Vec<_>>()
                    .join("\n");
                    playlist.extend(queueing_playlist.into_iter());
                    msg.reply(ctx, response).await?;
                }
            }
            
        }   
    }
    Ok(())
}
