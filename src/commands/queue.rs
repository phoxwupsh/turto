use crate::{
    messages::{
        TurtoMessage,
        TurtoMessageKind::{InvalidUrl, Queue},
    },
    models::{
        alias::{Context, Error},
        playlist_item::PlaylistItem,
        url::ParsedUrl,
        youtube_playlist::YouTubePlaylist,
    },
    utils::{get_http_client, ytdl::ytdl_playlist},
};
use songbird::input::{Compose, YoutubeDl};

enum QueueType {
    Single(PlaylistItem),
    Multiple(YouTubePlaylist),
}

#[poise::command(slash_command, guild_only)]
pub async fn queue(ctx: Context<'_>, #[rename = "url"] query: String) -> Result<(), Error> {
    let locale = ctx.locale();
    let Ok(parsed) = query.parse::<ParsedUrl>() else {
        ctx.say(TurtoMessage {
            locale,
            kind: InvalidUrl(None),
        })
        .await?;
        return Ok(());
    };

    let queue_item = match &parsed {
        ParsedUrl::Youtube(yt_url) => {
            if yt_url.is_playlist() {
                let Some(yt_playlist) = ytdl_playlist(&yt_url.to_string()) else {
                    ctx.say(TurtoMessage {
                        locale,
                        kind: InvalidUrl(Some(&parsed)),
                    })
                    .await?;
                    return Ok(());
                };
                QueueType::Multiple(yt_playlist)
            } else {
                let mut source = YoutubeDl::new(get_http_client(), yt_url.to_string());
                match source.aux_metadata().await {
                    Ok(metadata) => QueueType::Single(PlaylistItem::from(metadata)),
                    Err(_err) => {
                        ctx.say(TurtoMessage {
                            locale,
                            kind: InvalidUrl(Some(&parsed)),
                        })
                        .await?;
                        return Ok(());
                    }
                }
            }
        }
        ParsedUrl::Other(url) => {
            let mut source = YoutubeDl::new(get_http_client(), url.to_string());
            match source.aux_metadata().await {
                Ok(metadata) => QueueType::Single(PlaylistItem::from(metadata)),
                Err(_err) => {
                    ctx.say(TurtoMessage {
                        locale,
                        kind: InvalidUrl(Some(&parsed)),
                    })
                    .await?;
                    return Ok(());
                }
            }
        }
    };

    let guild_id = ctx.guild_id().unwrap();
    let mut guild_data = ctx.data().guilds.entry(guild_id).or_default();

    match queue_item {
        QueueType::Single(playlist_item) => {
            let title = playlist_item.title.clone();
            let response = TurtoMessage {
                locale,
                kind: Queue { title: &title },
            };
            guild_data.playlist.push_back(playlist_item);
            drop(guild_data);

            ctx.say(response).await?;
        }
        QueueType::Multiple(mut yt_playlist) => {
            let title = yt_playlist.title.take().unwrap_or_default();
            guild_data.playlist.extend(yt_playlist.into_iter());
            drop(guild_data);

            ctx.say(TurtoMessage {
                locale,
                kind: Queue { title: &title },
            })
            .await?;
        }
    }
    Ok(())
}
