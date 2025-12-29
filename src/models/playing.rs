use crate::ytdl::YouTubeDl;
use songbird::tracks::TrackHandle;

#[derive(Debug)]
pub struct Playing {
    pub track_handle: TrackHandle,
    pub ytdlfile: YouTubeDl, // Metadata here is only for read purpose and not write behavior is supposed to happen
}

#[derive(Debug, Clone, Copy)]
pub enum PlayState {
    Play,
    Pause,
    Skip,
    Stop
}