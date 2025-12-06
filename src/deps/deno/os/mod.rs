#[cfg(unix)]
pub mod unix;
#[cfg(unix)]
pub use unix::{get_archive_name, get_exec_name};

#[cfg(windows)]
pub mod windows;
#[cfg(windows)]
pub use windows::{get_archive_name, get_exec_name};
