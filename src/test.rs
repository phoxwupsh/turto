#[cfg(test)]
mod tests {
    use std::time::Duration;

    use crate::{
        models::{playlist_item::PlaylistItem, volume::GuildVolume, setting::GuildSetting},
        utils::i32_to_emoji,
    };

    #[test]
    fn test_i32_to_emoji() {
        assert_eq!(i32_to_emoji(42), "4️⃣2️⃣");
        assert_eq!(i32_to_emoji(123), "1️⃣2️⃣3️⃣");
        assert_eq!(i32_to_emoji(56789), "5️⃣6️⃣7️⃣8️⃣9️⃣");
        assert_eq!(i32_to_emoji(-999), "➖9️⃣9️⃣9️⃣")
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
        let gs_string = r#"{"auto_leave":true,"volume":0.33}"#;
        let gs = GuildSetting{
            auto_leave: true,
            volume: GuildVolume::try_from(0.33_f32).unwrap()
        };
        assert_eq!(serde_json::from_str::<GuildSetting>(gs_string).unwrap(), gs);
        assert_eq!(serde_json::to_string(&gs).unwrap(), gs_string);
    }



}
