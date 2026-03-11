use crate::deps::{DepsError, ytdlp::YtdlpVersion};
use std::{
    path::{Path, PathBuf},
    str::FromStr,
};

pub fn update_path_ptr(ytdlp_dir: &Path, tag: &str) -> std::io::Result<PathBuf> {
    let target = if ytdlp_dir.is_absolute() {
        ytdlp_dir.join(tag).join(get_exec_name())
    } else {
        PathBuf::from(tag).join(get_exec_name())
    };
    let next_ptr = ytdlp_dir.join("next");
    std::os::unix::fs::symlink(&target, &next_ptr)?;
    let curr_ptr = ytdlp_dir.join("current");
    std::fs::rename(&next_ptr, &curr_ptr)?;
    Ok(curr_ptr)
}

pub async fn get_local_ytdlp(
    ytdlp_dir: &Path,
) -> Result<Option<(YtdlpVersion, PathBuf)>, DepsError> {
    let ptr_path = ytdlp_dir.join("current");

    let cmd = tokio::process::Command::new(&ptr_path)
        .arg("--version")
        .stdout(std::process::Stdio::piped())
        .spawn()?;
    let output = cmd.wait_with_output().await?;
    let exe_ver_str = String::from_utf8_lossy(&output.stdout);

    Ok(Some((
        YtdlpVersion::from_str(exe_ver_str.trim())?,
        ptr_path,
    )))
}

#[cfg(all(target_os = "linux", target_arch = "x86_64", not(target_env = "musl")))]
pub fn get_exec_name() -> &'static str {
    "yt-dlp_linux"
}

#[cfg(all(target_os = "linux", target_arch = "x86_64", target_env = "musl"))]
pub fn get_exec_name() -> &'static str {
    "yt-dlp_musllinux"
}

#[cfg(all(target_os = "linux", target_arch = "aarch64", not(target_env = "musl")))]
pub fn get_exec_name() -> &'static str {
    "yt-dlp_linux_aarch64"
}

#[cfg(all(target_os = "linux", target_arch = "aarch64", target_env = "musl"))]
pub fn get_exec_name() -> &'static str {
    "yt-dlp_musllinux_aarch64"
}

#[cfg(target_os = "macos")]
pub fn get_exec_name() -> &'static str {
    "yt-dlp_macos"
}

#[cfg(all(target_os = "linux", target_arch = "x86_64", not(target_env = "musl")))]
pub fn get_archive_name() -> &'static str {
    "yt-dlp_linux.zip"
}

#[cfg(all(target_os = "linux", target_arch = "x86_64", target_env = "musl"))]
pub fn get_archive_name() -> &'static str {
    "yt-dlp_musllinux.zip"
}

#[cfg(all(target_os = "linux", target_arch = "aarch64", not(target_env = "musl")))]
pub fn get_archive_name() -> &'static str {
    "yt-dlp_linux_aarch64.zip"
}

#[cfg(all(target_os = "linux", target_arch = "aarch64", target_env = "musl"))]
pub fn get_archive_name() -> &'static str {
    "yt-dlp_musllinux_aarch64.zip"
}

#[cfg(target_os = "macos")]
pub fn get_archive_name() -> &'static str {
    "yt-dlp_macos.zip"
}
