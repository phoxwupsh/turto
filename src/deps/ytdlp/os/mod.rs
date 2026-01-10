#[cfg(unix)]
pub mod unix;
#[cfg(unix)]
pub use unix::{get_archive_name, get_local_ytdlp, update_path_ptr};

#[cfg(windows)]
pub mod windows;
#[cfg(windows)]
pub use windows::{get_archive_name, get_local_ytdlp, update_path_ptr};
