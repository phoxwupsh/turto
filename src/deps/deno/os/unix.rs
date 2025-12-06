pub fn get_exec_name() -> &'static str {
    "deno"
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
pub fn get_archive_name() -> &'static str {
    "deno-x86_64-unknown-linux-gnu.zip"
}

#[cfg(all(target_os = "linux", target_arch = "aarch64"))]
pub fn get_archive_name() -> &'static str {
    "deno-aarch64-unknown-linux-gnu.zip"
}

#[cfg(all(target_os = "macos"))]
pub fn get_archive_name() -> &'static str {
    "deno-aarch64-apple-darwin.zip"
}
