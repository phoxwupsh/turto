pub mod youtube_url;

use std::{collections::HashMap, fmt::Display, str::FromStr};

use url::{ParseError, Url};

use self::youtube_url::YouTubeUrl;

#[derive(Debug, PartialEq, Clone)]
pub enum ParsedUrl {
    Youtube(YouTubeUrl),
    Other(String),
}

impl Display for ParsedUrl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Youtube(yt_url) => f.write_str(yt_url.to_string().as_str()),
            Self::Other(url) => f.write_str(url),
        }
    }
}

impl FromStr for ParsedUrl {
    type Err = ParseError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parsed = match Url::parse(s) {
            Ok(parsed) => parsed,
            Err(err) => return Err(err),
        };
        let queries = parsed
            .query_pairs()
            .collect::<HashMap<_, _>>();
        match parsed.host_str() {
            Some("www.youtube.com") | Some("youtu.be") => {
                let id: Option<String>;
                let mut playlist_id: Option<String> = None;
                let time = queries
                    .get("t")
                    .and_then(|s| s.replace('s', "").parse::<u64>().ok());
                match parsed.path_segments().and_then(|mut seg| seg.next()) {
                    Some("playlist") | Some("watch") => {
                        id = queries.get("v").map(|s| s.to_string());
                        playlist_id = queries.get("list").map(|s| s.to_string());
                    }
                    Some(id_) => {
                        id = Some(id_.to_owned());
                    }
                    None => return Ok(Self::Other(s.to_owned())),
                }
                let yt_url = YouTubeUrl {
                    id,
                    time,
                    playlist_id,
                };
                Ok(Self::Youtube(yt_url))
            }
            Some(_other) => Ok(Self::Other(s.to_owned())),
            None => Err(ParseError::EmptyHost)
        }
    }
}
