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
use template::names::*;

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
            ($template:expr $(, $arg:tt)* $(,)?) => {{
                let mut renderer = templates.get($template, locale).renderer();
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
            NotPlaying => render!(NOT_PLAYING),
            UserNotInVoiceChannel => render!(USER_NO_VC),
            BotNotInVoiceChannel => render!(BOT_NO_VC),
            DifferentVoiceChannel { bot } => render!(DIFF_VC, ("bot_voice_channel", bot.mention())),
            Play { title } => render!(PLAY, title),
            Pause { title } => render!(PAUSE, title),
            Stop { title } => render!(STOP, title),
            Skip { title } => match title {
                Some(title) => render!(SKIP, title),
                None => render!(SKIP_SUCC),
            },
            Join(channel) => render!(JOIN, ("voice_channel", channel.mention())),
            Leave(channel) => render!(LEAVE, ("voice_channel", channel.mention())),
            Queue { title } => render!(QUEUE, title),
            Remove { title } => render!(REMOVE, ("title", title)),
            RemoveAll => render!(REMOVE_ALL),
            InvalidRemove { length } => {
                render!(INVALID_RM_IDX, ("playlist_length", length))
            }
            InvalidRangeRemove { from, to, length } => {
                render!(INVALID_RM_RANGE, from, to, ("playlist_length", length))
            }
            InvalidUrl(url) => match url {
                Some(url) => render!(URL_NOT_FOUND, url),
                None => render!(INVALID_URL),
            },
            SetVolume(val) => render!(VOLUME, ("volume", val.to_emoji())),
            SetAutoleave(res) => match res {
                AutoleaveType::On => render!(TOGGLE_AUTOLEAVE, ("autoleave_status", "on")),
                AutoleaveType::Empty => render!(TOGGLE_AUTOLEAVE, ("autoleave_status", "empty")),
                AutoleaveType::Silent => {
                    render!(TOGGLE_AUTOLEAVE, ("autoleave_status", "slient"))
                }
                AutoleaveType::Off => {
                    render!(TOGGLE_AUTOLEAVE, ("autoleave_status", "off"))
                }
            },
            SeekSuccess => render!(SEEK_SUCC),
            InvalidSeek { seek_limit } => render!(INVALID_SEEK, seek_limit),
            SeekNotAllow { backward } => match backward {
                true => render!(BACK_SEEK_NOT_ALLOW),
                false => render!(SEEK_NOT_ALLOW),
            },
            SeekNotLongEnough { title, length } => {
                render!(SEEK_TOO_SHORT, title, length)
            }
            AdministratorOnly => render!(ADMIN_ONLY),
            Ban { success, user } => match success {
                true => render!(BAN_USER, ("user", user.mention())),
                false => render!(BANNED_USER, ("user", user.mention())),
            },
            Unban { success, user } => match success {
                true => render!(UNBAN_USER, ("user", user.mention())),
                false => render!(USER_NOT_BANNED, ("user", user.mention())),
            },
            BannedUserResponse => render!(BANNED),
            Shuffle => render!(SHUFFLE),
            SetRepeat(repeat) => match repeat {
                true => render!(TOGGLE_REPEAT, ("repeat_status", "✅")),
                false => render!(TOGGLE_REPEAT, ("repeat_status", "❎")),
            },
            EmptyPlaylist => render!(EMPTY_PL),
            InvalidPlaylistPage { total_pages } => {
                render!(INVALID_PL_PAGE, ("total_pages", total_pages.to_emoji()),)
            }
            RemoveMany { removed_number } => {
                render!(REMOVE_MANY, ("removed_number", removed_number.to_emoji()))
            }
        }
    }
}

impl From<TurtoMessage<'_>> for String {
    fn from(value: TurtoMessage) -> Self {
        value.to_string()
    }
}
