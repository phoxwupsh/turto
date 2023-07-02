#[cfg(test)]
mod tests {
    use std::time::Duration;

    use crate::{
        utils::{convert_to_emoji},
        guild::{playlist::Metadata}, models::volume::GuildVolume
    };
    use songbird::input::Metadata as SongbirdMetadata;

    #[test]
    fn test_convert_to_emoji() {
        assert_eq!(convert_to_emoji(42), "4️⃣2️⃣");
        assert_eq!(convert_to_emoji(123), "1️⃣2️⃣3️⃣");
        assert_eq!(convert_to_emoji(56789), "5️⃣6️⃣7️⃣8️⃣9️⃣");
    }

    #[test]
    fn test_metadata_serialize(){
        let meta_str = r#"{"track":"","artist":"Suisei Channel","date":"20211008","channels":2,"channel":"Suisei Channel","start_time":{"secs":0,"nanos":0},"duration":{"secs":305,"nanos":0},"sample_rate":48000,"source_url":"https://www.youtube.com/watch?v=a51VH9BYzZA","title":"Stellar Stellar / 星街すいせい(official)","thumbnail":"https://i.ytimg.com/vi_webp/a51VH9BYzZA/maxresdefault.webp"}"#;
        let sb_meta = SongbirdMetadata {
            track: Some("".to_string()),
            artist: Some("Suisei Channel".to_string()),
            date: Some("20211008".to_string()),
            channels: Some(2),
            channel: Some("Suisei Channel".to_string()),
            start_time: Some(Duration::new(0, 0)),
            duration: Some(Duration::new(305, 0)),
            sample_rate: Some(48000),
            source_url: Some("https://www.youtube.com/watch?v=a51VH9BYzZA".to_string()),
            title: Some("Stellar Stellar / 星街すいせい(official)".to_string()),
            thumbnail: Some("https://i.ytimg.com/vi_webp/a51VH9BYzZA/maxresdefault.webp".to_string())
        };
        let meta = Metadata::from(sb_meta);
        assert_eq!(serde_json::from_str::<Metadata>(meta_str).unwrap(), meta);
    }

    #[test]
    fn test_metadata_deserialize(){
        let meta_string = r#"{"track":"","artist":"Suisei Channel","date":"20211008","channels":2,"channel":"Suisei Channel","start_time":{"secs":0,"nanos":0},"duration":{"secs":305,"nanos":0},"sample_rate":48000,"source_url":"https://www.youtube.com/watch?v=a51VH9BYzZA","title":"Stellar Stellar / 星街すいせい(official)","thumbnail":"https://i.ytimg.com/vi_webp/a51VH9BYzZA/maxresdefault.webp"}"#.to_string();
        let sb_meta = SongbirdMetadata {
            track: Some("".to_string()),
            artist: Some("Suisei Channel".to_string()),
            date: Some("20211008".to_string()),
            channels: Some(2),
            channel: Some("Suisei Channel".to_string()),
            start_time: Some(Duration::new(0, 0)),
            duration: Some(Duration::new(305, 0)),
            sample_rate: Some(48000),
            source_url: Some("https://www.youtube.com/watch?v=a51VH9BYzZA".to_string()),
            title: Some("Stellar Stellar / 星街すいせい(official)".to_string()),
            thumbnail: Some("https://i.ytimg.com/vi_webp/a51VH9BYzZA/maxresdefault.webp".to_string())
        };
        let meta = Metadata::from(sb_meta);
        assert_eq!(serde_json::to_string(&meta).unwrap(), meta_string);
    }

    #[test]
    fn test_guildvolume_serialize(){
        let gv_string = "0.13";
        let gv = GuildVolume::try_from(0.13_f32).unwrap();
        assert_eq!(serde_json::from_str::<GuildVolume>(gv_string).unwrap(), gv)
    }

    #[test]
    fn test_guildvolume_deserialize(){
        let gv = GuildVolume::try_from(0.13_f32).unwrap();
        let gv_string = "0.13";
        assert_eq!(serde_json::to_string(&gv).unwrap(), gv_string)
    }

}