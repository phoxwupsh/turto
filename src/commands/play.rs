use crate::{
    message::TurtoMessageKind::{DifferentVoiceChannel, InvalidUrl, UserNotInVoiceChannel},
    models::{alias::Context, error::CommandError, playing::PlayState},
    utils::{
        create_playing_embed,
        guild::{GuildUtil, VoiceChannelState},
        join_voice_channel,
        play::{PlayContext, play_ytdlfile_meta},
        turto_say,
    },
    ytdl::YouTubeDl,
};
use poise::CreateReply;
use songbird::tracks::PlayMode;
use tracing::{Span, instrument};
use url::Url;

#[poise::command(slash_command, guild_only)]
#[instrument(
    name = "play",
    skip_all,
    parent = ctx.invocation_data::<Span>().await.as_deref().unwrap_or(&Span::none())
    fields(query)
)]
pub async fn play(
    ctx: Context<'_>,
    #[rename = "url"] query: Option<String>,
) -> Result<(), CommandError> {
    tracing::info!("invoke");

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
            join_voice_channel(ctx, guild_id, user_vc).await?
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

        tracing::info!("play success");

        let embed = create_playing_embed(ctx, Some(PlayState::Play), &meta);
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
            playing.track_handle.play()?;

            let metadata = playing
                .ytdlfile
                .fetch_metadata(ctx.data().config.ytdlp.clone())
                .await?;

            tracing::info!(url = playing.ytdlfile.url(), "resume");

            let resp = create_playing_embed(ctx, Some(PlayState::Play), &metadata);
            ctx.send(CreateReply::default().embed(resp)).await?;

            return Ok(());
        }
        drop(playing_map);

        ctx.defer().await?;

        let mut guild_data = data.guilds.entry(guild_id).or_default();
        let next = guild_data
            .playlist
            .pop_front_prefetch(ctx.data().config.ytdlp.clone());
        drop(guild_data);

        if let Some(next) = next {
            tracing::info!(url = next.url(), "play first item in playlist");

            let meta_fut =
                play_ytdlfile_meta(PlayContext::from_ctx(ctx).unwrap(), call, next).await?;
            let metadata = meta_fut.await?;

            let resp = create_playing_embed(ctx, Some(PlayState::Play), &metadata);
            ctx.send(CreateReply::default().embed(resp)).await?;
        } else {
            // if the playlist is empty
            turto_say(ctx, InvalidUrl(None)).await?;
        }
    }

    Ok(())
}
