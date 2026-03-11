use crate::deps::{DepsError, ytdlp::version::YtdlpVersion};
use std::{
    io::Write,
    path::{Path, PathBuf},
    str::FromStr,
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

pub async fn get_local_ytdlp(
    ytdlp_dir: &Path,
) -> Result<Option<(YtdlpVersion, PathBuf)>, DepsError> {
    let ptr_path = ytdlp_dir.join("current");

    if !ptr_path.try_exists()? {
        return Ok(None);
    }

    let curr_ver_str = tokio::fs::read_to_string(&ptr_path).await?;
    let curr_exec = ytdlp_dir.join(&curr_ver_str).join("yt-dlp.exe");

    let cmd = tokio::process::Command::new(&curr_exec)
        .arg("--version")
        .stdout(std::process::Stdio::piped())
        .spawn()?;
    let output = cmd.wait_with_output().await?;
    let exe_ver_str = String::from_utf8_lossy(&output.stdout);

    let curr_ver = YtDlpVersion::from_str(&curr_ver_str.trim())?;
    let exe_ver = YtDlpVersion::from_str(&exe_ver_str.trim())?;

    if expect != exec_ver {
        return Err(crate::deps::DepsError::YtdlpVersionMismatch {
            expect: curr_ver,
            actual: exe_ver,
        });
    }

    return Ok(Some((curr_ver, curr_exec)));

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
