use crate::{
    message::TurtoMessageKind::{DifferentVoiceChannel, InvalidUrl, Play, UserNotInVoiceChannel},
    models::alias::{Context, Error},
    utils::{
        guild::{GuildUtil, VoiceChannelState},
        join_voice_channel,
        play::{PlayContext, play_ytdlfile_meta},
        turto_say,
    },
    ytdl::{YouTubeDl, YouTubeDlMetadata},
};
use poise::CreateReply;
use serenity::all::{CreateEmbed, CreateEmbedAuthor};
use songbird::tracks::PlayMode;
use tracing::error;
use url::Url;

#[poise::command(slash_command, guild_only)]
pub async fn play(ctx: Context<'_>, #[rename = "url"] query: Option<String>) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap();
    let bot_id = ctx.cache().current_user().id;
    let user_id = ctx.author().id;
    let vc_stat = ctx.guild().unwrap().cmp_voice_channel(&bot_id, &user_id);

    let call = match vc_stat {
        VoiceChannelState::None | VoiceChannelState::OnlyFirst(_) => {
            turto_say(ctx, UserNotInVoiceChannel).await?;
            return Ok(());
        }
        VoiceChannelState::Different(bot, _) => {
            turto_say(ctx, DifferentVoiceChannel { bot }).await?;
            return Ok(());
        }
        VoiceChannelState::OnlySecond(user_vc) => {
            match join_voice_channel(ctx, guild_id, user_vc).await {
                Ok(call) => call,
                Err(err) => {
                    error!(error = ?err, channel = ?user_vc, "failed to join voice channel");
                    return Ok(());
                }
            }
        }
        VoiceChannelState::Same(_) => songbird::get(ctx.serenity_context())
            .await
            .unwrap()
            .get(guild_id)
            .unwrap(),
    };

    let data = ctx.data();

    if let Some(query) = query {
        // If a valid url is provided then play the url
        if Url::parse(&query).is_err() {
            turto_say(ctx, InvalidUrl(None)).await?;
        }

        ctx.defer().await?;
        let ytdlfile = YouTubeDl::new(query);
        let meta_fut =
            play_ytdlfile_meta(PlayContext::from_ctx(ctx).unwrap(), call, ytdlfile).await?;
        let meta = meta_fut.await?;
        let embed = create_resp(&meta);
        ctx.send(CreateReply::default().embed(embed)).await?;
        return Ok(());
    } else {
        // If no url provided, check if there is a paused track or there is any song in the playlist
        let playing_map = data.playing.read().await;

        if let Some(playing) = playing_map.get(&guild_id)
            && let Ok(current_track_state) = playing.track_handle.get_info().await
            && current_track_state.playing == PlayMode::Pause
        {
            // If there is a paused song then play it
            if let Err(why) = playing.track_handle.play() {
                error!(error = ?why, ?playing, "failed to play track ");
            } else {
                let meta = playing
                    .ytdlfile
                    .fetch_metadata(ctx.data().config.ytdlp.clone())
                    .await?;
                let title = meta.title.as_deref().unwrap_or_default();
                turto_say(ctx, Play { title }).await?;
            }
            return Ok(());
        }
        drop(playing_map);

        ctx.defer().await?;

        let mut guild_data = data.guilds.entry(guild_id).or_default();
        let next = guild_data.playlist.pop_front();
        drop(guild_data);

        if let Some(next) = next {
            let meta_fut =
                play_ytdlfile_meta(PlayContext::from_ctx(ctx).unwrap(), call, next).await?;
            let metadata = meta_fut.await?;

            turto_say(
                ctx,
                Play {
                    title: metadata.title.as_deref().unwrap_or_default(),
                },
            )
            .await?;
        } else {
            // if the playlist is empty
            turto_say(ctx, InvalidUrl(None)).await?;
        }
    }

    Ok(())
}

fn create_resp(ytdl_data: &YouTubeDlMetadata) -> CreateEmbed {
    let mut embed = CreateEmbed::new();
    if let Some(thumbnail) = ytdl_data.thumbnail.as_deref() {
        embed = embed.image(thumbnail);
    }
    if let Some(webpage_url) = ytdl_data.webpage_url.as_deref() {
        embed = embed.url(webpage_url);
    }
    if let Some(title) = ytdl_data.title.as_deref() {
        embed = embed.title(title);
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
