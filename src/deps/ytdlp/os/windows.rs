use crate::deps::ytdlp::version::YtdlpVersion;
use std::{
    io::Write,
    path::{Path, PathBuf},
};

pub fn update_path_ptr(ytdlp_dir: impl AsRef<Path>, tag: &str) -> std::io::Result<PathBuf> {
    let ptr_path = ytdlp_dir.as_ref().join("current");
    let mut ptr = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(ptr_path)?;
    ptr.write_all(tag.as_bytes())?;

    let exec_path = ytdlp_dir.as_ref().join(tag).join("yt-dlp.exe");
    Ok(exec_path)
}

pub async fn get_local_ytdlp(ytdlp_dir: &Path) -> anyhow::Result<Option<(YtdlpVersion, PathBuf)>> {
    let ptr_path = ytdlp_dir.join("current");

    if !ptr_path.try_exists()? {
        return Ok(None);
    }

    if ptr_path.is_file() {
        let curr_ver_str = tokio::fs::read_to_string(&ptr_path).await?;
        let curr_exec = ytdlp_dir.join(&curr_ver_str).join("yt-dlp.exe");

        let cmd = tokio::process::Command::new(&curr_exec)
            .arg("--version")
            .stdout(std::process::Stdio::piped())
            .spawn()?;
        let output = cmd.wait_with_output().await?;
        let exe_ver_str = String::from_utf8_lossy(&output.stdout);

        if curr_ver_str != exe_ver_str.trim() {
            return Err(anyhow::anyhow!(
                "local version does not match: expected {}, got {}",
                curr_ver_str,
                exe_ver_str
            ));
        }

        return Ok(Some((
            YtdlpVersion::parse_from_tag_str(&curr_ver_str)?,
            curr_exec,
        )));
    } else if ptr_path.is_dir() {
        return Err(anyhow::anyhow!(
            "expected {} to be a file",
            ptr_path.as_os_str().display()
        ));
    }
    Ok(None)
}

#[cfg(all(target_os = "windows", target_arch = "x86_64"))]
pub fn get_archive_name() -> &'static str {
    "yt-dlp_win.zip"
}

#[cfg(all(target_os = "windows", target_arch = "aarch64"))]
pub fn get_archive_name() -> &'static str {
    "yt-dlp_win_arm64.zip"
}
