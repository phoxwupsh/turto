use crate::{
    models::{url_type::UrlType, volume::GuildVolume},
    utils::misc::ToEmoji,
};
use serenity::{model::prelude::{ChannelId, UserId}, prelude::Mentionable};
use std::fmt::Display;

pub enum TurtoMessage<'a> {
    NotPlaying,
    UserNotInVoiceChannel,
    BotNotInVoiceChannel,
    DifferentVoiceChannel { bot: &'a ChannelId },
    Play { title: &'a str },
    Pause { title: &'a str },
    Stop { title: &'a str },
    Join(&'a ChannelId),
    Leave(&'a ChannelId),
    Queue { title: &'a str },
    Remove { title: &'a str },
    InvalidUrl(Option<&'a UrlType>),
    SetVolume(Result<GuildVolume, ()>),
    SetAutoleave(Result<bool, ()>),
    Seek {seek_limit: u64},
    SeekToLong,
    SeekNotAllow,
    BackwardSeekNotAllow,
}

impl Display for TurtoMessage<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotPlaying => f.write_str("Not playing now."),
            Self::UserNotInVoiceChannel => f.write_str("You are not in a voice channel."),
            Self::BotNotInVoiceChannel => f.write_str("turto is not in a voice channel."),
            Self::DifferentVoiceChannel { bot } => {
                f.write_str(&format!("You are not in {}", bot.mention()))
            }
            Self::Play { title } => f.write_str(&("▶️ ".to_string() + title)),
            Self::Pause { title } => f.write_str(&("⏸️ ".to_string() + title)),
            Self::Stop { title } => f.write_str(&("⏹️ ".to_string() + title)),
            Self::Join(channel) => f.write_str(&format!("{}⬅️🐢", channel.mention())),
            Self::Leave(channel) => f.write_str(&format!("⬅️🐢{}", channel.mention())),
            Self::Queue { title } => f.write_str(&("✅ ".to_string() + title)),
            Self::Remove { title } => f.write_str(&("❎ ".to_string() + title)),
            Self::InvalidUrl(url) => match url {
                Some(url_) => f.write_str(&format!("Can't find {}", url_.to_string())),
                None => f.write_str("Please provide a valid url."),
            },
            Self::SetVolume(res) => match res {
                Ok(vol) => f.write_str(&("🔊".to_string() + &vol.to_emoji())),
                Err(_) => f.write_str("Enter a number ranges from 0 to 100."),
            },
            Self::SetAutoleave(res) => match res {
                Ok(toggle) => match toggle {
                    true => f.write_str("Autoleave: ✅"),
                    false => f.write_str("Autoleave: ❎"),
                },
                Err(_) => f.write_str("Please specify whether to turn on or off autoleave."),
            },
            Self::Seek { seek_limit } => f.write_str(&format!("Please enter a number between 0 ~ {}", seek_limit)),
            Self::SeekToLong => f.write_str("data"),
            Self::SeekNotAllow => f.write_str("data"),
            Self::BackwardSeekNotAllow => f.write_str("data"),
        }
    }
}
