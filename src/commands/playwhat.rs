use serenity::{
    framework::standard::{macros::command, Args, CommandResult},
    model::prelude::Message,
    prelude::Context,
};
use songbird::tracks::PlayMode;

use crate::{guild::playing::Playing, messages::NOT_PLAYING};

#[command]
async fn playwhat(ctx: &Context, msg: &Message, _args: Args) -> CommandResult {
    let guild = msg.guild(&ctx.cache).unwrap();
    let guild_id = guild.id;

    let playing_lock = {
        let data_read = ctx.data.read().await;
        data_read
            .get::<Playing>()
            .expect("Expected Playing in TypeMap")
            .clone()
    };
    {
        let playing = playing_lock.read().await;
        let current_track = match playing.get(&guild_id) {
            Some(track) => track,
            None => {
                msg.reply(ctx, NOT_PLAYING).await?;
                return Ok(());
            }
        };

        let title = current_track.metadata().title.clone().unwrap();

        let mut response = match current_track.get_info().await {
            Ok(track_state) => match track_state.playing {
                PlayMode::Play => "▶️ ".to_string(),
                PlayMode::Pause => "⏸️ ".to_string(),
                _ => {
                    msg.reply(ctx, NOT_PLAYING).await?;
                    return Ok(())
                },
            },
            Err(e) => format!("Error: {}", e),
        };

        response.push_str(&title);

        msg.channel_id.send_message(ctx, |m|{
            m.content(String::default())
                .embed(|e|{
                    e.title(response)
                    .url(current_track.metadata().source_url.clone().unwrap())
                    .description(current_track.metadata().channel.clone().unwrap())
                        .image(current_track.metadata().thumbnail.clone().unwrap())
                })
        }).await?;
    }
    Ok(())
}
