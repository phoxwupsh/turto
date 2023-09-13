use std::{
    collections::{vec_deque::IntoIter, VecDeque},
    io::{BufRead, BufReader},
    ops::{Deref, DerefMut},
    process::{Command, Stdio},
};

use super::playlist_item::PlaylistItem;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Serialize, Deserialize)]
pub struct Playlist(VecDeque<PlaylistItem>);

impl Playlist {
    pub fn new() -> Self {
        Playlist(VecDeque::<PlaylistItem>::new())
    }

    pub fn ytdl_playlist(url: &str) -> (Option<Playlist>, Option<YoutubePlaylistInfo>) {
        let args = vec![url, "--flat-playlist", "-j"];

        let mut child = Command::new("yt-dlp")
            .args(args)
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();

        let Some(stdout) = &mut child.stdout else {
            return (None, None)
        };

        let reader = BufReader::new(stdout);

        let mut res = Playlist::new();
        let mut iter_read = reader.lines().flatten();
        
        let (first, info) = {
            if let Some(first) = iter_read.next() {
                if let Ok(value) = serde_json::from_str::<Value>(&first) {
                    (PlaylistItem::try_from_ytdl_output(&value), YoutubePlaylistInfo::try_from_ytdl_output(&value))
                } else {
                    (None, None)
                }
            } else {
                (None, None)
            }
        };

        if let Some(first_) = first {
            res.push_back(first_);
        }

        for line in iter_read {
            if let Ok(value) = serde_json::from_str::<Value>(&line) {
                if let Some(new_playlist_item) = PlaylistItem::try_from_ytdl_output(&value) {
                    res.push_back(new_playlist_item);
                }
            }
        }
        (Some(res), info)
    }
}

impl Deref for Playlist {
    type Target = VecDeque<PlaylistItem>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Playlist {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Default for Playlist {
    fn default() -> Self {
        Self::new()
    }
}

impl IntoIterator for Playlist {
    type Item = PlaylistItem;
    type IntoIter = IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

pub struct YoutubePlaylistInfo {
    pub playlist_title: String,
    pub playlist_id: String,
    pub playlist_author: String,
}

impl YoutubePlaylistInfo {
    pub fn try_from_ytdl_output(value: &Value) -> Option<Self> {
        let obj = value.as_object()?;
        let playlist_id = obj
            .get("playlist_id")
            .and_then(Value::as_str)
            .map(str::to_string)?;
        let playlist_title = obj
            .get("playlist_title")
            .and_then(Value::as_str)
            .map(str::to_string)?;
        let playlist_author = obj
            .get("playlist_uploader")
            .and_then(Value::as_str)
            .map(str::to_string)?;
        Some(Self {
            playlist_title,
            playlist_id,
            playlist_author,
        })
    }
}
