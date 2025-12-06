use crate::{
    deps::{extract_to, fetch_github_latest},
    models::config::YtdlpConfig,
    utils::get_http_client,
};
use anyhow::Context;
use reqwest::header::USER_AGENT;
use std::{path::Path, process::Stdio, sync::OnceLock};
use tokio::io::AsyncWriteExt;

mod os;
use os::{get_archive_name, get_exec_name};

static DENO: OnceLock<String> = OnceLock::new();

pub fn get_deno_arg() -> &'static str {
    DENO.get().unwrap()
}

pub async fn setup_deno(config: &YtdlpConfig, deno_dir: impl AsRef<Path>) -> anyhow::Result<()> {
    if config.use_system_deno {
        let path = which::which("deno").context("expected deno in PATH")?;
        tracing::info!(path = %path.display(), "system deno found");
        DENO.set("deno:deno".to_string()).unwrap();
        return Ok(());
    }

    let deno_dir = deno_dir.as_ref();
    if !deno_dir.is_dir() {
        std::fs::create_dir_all(deno_dir)?;
    }
    let deno_exec = deno_dir.join(get_exec_name());
    let need_fetch = check_need_fetch_deno(&deno_exec)?;
    if need_fetch {
        tracing::warn!("local deno not found");
        let tag = fetch_github_latest("denoland/deno").await?;

        tracing::info!(version = tag, "found latest deno");

        let archive_name = get_archive_name();
        let url = format!(
            "https://github.com/denoland/deno/releases/download/{}/{}",
            tag, archive_name
        );
        let client = get_http_client();
        let mut resp = client
            .get(&url)
            .header(USER_AGENT, "phoxwupsh/turto")
            .send()
            .await?;

        let archive_path = deno_dir.join(archive_name);

        let mut archive = tokio::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&archive_path)
            .await?;

        tracing::info!(url = url, "fetching deno");

        while let Some(chunk) = resp.chunk().await? {
            archive.write_all(&chunk).await?;
        }

        extract_to(&archive_path, deno_dir)?;
        drop(archive);
        std::fs::remove_file(archive_path)?;
    } else {
        tracing::info!("found local deno");
    }
    DENO.set(format!("deno:{}", deno_exec.to_string_lossy()))
        .unwrap();
    Ok(())
}

fn check_need_fetch_deno(path: &Path) -> std::io::Result<bool> {
    if path.is_file() {
        let child = std::process::Command::new(path)
            .arg("--version")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        let output = child.wait_with_output()?;
        let stdout = output.stdout;
        return Ok(!stdout.trim_ascii_start().starts_with(b"deno"));
    }
    Ok(true)
}
