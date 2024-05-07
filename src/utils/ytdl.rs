use crate::models::youtube_playlist::{Output, YouTubePlaylist};
use std::process::Stdio;
use tokio::process::Command;
use url::Url;

pub async fn ytdl_playlist(url: &Url) -> Result<YouTubePlaylist, std::io::Error> {
    let args = vec![url.as_str(), "--flat-playlist", "-J"];

    let output = Command::new("yt-dlp")
        .args(args)
        .stdout(Stdio::piped())
        .output()
        .await?;

    Ok(serde_json::from_slice::<Output>(&output.stdout).map(Output::to_playlist)?)
}
