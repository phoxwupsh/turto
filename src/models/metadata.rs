use std::{
    ops::{Deref, DerefMut},
    time::Duration,
};

use serde::{
    de::{self, Deserialize, Deserializer, MapAccess, Visitor},
    ser::{Serialize, SerializeStruct, Serializer},
};
use songbird::input::Metadata as SongbirdMetadata;

#[derive(Debug, PartialEq)]
pub struct Metadata(SongbirdMetadata);

// #[derive(Serialize, Deserialize)] not working, implement manually
impl Serialize for Metadata {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut s = serializer.serialize_struct("Metadata", 11)?;
        s.serialize_field("track", &self.track)?;
        s.serialize_field("artist", &self.artist)?;
        s.serialize_field("date", &self.date)?;
        s.serialize_field("channels", &self.channels)?;
        s.serialize_field("channel", &self.channel)?;
        s.serialize_field("start_time", &self.start_time)?;
        s.serialize_field("duration", &self.duration)?;
        s.serialize_field("sample_rate", &self.sample_rate)?;
        s.serialize_field("source_url", &self.source_url)?;
        s.serialize_field("title", &self.title)?;
        s.serialize_field("thumbnail", &self.thumbnail)?;
        s.end()
    }
}

impl<'de> Deserialize<'de> for Metadata {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(serde::Deserialize)]
        #[serde(field_identifier, rename_all = "snake_case")]
        enum Field {
            Track,
            Artist,
            Date,
            Channels,
            Channel,
            StartTime,
            Duration,
            SampleRate,
            SourceUrl,
            Title,
            Thumbnail,
        }

        struct MetadataVisitor;

        impl<'de> Visitor<'de> for MetadataVisitor {
            type Value = Metadata;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("struct Metadata")
            }

            fn visit_map<V>(self, mut map: V) -> Result<Self::Value, V::Error>
            where
                V: MapAccess<'de>,
            {
                let mut track: Option<String> = None;
                let mut artist: Option<String> = None;
                let mut date: Option<String> = None;
                let mut channels: Option<u8> = None;
                let mut channel: Option<String> = None;
                let mut start_time: Option<Duration> = None;
                let mut duration: Option<Duration> = None;
                let mut sample_rate: Option<u32> = None;
                let mut source_url: Option<String> = None;
                let mut title: Option<String> = None;
                let mut thumbnail: Option<String> = None;

                while let Some(key) = map.next_key()? {
                    match key {
                        Field::Track => {
                            if track.is_some() {
                                return Err(de::Error::duplicate_field("track"));
                            }
                            track = map.next_value()?;
                        }
                        Field::Artist => {
                            if artist.is_some() {
                                return Err(de::Error::duplicate_field("artist"));
                            }
                            artist = map.next_value()?;
                        }
                        Field::Date => {
                            if date.is_some() {
                                return Err(de::Error::duplicate_field("date"));
                            }
                            date = map.next_value()?;
                        }
                        Field::Channels => {
                            if channels.is_some() {
                                return Err(de::Error::duplicate_field("channels"));
                            }
                            channels = map.next_value()?;
                        }
                        Field::Channel => {
                            if channel.is_some() {
                                return Err(de::Error::duplicate_field("channel"));
                            }
                            channel = map.next_value()?;
                        }
                        Field::StartTime => {
                            if start_time.is_some() {
                                return Err(de::Error::duplicate_field("start_time"));
                            }
                            start_time = map.next_value()?;
                        }
                        Field::Duration => {
                            if duration.is_some() {
                                return Err(de::Error::duplicate_field("duration"));
                            }
                            duration = map.next_value()?;
                        }
                        Field::SampleRate => {
                            if sample_rate.is_some() {
                                return Err(de::Error::duplicate_field("sample_rate"));
                            }
                            sample_rate = map.next_value()?;
                        }
                        Field::SourceUrl => {
                            if source_url.is_some() {
                                return Err(de::Error::duplicate_field("source_url"));
                            }
                            source_url = map.next_value()?;
                        }
                        Field::Title => {
                            if title.is_some() {
                                return Err(de::Error::duplicate_field("title"));
                            }
                            title = map.next_value()?;
                        }
                        Field::Thumbnail => {
                            if thumbnail.is_some() {
                                return Err(de::Error::duplicate_field("thumbnail"));
                            }
                            thumbnail = map.next_value()?;
                        }
                    }
                }
                let track = track.unwrap_or_default();
                let artist = artist.unwrap_or_default();
                let date = date.unwrap_or_default();
                let channels = channels.unwrap_or_default();
                let channel = channel.unwrap_or_default();
                let start_time = start_time.unwrap_or_default();
                let duration = duration.unwrap_or_default();
                let sample_rate = sample_rate.unwrap_or_default();
                let source_url = source_url.unwrap_or_default();
                let title = title.unwrap_or_default();
                let thumbnail = thumbnail.unwrap_or_default();

                let meta = SongbirdMetadata {
                    track: Some(track),
                    artist: Some(artist),
                    date: Some(date),
                    channels: Some(channels),
                    channel: Some(channel),
                    start_time: Some(start_time),
                    duration: Some(duration),
                    sample_rate: Some(sample_rate),
                    source_url: Some(source_url),
                    title: Some(title),
                    thumbnail: Some(thumbnail),
                };
                Ok(Metadata(meta))
            }
        }
        const FIELDS: &[&str] = &[
            "track",
            "artist",
            "date",
            "channels",
            "channel",
            "start_time",
            "duration",
            "sample_rate",
            "source_url",
            "title",
            "thumbnail",
        ];
        deserializer.deserialize_struct("Metadata", FIELDS, MetadataVisitor)
    }
}

impl Deref for Metadata {
    type Target = SongbirdMetadata;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Metadata {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl From<SongbirdMetadata> for Metadata {
    fn from(value: SongbirdMetadata) -> Self {
        Metadata(value)
    }
}
