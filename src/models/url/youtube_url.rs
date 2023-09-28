use std::fmt::Display;

#[derive(Debug, PartialEq, Clone)]
pub struct YouTubeUrl {
    pub id: Option<String>,
    pub time: Option<u64>,
    pub playlist_id: Option<String>
}

impl YouTubeUrl {
    pub fn playlist_url(&self) -> Option<String> {
        if let Some(playlist_id_) = &self.playlist_id {
            let mut res = String::with_capacity(72);
            res.push_str("https://www.youtube.com/playlist?list=");
            res.push_str(playlist_id_);
            return Some(res)
        }
        None
    }
    pub fn video_url(&self) -> VideoUrl {
        let mut res: Option<String> = None;
        if let Some(id_) = &self.id {
            let mut res_str = String::with_capacity(96);
            res_str.push_str("https://www.youtube.com/watch?v=");
            res_str.push_str(id_);
            let _ = res.insert(res_str);
        }
        VideoUrl {
            yt_url: self,
            acc: res
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct VideoUrl<'a> {
    yt_url: &'a YouTubeUrl,
    acc: Option<String>
}

impl VideoUrl<'_> {
    pub fn with_playlist(&mut self) -> &mut Self {
        if let Some(playlist_id) = &self.yt_url.playlist_id {
            if let Some(s) = &mut self.acc {
                s.push_str("&list=");
                s.push_str(playlist_id);
            }
        }
        self
    }
    pub fn with_time(&mut self) -> &mut Self {
        if let Some(time) = &self.yt_url.time {
            if let Some(s) = &mut self.acc {
                s.push_str("&t=");
                s.push_str(time.to_string().as_str());
                s.push('s');
            }
        }
        self
    }
    pub fn build(self) -> Option<String> {
        self.acc
    }
}

impl Display for YouTubeUrl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut res = String::with_capacity(96);
        res.push_str("https://www.youtube.com/");
        if let Some(id) = &self.id {
            res.push_str("watch?v=");
            res.push_str(id);
            if let Some(playlist_id) = &self.playlist_id {
                res.push_str("&list=");
                res.push_str(playlist_id);
            }
            if let Some(time) = &self.time {
                res.push_str("&t=");
                res.push_str(time.to_string().as_str());
                res.push('s');
            }
        } else if let Some(playlist_id) = &self.playlist_id {
            res.push_str("playlist?list=");
            res.push_str(playlist_id);
        }
        f.write_str(&res)
    }
}