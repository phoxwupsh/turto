use crate::{
    config::message_template::get_template,
    models::{guild::volume::GuildVolume, url::ParsedUrl},
    utils::misc::ToEmoji,
};
use serenity::{
    model::prelude::{ChannelId, UserId},
    prelude::Mentionable,
};
use std::fmt::Display;

pub enum TurtoMessage<'a> {
    NotPlaying,
    UserNotInVoiceChannel,
    BotNotInVoiceChannel,
    DifferentVoiceChannel { bot: ChannelId },
    Play { title: &'a str },
    Pause { title: &'a str },
    Skip { title: &'a str },
    Stop { title: &'a str },
    Join(ChannelId),
    Leave(ChannelId),
    Queue { title: &'a str },
    Remove { title: &'a str },
    RemovaAll,
    InvalidRemove { playlist_length: Option<usize> },
    InvalidUrl(Option<&'a ParsedUrl>),
    SetVolume(Result<GuildVolume, ()>),
    SetAutoleave(Result<bool, ()>),
    InvalidSeek { seek_limit: u64 },
    SeekNotAllow { backward: bool },
    SeekNotLongEnough { title: &'a str, length: u64 },
    AdministratorOnly,
    Ban { success: bool, user: UserId },
    Unban { success: bool, user: UserId },
    InvalidUser,
    BannedUserResponse,
    Help,
    CommandHelp { command_name: &'a str },
    Shuffle(Result<(), ()>),
    SetRepeat(Result<bool, ()>),
}

macro_rules! render {
    ($f:expr, $template:expr $(, ($key:expr, $value:expr))* $(,)?) => {{
        $f.write_str(&get_template($template).renderer()
        $(
            .add_arg($key, $value)
        )*
        .render())
    }};
}

impl Display for TurtoMessage<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotPlaying => render!(f, "not_playing"),
            Self::UserNotInVoiceChannel => render!(f, "user_not_in_voice_channel"),
            Self::BotNotInVoiceChannel => render!(f, "bot_not_in_voice_channel"),
            Self::DifferentVoiceChannel { bot } => render!(
                f,
                "different_voice_channel",
                ("bot_voice_channel", &bot.mention())
            ),
            Self::Play { title } => render!(f, "play", ("title", title)),
            Self::Pause { title } => render!(f, "pause", ("title", title)),
            Self::Stop { title } => render!(f, "stop", ("title", title)),
            Self::Skip { title } => render!(f, "skip", ("title", title)),
            Self::Join(channel) => render!(f, "join", ("voice_channel", &channel.mention())),
            Self::Leave(channel) => render!(f, "leave", ("voice_channel", &channel.mention())),
            Self::Queue { title } => render!(f, "queue", ("title", title)),
            Self::Remove { title } => render!(f, "remove", ("title", title)),
            Self::RemovaAll => render!(f, "remove_all"),
            Self::InvalidRemove { playlist_length } => match playlist_length {
                Some(length) => render!(f, "invalid_remove_index", ("playlist_length", length)),
                None => render!(f, "invalid_remove"),
            },
            Self::InvalidUrl(url) => match url {
                Some(url_) => render!(f, "url_not_found", ("url", url_)),
                None => render!(f, "invalid_url"),
            },
            Self::SetVolume(res) => match res {
                Ok(vol) => render!(f, "volume", ("volume", &vol.to_emoji())),
                Err(_) => render!(f, "invalid_volume"),
            },
            Self::SetAutoleave(res) => match res {
                Ok(toggle) => match *toggle {
                    true => render!(f, "toggle_autoleave", ("autoleave_status", &"✅")),
                    false => render!(f, "toggle_autoleave", ("autoleave_status", &"❎")),
                },
                Err(_) => render!(f, "invalid_autoleave"),
            },
            Self::InvalidSeek { seek_limit } => {
                render!(f, "invalid_seek", ("seek_limit", seek_limit))
            }
            Self::SeekNotAllow { backward } => match *backward {
                true => render!(f, "backward_seek_not_allow"),
                false => render!(f, "seek_not_allow"),
            },
            Self::SeekNotLongEnough { title, length } => render!(
                f,
                "seek_not_long_enough",
                ("title", title),
                ("length", length)
            ),
            Self::AdministratorOnly => render!(f, "administrator_only"),
            Self::Ban { success, user } => match *success {
                true => render!(f, "user_got_banned", ("user", &user.mention())),
                false => render!(f, "user_already_banned", ("user", &user.mention())),
            },
            Self::Unban { success, user } => match *success {
                true => render!(f, "user_got_unbanned", ("user", &user.mention())),
                false => render!(f, "user_not_banned", ("user", &user.mention())),
            },
            Self::InvalidUser => render!(f, "invalid_user"),
            Self::BannedUserResponse => render!(f, "banned_user_repsonse"),
            Self::Help => render!(f, "help"),
            Self::CommandHelp { command_name } => {
                render!(f, "command_help", ("command_name", command_name))
            }
            Self::Shuffle(res) => match res {
                Ok(_) => render!(f, "shuffle"),
                Err(_) => render!(f, "empty_playlist"),
            },
            Self::SetRepeat(repeat) => match repeat {
                Ok(toggle) => match *toggle {
                    true => render!(f, "toggle_repeat", ("repeat_status", &"✅")),
                    false => render!(f, "toggle_repeat", ("repeat_status", &"❎")),
                },
                Err(_) => render!(f, "invalid_repeat"),
            },
        }
    }
}

impl From<TurtoMessage<'_>> for String {
    fn from(value: TurtoMessage) -> Self {
        value.to_string()
    }
}
