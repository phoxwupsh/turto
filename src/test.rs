#[cfg(test)]
mod tests {
    use std::{collections::HashSet, time::Duration};

    use serenity::model::prelude::UserId;

    use crate::{
        models::{
            guild::{config::GuildConfig, volume::GuildVolume},
            playlist_item::PlaylistItem,
            url::{youtube_url::YouTubeUrl, ParsedUrl},
        },
        utils::misc::ToEmoji,
    };

    #[test]
    fn test_to_emoji() {
        assert_eq!(42.to_emoji(), "4️⃣2️⃣");
        assert_eq!(123.to_emoji(), "1️⃣2️⃣3️⃣");
        assert_eq!(56789.to_emoji(), "5️⃣6️⃣7️⃣8️⃣9️⃣");
    }

    #[test]
    fn test_playlist_item_serde() {
        let playlist_item_str = r#"{"url":"https://www.youtube.com/watch?v=a51VH9BYzZA","title":"Stellar Stellar / 星街すいせい(official)","channel":"Suisei Channel","duration":{"secs":305,"nanos":0},"thumbnail":"https://i.ytimg.com/vi_webp/a51VH9BYzZA/maxresdefault.webp"}"#;
        let playlist_item = PlaylistItem {
            channel: "Suisei Channel".to_string(),
            duration: Duration::new(305, 0),
            url: "https://www.youtube.com/watch?v=a51VH9BYzZA".to_string(),
            title: "Stellar Stellar / 星街すいせい(official)".to_string(),
            thumbnail: "https://i.ytimg.com/vi_webp/a51VH9BYzZA/maxresdefault.webp".to_string(),
        };
        assert_eq!(
            serde_json::from_str::<PlaylistItem>(playlist_item_str).unwrap(),
            playlist_item
        );
        assert_eq!(
            serde_json::to_string(&playlist_item).unwrap(),
            playlist_item_str.to_string()
        )
    }

    #[test]
    fn test_guildvolume_serialization() {
        let guild_volume_str = "0.13";
        let guild_volume = GuildVolume::try_from(0.13_f32).unwrap();
        assert_eq!(serde_json::from_str::<GuildVolume>(guild_volume_str).unwrap(), guild_volume);
        assert_eq!(serde_json::to_string(&guild_volume).unwrap(), guild_volume_str);
    }

    #[test]
    fn test_guild_config_serialization() {
        let guild_config_str = r#"{"auto_leave":true,"volume":0.33,"banned":["1000005"]}"#;
        let mut guild_config = GuildConfig {
            auto_leave: true,
            volume: GuildVolume::try_from(0.33_f32).unwrap(),
            banned: HashSet::<UserId>::new(),
        };
        guild_config.banned.insert(UserId(1000005));
        assert_eq!(serde_json::from_str::<GuildConfig>(guild_config_str).unwrap(), guild_config);
        assert_eq!(serde_json::to_string(&guild_config).unwrap(), guild_config_str);
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
            Ok(ParsedUrl::Youtube(YouTubeUrl {
                id: Some("NjdqQyC7Rkc".to_string()),
                time: None,
                playlist_id: None
            }))
        );
        assert_eq!(
            short_yt_url_time,
            Ok(ParsedUrl::Youtube(YouTubeUrl {
                id: Some("NjdqQyC7Rkc".to_string()),
                time: Some(8),
                playlist_id: None
            }))
        );
        assert_eq!(
            yt_url,
            Ok(ParsedUrl::Youtube(YouTubeUrl {
                id: Some("NjdqQyC7Rkc".to_string()),
                time: None,
                playlist_id: None
            }))
        );
        assert_eq!(
            yt_url_time,
            Ok(ParsedUrl::Youtube(YouTubeUrl {
                id: Some("NjdqQyC7Rkc".to_string()),
                time: Some(8),
                playlist_id: None
            }))
        );
        assert_eq!(
            yt_playlist_only,
            Ok(ParsedUrl::Youtube(YouTubeUrl {
                id: None,
                time: None,
                playlist_id: Some("PL_b-2lmqru6AkZDHmVN9i_gbtJS--hRQj".to_string())
            }))
        );
        assert_eq!(
            yt_url_with_playlist,
            Ok(ParsedUrl::Youtube(YouTubeUrl {
                id: Some("NjdqQyC7Rkc".to_string()),
                time: None,
                playlist_id: Some("PL_b-2lmqru6AkZDHmVN9i_gbtJS--hRQj".to_string())
            }))
        );
        assert_eq!(
            yt_url_with_playlist_and_time,
            Ok(ParsedUrl::Youtube(YouTubeUrl {
                id: Some("NjdqQyC7Rkc".to_string()),
                time: Some(8),
                playlist_id: Some("PL_b-2lmqru6AkZDHmVN9i_gbtJS--hRQj".to_string())
            }))
        );
        assert_eq!(
            other_url,
            Ok(ParsedUrl::Other(
                "https://soundcloud.com/kivawu/the-beautiful-ones".to_owned()
            ))
        );
        assert!(invalid_url.is_err());
    }
}
