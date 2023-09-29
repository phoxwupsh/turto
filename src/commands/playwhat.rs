use serenity::{
    framework::standard::{macros::command, Args, CommandResult},
    model::prelude::Message,
    prelude::Context,
};
use songbird::tracks::PlayMode;
use tracing::error;

use crate::{guild::playing::Playing, messages::TurtoMessage};

#[command]
#[bucket = "music"]
async fn playwhat(ctx: &Context, msg: &Message, _args: Args) -> CommandResult {
    let guild = msg.guild(&ctx.cache).unwrap();

    let playing_lock = ctx
        .data
        .read()
        .await
        .get::<Playing>()
        .unwrap()
        .clone();
    {
        let playing = playing_lock.read().await;
        let current_track = match playing.get(&guild.id) {
            Some(track) => track,
            None => {
                msg.reply(ctx, TurtoMessage::NotPlaying).await?;
                return Ok(());
            }
        };

        let title = current_track
            .metadata()
            .title
            .clone()
            .unwrap_or_else(|| "Unknown".to_string());

        let response = match current_track.get_info().await {
            Ok(track_state) => match track_state.playing {
                PlayMode::Play => TurtoMessage::Play { title: &title },
                PlayMode::Pause => TurtoMessage::Pause { title: &title },
                _ => {
                    msg.reply(ctx, TurtoMessage::NotPlaying).await?;
                    return Ok(());
                }
            },
            Err(e) => {
                error!("Error getting track: {:?}", e);
                return Ok(());
            }
        };

        msg.channel_id
            .send_message(ctx, |m| {
                m.content(String::default())
                    .reference_message(msg)
                    .embed(|embed| {
                        embed.title(response);
                        if let Some(url) = current_track.metadata().source_url.clone() {
                            embed.url(url);
                        }
                        if let Some(artist) = current_track.metadata().artist.clone() {
                            embed.description(artist);
                        }
                        if let Some(thumbnail) = current_track.metadata().thumbnail.clone() {
                            embed.thumbnail(thumbnail);
                        }
                        embed
                    })
            })
            .await?;
    }
    Ok(())
}
