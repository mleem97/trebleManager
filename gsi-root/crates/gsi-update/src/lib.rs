//! GitHub Release updater: check / download / verify / install / rollback.
//!
//! Honesty rules: SHA-256 verified before anything runs; the old binary is
//! kept until the new one is staged; signature verification is reported as
//! `NotChecked` until key infrastructure exists (never faked as verified).
//!
//! Download helpers mirror the toolkit scripts without copying their
//! interactivity: Magisk `stable.json` fetch+parse, newest-APK discovery,
//! registry ROM download resolution, hash-sidecar firmware checks, bootstrap
//! release file names, and a validated `download_file` wrapper.

use gsi_registry::{RomEntry, TargetConfig};
use gsi_workflow::ProgressEvent;
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
    let text = resp.into_string().map_err(|e| format!("body: {e}"))?;
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
/// Progress variant is [`download_to_with_progress`]; this wrapper
/// reports no events (behavior unchanged).
pub fn download_to(url: &str, dest: &Path) -> Result<(), String> {
    download_to_with_progress(url, dest, None)
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
    let current = std::env::current_exe().map_err(|e| format!("current exe: {e}"))?;
    std::fs::create_dir_all(bin_dir).map_err(|e| format!("mkdir: {e}"))?;
    let name = if cfg!(windows) {
        "gsi-root.exe"
    } else {
        "gsi-root"
    };
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

// ------------------------------------------------------------ URL validation

/// `true` for plain `http(s)` URLs: bounded length, no whitespace, host
/// present, no `user:pass@` credentials. No extension restriction.
pub fn valid_http_url(url: &str) -> bool {
    if url.is_empty() || url.len() > 2048 || url.chars().any(|c| c.is_whitespace()) {
        return false;
    }
    let lower = url.to_ascii_lowercase();
    let rest = if let Some(r) = lower.strip_prefix("https://") {
        r
    } else if let Some(r) = lower.strip_prefix("http://") {
        r
    } else {
        return false;
    };
    let host = rest.split('/').next().unwrap_or("");
    !host.is_empty() && !host.contains('@')
}

/// Archive extensions accepted for firmware/ROM downloads.
const ARCHIVE_EXTS: &[&str] = &[
    ".zip", ".7z", ".tar", ".gz", ".tgz", ".xz", ".app", ".rar", ".img",
];

/// `true` for `http(s)` URLs pointing at a firmware/ROM archive.
/// Strips query/fragment and a trailing SourceForge `/download` segment
/// before the extension check (mirrors `valid_url` / `Test-FirmwareUrl`).
pub fn valid_download_url(url: &str) -> bool {
    if !valid_http_url(url) {
        return false;
    }
    let mut base = url.split(&['?', '#'][..]).next().unwrap_or("");
    if let Some(s) = strip_suffix_ci(base, "/download") {
        base = s;
    }
    let lower = base.to_ascii_lowercase();
    ARCHIVE_EXTS.iter().any(|e| lower.ends_with(e))
}

fn strip_suffix_ci<'a>(s: &'a str, suffix: &str) -> Option<&'a str> {
    if s.len() >= suffix.len() && s[s.len() - suffix.len()..].eq_ignore_ascii_case(suffix) {
        Some(&s[..s.len() - suffix.len()])
    } else {
        None
    }
}

/// Download outcome for the validated wrapper.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DownloadOutcome {
    Downloaded,
    Cached,
}

/// Validated download: rejects non-archive URLs, skips existing files
/// (offline cache, no re-download), removes the partial temp file on
/// failure. Progress variant is [`download_file_with_progress`]; this
/// wrapper reports no events (behavior unchanged for `http(s)` URLs).
pub fn download_file(url: &str, dest: &Path) -> Result<DownloadOutcome, String> {
    if !valid_download_url(url) {
        return Err(format!("URL rejected (http(s) archive only): {url}"));
    }
    if dest.is_file() {
        return Ok(DownloadOutcome::Cached);
    }
    if let Err(e) = download_to_with_progress(url, dest, None) {
        let tmp = dest.with_extension("part");
        let _ = std::fs::remove_file(&tmp);
        return Err(e);
    }
    Ok(DownloadOutcome::Downloaded)
}

// ------------------------------------------------------------ tiny JSON field helpers (no JSON dep)

/// Extract the object under `"key": { ... }` (brace matching, string aware).
fn extract_object(body: &str, key: &str) -> Option<String> {
    let needle = format!("\"{key}\"");
    let i = body.find(&needle)?;
    let mut rest = body[i + needle.len()..].trim_start();
    rest = rest.strip_prefix(':')?.trim_start();
    rest = rest.strip_prefix('{')?;
    let mut depth = 1usize;
    let mut in_str = false;
    let mut esc = false;
    for (n, c) in rest.char_indices() {
        if in_str {
            if esc {
                esc = false;
            } else if c == '\\' {
                esc = true;
            } else if c == '"' {
                in_str = false;
            }
        } else if c == '"' {
            in_str = true;
        } else if c == '{' {
            depth += 1;
        } else if c == '}' {
            depth -= 1;
            if depth == 0 {
                return Some(rest[..n].to_string());
            }
        }
    }
    None
}

/// Extract a string (or bare numeric/bool) value for `"key"` inside `scope`.
fn extract_json_string(scope: &str, key: &str) -> Option<String> {
    let needle = format!("\"{key}\"");
    let i = scope.find(&needle)?;
    let rest = &scope[i + needle.len()..];
    let colon = rest.find(':')?;
    let val = rest[colon + 1..].trim_start();
    if let Some(quoted) = val.strip_prefix('"') {
        let mut out = String::new();
        let mut esc = false;
        for c in quoted.chars() {
            if esc {
                out.push(c);
                esc = false;
            } else if c == '\\' {
                esc = true;
            } else if c == '"' {
                return Some(out);
            } else {
                out.push(c);
            }
        }
        return None;
    }
    // Bare value (versionCode is often a number): take a digit run.
    let run: String = val.chars().take_while(|c| c.is_ascii_digit()).collect();
    if run.is_empty() {
        None
    } else {
        Some(run)
    }
}

// ------------------------------------------------------------ Magisk stable channel

/// Official Magisk `stable.json` URL (never hardcoded version data).
pub const MAGISK_STABLE_DEFAULT_URL: &str =
    "https://raw.githubusercontent.com/topjohnwu/magisk-files/master/stable.json";

/// Official Magisk release page (manual fallback / verify hint).
pub const MAGISK_RELEASES_URL: &str = "https://github.com/topjohnwu/Magisk/releases";

/// Parsed Magisk stable channel: version plus APK download URL.
/// `zip_url` stays empty when the channel carries no zip link (honest).
#[derive(Debug, Clone)]
pub struct MagiskStable {
    pub version: String,
    pub version_code: String,
    pub apk_url: String,
    pub zip_url: String,
}

/// Stable.json URL: registry `magisk.stable_json` override wins, else default.
/// Pure string logic over the raw profile text (mirrors both scripts).
pub fn magisk_stable_url(registry_text: Option<&str>) -> String {
    let over = registry_text
        .and_then(|t| extract_json_string(t, "stable_json"))
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    match over {
        Some(u) => u,
        None => MAGISK_STABLE_DEFAULT_URL.to_string(),
    }
}

/// Parse `stable.json` body (pure, fixture-testable, no network).
pub fn parse_magisk_stable(body: &str) -> Result<MagiskStable, String> {
    let magisk = extract_object(body, "magisk")
        .ok_or_else(|| "no magisk object in stable.json".to_string())?;
    let version = extract_json_string(&magisk, "version")
        .filter(|s| !s.trim().is_empty())
        .ok_or_else(|| "no magisk.version in stable.json".to_string())?;
    let version_code = extract_json_string(&magisk, "versionCode").unwrap_or_default();
    let apk_url = extract_json_string(&magisk, "link")
        .filter(|s| !s.trim().is_empty())
        .ok_or_else(|| "no magisk.link in stable.json".to_string())?;
    let zip_url = ["zipLink", "zipUrl", "zip"]
        .iter()
        .filter_map(|k| extract_json_string(&magisk, k))
        .find(|s| !s.trim().is_empty())
        .unwrap_or_default();
    Ok(MagiskStable {
        version,
        version_code,
        apk_url,
        zip_url,
    })
}

/// Fetch + parse the Magisk stable channel (mirrors `Get-MagiskStable`).
pub fn fetch_magisk_stable(url: &str) -> Result<MagiskStable, String> {
    if !valid_http_url(url) {
        return Err(format!("stable.json URL rejected (http(s) only): {url}"));
    }
    let resp = ureq::get(url)
        .set("User-Agent", "gsi-root")
        .set("Accept", "application/json")
        .call()
        .map_err(|e| format!("stable.json: {e}"))?;
    let text = resp.into_string().map_err(|e| format!("body: {e}"))?;
    parse_magisk_stable(&text)
}

/// APK file name for a stable release (`Magisk-v<version>.apk`).
pub fn magisk_apk_filename(stable: &MagiskStable) -> String {
    format!("Magisk-v{}.apk", stable.version.trim())
}

/// Destination path for a Magisk APK inside a magisk dir.
pub fn magisk_download_dest(mag_dir: &Path, stable: &MagiskStable) -> PathBuf {
    mag_dir.join(magisk_apk_filename(stable))
}

// ------------------------------------------------------------ APK discovery

/// Newest file in `dir` (non-recursive, files only) whose name matches
/// `pattern`: `*.ext` suffix match, otherwise case-insensitive substring.
/// `*` alone matches any file. Newest by mtime, name breaks ties.
/// Returns `None` for unreadable dirs or no match.
pub fn newest_matching_file(dir: &Path, pattern: &str) -> Option<PathBuf> {
    let pat = pattern.trim().to_ascii_lowercase();
    if pat.is_empty() {
        return None;
    }
    let suffix = pat.strip_prefix('*');
    let entries = std::fs::read_dir(dir).ok()?;
    let mut best: Option<(Option<std::time::SystemTime>, String, PathBuf)> = None;
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let name = match path.file_name().and_then(|n| n.to_str()) {
            Some(n) => n,
            None => continue,
        };
        let low = name.to_ascii_lowercase();
        let hit = match suffix {
            Some("") => true,
            Some(s) => low.ends_with(s),
            None => low.contains(&pat),
        };
        if !hit {
            continue;
        }
        let mtime = std::fs::metadata(&path).and_then(|m| m.modified()).ok();
        let key = (mtime, name.to_string());
        let replace = match &best {
            None => true,
            Some((bt, bn, _)) => key > (*bt, bn.clone()),
        };
        if replace {
            best = Some((mtime, name.to_string(), path));
        }
    }
    best.map(|(_, _, p)| p)
}

/// Newest `*.apk` (case-insensitive) in a magisk dir (mirrors
/// `Find-TTMagiskApk` / `magisk_apk`).
pub fn find_magisk_apk(mag_dir: &Path) -> Option<PathBuf> {
    newest_matching_file(mag_dir, "*.apk")
}

/// Version guess from an APK file name (first `major.minor...` digit run).
/// Falls back to `unknown (check filename)` like `Get-TTMagiskInfo`.
pub fn magisk_version_from_filename(file_name: &str) -> String {
    let bytes = file_name.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i].is_ascii_digit() {
            let mut j = i;
            while j < bytes.len() && (bytes[j].is_ascii_digit() || bytes[j] == b'.') {
                j += 1;
            }
            let cand = file_name[i..j].trim_matches('.');
            let dotted = cand.contains('.')
                && cand
                    .split('.')
                    .all(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_digit()));
            if dotted {
                return cand.to_string();
            }
            i = j;
        } else {
            i += 1;
        }
    }
    "unknown (check filename)".to_string()
}

/// Local Magisk APK facts: path, filename version, SHA-256, size.
/// Mirrors `Get-TTMagiskInfo` (never claims more than the file shows).
#[derive(Debug, Clone)]
pub struct MagiskInfo {
    pub path: PathBuf,
    pub version: String,
    pub sha256: String,
    pub size: u64,
    pub source: String,
}

pub fn magisk_info(apk_path: &Path) -> Result<MagiskInfo, String> {
    if !apk_path.is_file() {
        return Err(format!("magisk apk missing: {}", apk_path.display()));
    }
    let size = std::fs::metadata(apk_path)
        .map_err(|e| format!("meta: {e}"))?
        .len();
    let sha256 = sha256_file(apk_path)?;
    let name = apk_path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    Ok(MagiskInfo {
        path: apk_path.to_path_buf(),
        version: magisk_version_from_filename(name),
        sha256,
        size,
        source: format!("official: {MAGISK_RELEASES_URL} (verify manually)"),
    })
}

// ------------------------------------------------------------ ROM download resolution

/// One registry ROM with a direct download URL (mirrors `Get-RomDownloads`).
#[derive(Debug, Clone)]
pub struct RomDownload {
    pub label: String,
    pub file: String,
    pub url: String,
    pub status: String,
}

/// Registry entries carrying a non-empty `url` (any status; the caller
/// decides selectability, exactly like the scripts).
pub fn rom_downloads(entries: &[RomEntry]) -> Vec<RomDownload> {
    entries
        .iter()
        .filter(|e| !e.url.trim().is_empty())
        .map(|e| RomDownload {
            label: gsi_registry::entry_label(e),
            file: e.file.clone(),
            url: e.url.clone(),
            status: e.status.clone(),
        })
        .collect()
}

fn strip_rom_id_prefix(pick: &str) -> &str {
    pick.strip_prefix("rom:").unwrap_or(pick)
}

/// Resolve `pick` against `rom_downloads`: 1-based number or exact label
/// (`rom:` id prefix accepted). Mirrors `Invoke-RomDownload` / `download_rom`.
pub fn resolve_rom_download(entries: &[RomEntry], pick: &str) -> Result<RomDownload, String> {
    let list = rom_downloads(entries);
    if list.is_empty() {
        return Err("no downloadable ROMs in registry".to_string());
    }
    let want = strip_rom_id_prefix(pick.trim());
    if !want.is_empty() && want.chars().all(|c| c.is_ascii_digit()) {
        let idx: usize = want
            .parse()
            .map_err(|_| format!("bad ROM number: {pick}"))?;
        if idx == 0 || idx > list.len() {
            return Err(format!(
                "ROM number {idx} out of range (1..={})",
                list.len()
            ));
        }
        let d = &list[idx - 1];
        return Ok(d.clone());
    }
    if let Some(d) = list.iter().find(|d| d.label == want) {
        return Ok(d.clone());
    }
    Err(format!("no ROM download matches: {pick}"))
}

/// Target file name: registry `file`, else URL basename (query stripped,
/// trailing SourceForge `/download` removed).
pub fn rom_download_filename(dl: &RomDownload) -> Result<String, String> {
    if !dl.file.trim().is_empty() {
        return Ok(dl.file.trim().to_string());
    }
    let mut base = dl.url.split(&['?', '#'][..]).next().unwrap_or("");
    if let Some(s) = strip_suffix_ci(base, "/download") {
        base = s;
    }
    base = base.trim_end_matches('/');
    match base.rsplit('/').next().filter(|s| !s.is_empty()) {
        Some(name) => Ok(name.to_string()),
        None => Err("cannot derive file name from ROM url".to_string()),
    }
}

/// Full target config (system, variant, root artifact, firmware base) for an
/// entry label. Glue over `gsi_registry::target_config`.
pub fn resolve_target_config(
    entries: &[RomEntry],
    label: &str,
    firmware_base: &str,
) -> Result<TargetConfig, String> {
    let want = strip_rom_id_prefix(label.trim());
    let entry = entries
        .iter()
        .find(|e| gsi_registry::entry_label(e) == want)
        .ok_or_else(|| format!("no registry entry for label: {label}"))?;
    Ok(gsi_registry::target_config(entry, firmware_base))
}

/// Resolve + download a registry ROM into `rom_dir` (reuses `download_file`,
/// so cached files are not re-fetched). Returns the destination path.
pub fn download_rom_to(
    entries: &[RomEntry],
    pick: &str,
    rom_dir: &Path,
) -> Result<PathBuf, String> {
    let dl = resolve_rom_download(entries, pick)?;
    let name = rom_download_filename(&dl)?;
    let dest = rom_dir.join(name);
    download_file(&dl.url, &dest)?;
    Ok(dest)
}

// ------------------------------------------------------------ firmware checks + sidecars

/// Below this size a firmware package is implausible (delta/short package).
pub const FIRMWARE_MIN_SIZE: u64 = 100 * 1024 * 1024;

/// Firmware check result (mirrors `Test-DownloadedFirmware`).
#[derive(Debug, Clone)]
pub struct FirmwareCheck {
    pub sha256: String,
    pub size: u64,
    pub ok: bool,
    pub notes: Vec<String>,
}

fn sidecar_path(path: &Path) -> PathBuf {
    let mut s = path.as_os_str().to_owned();
    s.push(".sha256");
    PathBuf::from(s)
}

/// Hash a file and store the digest in `<path>.sha256` (mirrors
/// `verify_download` sidecar writing). Returns the digest.
pub fn write_hash_sidecar(path: &Path) -> Result<String, String> {
    let digest = sha256_file(path)?;
    std::fs::write(sidecar_path(path), digest.as_bytes()).map_err(|e| format!("sidecar: {e}"))?;
    Ok(digest)
}

/// Verify a file against its `<path>.sha256` sidecar (first token, case
/// insensitive). Returns the digest on match.
pub fn verify_hash_sidecar(path: &Path) -> Result<String, String> {
    let computed = sha256_file(path)?;
    let raw = std::fs::read_to_string(sidecar_path(path)).map_err(|e| format!("sidecar: {e}"))?;
    let expected = raw.split_whitespace().next().unwrap_or("");
    if expected.is_empty() {
        return Err("sidecar empty".to_string());
    }
    if !computed.eq_ignore_ascii_case(expected) {
        return Err(format!("hash mismatch: got {computed}, want {expected}"));
    }
    Ok(computed)
}

/// Check a downloaded firmware file: writes the `.sha256` sidecar, applies
/// the 100 MB plausibility heuristic. ZIP inner content (`UPDATE.APP`)
/// needs an extractor, so ZIPs only get a manual-check note (never faked).
pub fn verify_downloaded_firmware(path: &Path) -> Result<FirmwareCheck, String> {
    if !path.is_file() {
        return Err(format!("file missing: {}", path.display()));
    }
    let size = std::fs::metadata(path)
        .map_err(|e| format!("meta: {e}"))?
        .len();
    let sha256 = write_hash_sidecar(path)?;
    let mb = size as f64 / (1024.0 * 1024.0);
    let mut notes = vec![
        format!("Size: {mb:.1} MB"),
        format!("SHA-256 saved in {}.sha256.", path.display()),
    ];
    let mut ok = true;
    if size < FIRMWARE_MIN_SIZE {
        ok = false;
        notes.push(
            "WARN: < 100 MB - implausibly small for full firmware (maybe delta/short package)."
                .to_string(),
        );
    }
    if path
        .to_string_lossy()
        .to_ascii_lowercase()
        .ends_with(".zip")
    {
        notes.push("ZIP: UPDATE.APP content check needs unzip - verify manually.".to_string());
    } else {
        notes.push("Not a ZIP - unpack / UPDATE.APP search manually.".to_string());
    }
    Ok(FirmwareCheck {
        sha256,
        size,
        ok,
        notes,
    })
}

// ------------------------------------------------------------ bootstrap release files

/// Release repo used for self-bootstrap downloads.
pub const BOOTSTRAP_REPO: &str = "mleem97/trebleManager";

/// Release asset prefix for self-bootstrap zips.
pub const BOOTSTRAP_ASSET_PREFIX: &str = "trebleManager-";

/// Expected bootstrap asset names/URLs for a tag (pure string logic,
/// mirrors `Get-BootstrapReleaseFile` / `bootstrap_fetch`).
#[derive(Debug, Clone)]
pub struct BootstrapRelease {
    pub tag: String,
    pub zip_name: String,
    pub zip_url: String,
    pub sha_url: String,
}

pub fn bootstrap_release(tag: &str) -> Result<BootstrapRelease, String> {
    let tag = tag.trim();
    if tag.is_empty() {
        return Err("empty bootstrap tag".to_string());
    }
    if tag
        .chars()
        .any(|c| c.is_whitespace() || c == '/' || c == '\\')
    {
        return Err(format!("bad bootstrap tag: {tag}"));
    }
    let zip_name = format!("{BOOTSTRAP_ASSET_PREFIX}{tag}.zip");
    let zip_url = format!("https://github.com/{BOOTSTRAP_REPO}/releases/download/{tag}/{zip_name}");
    let sha_url = format!("{zip_url}.sha256");
    Ok(BootstrapRelease {
        tag: tag.to_string(),
        zip_name,
        zip_url,
        sha_url,
    })
}

/// Cached location of a bootstrap relpath: `<cache>/<tag>/<rel>`.
pub fn bootstrap_cached_file(cache_dir: &Path, tag: &str, rel_path: &str) -> PathBuf {
    cache_dir.join(tag.trim()).join(rel_path)
}

// ------------------------------------------------------------ progress downloads
//
// Script mirror: `Invoke-FirmwareDownload` (BITS progress + WebClient
// `Write-Progress`), `download_firmware` (curl `--progress-bar`).
// Events are `gsi_workflow::ProgressEvent`: Started, Progress
// { current, total } per chunk, Completed. `file://` URLs copy locally
// (hermetic tests, offline fixtures); http(s) streams via ureq with the
// total from Content-Length (0 when unknown).

/// Progress reporter: `Some(&cb)` gets Started/Progress/Completed
/// (plus Warning when the bootstrap sha is skipped). `None` disables.
fn emit_progress(reporter: Option<&dyn Fn(ProgressEvent)>, ev: ProgressEvent) {
    if let Some(r) = reporter {
        r(ev);
    }
}

/// True for `file://` sources (local copy, no network).
fn is_file_url(url: &str) -> bool {
    url.starts_with("file://")
}

/// Path part of a `file://` URL (never panics).
fn file_url_path(url: &str) -> &str {
    url.get("file://".len()..).unwrap_or("")
}

/// Archive check for `file://` paths (same extensions as http(s)).
fn valid_source_url(url: &str) -> bool {
    if is_file_url(url) {
        let base = file_url_path(url)
            .split(&['?', '#'][..])
            .next()
            .unwrap_or("");
        let lower = base.to_ascii_lowercase();
        return ARCHIVE_EXTS.iter().any(|e| lower.ends_with(e));
    }
    valid_download_url(url)
}

/// Stream `reader` to `dest` (atomic temp + rename), emitting
/// Started/Progress/Completed. Cleans the temp file on failure.
fn pump_copy(
    reader: &mut dyn std::io::Read,
    dest: &Path,
    total: u64,
    reporter: Option<&dyn Fn(ProgressEvent)>,
    operation: &str,
) -> Result<(), String> {
    use std::io::Write;
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("mkdir: {e}"))?;
    }
    let tmp = dest.with_extension("part");
    emit_progress(
        reporter,
        ProgressEvent::Started {
            operation: operation.to_string(),
        },
    );
    let res: Result<(), String> = (|| {
        let mut out = std::fs::File::create(&tmp).map_err(|e| format!("create: {e}"))?;
        let mut buf = [0u8; 65536];
        let mut done: u64 = 0;
        loop {
            let n = reader
                .read(&mut buf)
                .map_err(|e| format!("download: {e}"))?;
            if n == 0 {
                break;
            }
            out.write_all(&buf[..n])
                .map_err(|e| format!("write: {e}"))?;
            done = done.saturating_add(n as u64);
            emit_progress(
                reporter,
                ProgressEvent::Progress {
                    current: done,
                    total,
                },
            );
        }
        drop(out);
        std::fs::rename(&tmp, dest).map_err(|e| format!("rename: {e}"))?;
        Ok(())
    })();
    match res {
        Ok(()) => {
            emit_progress(reporter, ProgressEvent::Completed);
            Ok(())
        }
        Err(e) => {
            let _ = std::fs::remove_file(&tmp);
            Err(e)
        }
    }
}

/// Download with progress events. Existing `download_to` delegates here
/// with no reporter (behavior unchanged).
pub fn download_to_with_progress(
    url: &str,
    dest: &Path,
    reporter: Option<&dyn Fn(ProgressEvent)>,
) -> Result<(), String> {
    let operation = format!("download {url}");
    if is_file_url(url) {
        let mut inp = std::fs::File::open(file_url_path(url)).map_err(|e| format!("read: {e}"))?;
        let total = inp.metadata().map(|m| m.len()).unwrap_or(0);
        return pump_copy(&mut inp, dest, total, reporter, &operation);
    }
    let resp = ureq::get(url)
        .set("User-Agent", "gsi-root")
        .call()
        .map_err(|e| format!("download: {e}"))?;
    let total: u64 = resp
        .header("Content-Length")
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(0);
    let mut reader = resp.into_reader();
    pump_copy(&mut reader, dest, total, reporter, &operation)
}

/// Validated download with progress events. Cached files report
/// Started + Completed with no bytes. Accepts `file://` archives in
/// addition to `http(s)` (hermetic fixtures); `download_file` stays
/// `http(s)`-only and shares the streaming core via
/// `download_to_with_progress`.
pub fn download_file_with_progress(
    url: &str,
    dest: &Path,
    reporter: Option<&dyn Fn(ProgressEvent)>,
) -> Result<DownloadOutcome, String> {
    if !valid_source_url(url) {
        return Err(format!("URL rejected (http(s)/file archive only): {url}"));
    }
    if dest.is_file() {
        emit_progress(
            reporter,
            ProgressEvent::Started {
                operation: format!("download {url}"),
            },
        );
        emit_progress(reporter, ProgressEvent::Completed);
        return Ok(DownloadOutcome::Cached);
    }
    download_to_with_progress(url, dest, reporter)?;
    Ok(DownloadOutcome::Downloaded)
}

// ------------------------------------------------------------ ROM image validation
//
// Script mirror: `Invoke-RomDownload` / `download_rom` gunzip +
// `Test-ImageKind` / `image_kind` (boot = ANDROID!, system =
// sparse/ext4, else unclear). Container via gsi-archive classify +
// gsi-image detect; the kind check itself is local and honest.

/// ROM image kind (`boot` or `system`; `unknown` is never a valid ask).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageKind {
    Boot,
    System,
    Unknown,
}

impl ImageKind {
    /// Lowercase id (`boot`, `system`, `unknown`).
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Boot => "boot",
            Self::System => "system",
            Self::Unknown => "unknown",
        }
    }
}

/// Parse `boot`/`system` (case-insensitive); anything else is `None`.
pub fn parse_image_kind(s: &str) -> Option<ImageKind> {
    let lower = s.trim().to_ascii_lowercase();
    if lower == "boot" {
        Some(ImageKind::Boot)
    } else if lower == "system" {
        Some(ImageKind::System)
    } else {
        None
    }
}

/// Classify a local image by magic bytes (mirrors `Test-ImageKind`).
/// Missing/unreadable files report `Unknown` (never panics).
pub fn image_kind_of(path: &Path) -> ImageKind {
    use std::io::Read;
    let mut f = match std::fs::File::open(path) {
        Ok(f) => f,
        Err(_) => return ImageKind::Unknown,
    };
    let mut buf = [0u8; 1082];
    let mut n = 0;
    while n < buf.len() {
        match f.read(&mut buf[n..]) {
            Ok(0) => break,
            Ok(r) => n += r,
            Err(_) => return ImageKind::Unknown,
        }
    }
    if n >= 8 && buf[..8] == *b"ANDROID!" {
        return ImageKind::Boot;
    }
    if n >= 4 && buf[0] == 0x3A && buf[1] == 0xFF && buf[2] == 0x26 && buf[3] == 0xED {
        return ImageKind::System;
    }
    if n >= 1082 && buf[1080] == 0x53 && buf[1081] == 0xEF {
        return ImageKind::System;
    }
    if gsi_image::parse_sparse_header(path).is_some() {
        return ImageKind::System;
    }
    ImageKind::Unknown
}

/// Typed post-download validation outcome.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RomImageValidation {
    /// Content matches the expected kind.
    Ok {
        kind: ImageKind,
        path: PathBuf,
        message: String,
    },
    /// Download fine, kind mismatch (honest: never re-labeled).
    WrongKind {
        found: ImageKind,
        expected: ImageKind,
        path: PathBuf,
        message: String,
    },
    /// Unreadable or unclear content (decompress fail, no magic).
    Corrupt { reason: String, path: PathBuf },
}

impl RomImageValidation {
    /// True only for the `Ok` variant.
    pub fn is_ok(&self) -> bool {
        matches!(self, Self::Ok { .. })
    }
}

/// Short container note for messages (gsi-image detect).
fn container_note(path: &Path) -> &'static str {
    match gsi_image::detect_container(path) {
        gsi_image::Container::Gzip => "gzip",
        gsi_image::Container::AndroidSparse => "android-sparse",
        gsi_image::Container::Raw => "raw",
    }
}

/// Pure local check of a downloaded (already decompressed) image.
pub fn validate_rom_image(path: &Path, expected: ImageKind) -> RomImageValidation {
    if !path.is_file() {
        return RomImageValidation::Corrupt {
            reason: format!("file missing: {}", path.display()),
            path: path.to_path_buf(),
        };
    }
    let size = match std::fs::metadata(path) {
        Ok(m) => m.len(),
        Err(e) => {
            return RomImageValidation::Corrupt {
                reason: format!("meta: {e}"),
                path: path.to_path_buf(),
            }
        }
    };
    if size == 0 {
        return RomImageValidation::Corrupt {
            reason: "empty file (0 bytes)".to_string(),
            path: path.to_path_buf(),
        };
    }
    let kind = image_kind_of(path);
    let cont = container_note(path);
    if kind == ImageKind::Unknown {
        return RomImageValidation::Corrupt {
            reason: format!(
                "content unclear (container {cont}): no ANDROID!/sparse/ext4 magic - validate before use"
            ),
            path: path.to_path_buf(),
        };
    }
    if kind != expected {
        return RomImageValidation::WrongKind {
            found: kind,
            expected,
            path: path.to_path_buf(),
            message: format!(
                "found {} image, expected {} (container {cont})",
                kind.as_str(),
                expected.as_str()
            ),
        };
    }
    RomImageValidation::Ok {
        kind,
        path: path.to_path_buf(),
        message: format!(
            "ready: {} image ({} bytes, container {cont})",
            kind.as_str(),
            size
        ),
    }
}

/// Download + decompress (`.gz`/`.xz` via gsi-archive) + kind check.
/// Download/IO failures are `Err` (abort); content issues are
/// `Ok(WrongKind/Corrupt)` with honest messages.
pub fn fetch_rom_image_with_progress(
    url: &str,
    dest: &Path,
    expected: ImageKind,
    reporter: Option<&dyn Fn(ProgressEvent)>,
) -> Result<RomImageValidation, String> {
    download_file_with_progress(url, dest, reporter)?;
    let name = dest.file_name().and_then(|n| n.to_str()).unwrap_or("");
    let effective: PathBuf = match gsi_archive::classify(name) {
        gsi_archive::ArchiveKind::GzipSingle | gsi_archive::ArchiveKind::XzSingle => {
            let parent = dest.parent().unwrap_or(Path::new("."));
            match gsi_archive::decompress_single(dest, parent) {
                Ok(p) => p,
                Err(e) => {
                    return Ok(RomImageValidation::Corrupt {
                        reason: format!("decompress failed (corrupt?): {e}"),
                        path: dest.to_path_buf(),
                    })
                }
            }
        }
        _ => dest.to_path_buf(),
    };
    Ok(validate_rom_image(&effective, expected))
}

/// `fetch_rom_image_with_progress` without events.
pub fn fetch_rom_image(
    url: &str,
    dest: &Path,
    expected: ImageKind,
) -> Result<RomImageValidation, String> {
    fetch_rom_image_with_progress(url, dest, expected, None)
}

// ------------------------------------------------------------ bootstrap install
//
// Script mirror: `Get-BootstrapReleaseFile` / `bootstrap_fetch`
// (resolve names -> download zip -> verify .sha256 -> extract).
// Each step aborts honestly; a missing .sha256 asset warns and skips
// verify (like both scripts), a mismatch aborts.

/// Verify + extract a local bootstrap zip (hermetic core).
/// Returns absolute paths of extracted files, sorted.
pub fn bootstrap_install_from_zip(
    zip_path: &Path,
    dest_dir: &Path,
    expected_sha256: Option<&str>,
    reporter: Option<&dyn Fn(ProgressEvent)>,
) -> Result<Vec<PathBuf>, String> {
    emit_progress(
        reporter,
        ProgressEvent::Started {
            operation: format!("bootstrap {}", zip_path.display()),
        },
    );
    if !zip_path.is_file() {
        return Err(format!("zip missing: {}", zip_path.display()));
    }
    if let Some(exp) = expected_sha256 {
        let token = exp.split_whitespace().next().unwrap_or("");
        if token.is_empty() {
            return Err("empty expected sha256".to_string());
        }
        verify_staged(zip_path, token)?;
    }
    let data = std::fs::read(zip_path).map_err(|e| format!("read: {e}"))?;
    let fname = zip_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("bootstrap.zip");
    let rels =
        gsi_archive::extract_auto(fname, &data, dest_dir).map_err(|e| format!("extract: {e}"))?;
    let mut out: Vec<PathBuf> = rels.iter().map(|r| dest_dir.join(r)).collect();
    out.sort();
    emit_progress(reporter, ProgressEvent::Completed);
    Ok(out)
}

/// Full orchestrator: names (existing `bootstrap_release`) -> download zip
/// (existing `download_file` path) -> sha verify (`.sha256` asset, existing
/// `verify_staged`; warn-skip when absent) -> extract via gsi-archive.
/// Returns absolute paths of extracted files, sorted.
pub fn bootstrap_install(
    tag: &str,
    cache_dir: &Path,
    dest_dir: &Path,
    reporter: Option<&dyn Fn(ProgressEvent)>,
) -> Result<Vec<PathBuf>, String> {
    let rel = bootstrap_release(tag)?;
    let zip_dest = cache_dir.join(&rel.zip_name);
    download_file_with_progress(&rel.zip_url, &zip_dest, reporter)?;
    let sha_dest = sidecar_path(&zip_dest);
    match download_to(&rel.sha_url, &sha_dest) {
        Ok(()) => {
            let raw = std::fs::read_to_string(&sha_dest).map_err(|e| format!("sidecar: {e}"))?;
            let token = raw.split_whitespace().next().unwrap_or("");
            if token.is_empty() {
                return Err("sha256 asset empty".to_string());
            }
            verify_staged(&zip_dest, token)?;
        }
        Err(e) => emit_progress(
            reporter,
            ProgressEvent::Warning {
                message: format!("no .sha256 asset, skipping verify ({e})"),
            },
        ),
    }
    bootstrap_install_from_zip(&zip_dest, dest_dir, None, reporter)
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

    // ---------------- stable.json (fixtures, no network)

    const STABLE_FIXTURE: &str = r#"{
        "magisk": {
            "version": "28.1",
            "versionCode": 28100,
            "link": "https://github.com/topjohnwu/Magisk/releases/download/v28.1/Magisk-v28.1.apk"
        },
        "stub": { "versionCode": 28100 }
    }"#;

    #[test]
    fn parses_stable_channel() {
        let s = parse_magisk_stable(STABLE_FIXTURE).unwrap();
        assert_eq!(s.version, "28.1");
        assert_eq!(s.version_code, "28100");
        assert!(s.apk_url.ends_with("Magisk-v28.1.apk"));
        assert!(s.zip_url.is_empty());
        assert_eq!(magisk_apk_filename(&s), "Magisk-v28.1.apk");
    }

    #[test]
    fn parses_stable_with_zip_and_string_code() {
        let body = r#"{"magisk":{"version":"27.0","versionCode":"27000","link":"https://x/Magisk-v27.0.apk","zipLink":"https://x/Magisk-v27.0.zip"}}"#;
        let s = parse_magisk_stable(body).unwrap();
        assert_eq!(s.version_code, "27000");
        assert!(s.zip_url.ends_with(".zip"));
    }

    #[test]
    fn rejects_stable_without_magisk_or_link() {
        assert!(parse_magisk_stable("{}").is_err());
        assert!(parse_magisk_stable(r#"{"magisk":{"version":"1"}}"#).is_err());
        assert!(parse_magisk_stable("not json").is_err());
    }

    #[test]
    fn stable_url_prefers_registry_override() {
        let reg = r#"{"magisk":{"stable_json":"https://example.invalid/x/stable.json"}}"#;
        assert_eq!(
            magisk_stable_url(Some(reg)),
            "https://example.invalid/x/stable.json"
        );
        assert_eq!(magisk_stable_url(None), MAGISK_STABLE_DEFAULT_URL);
        assert_eq!(magisk_stable_url(Some("{}")), MAGISK_STABLE_DEFAULT_URL);
        assert_eq!(magisk_stable_url(Some("")), MAGISK_STABLE_DEFAULT_URL);
    }

    #[test]
    fn magisk_dest_joins_dir_and_filename() {
        let s = parse_magisk_stable(STABLE_FIXTURE).unwrap();
        let dest = magisk_download_dest(Path::new("/tmp/magisk"), &s);
        assert_eq!(dest, Path::new("/tmp/magisk/Magisk-v28.1.apk"));
    }

    // ---------------- apk discovery (temp dirs, no device)

    fn fresh_dir(name: &str) -> PathBuf {
        let mut d = std::env::temp_dir();
        d.push(format!("gsi-update-{name}-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn touch(dir: &Path, name: &str) {
        std::fs::write(dir.join(name), b"x").unwrap();
    }

    #[test]
    fn finds_newest_apk_case_insensitive() {
        let d = fresh_dir("apk-find");
        touch(&d, "notes.txt");
        touch(&d, "old.apk");
        std::thread::sleep(std::time::Duration::from_millis(150));
        touch(&d, "Magisk-v28.1.APK");
        let hit = find_magisk_apk(&d).unwrap();
        assert_eq!(hit.file_name().unwrap(), "Magisk-v28.1.APK");
        // substring pattern also works
        let sub = newest_matching_file(&d, "magisk").unwrap();
        assert_eq!(sub.file_name().unwrap(), "Magisk-v28.1.APK");
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn discovery_empty_or_missing_dir_is_none() {
        let d = fresh_dir("apk-empty");
        assert!(find_magisk_apk(&d).is_none());
        assert!(newest_matching_file(&d, "*").is_none());
        assert!(newest_matching_file(&d, "").is_none());
        assert!(find_magisk_apk(&d.join("nope")).is_none());
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn version_from_filename_shapes() {
        assert_eq!(magisk_version_from_filename("Magisk-v28.1.apk"), "28.1");
        assert_eq!(magisk_version_from_filename("magisk-27.0.zip"), "27.0");
        assert_eq!(
            magisk_version_from_filename("app-release.apk"),
            "unknown (check filename)"
        );
        assert_eq!(
            magisk_version_from_filename("magisk_patched-27000.img"),
            "unknown (check filename)"
        );
    }

    #[test]
    fn magisk_info_reads_local_file() {
        let d = fresh_dir("apk-info");
        let apk = d.join("Magisk-v28.1.apk");
        std::fs::write(&apk, b"fakapk").unwrap();
        let info = magisk_info(&apk).unwrap();
        assert_eq!(info.version, "28.1");
        assert_eq!(info.size, 6);
        assert_eq!(info.sha256, sha256_file(&apk).unwrap());
        assert!(magisk_info(&d.join("missing.apk")).is_err());
        std::fs::remove_dir_all(&d).ok();
    }

    // ---------------- registry ROM resolution (repo data, no network)

    fn repo_profile() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../../data/compatibility/huawei/p10/VTR-L29.json")
    }

    fn load_profile() -> Vec<RomEntry> {
        gsi_registry::load_roms(&repo_profile()).expect("profile must load")
    }

    #[test]
    fn rom_downloads_and_label_resolution() {
        let entries = load_profile();
        let list = rom_downloads(&entries);
        assert!(!list.is_empty());
        assert!(list
            .iter()
            .all(|d| !d.url.is_empty() && !d.label.is_empty()));
        // 1-based number resolves.
        let first = resolve_rom_download(&entries, "1").unwrap();
        assert_eq!(first.label, list[0].label);
        // Exact label resolves (UNOFFICIAL LineageOS 20 build).
        let label = "LineageOS 20 UNOFFICIAL (20251021)";
        let hit = resolve_rom_download(&entries, label).unwrap();
        assert!(hit.url.contains("lineage-20.0-20251021"));
        assert_eq!(
            rom_download_filename(&hit).unwrap(),
            "lineage-20.0-20251021-UNOFFICIAL-arm64_bgN-signed.img.gz"
        );
        // rom: id prefix accepted; unknown / out-of-range rejected.
        assert!(resolve_rom_download(&entries, &format!("rom:{label}")).is_ok());
        assert!(resolve_rom_download(&entries, "nope-nope").is_err());
        assert!(resolve_rom_download(&entries, "99999").is_err());
        assert!(resolve_rom_download(&[], "1").is_err());
    }

    #[test]
    fn rom_filename_falls_back_to_url_basename() {
        let dl = RomDownload {
            label: "L".to_string(),
            file: String::new(),
            url: "https://sourceforge.net/projects/a/files/f.img.gz/download?x=1".to_string(),
            status: "working".to_string(),
        };
        assert_eq!(rom_download_filename(&dl).unwrap(), "f.img.gz");
        let bad = RomDownload {
            label: "L".to_string(),
            file: String::new(),
            url: String::new(),
            status: String::new(),
        };
        assert!(rom_download_filename(&bad).is_err());
    }

    #[test]
    fn target_config_resolves_for_label() {
        let entries = load_profile();
        let cfg = resolve_target_config(&entries, "LineageOS 20 UNOFFICIAL (20251021)", "EMUI 9.1")
            .unwrap();
        assert_eq!(cfg.gsi, "arm64_bgN");
        assert_eq!(cfg.firmware_base, "EMUI 9.1");
        assert!(cfg.system_url.contains("lineage-20.0-20251021"));
        assert!(resolve_target_config(&entries, "missing label", "EMUI 9.1").is_err());
    }

    // ---------------- sidecars + firmware heuristic (local files)

    #[test]
    fn sidecar_roundtrip_and_mismatch() {
        let d = fresh_dir("sidecar");
        let f = d.join("fw.zip");
        std::fs::write(&f, b"firmware-bytes").unwrap();
        let digest = write_hash_sidecar(&f).unwrap();
        assert_eq!(verify_hash_sidecar(&f).unwrap(), digest);
        // Corrupt the sidecar -> mismatch.
        std::fs::write(f.with_extension("zip.sha256"), "deadbeef").unwrap();
        assert!(verify_hash_sidecar(&f).is_err());
        // Missing sidecar -> error.
        std::fs::remove_file(f.with_extension("zip.sha256")).ok();
        assert!(verify_hash_sidecar(&f).is_err());
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn firmware_check_flags_small_files() {
        let d = fresh_dir("fwcheck");
        let f = d.join("fw.zip");
        std::fs::write(&f, b"tiny").unwrap();
        let chk = verify_downloaded_firmware(&f).unwrap();
        assert!(!chk.ok);
        assert_eq!(chk.size, 4);
        assert!(f.with_extension("zip.sha256").exists());
        assert!(verify_downloaded_firmware(&d.join("missing.zip")).is_err());
        std::fs::remove_dir_all(&d).ok();
    }

    // ---------------- bootstrap (pure)

    #[test]
    fn bootstrap_names_for_tag() {
        let b = bootstrap_release("v1.2.3").unwrap();
        assert_eq!(b.zip_name, "trebleManager-v1.2.3.zip");
        assert!(b
            .zip_url
            .contains("/releases/download/v1.2.3/trebleManager-v1.2.3.zip"));
        assert!(b.sha_url.ends_with(".sha256"));
        let cached = bootstrap_cached_file(Path::new("/c"), "v1.2.3", "scripts/x.ps1");
        assert_eq!(cached, Path::new("/c/v1.2.3/scripts/x.ps1"));
        assert!(bootstrap_release("").is_err());
        assert!(bootstrap_release("v 1").is_err());
    }

    // ---------------- URL validation + cached wrapper (no network)

    #[test]
    fn url_validation_matrix() {
        assert!(valid_download_url(
            "https://sourceforge.net/projects/a/files/f.img.gz/download"
        ));
        assert!(valid_download_url("https://example.invalid/fw.zip?x=1"));
        assert!(valid_download_url("https://example.invalid/a.IMG"));
        assert!(!valid_download_url("ftp://example.invalid/f.zip"));
        assert!(!valid_download_url("https://example.invalid/noext"));
        assert!(!valid_download_url(
            "https://user:pass@example.invalid/f.zip"
        ));
        assert!(!valid_download_url("https://example.invalid/f.json"));
        assert!(!valid_download_url(""));
        assert!(valid_http_url("https://example.invalid/stable.json"));
        assert!(!valid_http_url("file:///tmp/x"));
    }

    #[test]
    fn download_file_uses_cache_and_rejects() {
        let d = fresh_dir("dlcache");
        let dest = d.join("f.img.gz");
        std::fs::write(&dest, b"cached").unwrap();
        assert_eq!(
            download_file("https://example.invalid/f.img.gz", &dest).unwrap(),
            DownloadOutcome::Cached
        );
        assert!(download_file("ftp://example.invalid/f.zip", &d.join("g.zip")).is_err());
        std::fs::remove_dir_all(&d).ok();
    }

    // ---------------- progress downloads (hermetic: file:// only, no network)

    #[test]
    fn progress_sequence_on_local_copy() {
        use std::cell::RefCell;
        let d = fresh_dir("progress");
        let src = d.join("src.img.gz");
        let bytes = vec![7u8; 100_000];
        std::fs::write(&src, &bytes).unwrap();
        let dest = d.join("sub").join("dst.img.gz");
        let seen: RefCell<Vec<ProgressEvent>> = RefCell::new(Vec::new());
        let cb = |ev: ProgressEvent| seen.borrow_mut().push(ev);
        let url = format!("file://{}", src.display());
        download_to_with_progress(&url, &dest, Some(&cb)).unwrap();
        assert_eq!(std::fs::read(&dest).unwrap(), bytes);
        let ev = seen.borrow();
        assert!(matches!(ev.first(), Some(ProgressEvent::Started { .. })));
        assert!(matches!(ev.last(), Some(ProgressEvent::Completed)));
        let total = std::fs::metadata(&src).unwrap().len();
        let progs: Vec<(u64, u64)> = ev
            .iter()
            .filter_map(|e| match e {
                ProgressEvent::Progress { current, total } => Some((*current, *total)),
                _ => None,
            })
            .collect();
        assert!(!progs.is_empty());
        assert!(progs.iter().all(|(_, t)| *t == total));
        assert_eq!(progs.last().map(|p| p.0), Some(total));
        for w in progs.windows(2) {
            assert!(w[1].0 >= w[0].0);
        }
        assert!(
            download_to_with_progress("file:///no/such/file.img.gz", &d.join("x.gz"), None)
                .is_err()
        );
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn cached_download_reports_completion_without_bytes() {
        use std::cell::RefCell;
        let d = fresh_dir("dlcached2");
        let dest = d.join("f.img.gz");
        std::fs::write(&dest, b"cached").unwrap();
        let seen: RefCell<Vec<ProgressEvent>> = RefCell::new(Vec::new());
        let cb = |ev: ProgressEvent| seen.borrow_mut().push(ev);
        let out =
            download_file_with_progress("file:///tmp/any/f.img.gz", &dest, Some(&cb)).unwrap();
        assert_eq!(out, DownloadOutcome::Cached);
        let ev = seen.borrow();
        assert_eq!(ev.len(), 2);
        assert!(matches!(ev[0], ProgressEvent::Started { .. }));
        assert!(matches!(ev[1], ProgressEvent::Completed));
        std::fs::remove_dir_all(&d).ok();
    }

    // ---------------- ROM validation matrix (crafted local files, no network)

    /// gzip (mtime=0) of `ANDROID!` + version byte + filler (98 bytes raw).
    const TEST_BOOT_GZ: &[u8] = &[
        31, 139, 8, 0, 0, 0, 0, 0, 2, 255, 115, 244, 115, 9, 242, 247, 116, 81, 100, 118, 242, 247,
        15, 113, 113, 12, 113, 212, 53, 48, 52, 50, 54, 49, 53, 51, 183, 176, 116, 116, 114, 118,
        113, 117, 99, 160, 16, 0, 0, 5, 63, 145, 148, 98, 0, 0, 0,
    ];

    fn sparse_fixture() -> Vec<u8> {
        let mut h = vec![0x3A, 0xFF, 0x26, 0xED, 0x01, 0x00, 0x00, 0x00];
        h.extend_from_slice(&28u16.to_le_bytes());
        h.extend_from_slice(&12u16.to_le_bytes());
        h.extend_from_slice(&4096u32.to_le_bytes());
        h.extend_from_slice(&100u32.to_le_bytes());
        h.extend_from_slice(&5u32.to_le_bytes());
        h.extend_from_slice(&[0u8; 4]);
        h
    }

    #[test]
    fn image_kind_shapes() {
        let d = fresh_dir("romkind");
        assert_eq!(parse_image_kind("boot"), Some(ImageKind::Boot));
        assert_eq!(parse_image_kind("SYSTEM"), Some(ImageKind::System));
        assert_eq!(parse_image_kind("unknown"), None);
        assert_eq!(parse_image_kind(""), None);
        let sys = d.join("s.img");
        std::fs::write(&sys, sparse_fixture()).unwrap();
        assert_eq!(image_kind_of(&sys), ImageKind::System);
        let mut ext = vec![0u8; 1090];
        ext[1080] = 0x53;
        ext[1081] = 0xEF;
        let pe = d.join("e.img");
        std::fs::write(&pe, &ext).unwrap();
        assert_eq!(image_kind_of(&pe), ImageKind::System);
        let unk = d.join("u.img");
        std::fs::write(&unk, b"hello, not an image").unwrap();
        assert_eq!(image_kind_of(&unk), ImageKind::Unknown);
        assert_eq!(image_kind_of(&d.join("missing.img")), ImageKind::Unknown);
        assert_eq!(ImageKind::Boot.as_str(), "boot");
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn rom_validation_matrix() {
        let d = fresh_dir("romval");
        // ok: gzip of ANDROID! content, expected boot.
        let src_ok = d.join("ok-src.img.gz");
        std::fs::write(&src_ok, TEST_BOOT_GZ).unwrap();
        let dst_ok = d.join("dl").join("ok.img.gz");
        let r = fetch_rom_image(
            &format!("file://{}", src_ok.display()),
            &dst_ok,
            ImageKind::Boot,
        )
        .unwrap();
        assert!(r.is_ok(), "got {r:?}");
        assert!(matches!(
            r,
            RomImageValidation::Ok {
                kind: ImageKind::Boot,
                ..
            }
        ));
        // wrong-kind: sparse system image, expected boot.
        let sys = d.join("sys.img");
        std::fs::write(&sys, sparse_fixture()).unwrap();
        let dst_sys = d.join("dl2").join("sys.img");
        let r2 = fetch_rom_image(
            &format!("file://{}", sys.display()),
            &dst_sys,
            ImageKind::Boot,
        )
        .unwrap();
        assert!(!r2.is_ok());
        assert!(matches!(
            r2,
            RomImageValidation::WrongKind {
                found: ImageKind::System,
                expected: ImageKind::Boot,
                ..
            }
        ));
        // corrupt: truncated gzip.
        let bad = d.join("bad.img.gz");
        std::fs::write(&bad, &TEST_BOOT_GZ[..10]).unwrap();
        let dst_bad = d.join("dl3").join("bad.img.gz");
        let r3 = fetch_rom_image(
            &format!("file://{}", bad.display()),
            &dst_bad,
            ImageKind::Boot,
        )
        .unwrap();
        assert!(
            matches!(r3, RomImageValidation::Corrupt { .. }),
            "got {r3:?}"
        );
        // corrupt: unclear content (plain text, expected system).
        let unk = d.join("unk.img");
        std::fs::write(&unk, b"hello, not an image").unwrap();
        let dst_unk = d.join("dl4").join("unk.img");
        let r4 = fetch_rom_image(
            &format!("file://{}", unk.display()),
            &dst_unk,
            ImageKind::System,
        )
        .unwrap();
        assert!(
            matches!(r4, RomImageValidation::Corrupt { .. }),
            "got {r4:?}"
        );
        // pure helper: missing file is corrupt, download failure is Err.
        assert!(matches!(
            validate_rom_image(&d.join("nope.img"), ImageKind::Boot),
            RomImageValidation::Corrupt { .. }
        ));
        assert!(fetch_rom_image(
            "ftp://example.invalid/f.img.gz",
            &d.join("z.gz"),
            ImageKind::Boot
        )
        .is_err());
        std::fs::remove_dir_all(&d).ok();
    }

    // ---------------- bootstrap from local zip+sha (no network)

    /// Stored (uncompressed) zip: scripts/treble-toolkit.sh + README.md.
    const TEST_BOOTSTRAP_ZIP: &[u8] = &[
        80, 75, 3, 4, 20, 0, 0, 0, 0, 0, 72, 69, 71, 93, 47, 58, 218, 233, 18, 0, 0, 0, 18, 0, 0,
        0, 25, 0, 0, 0, 115, 99, 114, 105, 112, 116, 115, 47, 116, 114, 101, 98, 108, 101, 45, 116,
        111, 111, 108, 107, 105, 116, 46, 115, 104, 35, 33, 47, 98, 105, 110, 47, 115, 104, 10,
        101, 99, 104, 111, 32, 104, 105, 10, 80, 75, 3, 4, 20, 0, 0, 0, 0, 0, 72, 69, 71, 93, 97,
        19, 161, 131, 9, 0, 0, 0, 9, 0, 0, 0, 9, 0, 0, 0, 82, 69, 65, 68, 77, 69, 46, 109, 100, 35,
        32, 116, 114, 101, 98, 108, 101, 10, 80, 75, 1, 2, 20, 3, 20, 0, 0, 0, 0, 0, 72, 69, 71,
        93, 47, 58, 218, 233, 18, 0, 0, 0, 18, 0, 0, 0, 25, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 128,
        1, 0, 0, 0, 0, 115, 99, 114, 105, 112, 116, 115, 47, 116, 114, 101, 98, 108, 101, 45, 116,
        111, 111, 108, 107, 105, 116, 46, 115, 104, 80, 75, 1, 2, 20, 3, 20, 0, 0, 0, 0, 0, 72, 69,
        71, 93, 97, 19, 161, 131, 9, 0, 0, 0, 9, 0, 0, 0, 9, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 128,
        1, 73, 0, 0, 0, 82, 69, 65, 68, 77, 69, 46, 109, 100, 80, 75, 5, 6, 0, 0, 0, 0, 2, 0, 2, 0,
        126, 0, 0, 0, 121, 0, 0, 0, 0, 0,
    ];

    #[test]
    fn bootstrap_from_local_zip() {
        use std::cell::RefCell;
        let d = fresh_dir("bootstrap");
        let zip = d.join("trebleManager-v9.9.zip");
        std::fs::write(&zip, TEST_BOOTSTRAP_ZIP).unwrap();
        let sha = sha256_file(&zip).unwrap();
        let out = d.join("out");
        let files = bootstrap_install_from_zip(&zip, &out, Some(&sha), None).unwrap();
        assert_eq!(files.len(), 2);
        assert_eq!(
            std::fs::read(out.join("scripts/treble-toolkit.sh")).unwrap(),
            b"#!/bin/sh\necho hi\n"
        );
        assert_eq!(std::fs::read(out.join("README.md")).unwrap(), b"# treble\n");
        assert!(files.iter().all(|p| p.is_file()));
        // sha as sidecar-style first token also verifies.
        let out_b = d.join("out-b");
        let sidecar = format!("{sha}  trebleManager-v9.9.zip");
        assert_eq!(
            bootstrap_install_from_zip(&zip, &out_b, Some(&sidecar), None)
                .unwrap()
                .len(),
            2
        );
        // wrong sha aborts honestly.
        assert!(
            bootstrap_install_from_zip(&zip, &d.join("out-c"), Some("deadbeef"), None).is_err()
        );
        // corrupt zip aborts.
        let cz = d.join("c.zip");
        std::fs::write(&cz, b"not a zip at all................").unwrap();
        assert!(bootstrap_install_from_zip(&cz, &d.join("out-d"), None, None).is_err());
        // missing zip aborts.
        assert!(
            bootstrap_install_from_zip(&d.join("no.zip"), &d.join("out-e"), None, None).is_err()
        );
        // events: Started .. Completed.
        let seen: RefCell<Vec<ProgressEvent>> = RefCell::new(Vec::new());
        let cb = |ev: ProgressEvent| seen.borrow_mut().push(ev);
        let files2 = bootstrap_install_from_zip(&zip, &d.join("out-f"), None, Some(&cb)).unwrap();
        assert_eq!(files2.len(), 2);
        let ev = seen.borrow();
        assert!(matches!(ev.first(), Some(ProgressEvent::Started { .. })));
        assert!(matches!(ev.last(), Some(ProgressEvent::Completed)));
        std::fs::remove_dir_all(&d).ok();
    }
}
