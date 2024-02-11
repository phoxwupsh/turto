pub mod youtube_url;

use std::{collections::HashMap, fmt::Display, str::FromStr};

use url::{ParseError, Url};

use self::youtube_url::YouTubeUrl;

#[derive(Debug, Clone)]
#[cfg_attr(test, derive(PartialEq, Eq))]
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
        let queries = parsed.query_pairs().collect::<HashMap<_, _>>();
        match parsed.host_str() {
            Some("www.youtube.com") | Some("youtu.be") => {
                let mut res = YouTubeUrl::builder();
                match parsed.path_segments() {
                    Some(mut segs) => match segs.next() {
                        Some("playlist") | Some("watch") => {
                            if let Some(video_id) = queries.get("v") {
                                res.video_id(video_id.as_ref());
                            }
                            if let Some(playlist_id) = queries.get("list") {
                                res.playlist_id(playlist_id.as_ref());
                            }
                        }
                        Some("shorts") => {
                            if let Some(video_id) = segs.next() {
                                res.video_id(video_id);
                            }
                        }
                        Some(video_id) => {
                            res.video_id(video_id.to_owned());
                        }
                        None => ()
                    }
                    None => return Ok(Self::Other(s.to_owned())),
                }
                if let Some(time) = queries
                    .get("t")
                    .and_then(|s| s.replace('s', "").parse::<u64>().ok())
                {
                    res.time(time);
                }
                match res.build() {
                    Some(yt_url) => Ok(Self::Youtube(yt_url)),
                    None => Ok(Self::Other(s.to_owned()))
                }
            }
            Some(_other) => Ok(Self::Other(s.to_owned())),
            None => Err(ParseError::EmptyHost),
        }
    }
}
