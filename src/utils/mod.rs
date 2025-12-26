use crate::{
    message::{
        TurtoMessage,
        TurtoMessageKind::{self, Join},
    },
    models::{alias::Context, error::CommandError},
};
use poise::ReplyHandle;
use reqwest::Client;
use serenity::all::{ChannelId, GuildId};
use songbird::Call;
use std::{
    future::Future,
    sync::{Arc, OnceLock},
};
use tokio::sync::Mutex;

pub mod guild;
pub mod json;
pub mod misc;
pub mod play;
pub mod queue;

static HTTP_CLIENT: OnceLock<Client> = OnceLock::new();

pub fn get_http_client() -> Client {
    HTTP_CLIENT.get_or_init(Client::new).clone()
}

pub async fn join_voice_channel(
    ctx: Context<'_>,
    guild_id: GuildId,
    channel_id: ChannelId,
) -> Result<Arc<Mutex<Call>>, CommandError> {
    // there is some time limit of a command to be response,
    // joining a voice can take time and cause timeout
    // so use defer to prevent timeout
    ctx.defer().await?;
    let call = songbird::get(ctx.serenity_context())
        .await
        .unwrap()
        .join(guild_id, channel_id)
        .await?;

    tracing::info!(channel = %channel_id, "join voice channel success");

    turto_say(ctx, Join(channel_id)).await?;
    Ok(call)
}

#[inline]
pub fn turto_say<'a>(
    ctx: Context<'a>,
    msg: TurtoMessageKind<'a>,
) -> impl Future<Output = Result<ReplyHandle<'a>, serenity::Error>> {
    ctx.say(TurtoMessage::new(ctx, msg))
}
