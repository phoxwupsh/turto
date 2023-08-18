use std::{
    collections::{VecDeque, vec_deque::IntoIter},
    ops::{Deref, DerefMut}, process::{Command, Stdio}, io::{BufRead, BufReader},
};

use serde::{Serialize, Deserialize};
use serde_json::Value;
use super::playlist_item::PlaylistItem;

#[derive(Debug, Serialize, Deserialize)]
pub struct Playlist(VecDeque<PlaylistItem>);

impl Playlist {
    pub fn new() -> Self {
        Playlist(VecDeque::<PlaylistItem>::new())
    }

    pub fn ytdl_playlist(url: &str) -> Option<Playlist> {
        let args = vec![url, "--flat-playlist", "-j"];
    
        let mut child = Command::new("yt-dlp")
            .args(args)
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
    
        let Some(stdout) = &mut child.stdout else {
            return None;
        };
    
        let reader = BufReader::new(stdout);
    
        let mut res = Playlist::new();
    
        for line in reader.lines().flatten() {
            if let Ok(value) = serde_json::from_str::<Value>(&line) {
                if let Some(new_playlist_item) = PlaylistItem::try_from_ytdl_output(&value) {
                    res.push_back(new_playlist_item);
                }
            }
        }
        Some(res)
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