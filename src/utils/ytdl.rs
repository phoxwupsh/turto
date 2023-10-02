use crate::models::{playlist_item::PlaylistItem, youtube_playlist::YouTubePlaylist};
use serde_json::Value;
use std::{
    io::{BufRead, BufReader},
    process::{Command, Stdio},
};

pub fn ytdl_playlist(url: &str) -> Option<YouTubePlaylist> {
    let args = vec![url, "--flat-playlist", "-j"];

    let mut child = Command::new("yt-dlp")
        .args(args)
        .stdout(Stdio::piped())
        .spawn()
        .unwrap_or_else(|err| panic!("yt-dlp command failed to run: {}", err));

    let Some(stdout) = &mut child.stdout else {
        return None
    };

    let reader = BufReader::new(stdout);

    let mut res = YouTubePlaylist::new();
    let mut iter_read = reader.lines().flatten();

    if let Some(first) = iter_read.next() {
        if let Ok(value) = serde_json::from_str::<Value>(&first) {
            let obj = value.as_object()?;
            res.playlist_id = obj
                .get("playlist_id")
                .and_then(Value::as_str)
                .map(str::to_string);
            res.title = obj
                .get("playlist_title")
                .and_then(Value::as_str)
                .map(str::to_string);
            res.author = obj
                .get("playlist_uploader")
                .and_then(Value::as_str)
                .map(str::to_string);
            if let Some(first_) = PlaylistItem::try_from_ytdl_output(&value) {
                res.push_back(first_);
            }
        }
    } else {
        return None;
    }

    for line in iter_read {
        if let Ok(value) = serde_json::from_str::<Value>(&line) {
            if let Some(new_playlist_item) = PlaylistItem::try_from_ytdl_output(&value) {
                res.push_back(new_playlist_item);
            }
        }
    }
    Some(res)
}
