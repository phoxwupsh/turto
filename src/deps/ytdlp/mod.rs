use crate::{deps::extract_to, models::config::YtdlpConfig};
use anyhow::Context;
use arc_swap::ArcSwap;
use std::{
    path::{Path, PathBuf},
    sync::{Arc, OnceLock},
};
use version::YtdlpVersion;

mod os;
pub use os::{get_local_ytdlp, update_path_ptr};

pub mod version;

static YTDLP_PATH: OnceLock<ArcSwap<PathBuf>> = OnceLock::new();

// static YTDLP: OnceLock<String> = OnceLock::new();

// pub fn get_ytdlp_path() -> &'static str {
//     YTDLP.get().unwrap()
// }

pub fn get_ytdlp_path() -> Arc<PathBuf> {
    YTDLP_PATH.get().unwrap().load_full()
}

pub fn set_ytdlp_path(path: PathBuf) {
    YTDLP_PATH.get().unwrap().store(Arc::new(path));
}

pub async fn setup_ytdlp(config: &YtdlpConfig, ytdlp_dir: impl AsRef<Path>) -> anyhow::Result<()> {
    if config.use_system_ytdlp {
        let path = which::which("yt-dlp").context("expected yt-dlp in PATH")?;
        tracing::info!(path = %path.display(), "system yt-dlp found");
        YTDLP_PATH
            .set(ArcSwap::from_pointee("yt-dlp".into()))
            .unwrap();
        return Ok(());
    }

    let ytdlp_dir = ytdlp_dir.as_ref();
    if !ytdlp_dir.is_dir() {
        std::fs::create_dir_all(ytdlp_dir)?;
    }

    let exec_path = match get_local_ytdlp(ytdlp_dir).await? {
        Some((local_ver, local_path)) => {
            tracing::info!(version = local_ver.tag(), "found local yt-dlp");

            let latest_ver = YtdlpVersion::fetch_lastest(config.use_nightly).await?;
            tracing::info!(version = latest_ver.tag(), "found lastest yt-dlp");

            if latest_ver > local_ver {
                update_to(ytdlp_dir, &latest_ver).await?
            } else {
                local_path
            }
        }
        None => {
            tracing::warn!("local yt-dlp not found");

            let latest_ver = YtdlpVersion::fetch_lastest(config.use_nightly).await?;
            tracing::info!(version = latest_ver.tag(), "found lastest yt-dlp");

            update_to(ytdlp_dir, &latest_ver).await?
        }
    };

    // YTDLP.set(exec_path.to_string_lossy().into_owned()).unwrap();
    YTDLP_PATH.set(ArcSwap::from_pointee(exec_path)).unwrap();
    Ok(())
}

pub async fn update_to(
    ytdlp_dir: impl AsRef<Path>,
    target_ver: &YtdlpVersion,
) -> anyhow::Result<PathBuf> {
    let ytdlp_dir = ytdlp_dir.as_ref();

    let target_tag = target_ver.tag();
    let extract_path = ytdlp_dir.join(&target_tag);
    std::fs::create_dir_all(&extract_path)?;

    let archive_path = target_ver.fetch_archive(ytdlp_dir).await?;
    extract_to(&archive_path, &extract_path)?;
    std::fs::remove_file(&archive_path)?;
    Ok(update_path_ptr(ytdlp_dir, &target_tag)?)
}
