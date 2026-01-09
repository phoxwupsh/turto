use super::Template;
use strum::{AsRefStr, EnumCount, EnumIter, EnumString};

#[derive(Debug, Clone, Copy, AsRefStr, EnumString, EnumCount, EnumIter, Hash, PartialEq, Eq)]
#[strum(serialize_all = "snake_case")]
pub enum TemplateName {
    NotPlaying,
    UserNotInVoiceChannel,
    BotNotInVoiceChannel,
    DifferentVoiceChannel,
    Play,
    Pause,
    Skip,
    SkipSuccess,
    Stop,
    Join,
    Leave,
    Queue,
    Remove,
    RemoveAll,
    InvalidRemoveIndex,
    UrlNotFound,
    InvalidUrl,
    Volume,
    ToggleAutoleave,
    SeekSuccess,
    InvalidSeek,
    SeekNotAllow,
    BackwardSeekNotAllow,
    SeekNotLongEnough,
    AdministratorOnly,
    UserGotBanned,
    UserAlreadyBanned,
    UserGotUnbanned,
    UserNotBanned,
    BannedUserRepsonse,
    EmptyPlaylist,
    Shuffle,
    ToggleRepeat,
    InvalidPlaylistPage,
    RemoveMany,
    InvalidRemoveRange,
}

impl TemplateName {
    pub fn default_template(&self) -> Template {
        use TemplateName::*;
        match self {
            NotPlaying => Template::parse("Not playing now."),
            UserNotInVoiceChannel => Template::parse("You are not in a voice channel."),
            BotNotInVoiceChannel => Template::parse("turto is not in a voice channel."),
            DifferentVoiceChannel => Template::parse("You are not in {bot_voice_channel}"),
            Play => Template::parse("▶️ {title}"),
            Pause => Template::parse("⏸️ {title}"),
            Skip => Template::parse("⏭️ {title}"),
            SkipSuccess => Template::parse("⏭️✅"),
            Stop => Template::parse("⏹️ {title}"),
            Join => Template::parse("{voice_channel}⬅️🐢"),
            Leave => Template::parse("⬅️🐢{voice_channel}"),
            Queue => Template::parse("✅ {title}"),
            Remove => Template::parse("❎ {title}"),
            RemoveAll => Template::parse("The playlist has been cleared."),
            InvalidRemoveIndex => {
                Template::parse("Please enter a number or range between 1 and {playlist_length}.")
            }
            UrlNotFound => Template::parse("Can't find `{url}`"),
            InvalidUrl => Template::parse("Please provide a valid url."),
            Volume => Template::parse("🔊{volume}"),
            ToggleAutoleave => Template::parse("Autoleave: `{autoleave_status}`"),
            SeekSuccess => Template::parse("⏩✅"),
            InvalidSeek => Template::parse(
                "Please enter a number between 0 and the seek limitation {seek_limit}.",
            ),
            SeekNotAllow => Template::parse("Seeking is not allow."),
            BackwardSeekNotAllow => Template::parse("Backward seeking is not allowed."),
            SeekNotLongEnough => Template::parse("`{title}` is only {length} seconds long."),
            AdministratorOnly => {
                Template::parse("This command can only be invoked by an adminstrator.")
            }
            UserGotBanned => Template::parse("{user} has been banned."),
            UserAlreadyBanned => Template::parse("{user} had already been banned."),
            UserGotUnbanned => Template::parse("{user} has been unbanned."),
            UserNotBanned => Template::parse("{user} hasn't been banned yet."),
            BannedUserRepsonse => {
                Template::parse("You are not allow to invoke any command because you are banned.")
            }
            EmptyPlaylist => Template::parse("The playlist is empty."),
            Shuffle => Template::parse("🔀✅"),
            ToggleRepeat => Template::parse("🔂{repeat_status}"),
            InvalidPlaylistPage => Template::parse("There's only {total_pages} in the playlist."),
            RemoveMany => Template::parse("🗑️{removed_number}"),
            InvalidRemoveRange => {
                Template::parse("Unable to remove from {from} to {to} in playlist")
            }
        }
    }
}
