use crate::{
    config::message_template::get_template,
    models::{autoleave::AutoleaveType, guild::volume::GuildVolume},
    utils::misc::ToEmoji,
};
use serenity::{
    model::prelude::{ChannelId, UserId},
    prelude::Mentionable,
};
use std::fmt::Display;

pub struct TurtoMessage<'a> {
    pub locale: Option<&'a str>,
    pub kind: TurtoMessageKind<'a>,
}

use TurtoMessageKind::*;
pub enum TurtoMessageKind<'a> {
    NotPlaying,
    UserNotInVoiceChannel,
    BotNotInVoiceChannel,
    DifferentVoiceChannel { bot: ChannelId },
    Play { title: &'a str },
    Pause { title: &'a str },
    Skip { title: Option<&'a str> },
    Stop { title: &'a str },
    Join(ChannelId),
    Leave(ChannelId),
    Queue { title: &'a str },
    Remove { title: &'a str },
    RemoveAll,
    InvalidRemove { length: usize },
    InvalidRangeRemove { from: usize, to: usize, length: usize },
    InvalidUrl(Option<&'a str>),
    SetVolume(GuildVolume),
    SetAutoleave(AutoleaveType),
    SeekSuccess,
    InvalidSeek { seek_limit: u64 },
    SeekNotAllow { backward: bool },
    SeekNotLongEnough { title: &'a str, length: u64 },
    AdministratorOnly,
    Ban { success: bool, user: UserId },
    Unban { success: bool, user: UserId },
    BannedUserResponse,
    Shuffle,
    SetRepeat(bool),
    EmptyPlaylist,
    InvalidPlaylistPage { total_pages: usize },
    RemoveMany { removed_number: usize },
}

impl Display for TurtoMessage<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let locale = self.locale;

        macro_rules! render {
            ($template:expr $(, ($key:expr, $value:expr))* $(,)?) => {{
                f.write_str(&get_template($template, locale).renderer()
                $(.add_arg($key, &$value))*
                .render())
            }};
        }

        match self.kind {
            NotPlaying => render!("not_playing"),
            UserNotInVoiceChannel => render!("user_not_in_voice_channel"),
            BotNotInVoiceChannel => render!("bot_not_in_voice_channel"),
            DifferentVoiceChannel { bot } => render!(
                "different_voice_channel",
                ("bot_voice_channel", bot.mention())
            ),
            Play { title } => render!("play", ("title", title)),
            Pause { title } => render!("pause", ("title", title)),
            Stop { title } => render!("stop", ("title", title)),
            Skip { title } => match title {
                Some(title) => render!("skip", ("title", title)),
                None => render!("skip_success"),
            },
            Join(channel) => render!("join", ("voice_channel", channel.mention())),
            Leave(channel) => render!("leave", ("voice_channel", channel.mention())),
            Queue { title } => render!("queue", ("title", title)),
            Remove { title } => render!("remove", ("title", title)),
            RemoveAll => render!("remove_all"),
            InvalidRemove { length } => {
                render!("invalid_remove_index", ("playlist_length", length))
            }
            InvalidRangeRemove { from, to, length } => render!(
                "invalid_remove_range",
                ("from", from),
                ("to", to),
                ("playlist_length", length)
            ),
            InvalidUrl(url) => match url {
                Some(url) => render!("url_not_found", ("url", url)),
                None => render!("invalid_url"),
            },
            SetVolume(val) => render!("volume", ("volume", val.to_emoji())),
            SetAutoleave(res) => match res {
                AutoleaveType::On => {
                    render!("toggle_autoleave", ("autoleave_status", "on"))
                }
                AutoleaveType::Empty => render!("toggle_autoleave", ("autoleave_status", "empty")),
                AutoleaveType::Silent => {
                    render!("toggle_autoleave", ("autoleave_status", "slient"))
                }
                AutoleaveType::Off => {
                    render!("toggle_autoleave", ("autoleave_status", "off"))
                }
            },
            SeekSuccess => render!("seek_success"),
            InvalidSeek { seek_limit } => {
                render!("invalid_seek", ("seek_limit", seek_limit))
            }
            SeekNotAllow { backward } => match backward {
                true => render!("backward_seek_not_allow"),
                false => render!("seek_not_allow"),
            },
            SeekNotLongEnough { title, length } => {
                render!("seek_not_long_enough", ("title", title), ("length", length))
            }
            AdministratorOnly => render!("administrator_only"),
            Ban { success, user } => match success {
                true => render!("user_got_banned", ("user", user.mention())),
                false => render!("user_already_banned", ("user", user.mention())),
            },
            Unban { success, user } => match success {
                true => render!("user_got_unbanned", ("user", user.mention())),
                false => render!("user_not_banned", ("user", user.mention())),
            },
            BannedUserResponse => render!("banned_user_repsonse"),
            Shuffle => render!("shuffle"),
            SetRepeat(repeat) => match repeat {
                true => render!("toggle_repeat", ("repeat_status", "✅")),
                false => render!("toggle_repeat", ("repeat_status", "❎")),
            },
            EmptyPlaylist => render!("empty_playlist"),
            InvalidPlaylistPage { total_pages } => render!(
                "invalid_playlist_page",
                ("total_pages", total_pages.to_emoji()),
            ),
            RemoveMany { removed_number } => {
                render!("remove_many", ("removed_number", removed_number.to_emoji()))
            }
        }
    }
}

impl From<TurtoMessage<'_>> for String {
    fn from(value: TurtoMessage) -> Self {
        value.to_string()
    }
}
