use crate::{
    deps::{fetch_github_latest, ytdlp::os::get_archive_name},
    utils::get_http_client,
};
use reqwest::header::USER_AGENT;
use std::path::{Path, PathBuf};
use tokio::io::AsyncWriteExt;

#[derive(Debug, PartialEq, Eq)]
pub enum YtdlpVersion {
    Stable(chrono::NaiveDate),
    Nightly(chrono::NaiveDateTime),
}

impl Ord for YtdlpVersion {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        let self_datetime = match self {
            YtdlpVersion::Nightly(datetime) => *datetime,
            YtdlpVersion::Stable(date) => date.and_hms_opt(0, 0, 0).unwrap(),
        };
        let other_datetime = match other {
            YtdlpVersion::Nightly(datetime) => *datetime,
            YtdlpVersion::Stable(date) => date.and_hms_opt(0, 0, 0).unwrap(),
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
    const STABLE_TAG_FORMAT: &str = "%Y.%m.%d";
    const NIGHTLY_TAG_FORMAT: &str = "%Y.%m.%d.%H%M%S";

    pub fn tag(&self) -> String {
        match self {
            Self::Stable(date) => date.format(Self::STABLE_TAG_FORMAT).to_string(),
            Self::Nightly(datetime) => datetime.format(Self::NIGHTLY_TAG_FORMAT).to_string(),
        }
    }

    pub fn parse_from_tag_str(s: &str) -> anyhow::Result<Self> {
        let it = s.split('.').collect::<Vec<_>>();
        if it.len() == 3 || it.len() == 4 {
            let year = it[0].parse::<i32>()?;
            let month = it[1].parse::<u32>()?;
            let day = it[2].parse::<u32>()?;
            let Some(date) = chrono::NaiveDate::from_ymd_opt(year, month, day) else {
                return Err(anyhow::anyhow!("invalid yt-dlp tag date time"));
            };
            if it.len() == 3 {
                return Ok(Self::Stable(date));
            }
            let time = it
                .get(3)
                .filter(|time_str| time_str.len() == 6)
                .and_then(|time_str| {
                    let hour = time_str[0..2].parse::<u32>().ok()?;
                    let min = time_str[2..4].parse::<u32>().ok()?;
                    let sec = time_str[4..6].parse::<u32>().ok()?;
                    chrono::NaiveTime::from_hms_opt(hour, min, sec)
                })
                .unwrap_or_default();

            let datetime = date.and_time(time);
            return Ok(Self::Nightly(datetime));
        }
        Err(anyhow::anyhow!("invalid yt-dlp tag format"))
    }

    pub async fn fetch_lastest(nightly: bool) -> anyhow::Result<Self> {
        let repo_slug = if nightly {
            "yt-dlp/yt-dlp-nightly-builds"
        } else {
            "yt-dlp/yt-dlp"
        };
        let tag = fetch_github_latest(repo_slug).await?;

        YtdlpVersion::parse_from_tag_str(&tag)
    }

    pub async fn fetch_archive(&self, ytdlp_dir: impl AsRef<Path>) -> anyhow::Result<PathBuf> {
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

        tracing::info!(url = url, "fetching yt-dlp");

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
            self.tag(),
            get_archive_name()
        )
    }
}
