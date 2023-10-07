use crate::{typemap::guild_data::GuildDataMap, messages::TurtoMessage};
use rand::{seq::SliceRandom, thread_rng};
use serenity::{
    client::Context,
    framework::standard::{macros::command, CommandResult},
    model::prelude::Message,
};

#[command]
#[bucket = "turto"]
async fn shuffle(ctx: &Context, msg: &Message) -> CommandResult {
    let guild = msg.guild(ctx).unwrap();
    let guild_data_map = ctx.data.read().await.get::<GuildDataMap>().unwrap().clone();
    let mut guild_data = guild_data_map.entry(guild.id).or_default();
    if guild_data.playlist.is_empty() {
        drop(guild_data);
        msg.reply(ctx, TurtoMessage::Shuffle(Err(()))).await?;
        return Ok(());
    }
    let playlist = guild_data.playlist.make_contiguous();
    playlist.shuffle(&mut thread_rng());
    drop(guild_data);

    msg.reply(ctx, TurtoMessage::Shuffle(Ok(()))).await?;
    Ok(())
}
