use crate::{
    message::{
        TurtoMessage,
        TurtoMessageKind::{self, Join},
    },
    models::{alias::Context, error::CommandError, playing::PlayState},
    ytdl::YouTubeDlMetadata,
};
use poise::ReplyHandle;
use reqwest::Client;
use serenity::all::{ChannelId, CreateEmbed, CreateEmbedAuthor, GuildId};
use songbird::Call;
use std::{
    future::Future,
    sync::{Arc, OnceLock},
    time::Duration,
};
use tokio::sync::Mutex;

pub mod guild;
pub mod json;
pub mod misc;
pub mod queue;

static HTTP_CLIENT: OnceLock<Client> = OnceLock::new();

/// Ceiling on establishing a connection to a media host.
const HTTP_CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

/// Ceiling on the gap between two reads of one response -- the headers, and then each
/// body chunk. It **resets after every successful read**, so a slow but live download
/// runs as long as it likes; only a connection that has gone silent trips it.
///
/// The byte paths need this rather than a whole-request timeout: a range request is up
/// to 10 MB and a playlist segment arrives in pieces, so any total bound would either
/// be uselessly large or cut a healthy fetch short. Without it a black-holed
/// connection parks [`crate::ytdl`]'s producer forever -- the track never plays, never
/// fails, and cannot even be skipped, since the cancel signal is only observed between
/// chunks.
const HTTP_READ_TIMEOUT: Duration = Duration::from_secs(30);

/// The shared client for every outbound fetch except the sidecar's, which builds its
/// own (a `/download` streams for minutes at yt-dlp's pace and is bounded by the child
/// process instead).
pub fn get_http_client() -> Client {
    HTTP_CLIENT
        .get_or_init(|| {
            Client::builder()
                .connect_timeout(HTTP_CONNECT_TIMEOUT)
                .read_timeout(HTTP_READ_TIMEOUT)
                .build()
                .expect("the default http client must build")
        })
        .clone()
}

pub async fn join_voice_channel(
    ctx: Context<'_>,
    guild_id: GuildId,
    channel_id: ChannelId,
) -> Result<Arc<Mutex<Call>>, CommandError> {
    // there is some time limit of a command to be response,
    // joining a voice can take time and cause timeout
    // so use defer to prevent timeout
    ctx.defer().await?;
    let call = songbird::get(ctx.serenity_context())
        .await
        .unwrap()
        .join(guild_id, channel_id)
        .await?;

    tracing::info!(channel = %channel_id, "join voice channel success");

    turto_say(ctx, Join(channel_id)).await?;
    Ok(call)
}

#[inline]
pub fn turto_say<'a>(
    ctx: Context<'a>,
    msg: TurtoMessageKind<'a>,
) -> impl Future<Output = Result<ReplyHandle<'a>, serenity::Error>> {
    ctx.say(TurtoMessage::new(ctx, msg))
}

pub fn create_playing_embed(
    ctx: Context<'_>,
    play_mode: Option<PlayState>,
    ytdl_data: &YouTubeDlMetadata,
) -> CreateEmbed {
    let mut embed = CreateEmbed::new();
    if let Some(thumbnail) = ytdl_data.thumbnail.as_deref() {
        embed = embed.image(thumbnail);
    }
    if let Some(webpage_url) = ytdl_data.webpage_url.as_deref() {
        embed = embed.url(webpage_url);
    }
    if let Some(title) = ytdl_data.title.as_deref() {
        match play_mode {
            Some(play_mode) => {
                let kind = match play_mode {
                    PlayState::Play => TurtoMessageKind::Play { title },
                    PlayState::Pause => TurtoMessageKind::Pause { title },
                    PlayState::Stop => TurtoMessageKind::Stop { title },
                    PlayState::Skip => TurtoMessageKind::Skip { title: Some(title) },
                };
                embed = embed.title(TurtoMessage::new(ctx, kind).to_string());
            }
            None => embed = embed.title(title),
        }
    }
    if let Some(timestamp) = ytdl_data.timestamp {
        embed = embed.timestamp(
            serenity::model::Timestamp::from_unix_timestamp(timestamp).unwrap_or_default(),
        );
    }
    if let Some(author_name) = ytdl_data
        .channel
        .as_deref()
        .or(ytdl_data.uploader.as_deref())
    {
        let mut author = CreateEmbedAuthor::new(author_name);
        if let Some(author_url) = ytdl_data
            .channel_url
            .as_deref()
            .or(ytdl_data.uploader_url.as_deref())
        {
            author = author.url(author_url);
        }
        embed = embed.author(author);
    }
    embed
}
