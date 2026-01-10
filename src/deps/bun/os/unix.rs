pub fn get_exec_name() -> &'static str {
    "bun"
}

#[cfg(all(target_os = "linux", target_arch = "x86_64", not(target_env = "musl")))]
pub fn get_dir_name() -> &'static str {
    "bun-linux-x64"
}

#[cfg(all(target_os = "linux", target_arch = "x86_64", target_env = "musl"))]
pub fn get_dir_name() -> &'static str {
    "bun-linux-x64-musl"
}

#[cfg(all(target_os = "linux", target_arch = "aarch64", not(target_env = "musl")))]
pub fn get_dir_name() -> &'static str {
    "bun-linux-aarch64"
}

#[cfg(all(target_os = "linux", target_arch = "aarch64", target_env = "musl"))]
pub fn get_dir_name() -> &'static str {
    "bun-linux-aarch64-musl"
}

#[cfg(all(target_os = "macos"))]
pub fn get_dir_name() -> &'static str {
    "bun-darwin-aarch64"
}

#[cfg(all(target_os = "linux", target_arch = "x86_64", not(target_env = "musl")))]
pub fn get_archive_name() -> &'static str {
    "bun-linux-x64.zip"
}

#[cfg(all(target_os = "linux", target_arch = "x86_64", target_env = "musl"))]
pub fn get_archive_name() -> &'static str {
    "bun-linux-x64-musl.zip"
}

#[cfg(all(target_os = "linux", target_arch = "aarch64", not(target_env = "musl")))]
pub fn get_archive_name() -> &'static str {
    "bun-linux-aarch64.zip"
}

#[cfg(all(target_os = "linux", target_arch = "aarch64", target_env = "musl"))]
pub fn get_archive_name() -> &'static str {
    "bun-linux-aarch64-musl.zip"
}

#[cfg(all(target_os = "macos"))]
pub fn get_archive_name() -> &'static str {
    "bun-darwin-aarch64.zip"
}
