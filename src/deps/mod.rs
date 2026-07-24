use crate::{models::config::YtdlpConfig, utils::get_http_client};
use reqwest::header::{ACCEPT, HeaderName, HeaderValue, USER_AGENT};
use std::path::Path;
use zip::ZipArchive;

pub mod bun;
pub mod uv;

/// Install the external tools the sidecar runs on: bun (the JS runtime for nsig
/// solving) and uv (the managed Python venv + yt-dlp). This owns *only* the
/// external deps; bringing the sidecar itself up is a separate step driven from
/// the bot's startup, so the dependency runs one way -- the sidecar uses these
/// deps, not the reverse.
pub async fn setup_ext_deps(config: &YtdlpConfig) -> Result<(), DepsError> {
    bun::setup_bun(config, "bun").await?;
    uv::setup_uv(config, "uv").await?;
    Ok(())
}

async fn fetch_github_latest(repo_slug: &str) -> Result<String, reqwest::Error> {
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

    #[derive(serde::Deserialize)]
    struct ApiResp {
        tag_name: String,
    }

    let resp = resp.json::<ApiResp>().await?;

    Ok(resp.tag_name)
}

pub fn extract_to(arhive: impl AsRef<Path>, target: impl AsRef<Path>) -> Result<(), DepsError> {
    let file = std::fs::OpenOptions::new().read(true).open(arhive)?;
    let rdr = std::io::BufReader::new(file);
    ZipArchive::new(rdr)
        .and_then(|mut zip_file| zip_file.extract(target))
        .map_err(Into::into)
}

pub fn extract_targz_to(
    archive: impl AsRef<Path>,
    target: impl AsRef<Path>,
) -> Result<(), DepsError> {
    let file = std::fs::OpenOptions::new().read(true).open(archive)?;
    let decoder = flate2::read::GzDecoder::new(std::io::BufReader::new(file));
    let mut ar = tar::Archive::new(decoder);
    ar.set_preserve_permissions(true);
    ar.unpack(target)?;
    Ok(())
}

#[derive(Debug, thiserror::Error)]
pub enum DepsError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("error from reqwest: {0}")]
    Reqwest(#[from] reqwest::Error),

    #[error("failed to process zip file: {0}")]
    Archive(#[from] zip::result::ZipError),

    #[error("uv command failed: {0}")]
    Uv(String),

    #[error("uv binary not found after extraction")]
    UvNotFound,
}
