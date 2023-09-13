use super::{
    playlist::{Playlist, YoutubePlaylistInfo},
    playlist_item::PlaylistItem,
};

pub enum Queueing {
    Single(PlaylistItem),
    Multiple {
        playlist: Playlist,
        playlist_info: YoutubePlaylistInfo,
    },
}
