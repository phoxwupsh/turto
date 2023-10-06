#[cfg(test)]
mod tests {

    use crate::{
        models::url::{youtube_url::YouTubeUrl, ParsedUrl},
        utils::misc::ToEmoji,
    };

    #[test]
    fn test_to_emoji() {
        assert_eq!(42.to_emoji(), "4️⃣2️⃣");
        assert_eq!(123.to_emoji(), "1️⃣2️⃣3️⃣");
        assert_eq!(56789.to_emoji(), "5️⃣6️⃣7️⃣8️⃣9️⃣");
    }

    #[test]
    fn test_parse_ytdl_url() {
        let short_yt_url = "https://youtu.be/NjdqQyC7Rkc".parse::<ParsedUrl>();
        let short_yt_url_time = "https://youtu.be/NjdqQyC7Rkc?t=8".parse::<ParsedUrl>();
        let yt_url = "https://www.youtube.com/watch?v=NjdqQyC7Rkc".parse::<ParsedUrl>();
        let yt_url_time = "https://www.youtube.com/watch?v=NjdqQyC7Rkc&t=8s".parse::<ParsedUrl>();
        let yt_playlist_only =
            "https://www.youtube.com/playlist?list=PL_b-2lmqru6AkZDHmVN9i_gbtJS--hRQj"
                .parse::<ParsedUrl>();
        let yt_url_with_playlist =
            "https://www.youtube.com/watch?v=NjdqQyC7Rkc&list=PL_b-2lmqru6AkZDHmVN9i_gbtJS--hRQj"
                .parse::<ParsedUrl>();
        let yt_url_with_playlist_and_time =
            "https://www.youtube.com/watch?v=NjdqQyC7Rkc&list=PL_b-2lmqru6AkZDHmVN9i_gbtJS--hRQj&t=8s"
                .parse::<ParsedUrl>();
        let other_url = "https://soundcloud.com/kivawu/the-beautiful-ones".parse::<ParsedUrl>();
        let invalid_url = "some_invalid_url".parse::<ParsedUrl>();

        assert_eq!(
            short_yt_url,
            Ok(ParsedUrl::Youtube(
                YouTubeUrl::builder()
                    .video_id("NjdqQyC7Rkc")
                    .build()
                    .unwrap()
            ))
        );
        assert_eq!(
            short_yt_url_time,
            Ok(ParsedUrl::Youtube(
                YouTubeUrl::builder()
                    .video_id("NjdqQyC7Rkc")
                    .time(8)
                    .build()
                    .unwrap()
            ))
        );
        assert_eq!(
            yt_url,
            Ok(ParsedUrl::Youtube(
                YouTubeUrl::builder()
                    .video_id("NjdqQyC7Rkc")
                    .build()
                    .unwrap()
            ))
        );
        assert_eq!(
            yt_url_time,
            Ok(ParsedUrl::Youtube(
                YouTubeUrl::builder()
                    .video_id("NjdqQyC7Rkc")
                    .time(8)
                    .build()
                    .unwrap()
            ))
        );
        assert_eq!(
            yt_playlist_only,
            Ok(ParsedUrl::Youtube(
                YouTubeUrl::builder()
                    .playlist_id("PL_b-2lmqru6AkZDHmVN9i_gbtJS--hRQj")
                    .build()
                    .unwrap()
            ))
        );
        assert_eq!(
            yt_url_with_playlist,
            Ok(ParsedUrl::Youtube(
                YouTubeUrl::builder()
                    .video_id("NjdqQyC7Rkc")
                    .playlist_id("PL_b-2lmqru6AkZDHmVN9i_gbtJS--hRQj")
                    .build()
                    .unwrap()
            ))
        );
        assert_eq!(
            yt_url_with_playlist_and_time,
            Ok(ParsedUrl::Youtube(
                YouTubeUrl::builder()
                    .video_id("NjdqQyC7Rkc")
                    .playlist_id("PL_b-2lmqru6AkZDHmVN9i_gbtJS--hRQj")
                    .time(8)
                    .build()
                    .unwrap()
            ))
        );
        assert_eq!(
            other_url,
            Ok(ParsedUrl::Other(
                "https://soundcloud.com/kivawu/the-beautiful-ones".to_owned()
            ))
        );
        assert!(invalid_url.is_err());
    }

    #[test]
    fn test_ytdl_url_to_string() {
        let video = YouTubeUrl::builder()
            .video_id("NjdqQyC7Rkc")
            .build()
            .unwrap();
        let video_time = YouTubeUrl::builder()
            .video_id("NjdqQyC7Rkc")
            .time(8)
            .build()
            .unwrap();
        let video_time_playlist = YouTubeUrl::builder()
            .video_id("NjdqQyC7Rkc")
            .playlist_id("PL_b-2lmqru6AkZDHmVN9i_gbtJS--hRQj")
            .time(8)
            .build()
            .unwrap();
        let video_playlist = YouTubeUrl::builder()
            .video_id("NjdqQyC7Rkc")
            .playlist_id("PL_b-2lmqru6AkZDHmVN9i_gbtJS--hRQj")
            .build()
            .unwrap();
        let playlist = YouTubeUrl::builder()
            .playlist_id("PL_b-2lmqru6AkZDHmVN9i_gbtJS--hRQj")
            .build()
            .unwrap();
        assert_eq!(
            video.to_string(),
            "https://www.youtube.com/watch?v=NjdqQyC7Rkc".to_owned()
        );
        assert_eq!(
            video_time.to_string(),
            "https://www.youtube.com/watch?v=NjdqQyC7Rkc&t=8s".to_owned()
        );
        assert_eq!(video_time_playlist.to_string(), "https://www.youtube.com/watch?v=NjdqQyC7Rkc&list=PL_b-2lmqru6AkZDHmVN9i_gbtJS--hRQj&t=8s".to_owned());
        assert_eq!(
            video_playlist.to_string(),
            "https://www.youtube.com/watch?v=NjdqQyC7Rkc&list=PL_b-2lmqru6AkZDHmVN9i_gbtJS--hRQj"
                .to_owned()
        );
        assert_eq!(
            playlist.to_string(),
            "https://www.youtube.com/playlist?list=PL_b-2lmqru6AkZDHmVN9i_gbtJS--hRQj".to_owned()
        )
    }
}
