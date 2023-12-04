use serde::{Deserialize, Serialize};
use serde_json::Value;
use songbird::input::AuxMetadata;
use std::time::Duration;

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
pub struct PlaylistItem {
    pub url: String,
    pub title: String,
    pub channel: String,
    pub duration: Duration,
    pub thumbnail: String,
}

impl From<AuxMetadata> for PlaylistItem {
    fn from(value: AuxMetadata) -> Self {
        PlaylistItem {
            url: value.source_url.unwrap_or_default(),
            title: value.title.unwrap_or_default(),
            channel: value.channel.unwrap_or_default(),
            duration: value.duration.unwrap_or_default(),
            thumbnail: value.thumbnail.unwrap_or_default(),
        }
    }
}

impl PlaylistItem {
    pub fn try_from_ytdl_output(value: &Value) -> Option<Self> {
        let obj = value.as_object()?;

        let url = obj
            .get("webpage_url")
            .and_then(Value::as_str)
            .map(str::to_string)?;

        let title = obj
            .get("title")
            .and_then(Value::as_str)
            .map(str::to_string)?;

        let channel = obj
            .get("channel")
            .and_then(Value::as_str)
            .map(str::to_string)?;

        let duration = obj
            .get("duration")
            .and_then(Value::as_f64)
            .map(Duration::from_secs_f64)?;

        let thumbnail = obj
            .get("thumbnails")
            .and_then(Value::as_array)
            .map(|t| t.last().unwrap_or(&Value::Null))
            .and_then(Value::as_object)
            .and_then(|t| t.get("url"))
            .and_then(Value::as_str)
            .map(str::to_string)?;

        Some(Self {
            url,
            title,
            channel,
            duration,
            thumbnail,
        })
    }
}
