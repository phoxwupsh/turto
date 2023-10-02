use std::fmt::Display;

#[derive(Debug, PartialEq, Clone)]
pub enum YouTubeUrl {
    Video {
        video_id: String,
        playlist_id: Option<String>,
        time: Option<u64>,
    },
    Playlist {
        playlist_id: String,
    },
}

pub struct YouTubeUrlBuilder {
    video_id: Option<String>,
    time: Option<u64>,
    playlist_id: Option<String>,
}

impl YouTubeUrl {
    pub fn builder() -> YouTubeUrlBuilder {
        YouTubeUrlBuilder {
            video_id: None,
            time: None,
            playlist_id: None,
        }
    }
    pub fn is_playlist(&self) -> bool {
        matches!(self, Self::Playlist { .. })
    }
    pub fn is_video(&self) -> bool {
        matches!(self, Self::Video { .. })
    }
}

impl YouTubeUrlBuilder {
    pub fn video_id<T: Into<String>>(&mut self, video_id: T) -> &mut Self {
        let _ = self.video_id.insert(video_id.into());
        self
    }
    pub fn playlist_id<T: Into<String>>(&mut self, playlist_id: T) -> &mut Self {
        let _ = self.playlist_id.insert(playlist_id.into());
        self
    }
    pub fn time(&mut self, time_sec: u64) -> &mut Self {
        let _ = self.time.insert(time_sec);
        self
    }
    pub fn build(&self) -> Option<YouTubeUrl> {
        if let Some(v) = &self.video_id {
            Some(YouTubeUrl::Video {
                video_id: v.to_owned(),
                playlist_id: self.playlist_id.clone(),
                time: self.time,
            })
        } else {
            self.playlist_id.as_ref().map(|pl| YouTubeUrl::Playlist {
                playlist_id: pl.to_owned(),
            })
        }
    }
}

impl Display for YouTubeUrl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut res = String::with_capacity(96);
        res.push_str("https://www.youtube.com/");
        match self {
            Self::Video {
                video_id,
                playlist_id,
                time,
            } => {
                res.push_str("watch?v=");
                res.push_str(video_id);
                if let Some(pl) = playlist_id {
                    res.push_str("&list=");
                    res.push_str(pl);
                }
                if let Some(t) = time {
                    res.push_str("&t=");
                    res.push_str(t.to_string().as_str());
                    res.push('s');
                }
                f.write_str(&res)
            }
            Self::Playlist { playlist_id } => {
                res.push_str("playlist?list=");
                res.push_str(playlist_id);
                f.write_str(&res)
            }
        }
    }
}
