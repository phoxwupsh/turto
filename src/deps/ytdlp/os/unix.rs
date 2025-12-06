use crate::ytdlp::YtdlpVersion;
use std::{
    io::Write,
    path::{Path, PathBuf},
};

pub fn update_path_ptr(ytdlp_dir: &Path, tag: &str) -> std::io::Result<PathBuf> {
    let target = ytdlp_dir.join(tag).join("yt-dlp");
    let next_ptr = ytdlp_dir.join("next");
    std::os::unix::fs::symlink(&target, &next_ptr)?;
    let curr_ptr = ytdlp_dir.join("current");
    std::fs::rename(&next_ptr, &curr_ptr)?;
    Ok(curr_ptr)
}

pub async fn get_local_ytdlp(ytdlp_dir: &Path) -> anyhow::Result<Option<(YtdlpVersion, PathBuf)>> {
    let ptr_path = ytdlp_dir.join("current");

    if !ptr_path.try_exists()? {
        return Ok(None);
    }

    if ptr_path.is_symlink() {
        let cmd = tokio::process::Command::new(&ptr_path)
            .arg("--version")
            .stdout(std::process::Stdio::piped())
            .spawn()?;
        let output = cmd.wait_with_output().await?;
        let exe_ver_str = String::from_utf8_lossy(&output.stdout);

        return Ok(Some((
            YtdlpVersion::parse_from_tag_str(&curr_ver_str)?,
            ptr_path,
        )));
    } else if ptr_path.is_dir() {
        return Err(anyhow::anyhow!(
            "expected {} to be a symlink",
            ptr_path.as_os_str().display()
        ));
    }
    Ok(None)
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
pub fn get_archive_name() -> &'static str {
    "yt-dlp_musllinux.zip"
}

#[cfg(all(target_os = "linux", target_arch = "aarch64"))]
pub fn get_archive_name() -> &'static str {
    "yt-dlp_linux_aarch64.zip"
}

#[cfg(all(target_os = "macos"))]
pub fn get_archive_name() -> &'static str {
    "yt-dlp_macos.zip"
}
