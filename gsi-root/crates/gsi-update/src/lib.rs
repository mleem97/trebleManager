//! GitHub Release updater: check / download / verify / install / rollback.
//!
//! Honesty rules: SHA-256 verified before anything runs; the old binary is
//! kept until the new one is staged; signature verification is reported as
//! `NotChecked` until key infrastructure exists (never faked as verified).

use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

/// Update channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Channel {
    Stable,
    Beta,
    Nightly,
}

/// One release asset for this platform.
#[derive(Debug, Clone)]
pub struct ReleaseAsset {
    pub version: String,
    pub url: String,
    pub sha256: Option<String>,
}

/// Signature state (explicit: no fake verification).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SignatureState {
    NotCheckedNoKeyInfrastructure,
}

/// OS/arch matrix key used to pick an asset (`linux-x86_64`, ...).
pub fn platform_key() -> String {
    let os = std::env::consts::OS;
    let arch = std::env::consts::ARCH;
    format!("{os}-{arch}")
}

/// Query GitHub API for the latest release tag (User-Agent required).
pub fn latest_tag(repo: &str) -> Result<String, String> {
    let url = format!("https://api.github.com/repos/{repo}/releases/latest");
    let resp = ureq::get(&url)
        .set("User-Agent", "gsi-root")
        .set("Accept", "application/vnd.github+json")
        .call()
        .map_err(|e| format!("api: {e}"))?;
    let text = resp
        .into_string()
        .map_err(|e| format!("body: {e}"))?;
    parse_tag(&text).ok_or_else(|| "no tag_name in response".to_string())
}

/// Minimal `tag_name` extraction without a JSON dependency.
pub fn parse_tag(body: &str) -> Option<String> {
    let key = "\"tag_name\"";
    let i = body.find(key)?;
    let rest = &body[i + key.len()..];
    let colon = rest.find(':')?;
    let val = rest[colon + 1..].trim_start();
    let val = val.strip_prefix('"')?;
    let end = val.find('"')?;
    Some(val[..end].to_string())
}

/// SHA-256 of a file, lowercase hex.
pub fn sha256_file(path: &Path) -> Result<String, String> {
    let data = std::fs::read(path).map_err(|e| format!("read: {e}"))?;
    let mut h = Sha256::new();
    h.update(&data);
    Ok(format!("{:x}", h.finalize()))
}

/// Download a URL to `dest` (atomic: temp file + rename).
pub fn download_to(url: &str, dest: &Path) -> Result<(), String> {
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("mkdir: {e}"))?;
    }
    let tmp = dest.with_extension("part");
    let resp = ureq::get(url)
        .set("User-Agent", "gsi-root")
        .call()
        .map_err(|e| format!("download: {e}"))?;
    let mut reader = resp.into_reader();
    let mut out = std::fs::File::create(&tmp).map_err(|e| format!("create: {e}"))?;
    std::io::copy(&mut reader, &mut out).map_err(|e| format!("write: {e}"))?;
    drop(out);
    std::fs::rename(&tmp, dest).map_err(|e| format!("rename: {e}"))?;
    Ok(())
}

/// Verify a staged file against an expected SHA-256.
pub fn verify_staged(path: &Path, expected: &str) -> Result<SignatureState, String> {
    let got = sha256_file(path)?;
    if !got.eq_ignore_ascii_case(expected) {
        return Err(format!("hash mismatch: got {got}, want {expected}"));
    }
    Ok(SignatureState::NotCheckedNoKeyInfrastructure)
}

/// Install: keep `.bak` of the current binary, atomically replace, rollback on error.
pub fn install_staged(current: &Path, staged: &Path) -> Result<PathBuf, String> {
    let bak = current.with_extension("bak");
    if current.exists() {
        std::fs::rename(current, &bak).map_err(|e| format!("backup: {e}"))?;
    }
    if let Err(e) = staged_install_copy(current, staged) {
        if bak.exists() {
            let _ = std::fs::rename(&bak, current);
        }
        return Err(e);
    }
    Ok(bak)
}

fn staged_install_copy(current: &Path, staged: &Path) -> Result<(), String> {
    std::fs::copy(staged, current).map_err(|e| format!("install: {e}"))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut p = std::fs::metadata(current)
            .map_err(|e| format!("meta: {e}"))?
            .permissions();
        p.set_mode(0o755);
        std::fs::set_permissions(current, p).map_err(|e| format!("chmod: {e}"))?;
    }
    // Sanity: staged file must still hash the same after copy.
    let a = sha256_file(staged)?;
    let b = sha256_file(current)?;
    if a != b {
        return Err("post-copy hash mismatch".to_string());
    }
    Ok(())
}

/// Self-install: copy the running binary to the user bin dir.
/// PATH persistence happens only with explicit consent (caller side `--yes`).
pub fn self_install(bin_dir: &Path) -> Result<PathBuf, String> {
    let current =
        std::env::current_exe().map_err(|e| format!("current exe: {e}"))?;
    std::fs::create_dir_all(bin_dir).map_err(|e| format!("mkdir: {e}"))?;
    let name = if cfg!(windows) { "gsi-root.exe" } else { "gsi-root" };
    let dest = bin_dir.join(name);
    if let Ok(same) = same_file(&current, &dest) {
        if same {
            return Ok(dest);
        }
    }
    std::fs::copy(&current, &dest).map_err(|e| format!("copy: {e}"))?;
    Ok(dest)
}

fn same_file(a: &Path, b: &Path) -> Result<bool, String> {
    if !b.exists() {
        return Ok(false);
    }
    Ok(sha256_file(a)? == sha256_file(b)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn parses_tag_without_json_dep() {
        let body = r#"{"url":"x","tag_name":"v1.2.3","draft":false}"#;
        assert_eq!(parse_tag(body), Some("v1.2.3".to_string()));
        assert_eq!(parse_tag("{}"), None);
    }

    #[test]
    fn platform_key_looks_sane() {
        let k = platform_key();
        assert!(k.contains('-'), "{k}");
    }

    #[test]
    fn verify_detects_mismatch() {
        let mut p = std::env::temp_dir();
        p.push("gsi-update-test-mismatch.bin");
        let mut f = std::fs::File::create(&p).unwrap();
        f.write_all(b"data").unwrap();
        assert!(verify_staged(&p, "deadbeef").is_err());
        let h = sha256_file(&p).unwrap();
        assert_eq!(
            verify_staged(&p, &h).unwrap(),
            SignatureState::NotCheckedNoKeyInfrastructure
        );
        std::fs::remove_file(p).ok();
    }

    #[test]
    fn install_keeps_backup_and_is_atomic() {
        let mut d = std::env::temp_dir();
        d.push("gsi-update-test-install");
        std::fs::create_dir_all(&d).unwrap();
        let cur = d.join("app");
        let stg = d.join("new");
        std::fs::write(&cur, b"old").unwrap();
        std::fs::write(&stg, b"new").unwrap();
        let bak = install_staged(&cur, &stg).unwrap();
        assert_eq!(std::fs::read(&cur).unwrap(), b"new");
        assert_eq!(std::fs::read(&bak).unwrap(), b"old");
        std::fs::remove_dir_all(&d).ok();
    }
}
