use crate::{
    deps::{DepsError, fetch_github_latest, ytdlp::os::get_archive_name},
    utils::get_http_client,
};
use reqwest::header::USER_AGENT;
use std::{
    fmt::Display,
    path::{Path, PathBuf},
    str::FromStr,
};
use time::{
    Date, PrimitiveDateTime,
    format_description::BorrowedFormatItem,
    macros::{format_description, time},
};
use tokio::io::AsyncWriteExt;

#[derive(Debug, PartialEq, Eq)]
pub enum YtdlpVersion {
    Stable(time::Date),
    Nightly(time::PrimitiveDateTime),
}

impl Ord for YtdlpVersion {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        let self_datetime = match self {
            YtdlpVersion::Nightly(datetime) => *datetime,
            YtdlpVersion::Stable(date) => date.with_time(time!(00:00)),
        };
        let other_datetime = match other {
            YtdlpVersion::Nightly(datetime) => *datetime,
            YtdlpVersion::Stable(date) => date.with_time(time!(00:00)),
        };
        self_datetime.cmp(&other_datetime)
    }
}

impl PartialOrd for YtdlpVersion {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl YtdlpVersion {
    const STABLE_TAG_FORMAT: &[BorrowedFormatItem<'static>] =
        format_description!("[year].[month].[day]");
    const NIGHTLY_TAG_FORMAT: &[BorrowedFormatItem<'static>] =
        format_description!("[year].[month].[day].[hour][minute][second]");

    pub async fn fetch_lastest(nightly: bool) -> Result<Self, DepsError> {
        let repo_slug = if nightly {
            "yt-dlp/yt-dlp-nightly-builds"
        } else {
            "yt-dlp/yt-dlp"
        };
        let tag = fetch_github_latest(repo_slug).await?;

        YtdlpVersion::from_str(&tag).map_err(Into::into)
    }

    pub async fn fetch_archive(&self, ytdlp_dir: impl AsRef<Path>) -> Result<PathBuf, DepsError> {
        let url = self.download_url();
        let client = get_http_client();
        let mut resp = client
            .get(&url)
            .header(USER_AGENT, "turto")
            .send()
            .await?
            .error_for_status()?;

        tokio::fs::create_dir_all(&ytdlp_dir).await?;

        let archive_path = ytdlp_dir.as_ref().join(get_archive_name());
        let mut archive = tokio::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&archive_path)
            .await?;

        tracing::info!(url, "fetching yt-dlp");

        while let Some(chunk) = resp.chunk().await? {
            archive.write_all(&chunk).await?;
        }

        Ok(archive_path)
    }

    fn download_url(&self) -> String {
        let repo_name = match self {
            Self::Stable(_) => "yt-dlp",
            Self::Nightly(_) => "yt-dlp-nightly-builds",
        };

        format!(
            "https://github.com/yt-dlp/{}/releases/download/{}/{}",
            repo_name,
            self,
            get_archive_name()
        )
    }
}

impl Display for YtdlpVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Stable(date) => f.write_str(&date.format(Self::STABLE_TAG_FORMAT).unwrap()),
            Self::Nightly(datetime) => {
                f.write_str(&datetime.format(Self::NIGHTLY_TAG_FORMAT).unwrap())
            }
        }
    }
}

impl FromStr for YtdlpVersion {
    type Err = time::error::Parse;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Ok(date) = Date::parse(s, Self::STABLE_TAG_FORMAT) {
            Ok(YtdlpVersion::Stable(date))
        } else {
            PrimitiveDateTime::parse(s, Self::NIGHTLY_TAG_FORMAT).map(YtdlpVersion::Nightly)
        }
    }
}

#[test]
fn test_parse_stable_tag() {
    let tag = "2026.03.03";
    assert_eq!(
        YtdlpVersion::from_str(tag).unwrap(),
        YtdlpVersion::Stable(time::Date::from_calendar_date(2026, time::Month::March, 3).unwrap())
    );
}

#[test]
fn test_parse_nightly_tag() {
    let tag = "2026.03.03.162408";
    assert_eq!(
        YtdlpVersion::from_str(tag).unwrap(),
        YtdlpVersion::Nightly(
            time::Date::from_calendar_date(2026, time::Month::March, 3)
                .unwrap()
                .with_time(time!(16:24:08))
        )
    );
}

#[cfg(test)]
mod test {
    use super::YtdlpVersion;
    use std::str::FromStr;
    use time::macros::time;

    #[test]
    fn test_format_stable_tag() {
        let ver = YtdlpVersion::Stable(
            time::Date::from_calendar_date(2026, time::Month::March, 3).unwrap(),
        );
        assert_eq!(ver.to_string().as_str(), "2026.03.03");
    }

    #[test]
    fn test_format_nightly_tag() {
        let ver = YtdlpVersion::Nightly(
            time::Date::from_calendar_date(2026, time::Month::March, 3)
                .unwrap()
                .with_time(time!(16:24:08)),
        );
        assert_eq!(ver.to_string().as_str(), "2026.03.03.162408");
    }

    #[test]
    fn test_parse_invalid_tag() {
        let tag = "some invalid tag";
        assert!(YtdlpVersion::from_str(tag).is_err());
    }
}
