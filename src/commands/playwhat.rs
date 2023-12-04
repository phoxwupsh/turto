use crate::{messages::TurtoMessage, typemap::playing::PlayingMap};
use serenity::{
    builder::{CreateEmbed, CreateMessage},
    framework::standard::{macros::command, Args, CommandResult},
    model::prelude::Message,
    prelude::Context,
};
use songbird::tracks::PlayMode;
use tracing::error;

#[command]
#[bucket = "turto"]
async fn playwhat(ctx: &Context, msg: &Message, _args: Args) -> CommandResult {
    let guild = msg.guild_id.unwrap();

    let playing_lock = ctx.data.read().await.get::<PlayingMap>().unwrap().clone();
    let playing_map = playing_lock.read().await;
    let Some(playing) = playing_map.get(&guild) else {
        msg.reply(ctx, TurtoMessage::NotPlaying).await?;
        return Ok(());
    };

    let title = playing.metadata.title.clone().unwrap_or_default();
    let embed_title = match playing.track_handle.get_info().await {
        Ok(track_state) => match track_state.playing {
            PlayMode::Play => TurtoMessage::Play { title: &title },
            PlayMode::Pause => TurtoMessage::Pause { title: &title },
            _ => {
                msg.reply(ctx, TurtoMessage::NotPlaying).await?;
                return Ok(());
            }
        },
        Err(err) => {
            error!("Error getting track: {err}");
            return Ok(());
        }
    };

    let mut embed = CreateEmbed::new().title(embed_title);
    if let Some(url) = &playing.metadata.source_url {
        embed = embed.url(url);
    }
    if let Some(artist) = &playing.metadata.artist {
        embed = embed.description(artist);
    }
    if let Some(thumbnail) = &playing.metadata.thumbnail {
        embed = embed.image(thumbnail);
    }
    drop(playing_map);

    let response = CreateMessage::new()
        .content(String::default())
        .reference_message(msg)
        .embed(embed);

    msg.channel_id.send_message(ctx, response).await?;

    Ok(())
}
