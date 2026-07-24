use std::path::{Path, PathBuf};

pub fn get_exec_name() -> &'static str {
    "uv"
}

/// Path to the Python interpreter inside a uv-created venv.
pub fn get_venv_python(venv_dir: &Path) -> PathBuf {
    venv_dir.join("bin").join("python")
}

/// Path to the interpreter inside a uv-managed CPython install directory
/// (e.g. `<install-dir>/cpython-3.13.14-<triple>/bin/python3`).
pub fn get_managed_python_exe(install: &Path) -> PathBuf {
    install.join("bin").join("python3")
}

#[cfg(all(target_os = "linux", target_arch = "x86_64", not(target_env = "musl")))]
pub fn get_archive_name() -> &'static str {
    "uv-x86_64-unknown-linux-gnu.tar.gz"
}

#[cfg(all(target_os = "linux", target_arch = "x86_64", target_env = "musl"))]
pub fn get_archive_name() -> &'static str {
    "uv-x86_64-unknown-linux-musl.tar.gz"
}

#[cfg(all(target_os = "linux", target_arch = "aarch64", not(target_env = "musl")))]
pub fn get_archive_name() -> &'static str {
    "uv-aarch64-unknown-linux-gnu.tar.gz"
}

#[cfg(all(target_os = "linux", target_arch = "aarch64", target_env = "musl"))]
pub fn get_archive_name() -> &'static str {
    "uv-aarch64-unknown-linux-musl.tar.gz"
}

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
pub fn get_archive_name() -> &'static str {
    "uv-aarch64-apple-darwin.tar.gz"
}

#[cfg(all(target_os = "macos", target_arch = "x86_64"))]
pub fn get_archive_name() -> &'static str {
    "uv-x86_64-apple-darwin.tar.gz"
}
