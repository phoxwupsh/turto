use url::Url;

pub trait UrlExt {
    fn is_yt_playlist(&self) -> bool;
}

impl UrlExt for Url {
    fn is_yt_playlist(&self) -> bool {
        match self.host_str() {
            Some("www.youtube.com") | Some("youtube.com") | Some("youtu.be") => {
                self.query_pairs().any(|(k, _)| k == "list")
            }
            _ => false,
        }
    }
}
