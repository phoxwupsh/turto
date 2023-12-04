use crate::{
    config::message_template::get_renderer,
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

impl Display for TurtoMessage<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotPlaying => f.write_str(&get_renderer("not_playing").render()),
            Self::UserNotInVoiceChannel => {
                f.write_str(&get_renderer("user_not_in_voice_channel").render())
            }
            Self::BotNotInVoiceChannel => {
                f.write_str(&get_renderer("bot_not_in_voice_channel").render())
            }
            Self::DifferentVoiceChannel { bot } => f.write_str(
                &get_renderer("different_voice_channel")
                    .add_arg("bot_voice_channel", &bot.mention())
                    .render(),
            ),
            Self::Play { title } => {
                f.write_str(&get_renderer("play").add_arg("title", title).render())
            }
            Self::Pause { title } => {
                f.write_str(&get_renderer("pause").add_arg("title", title).render())
            }
            Self::Stop { title } => {
                f.write_str(&get_renderer("stop").add_arg("title", title).render())
            }
            Self::Skip { title } => {
                f.write_str(&get_renderer("skip").add_arg("title", title).render())
            }
            Self::Join(channel) => f.write_str(
                &get_renderer("join")
                    .add_arg("voice_channel", &channel.mention())
                    .render(),
            ),
            Self::Leave(channel) => f.write_str(
                &get_renderer("leave")
                    .add_arg("voice_channel", &channel.mention())
                    .render(),
            ),
            Self::Queue { title } => {
                f.write_str(&get_renderer("queue").add_arg("title", title).render())
            }
            Self::Remove { title } => {
                f.write_str(&get_renderer("remove").add_arg("title", title).render())
            }
            Self::RemovaAll => f.write_str(&get_renderer("remove_all").render()),
            Self::InvalidRemove { playlist_length } => match playlist_length {
                Some(length) => f.write_str(
                    &get_renderer("invalid_remove_index")
                        .add_arg("playlist_length", length)
                        .render(),
                ),
                None => f.write_str(&get_renderer("invalid_remove").render()),
            },
            Self::InvalidUrl(url) => match url {
                Some(url_) => {
                    f.write_str(&get_renderer("url_not_found").add_arg("url", url_).render())
                }
                None => f.write_str(&get_renderer("invalid_url").render()),
            },
            Self::SetVolume(res) => match res {
                Ok(vol) => f.write_str(
                    &get_renderer("volume")
                        .add_arg("volume", &vol.to_emoji())
                        .render(),
                ),
                Err(_) => f.write_str(&get_renderer("invalid_volume").render()),
            },
            Self::SetAutoleave(res) => match res {
                Ok(toggle) => {
                    let mut res = get_renderer("toggle_autoleave");
                    let autoleave_status = if *toggle { "✅" } else { "❎" };
                    f.write_str(&res.add_arg("autoleave_status", &autoleave_status).render())
                }
                Err(_) => f.write_str(&get_renderer("invalid_autoleave").render()),
            },
            Self::InvalidSeek { seek_limit } => f.write_str(
                &get_renderer("invalid_seek")
                    .add_arg("seek_limit", seek_limit)
                    .render(),
            ),
            Self::SeekNotAllow { backward } => match *backward {
                true => f.write_str(&get_renderer("backward_seek_not_allow").render()),
                false => f.write_str(&get_renderer("seek_not_allow").render()),
            },
            Self::SeekNotLongEnough { title, length } => f.write_str(
                &get_renderer("seek_not_long_enough")
                    .add_arg("title", title)
                    .add_arg("length", length)
                    .render(),
            ),
            Self::AdministratorOnly => f.write_str(&get_renderer("administrator_only").render()),
            Self::Ban { success, user } => {
                let mut res = match success {
                    true => get_renderer("user_got_banned"),
                    false => get_renderer("user_already_banned"),
                };
                f.write_str(&res.add_arg("user", &user.mention()).render())
            }
            Self::Unban { success, user } => {
                let mut res = match success {
                    true => get_renderer("user_got_unbanned"),
                    false => get_renderer("user_not_banned"),
                };
                f.write_str(&res.add_arg("user", &user.mention()).render())
            }
            Self::InvalidUser => f.write_str(&get_renderer("invalid_user").render()),
            Self::BannedUserResponse => f.write_str(&get_renderer("banned_user_repsonse").render()),
            Self::Help => f.write_str(&get_renderer("help").render()),
            Self::CommandHelp { command_name } => f.write_str(
                &get_renderer("command_help")
                    .add_arg("command_name", command_name)
                    .render(),
            ),
            Self::Shuffle(res) => {
                let res = match res {
                    Ok(_) => get_renderer("shuffle"),
                    Err(_) => get_renderer("empty_playlist"),
                };
                f.write_str(&res.render())
            }
            Self::SetRepeat(repeat) => match repeat {
                Ok(toggle) => {
                    let mut res = get_renderer("toggle_repeat");
                    let repeat_status = if *toggle { "✅" } else { "❎" };
                    f.write_str(&res.add_arg("repeat_status", &repeat_status).render())
                }
                Err(_) => f.write_str(&get_renderer("invalid_repeat").render()),
            },
        }
    }
}

impl From<TurtoMessage<'_>> for String {
    fn from(value: TurtoMessage) -> Self {
        value.to_string()
    }
}