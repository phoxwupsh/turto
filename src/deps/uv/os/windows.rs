use std::path::{Path, PathBuf};

pub fn get_exec_name() -> &'static str {
    "uv.exe"
}

/// Path to the Python interpreter inside a uv-created venv.
pub fn get_venv_python(venv_dir: &Path) -> PathBuf {
    venv_dir.join("Scripts").join("python.exe")
}

/// Path to the interpreter inside a uv-managed CPython install directory
/// (e.g. `<install-dir>/cpython-3.13.14-<triple>/python.exe`).
pub fn get_managed_python_exe(install: &Path) -> PathBuf {
    install.join("python.exe")
}

pub fn get_archive_name() -> &'static str {
    "uv-x86_64-pc-windows-msvc.zip"
}
