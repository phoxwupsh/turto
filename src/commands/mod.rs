use self::{
    autoleave::AUTOLEAVE_COMMAND, ban::BAN_COMMAND, help::HELP_COMMAND, join::JOIN_COMMAND,
    leave::LEAVE_COMMAND, pause::PAUSE_COMMAND, play::PLAY_COMMAND, playlist::PLAYLIST_COMMAND,
    playwhat::PLAYWHAT_COMMAND, queue::QUEUE_COMMAND, remove::REMOVE_COMMAND,
    repeat::REPEAT_COMMAND, seek::SEEK_COMMAND, shuffle::SHUFFLE_COMMAND, skip::SKIP_COMMAND,
    stop::STOP_COMMAND, unban::UNBAN_COMMAND, volume::VOLUME_COMMAND,
};
use serenity::framework::standard::macros::group;

pub mod autoleave;
pub mod ban;
pub mod help;
pub mod join;
pub mod leave;
pub mod pause;
pub mod play;
pub mod playlist;
pub mod playwhat;
pub mod queue;
pub mod remove;
pub mod repeat;
pub mod seek;
pub mod shuffle;
pub mod skip;
pub mod stop;
pub mod unban;
pub mod volume;

#[group]
#[commands(
    play, pause, playwhat, stop, volume, playlist, queue, remove, join, leave, skip, seek, help,
    autoleave, ban, unban, shuffle, repeat
)]
#[only_in(guilds)]
struct TurtoCommands;
