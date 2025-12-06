use crate::{
    deps::{bun::os::get_dir_name, extract_to, fetch_github_latest},
    models::config::YtdlpConfig,
    utils::get_http_client,
};
use anyhow::Context;
use reqwest::header::USER_AGENT;
use std::{path::Path, process::Stdio, sync::OnceLock};
use tokio::io::AsyncWriteExt;

mod os;
use os::{get_archive_name, get_exec_name};

static BUN: OnceLock<String> = OnceLock::new();

pub fn get_bun_arg() -> &'static str {
    BUN.get().unwrap()
}

pub async fn setup_bun(config: &YtdlpConfig, bun_dir: impl AsRef<Path>) -> anyhow::Result<()> {
    if config.use_system_deno {
        let path = which::which("bun").context("expected bun in PATH")?;
        tracing::info!(path = %path.display(), "system bun found");
        BUN.set("bun".to_string()).unwrap();
        return Ok(());
    }

    let bun_dir = bun_dir.as_ref();
    if !bun_dir.is_dir() {
        std::fs::create_dir_all(bun_dir)?;
    }
    let bun_exec = bun_dir.join(get_dir_name()).join(get_exec_name());
    let need_fetch = check_need_fetch_bun(&bun_exec)?;
    if need_fetch {
        tracing::warn!("local bun not found");
        let tag = fetch_github_latest("oven-sh/bun").await?;

        tracing::info!(version = tag, "found latest bun");

        let archive_name = get_archive_name();
        let url = format!(
            "https://github.com/oven-sh/bun/releases/download/{}/{}",
            tag, archive_name
        );
        let client = get_http_client();
        let mut resp = client
            .get(&url)
            .header(USER_AGENT, "phoxwupsh/turto")
            .send()
            .await?;

        let archive_path = bun_dir.join(archive_name);

        let mut archive = tokio::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&archive_path)
            .await?;

        tracing::info!(url = url, "fetching bun");

        while let Some(chunk) = resp.chunk().await? {
            archive.write_all(&chunk).await?;
        }

        extract_to(&archive_path, bun_dir)?;
        drop(archive);
        std::fs::remove_file(archive_path)?;
    } else {
        tracing::info!("found local bun");
    }
    BUN.set(format!("bun:{}", bun_exec.to_string_lossy()))
        .unwrap();
    Ok(())
}

fn check_need_fetch_bun(path: &Path) -> std::io::Result<bool> {
    if path.is_file() {
        let child = std::process::Command::new(path)
            .env("NO_COLOR", "1")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        let output = child.wait_with_output()?;
        let stdout = output.stdout;
        return Ok(!stdout.trim_ascii_start().starts_with(b"Bun"));
    }
    Ok(true)
}
