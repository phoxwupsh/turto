use crate::{
    models::{alias::Context, autoleave::AutoleaveType, guild::volume::GuildVolume},
    utils::misc::ToEmoji,
};
use serenity::{
    model::prelude::{ChannelId, UserId},
    prelude::Mentionable,
};
use std::fmt::Display;

pub mod template;
use template::names::TemplateName;

pub struct TurtoMessage<'a> {
    pub ctx: Context<'a>,
    pub kind: TurtoMessageKind<'a>,
}

impl<'a> TurtoMessage<'a> {
    pub fn new(ctx: Context<'a>, kind: TurtoMessageKind<'a>) -> TurtoMessage<'a> {
        Self { ctx, kind }
    }
}

pub enum TurtoMessageKind<'a> {
    NotPlaying,
    UserNotInVoiceChannel,
    BotNotInVoiceChannel,
    DifferentVoiceChannel {
        bot: ChannelId,
    },
    Play {
        title: &'a str,
    },
    Pause {
        title: &'a str,
    },
    Skip {
        title: Option<&'a str>,
    },
    Stop {
        title: &'a str,
    },
    Join(ChannelId),
    Leave(ChannelId),
    Queue {
        title: &'a str,
    },
    Remove {
        title: &'a str,
    },
    RemoveAll,
    InvalidRemove {
        length: usize,
    },
    InvalidRangeRemove {
        from: usize,
        to: usize,
        length: usize,
    },
    InvalidUrl(Option<&'a str>),
    SetVolume(GuildVolume),
    SetAutoleave(AutoleaveType),
    SeekSuccess,
    InvalidSeek {
        seek_limit: u64,
    },
    SeekNotAllow {
        backward: bool,
    },
    SeekNotLongEnough {
        title: &'a str,
        length: u64,
    },
    AdministratorOnly,
    Ban {
        success: bool,
        user: UserId,
    },
    Unban {
        success: bool,
        user: UserId,
    },
    BannedUserResponse,
    Shuffle,
    SetRepeat(bool),
    EmptyPlaylist,
    InvalidPlaylistPage {
        total_pages: usize,
    },
    RemoveMany {
        removed_number: usize,
    },
}

impl Display for TurtoMessage<'_> {
    #[allow(unused_mut)]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use TurtoMessageKind::*;
        let locale = self.ctx.locale();
        let templates = &self.ctx.data().templates;

        macro_rules! render {
            ($template:ident $(, $arg:tt)* $(,)?) => {{
                let mut renderer = templates.get_with_fallback(TemplateName::$template, locale).renderer();
                $(render!(@arg renderer $arg);)*
                for s in renderer.render_iter() {
                    f.write_str(s)?
                }
                Ok(())
            }};

            (@arg $ren:ident ($key:expr, $value:expr)) => {
                $ren.add_arg($key, &$value)
            };

            (@arg $ren:ident $ident:ident) => {
                $ren.add_arg(stringify!($ident), &$ident)
            };
        }

        match self.kind {
            NotPlaying => render!(NotPlaying),
            UserNotInVoiceChannel => render!(UserNotInVoiceChannel),
            BotNotInVoiceChannel => render!(BotNotInVoiceChannel),
            DifferentVoiceChannel { bot } => {
                render!(DifferentVoiceChannel, ("bot_voice_channel", bot.mention()))
            }
            Play { title } => render!(Play, title),
            Pause { title } => render!(Pause, title),
            Stop { title } => render!(Stop, title),
            Skip { title } => match title {
                Some(title) => render!(Skip, title),
                None => render!(SkipSuccess),
            },
            Join(channel) => render!(Join, ("voice_channel", channel.mention())),
            Leave(channel) => render!(Leave, ("voice_channel", channel.mention())),
            Queue { title } => render!(Queue, title),
            Remove { title } => render!(Remove, ("title", title)),
            RemoveAll => render!(RemoveAll),
            InvalidRemove { length } => {
                render!(InvalidRemoveIndex, ("playlist_length", length))
            }
            InvalidRangeRemove { from, to, length } => {
                render!(InvalidRemoveRange, from, to, ("playlist_length", length))
            }
            InvalidUrl(url) => match url {
                Some(url) => render!(UrlNotFound, url),
                None => render!(InvalidUrl),
            },
            SetVolume(val) => render!(Volume, ("volume", val.to_emoji())),
            SetAutoleave(res) => match res {
                AutoleaveType::On => render!(ToggleAutoleave, ("autoleave_status", "on")),
                AutoleaveType::Empty => render!(ToggleAutoleave, ("autoleave_status", "empty")),
                AutoleaveType::Silent => {
                    render!(ToggleAutoleave, ("autoleave_status", "slient"))
                }
                AutoleaveType::Off => {
                    render!(ToggleAutoleave, ("autoleave_status", "off"))
                }
            },
            SeekSuccess => render!(SeekSuccess),
            InvalidSeek { seek_limit } => render!(InvalidSeek, seek_limit),
            SeekNotAllow { backward } => match backward {
                true => render!(BackwardSeekNotAllow),
                false => render!(SeekNotAllow),
            },
            SeekNotLongEnough { title, length } => {
                render!(SeekNotLongEnough, title, length)
            }
            AdministratorOnly => render!(AdministratorOnly),
            Ban { success, user } => match success {
                true => render!(UserGotBanned, ("user", user.mention())),
                false => render!(UserAlreadyBanned, ("user", user.mention())),
            },
            Unban { success, user } => match success {
                true => render!(UserGotUnbanned, ("user", user.mention())),
                false => render!(UserNotBanned, ("user", user.mention())),
            },
            BannedUserResponse => render!(BannedUserRepsonse),
            Shuffle => render!(Shuffle),
            SetRepeat(repeat) => match repeat {
                true => render!(ToggleRepeat, ("repeat_status", "✅")),
                false => render!(ToggleRepeat, ("repeat_status", "❎")),
            },
            EmptyPlaylist => render!(EmptyPlaylist),
            InvalidPlaylistPage { total_pages } => {
                render!(InvalidPlaylistPage, ("total_pages", total_pages.to_emoji()),)
            }
            RemoveMany { removed_number } => {
                render!(RemoveMany, ("removed_number", removed_number.to_emoji()))
            }
        }
    }
}

impl From<TurtoMessage<'_>> for String {
    fn from(value: TurtoMessage) -> Self {
        value.to_string()
    }
}
