use crate::{
    message::TurtoMessageKind::{DifferentVoiceChannel, InvalidUrl, UserNotInVoiceChannel},
    models::{alias::Context, error::CommandError, playing::PlayState},
    player::{self, PlayContext},
    turto_command,
    utils::{
        create_playing_embed,
        guild::{GuildUtil, VoiceChannelState},
        join_voice_channel, turto_say,
    },
    ytdl::YouTubeDl,
};
use poise::CreateReply;
use songbird::tracks::PlayMode;
use tracing::{Span, instrument};
use url::Url;

#[turto_command(
    short = "Start playback.",
    long = "Start playback. If turto is not in another voice channel, it will join your current one. Depending on the situation, there are several possibilities:\n\
            1. If `url` is provided, it will interrupt the currently playing item, and start playing it. Supported sources include YouTube, Bilibili videos and Soundcloud music (you can try other platform, as long as it's supported by yt-dlp).\n\
            2. If no `url` is provided and there is a paused item, it will resume playing that item.\n\
            3. If no `url` is provided and there is no paused item, it will start playing the playlist from the beginning."
)]
#[poise::command(slash_command, guild_only)]
#[instrument(
    name = "play",
    skip_all,
    parent = ctx.invocation_data::<Span>().await.as_deref().unwrap_or(&Span::none())
    fields(query)
)]
pub async fn play(
    ctx: Context<'_>,
    #[description = "Optional, the link to what you want to play"]
    #[rename = "url"]
    query: Option<String>,
) -> Result<(), CommandError> {
    tracing::info!("invoke");

    let guild_id = ctx.guild_id().ok_or(CommandError::GuildOnly)?;
    let bot_id = ctx.cache().current_user().id;
    let user_id = ctx.author().id;
    let vc_stat = ctx
        .guild()
        .ok_or(CommandError::GuildOnly)?
        .cmp_voice_channel(&bot_id, &user_id);

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
            join_voice_channel(ctx, guild_id, user_vc).await?
        }
        VoiceChannelState::Same(_) => songbird::get(ctx.serenity_context())
            .await
            .unwrap()
            .get(guild_id)
            .ok_or(CommandError::InvalidOperation {
                cause: "no valid connection found, you may need to disconnect the bot manually and retry",
            })?,
    };

    let data = ctx.data();

    if let Some(query) = query {
        // If a valid url is provided then play the url
        if Url::parse(&query).is_err() {
            turto_say(ctx, InvalidUrl(None)).await?;
            return Ok(());
        }

        ctx.defer().await?;
        let ytdlfile = YouTubeDl::new(query);
        let meta = player::play_track(PlayContext::try_from(ctx)?, call, ytdlfile).await?;

        tracing::info!("play success");

        let embed = create_playing_embed(ctx, Some(PlayState::Play), &meta);
        ctx.send(CreateReply::default().embed(embed)).await?;
        return Ok(());
    } else {
        // If no url provided, check if there is a paused track or there is any song in
        // the playlist. Clone out and release the guard before awaiting: `playing` is
        // one map shared by every guild, and holding it read-locked across a Discord
        // round-trip blocks the write `player::spawn_playback` needs to start a track
        // anywhere.
        let current = data
            .playing
            .read()
            .await
            .get(&guild_id)
            .map(|playing| (playing.ytdlfile.clone(), playing.track_handle.clone()));

        if let Some((ytdlfile, track_handle)) = current
            && let Ok(current_track_state) = track_handle.get_info().await
            && current_track_state.playing == PlayMode::Pause
        {
            // If there is a paused song then play it
            track_handle.play()?;

            let metadata = ytdlfile.fetch_metadata().await?;

            tracing::info!(url = ytdlfile.url(), "resume");

            let resp = create_playing_embed(ctx, Some(PlayState::Play), &metadata);
            ctx.send(CreateReply::default().embed(resp)).await?;

            return Ok(());
        }

        ctx.defer().await?;

        let mut guild_data = data.guilds.entry(guild_id).or_default();
        let next = guild_data.playlist.pop_front();
        drop(guild_data);

        if let Some(next) = next {
            tracing::info!(url = next.url(), "play first item in playlist");

            let metadata = player::play_track(PlayContext::try_from(ctx)?, call, next).await?;

            let resp = create_playing_embed(ctx, Some(PlayState::Play), &metadata);
            ctx.send(CreateReply::default().embed(resp)).await?;
        } else {
            // if the playlist is empty
            turto_say(ctx, InvalidUrl(None)).await?;
        }
    }

    Ok(())
}
