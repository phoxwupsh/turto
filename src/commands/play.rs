use crate::{
    messages::TurtoMessageKind::{DifferentVoiceChannel, InvalidUrl, Play, UserNotInVoiceChannel},
    models::alias::{Context, Error},
    utils::{
        guild::{GuildUtil, VoiceChannelState},
        join_voice_channel,
        play::{play_next, play_url},
        turto_say,
    },
};
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
                    error!("Failed to join voice channel {user_vc}: {err}");
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
        let meta = play_url(
            call,
            data.guilds.clone(),
            data.playing.clone(),
            guild_id,
            query,
        )
        .await?;

        let title = meta.title.as_deref().unwrap_or_default();
        turto_say(ctx, Play { title }).await?;
    } else {
        // If no url provided, check if there is a paused track or there is any song in the playlist
        let playing_map = data.playing.read().await;
        if let Some(playing) = playing_map.get(&guild_id) {
            if let Ok(current_track_state) = playing.track_handle.get_info().await {
                if current_track_state.playing == PlayMode::Pause {
                    // If there is a paused song then play it
                    if let Err(why) = playing.track_handle.play() {
                        let uuid = playing.track_handle.uuid();
                        error!("Failed to play track {uuid}: {why}");
                    } else {
                        let title = playing.metadata.title.as_deref().unwrap_or_default();
                        turto_say(ctx, Play { title }).await?;
                    }
                    return Ok(());
                }
            }
        }
        drop(playing_map);

        ctx.defer().await?;
        if let Some(Ok(meta)) =
            play_next(call, data.guilds.clone(), data.playing.clone(), guild_id).await
        {
            // if there is any song in the play list
            let title = meta.title.as_deref().unwrap_or_default();
            turto_say(ctx, Play { title }).await?;
        } else {
            // if the playlist is empty
            turto_say(ctx, InvalidUrl(None)).await?;
        }
    }

    Ok(())
}
