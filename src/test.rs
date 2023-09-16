#[cfg(test)]
mod tests {
    use std::{time::Duration, collections::HashSet};

    use serenity::model::prelude::UserId;

    use crate::{
        models::{playlist_item::PlaylistItem, volume::GuildVolume, guild_setting::GuildSetting, url_type::UrlType},
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
        assert_eq!(serde_json::from_str::<PlaylistItem>(playlist_item_str).unwrap(), playlist_item);
        assert_eq!(serde_json::to_string(&playlist_item).unwrap(), playlist_item_str.to_string())
    }

    #[test]
    fn test_guildvolume_serialization() {
        let gv_string = "0.13";
        let gv = GuildVolume::try_from(0.13_f32).unwrap();
        assert_eq!(serde_json::from_str::<GuildVolume>(gv_string).unwrap(), gv);
        assert_eq!(serde_json::to_string(&gv).unwrap(), gv_string);
    }

    #[test]
    fn test_guildsetting_serialization(){
        let gs_string = r#"{"auto_leave":true,"volume":0.33,"banned":["1000005"]}"#;
        let mut gs = GuildSetting{
            auto_leave: true,
            volume: GuildVolume::try_from(0.33_f32).unwrap(),
            banned: HashSet::<UserId>::new()
        };
        gs.banned.insert(UserId(1000005));
        assert_eq!(serde_json::from_str::<GuildSetting>(gs_string).unwrap(), gs);
        assert_eq!(serde_json::to_string(&gs).unwrap(), gs_string);
    }

    #[test]
    fn test_parse_ytdl_url(){
        let short_yt_url = "https://youtu.be/NjdqQyC7Rkc".parse::<UrlType>();
        let short_yt_url_time = "https://youtu.be/NjdqQyC7Rkc?t=8".parse::<UrlType>();
        let yt_url = "https://www.youtube.com/watch?v=NjdqQyC7Rkc".parse::<UrlType>();
        let yt_url_time = "https://www.youtube.com/watch?v=NjdqQyC7Rkc&t=8s".parse::<UrlType>();
        let yt_playlist_only = "https://www.youtube.com/playlist?list=PL_b-2lmqru6AkZDHmVN9i_gbtJS--hRQj".parse::<UrlType>();
        let yt_url_with_playlist = "https://www.youtube.com/watch?v=NjdqQyC7Rkc&list=PL_b-2lmqru6AkZDHmVN9i_gbtJS--hRQj".parse::<UrlType>();
        let other_url = "https://soundcloud.com/kivawu/the-beautiful-ones".parse::<UrlType>();
        let invalid_url = "some_invalid_url".parse::<UrlType>();

        assert_eq!(short_yt_url, Ok(UrlType::Youtube { id: "NjdqQyC7Rkc".to_owned(), time: None }));
        assert_eq!(short_yt_url_time, Ok(UrlType::Youtube { id: "NjdqQyC7Rkc".to_owned(), time: Some(8) }));
        assert_eq!(yt_url, Ok(UrlType::Youtube { id: "NjdqQyC7Rkc".to_owned(), time: None }));
        assert_eq!(yt_url_time, Ok(UrlType::Youtube { id: "NjdqQyC7Rkc".to_owned(), time: Some(8) }));
        assert_eq!(yt_playlist_only, Ok(UrlType::YoutubePlaylist { playlist_id: "PL_b-2lmqru6AkZDHmVN9i_gbtJS--hRQj".to_owned() }));
        assert_eq!(yt_url_with_playlist, Ok(UrlType::YoutubePlaylist { playlist_id: "PL_b-2lmqru6AkZDHmVN9i_gbtJS--hRQj".to_owned() }));
        assert_eq!(other_url, Ok(UrlType::Other("https://soundcloud.com/kivawu/the-beautiful-ones".to_owned())));
        assert!(invalid_url.is_err());
    }

}
