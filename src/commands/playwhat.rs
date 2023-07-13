use serenity::{
    framework::standard::{macros::command, Args, CommandResult},
    model::prelude::Message,
    prelude::Context,
};
use songbird::tracks::PlayMode;
use tracing::error;

use crate::{guild::playing::Playing, messages::NOT_PLAYING};

#[command]
#[bucket = "music"]
async fn playwhat(ctx: &Context, msg: &Message, _args: Args) -> CommandResult {
    let guild = msg.guild(&ctx.cache).unwrap();
    let guild_id = guild.id;

    let playing_lock = ctx
        .data
        .read()
        .await
        .get::<Playing>()
        .expect("Expected Playing in TypeMap")
        .clone();
    {
        let playing = playing_lock.read().await;
        let current_track = match playing.get(&guild_id) {
            Some(track) => track,
            None => {
                msg.reply(ctx, NOT_PLAYING).await?;
                return Ok(());
            }
        };

        let title = current_track
            .metadata()
            .title
            .clone()
            .unwrap_or_else(|| "Unknown".to_string());

        let mut response = match current_track.get_info().await {
            Ok(track_state) => match track_state.playing {
                PlayMode::Play => "▶️ ".to_string(),
                PlayMode::Pause => "⏸️ ".to_string(),
                _ => {
                    msg.reply(ctx, NOT_PLAYING).await?;
                    return Ok(());
                }
            },
            Err(e) => {
                error!("Error getting track: {:?}", e);
                return Ok(());
            }
        };

        response.push_str(&title);

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
