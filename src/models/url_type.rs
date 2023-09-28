use std::{collections::HashMap, str::FromStr, fmt::Display};

use url::{Url, ParseError};

#[derive(Debug, PartialEq, Clone)]
pub enum UrlType {
    Youtube { id: String, time: Option<u32> },
    YoutubePlaylist { playlist_id: String },
    Other(String)
}

impl Display for UrlType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Youtube { id, time } => {
                let mut res = String::with_capacity(172);
                res.push_str("https://www.youtube.com/watch?v=");
                res.push_str(id);
                if let Some(t) = time {
                    res.push_str("&t=");
                    res.push_str(t.to_string().as_str());
                }
                f.write_str(&res)
            }
            Self::YoutubePlaylist { playlist_id } => {
                let mut res = String::with_capacity(288);
                res.push_str("https://www.youtube.com/playlist?list=");
                res.push_str(playlist_id);
                f.write_str(&res)
            },
            Self::Other(url) => f.write_str(url)
        }   
    }
}

impl FromStr for UrlType {
    type Err = ParseError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parsed = match Url::parse(s) {
            Ok(parsed) => parsed,
            Err(err) => return Err(err)
        };
        let queries = parsed.query_pairs().map(|(k, v)| (k.to_string(), v.to_string())).collect::<HashMap<_, _>>();
        let time = queries.get("t").and_then(|s|s.replace('s', "").parse::<u32>().ok());
        match parsed.host_str().unwrap() {
            "www.youtube.com" => {
                if let Some(playlist_id) = queries.get("list") {
                    return Ok(Self::YoutubePlaylist { playlist_id: playlist_id.to_string() });
                }
                let Some(v) = queries.get("v") else {
                    return Ok(Self::Other(s.to_owned()))
                };
                Ok(Self::Youtube { id: v.to_owned(), time })
            },
            "youtu.be" => {
                let Some(id) = parsed.path_segments().and_then(|mut s|s.next()) else {
                    return Ok(UrlType::Other(s.to_owned()))
                };
                Ok(Self::Youtube { id: id.to_owned(), time })
            }
            _other => Ok(Self::Other(s.to_owned()))
        }
    }
}