use std::path::Path;

use crate::{models::config::YtdlpConfig, utils::get_http_client};
use reqwest::header::{ACCEPT, HeaderName, HeaderValue, USER_AGENT};
use zip::ZipArchive;

pub mod deno;
pub mod ytdlp;

pub async fn setup_ext_deps(config: &YtdlpConfig) -> anyhow::Result<()> {
    ytdlp::setup_ytdlp(config, "yt-dlp").await?;
    deno::setup_deno(config, "deno").await?;
    Ok(())
}

async fn fetch_github_latest(repo_slug: &str) -> anyhow::Result<String> {
    let mut headers = reqwest::header::HeaderMap::new();
    headers.append(
        ACCEPT,
        HeaderValue::from_static("application/vnd.github+json"),
    );
    headers.append(
        HeaderName::from_static("x-github-api-version"),
        HeaderValue::from_static("2022-11-28"),
    );
    headers.append(USER_AGENT, HeaderValue::from_static("phoxwupsh/turto"));

    let repo_url = format!("https://api.github.com/repos/{}/releases/latest", repo_slug);
    let client = get_http_client();
    let resp = client
        .get(repo_url)
        .headers(headers)
        .send()
        .await?
        .error_for_status()?;
    let json = resp.text().await?;

    let mut map = serde_json::from_str::<serde_json::Map<String, serde_json::Value>>(&json)?;
    let Some(serde_json::Value::String(tag)) = map.get_mut("tag_name").map(serde_json::Value::take)
    else {
        return Err(anyhow::anyhow!("expected tag_name to be a string"));
    };
    Ok(tag)
}

pub fn extract_to(arhive: impl AsRef<Path>, target: impl AsRef<Path>) -> anyhow::Result<()> {
    let file = std::fs::OpenOptions::new().read(true).open(arhive)?;
    let mut zip_file = ZipArchive::new(file)?;
    zip_file.extract(target)?;
    Ok(())
}
