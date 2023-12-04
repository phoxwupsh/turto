use crate::{messages::TurtoMessage, typemap::guild_data::GuildDataMap};
use serenity::{framework::standard::macros::hook, model::prelude::Message, prelude::Context};

#[hook]
pub async fn before_hook(ctx: &Context, msg: &Message, _cmd_name: &str) -> bool {
    let guild = msg.guild(&ctx.cache).unwrap().clone();
    let guild_data_map = ctx.data.read().await.get::<GuildDataMap>().unwrap().clone();
    let guild_data = guild_data_map.entry(guild.id).or_default();
    let banned = guild_data.config.banned.get(&msg.author.id).is_some();
    drop(guild_data);

    if banned {
        msg.reply(ctx, TurtoMessage::BannedUserResponse)
            .await
            .unwrap_or_else(|err| panic!("Error sending message: {}", err));
        return false;
    }
    true
}
