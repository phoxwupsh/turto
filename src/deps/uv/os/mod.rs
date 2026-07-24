#[cfg(unix)]
pub mod unix;
#[cfg(unix)]
pub use unix::{get_archive_name, get_exec_name, get_managed_python_exe, get_venv_python};

#[cfg(windows)]
pub mod windows;
#[cfg(windows)]
pub use windows::{get_archive_name, get_exec_name, get_managed_python_exe, get_venv_python};
