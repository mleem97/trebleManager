//! Platform-correct application directories.
//!
//! No hardcoded `/home/...` or `C:\Users\...` paths. Resolution order per
//! directory: explicit override parameter → OS-correct user location.
//! The user can relocate everything (documented, no hidden state).

use std::path::PathBuf;

fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .filter(|p| !p.as_os_str().is_empty())
}

/// Config directory: `%APPDATA%/gsi-root` (Windows) or
/// `$XDG_CONFIG_HOME/gsi-root` / `~/.config/gsi-root` (Unix).
pub fn config_dir() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        std::env::var_os("APPDATA")
            .map(|v| PathBuf::from(v).join("gsi-root"))
            .filter(|p| !p.as_os_str().is_empty())
    }
    #[cfg(not(windows))]
    {
        if let Some(x) = std::env::var_os("XDG_CONFIG_HOME") {
            let p = PathBuf::from(x).join("gsi-root");
            if !p.as_os_str().is_empty() {
                return Some(p);
            }
        }
        home_dir().map(|h| h.join(".config").join("gsi-root"))
    }
}

/// Cache directory: `%LOCALAPPDATA%/gsi-root` (Windows) or
/// `$XDG_CACHE_HOME/gsi-root` / `~/.cache/gsi-root` (Unix).
pub fn cache_dir() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        std::env::var_os("LOCALAPPDATA")
            .map(|v| PathBuf::from(v).join("gsi-root"))
            .filter(|p| !p.as_os_str().is_empty())
    }
    #[cfg(not(windows))]
    {
        if let Some(x) = std::env::var_os("XDG_CACHE_HOME") {
            let p = PathBuf::from(x).join("gsi-root");
            if !p.as_os_str().is_empty() {
                return Some(p);
            }
        }
        home_dir().map(|h| h.join(".cache").join("gsi-root"))
    }
}

/// User binary directory for self-install (`install` command).
/// Unix: `~/.local/bin`. Windows: `%LOCALAPPDATA%/gsi-root/bin`.
pub fn user_bin_dir() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        std::env::var_os("LOCALAPPDATA")
            .map(|v| PathBuf::from(v).join("gsi-root").join("bin"))
            .filter(|p| !p.as_os_str().is_empty())
    }
    #[cfg(not(windows))]
    {
        home_dir().map(|h| h.join(".local").join("bin"))
    }
}

/// Resolve a workspace subdirectory (`downloads`, `artifacts`, `logs`,
/// `backups`) under an explicit base dir (pure, testable).
pub fn subdir(base: &std::path::Path, name: &str) -> PathBuf {
    base.join(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subdir_joins() {
        let base = PathBuf::from("/tmp/x");
        assert_eq!(subdir(&base, "downloads"), PathBuf::from("/tmp/x/downloads"));
    }

    #[test]
    fn dirs_resolve_or_none_without_panic() {
        // Must never panic, even with empty environment.
        let _ = config_dir();
        let _ = cache_dir();
        let _ = user_bin_dir();
    }

    #[test]
    fn xdg_override_is_honored() {
        let prev = std::env::var_os("XDG_CONFIG_HOME");
        std::env::set_var("XDG_CONFIG_HOME", "/tmp/gsi-config-test-xdg");
        let got = config_dir();
        match prev {
            Some(v) => std::env::set_var("XDG_CONFIG_HOME", v),
            None => std::env::remove_var("XDG_CONFIG_HOME"),
        }
        assert_eq!(
            got,
            Some(PathBuf::from("/tmp/gsi-config-test-xdg/gsi-root"))
        );
    }
}
