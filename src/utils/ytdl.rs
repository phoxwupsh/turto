use crate::models::{playlist_item::PlaylistItem, youtube_playlist::YouTubePlaylist};
use serde_json::{Map, Value};
use std::{
    io::{BufRead, BufReader},
    process::{Command, Stdio},
};
use tracing::error;

pub fn ytdl_playlist(url: &str) -> Option<YouTubePlaylist> {
    let args = vec![url, "--flat-playlist", "-j"];

    let child = match Command::new("yt-dlp")
        .args(args)
        .stdout(Stdio::piped())
        .spawn()
    {
        Ok(child) => child,
        Err(err) => {
            error!("Failed to run yt-dlp: {}", err);
            return None;
        }
    };

    let Some(reader) = child.stdout.map(BufReader::new) else {
        error!("Failed to read stdout");
        return None;
    };

    let mut res = YouTubePlaylist::new();
    let mut iter_read = reader.lines().map_while(Result::ok);

    let first = iter_read.next()?;
    let obj = match serde_json::from_str::<Map<String, Value>>(&first) {
        Ok(obj) => obj,
        Err(err) => {
            error!("Failed to parse yt-dlp output: {}", err);
            return None;
        }
    };

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
    if let Some(first_) = PlaylistItem::try_from_ytdl_output(&obj) {
        res.push_back(first_);
    }

    for line in iter_read {
        if let Ok(Some(new_playlist_item)) = serde_json::from_str::<Map<String, Value>>(&line)
            .map(|value| PlaylistItem::try_from_ytdl_output(&value))
        {
            res.push_back(new_playlist_item);
        }
    }
    Some(res)
}
