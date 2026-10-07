//! Platform-correct application directories.
//!
//! No hardcoded `/home/...` or `C:\Users\...` paths. Resolution order per
//! directory: explicit override parameter → OS-correct user location.
//! The user can relocate everything (documented, no hidden state).
//!
//! Plus pure discovery rules ported from the scripts (`scripts/Treble-Toolkit.ps1`,
//! `scripts/treble-toolkit.sh`, `scripts/Setup-Windows.ps1`): image finders over
//! caller-passed dirs, install-base/tool-link path rules, script/tool root,
//! tool-path JSON config, compat profile resolution, image magic + size policy.

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

// ------------------------------------------------- file discovery (explicit dirs only)

fn collect_files(dir: &std::path::Path, out: &mut Vec<PathBuf>) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        let ftype = match entry.file_type() {
            Ok(t) => t,
            Err(_) => continue,
        };
        if ftype.is_symlink() {
            continue;
        }
        let path = entry.path();
        if ftype.is_dir() {
            collect_files(&path, out);
        } else if ftype.is_file() {
            out.push(path);
        }
    }
}

fn file_mtime(path: &std::path::Path) -> std::time::SystemTime {
    match std::fs::metadata(path) {
        Ok(m) => match m.modified() {
            Ok(t) => t,
            Err(_) => std::time::SystemTime::UNIX_EPOCH,
        },
        Err(_) => std::time::SystemTime::UNIX_EPOCH,
    }
}

/// Newest file under `dir` (recursive) whose file name satisfies `pred`.
/// Ties break toward the lexicographically smaller path (deterministic).
/// Mirrors `rom_base_image` (bash) / `Find-TTRomBaseImage` newest-wins rule.
pub fn newest_matching_file(
    dir: &std::path::Path,
    pred: impl Fn(&str) -> bool,
) -> Option<PathBuf> {
    let mut files = Vec::new();
    collect_files(dir, &mut files);
    let mut best: Option<(std::time::SystemTime, PathBuf)> = None;
    for path in files {
        let name = match path.file_name().and_then(|n| n.to_str()) {
            Some(n) => n,
            None => continue,
        };
        if !pred(name) {
            continue;
        }
        let mtime = file_mtime(&path);
        let replace = match &best {
            None => true,
            Some((t, p)) => mtime > *t || (mtime == *t && path < *p),
        };
        if replace {
            best = Some((mtime, path));
        }
    }
    best.map(|(_, p)| p)
}

/// Newest match across several dirs (caller passes dirs, never CWD).
pub fn newest_matching_in_dirs(
    dirs: &[PathBuf],
    pred: impl Fn(&str) -> bool,
) -> Option<PathBuf> {
    let mut best: Option<(std::time::SystemTime, PathBuf)> = None;
    for dir in dirs {
        if let Some(p) = newest_matching_file(dir, &pred) {
            let mtime = file_mtime(&p);
            let replace = match &best {
                None => true,
                Some((t, q)) => mtime > *t || (mtime == *t && p < *q),
            };
            if replace {
                best = Some((mtime, p));
            }
        }
    }
    best.map(|(_, p)| p)
}

/// `*.img` (case-insensitive).
pub fn is_img_name(name: &str) -> bool {
    name.to_ascii_lowercase().ends_with(".img")
}

/// Exact stock recovery file names (case-insensitive).
/// Canonical list, mirrors `StockFileNames` in the PowerShell profiles.
pub fn stock_recovery_names() -> [&'static str; 3] {
    [
        "RECOVERY_RAMDISK.img",
        "RECOVERY_RAMDIS.img",
        "recovery_ramdisk.img",
    ]
}

/// Exact stock-name hit (caller passes the profile list explicitly).
pub fn is_stock_recovery_name(name: &str, stock_names: &[&str]) -> bool {
    stock_names.iter().any(|s| name.eq_ignore_ascii_case(s))
}

/// Fallback matcher: name holds `RECOVERY` + `RAMDIS` (case-insensitive).
/// Mirrors `*RECOVERY*RAMDIS*` fallback in `Find-TTRecoveryImage`.
pub fn is_recovery_fallback_name(name: &str) -> bool {
    let n = name.to_ascii_lowercase();
    n.contains("recovery") && n.contains("ramdis")
}

/// Magisk on-device output names: `magisk_patched*` / `magisk-patched*`
/// (case-insensitive), e.g. `magisk_patched-XXXX.img`.
pub fn is_magisk_patched_name(name: &str) -> bool {
    let n = name.to_ascii_lowercase();
    n.starts_with("magisk_patched") || n.starts_with("magisk-patched")
}

/// Exactly `UPDATE.APP` (case-insensitive).
pub fn is_update_app_name(name: &str) -> bool {
    name.eq_ignore_ascii_case("UPDATE.APP")
}

/// Registry system-image pattern `*-arm64_*.img` (case-insensitive).
/// Mirrors the `Find-LocalSystemImage` fallback filter.
pub fn is_system_image_name(name: &str) -> bool {
    let n = name.to_ascii_lowercase();
    if !n.ends_with(".img") {
        return false;
    }
    n[..n.len() - 4].contains("-arm64_")
}

/// Stock recovery images over explicit dirs: exact `stock_names` hits first
/// (sorted), then `*RECOVERY*RAMDIS*` fallback extras (sorted, deduplicated).
/// Mirrors `Find-TTRecoveryImage` / `screen_extract` search set.
pub fn find_recovery_images(dirs: &[PathBuf], stock_names: &[&str]) -> Vec<PathBuf> {
    let mut exact = Vec::new();
    let mut fallback = Vec::new();
    for dir in dirs {
        let mut files = Vec::new();
        collect_files(dir, &mut files);
        for path in files {
            let name = match path.file_name().and_then(|n| n.to_str()) {
                Some(n) => n,
                None => continue,
            };
            if is_stock_recovery_name(name, stock_names) {
                if !exact.contains(&path) {
                    exact.push(path);
                }
            } else if is_recovery_fallback_name(name) && !exact.contains(&path) {
                if !fallback.contains(&path) {
                    fallback.push(path);
                }
            }
        }
    }
    exact.sort();
    fallback.sort();
    exact.extend(fallback);
    exact
}

/// Newest `*.img` over explicit dirs (ROM export output).
/// Mirrors `rom_base_image` (bash) / `Find-TTRomBaseImage`.
pub fn find_rom_base_image(dirs: &[PathBuf]) -> Option<PathBuf> {
    newest_matching_in_dirs(dirs, is_img_name)
}

/// Newest `magisk_patched*` image over explicit dirs.
pub fn find_magisk_patched(dirs: &[PathBuf]) -> Option<PathBuf> {
    newest_matching_in_dirs(dirs, is_magisk_patched_name)
}

/// All `*.APP` firmware containers over explicit dirs (sorted).
/// Mirrors `-iname '*.APP'` / `-Filter "*.APP"` search in both scripts.
pub fn find_update_apps(dirs: &[PathBuf]) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for dir in dirs {
        let mut files = Vec::new();
        collect_files(dir, &mut files);
        for path in files {
            let is_app = match path.extension().and_then(|e| e.to_str()) {
                Some(e) => e.eq_ignore_ascii_case("app"),
                None => false,
            };
            if is_app && !out.contains(&path) {
                out.push(path);
            }
        }
    }
    out.sort();
    out
}

/// Local ready system image: exact registry `want_file_name` first (sorted
/// first hit), else newest `*-arm64_*.img`. Mirrors `Find-LocalSystemImage`.
pub fn find_system_image(dirs: &[PathBuf], want_file_name: Option<&str>) -> Option<PathBuf> {
    if let Some(want) = want_file_name {
        if !want.is_empty() {
            let mut hits = Vec::new();
            for dir in dirs {
                let mut files = Vec::new();
                collect_files(dir, &mut files);
                for path in files {
                    let same = match path.file_name().and_then(|n| n.to_str()) {
                        Some(n) => n.eq_ignore_ascii_case(want),
                        None => false,
                    };
                    if same && !hits.contains(&path) {
                        hits.push(path);
                    }
                }
            }
            hits.sort();
            if let Some(first) = hits.into_iter().next() {
                return Some(first);
            }
        }
    }
    newest_matching_in_dirs(dirs, is_system_image_name)
}

// ------------------------------------------------- install base + tool linking

/// Central tools folder rule (pure): repo `tool_dir` when writable, else
/// `~/.local/share/trebleManager/tools`. Mirrors `install_base_dir` (bash);
/// writability is an explicit input (caller probes or creates the dir).
pub fn install_base_dir(
    tool_dir: &std::path::Path,
    tool_dir_writable: bool,
    home: &std::path::Path,
) -> PathBuf {
    if tool_dir_writable {
        tool_dir.to_path_buf()
    } else {
        home.join(".local")
            .join("share")
            .join("trebleManager")
            .join("tools")
    }
}

/// Symlink plan for `link_into_tools` (bash): `(source, link)` pairs for the
/// six known binary names. Pure computation, caller creates the links.
pub fn tool_link_plan(src_dir: &std::path::Path, base_dir: &std::path::Path) -> Vec<(PathBuf, PathBuf)> {
    const NAMES: [&str; 6] = [
        "adb",
        "adb.exe",
        "fastboot",
        "fastboot.exe",
        "scrcpy",
        "scrcpy.exe",
    ];
    NAMES
        .iter()
        .map(|n| (src_dir.join(n), base_dir.join(n)))
        .collect()
}

#[cfg(unix)]
fn is_runnable(path: &std::path::Path) -> bool {
    match std::fs::metadata(path) {
        Ok(m) => {
            if !m.is_file() {
                return false;
            }
            use std::os::unix::fs::PermissionsExt;
            m.permissions().mode() & 0o111 != 0
        }
        Err(_) => false,
    }
}

#[cfg(not(unix))]
fn is_runnable(path: &std::path::Path) -> bool {
    path.is_file()
}

/// Adopt `adb`/`fastboot`/`scrcpy` found in `dir`: direct `<tool>[.exe]`
/// candidates first, then one nested level (sorted, wins like the bash
/// `find -maxdepth 2` pass). Mirrors `scan_dir_for_tools` (bash) as
/// discovery over an explicit dir; caller assigns the hits.
pub fn scan_dir_for_tools(dir: &std::path::Path) -> Vec<(&'static str, PathBuf)> {
    const TOOLS: [&str; 3] = ["adb", "fastboot", "scrcpy"];
    let mut nested: Vec<PathBuf> = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries {
            let entry = match entry {
                Ok(e) => e,
                Err(_) => continue,
            };
            if let Ok(t) = entry.file_type() {
                if t.is_dir() && !t.is_symlink() {
                    if let Ok(inner) = std::fs::read_dir(entry.path()) {
                        for f in inner {
                            if let Ok(f) = f {
                                nested.push(f.path());
                            }
                        }
                    }
                }
            }
        }
    }
    nested.sort();
    let mut out = Vec::new();
    for tool in TOOLS {
        let mut hit: Option<PathBuf> = None;
        for cand in [dir.join(tool), dir.join(format!("{tool}.exe"))] {
            if is_runnable(&cand) {
                hit = Some(cand);
                break;
            }
        }
        for path in &nested {
            let name = match path.file_name().and_then(|n| n.to_str()) {
                Some(n) => n,
                None => continue,
            };
            if name.eq_ignore_ascii_case(tool) || name.eq_ignore_ascii_case(&format!("{tool}.exe")) {
                if is_runnable(path) {
                    hit = Some(path.clone());
                    break;
                }
            }
        }
        if let Some(p) = hit {
            out.push((tool, p));
        }
    }
    out
}

// ------------------------------------------------- script root / tool root

/// Script-dir rule, mirrors `Get-TTScriptRoot` (script location first, CWD
/// last): explicit `exe_dir` wins, else `$TT_SCRIPT_ROOT` override (for
/// tests), else current-exe parent, else CWD. This is the only function here
/// that reads env/process state.
pub fn script_root(exe_dir: Option<&std::path::Path>) -> PathBuf {
    if let Some(d) = exe_dir {
        if !d.as_os_str().is_empty() {
            return d.to_path_buf();
        }
    }
    if let Some(v) = std::env::var_os("TT_SCRIPT_ROOT") {
        let p = PathBuf::from(v);
        if !p.as_os_str().is_empty() {
            return p;
        }
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            if !parent.as_os_str().is_empty() {
                return parent.to_path_buf();
            }
        }
    }
    match std::env::current_dir() {
        Ok(c) => c,
        Err(_) => PathBuf::from("."),
    }
}

/// Tool-root derivation: parent when the script dir leaf is `scripts`,
/// else the dir itself. Mirrors `Get-TTToolRoot` / bash `TOOL_ROOT` logic.
pub fn tool_root(script_dir: &std::path::Path) -> PathBuf {
    let leaf_is_scripts = match script_dir.file_name().and_then(|n| n.to_str()) {
        Some(n) => n.eq_ignore_ascii_case("scripts"),
        None => false,
    };
    if leaf_is_scripts {
        match script_dir.parent() {
            Some(p) => p.to_path_buf(),
            None => script_dir.to_path_buf(),
        }
    } else {
        script_dir.to_path_buf()
    }
}

// ------------------------------------------------- tool-path JSON config

/// Resolved external tool paths, mirrors the `data/config.json` shape written
/// by `save_config` (bash) and `Setup-Windows.ps1` (`adb`, `fastboot`,
/// `scrcpy`, `updated`; extra keys like `installDir` are tolerated on load).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolPaths {
    pub adb: String,
    pub fastboot: String,
    pub scrcpy: String,
    pub updated: String,
}

impl ToolPaths {
    /// Build from explicit values (pure).
    pub fn new(adb: &str, fastboot: &str, scrcpy: &str, updated: &str) -> Self {
        Self {
            adb: adb.to_string(),
            fastboot: fastboot.to_string(),
            scrcpy: scrcpy.to_string(),
            updated: updated.to_string(),
        }
    }

    /// Serialize in script field order (pure, no external JSON dep).
    pub fn to_json(&self) -> String {
        format!(
            "{{\"adb\":\"{}\",\"fastboot\":\"{}\",\"scrcpy\":\"{}\",\"updated\":\"{}\"}}\n",
            escape_json(&self.adb),
            escape_json(&self.fastboot),
            escape_json(&self.scrcpy),
            escape_json(&self.updated)
        )
    }

    /// Parse the config shape; `None` unless all four string fields exist.
    /// Unknown extra fields are ignored (pure).
    pub fn parse(src: &str) -> Option<Self> {
        Some(Self {
            adb: json_field(src, "adb")?,
            fastboot: json_field(src, "fastboot")?,
            scrcpy: json_field(src, "scrcpy")?,
            updated: json_field(src, "updated")?,
        })
    }
}

fn escape_json(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                out.push_str(&format!("\\u{:04x}", c as u32));
            }
            c => out.push(c),
        }
    }
    out
}

fn parse_str_lit(src: &str) -> Option<(String, &str)> {
    let b = src.as_bytes();
    let mut out = String::new();
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b'"' => return Some((out, &src[i + 1..])),
            b'\\' => {
                i += 1;
                if i >= b.len() {
                    return None;
                }
                match b[i] {
                    b'"' => out.push('"'),
                    b'\\' => out.push('\\'),
                    b'/' => out.push('/'),
                    b'n' => out.push('\n'),
                    b'r' => out.push('\r'),
                    b't' => out.push('\t'),
                    b'b' => out.push('\x08'),
                    b'f' => out.push('\x0C'),
                    b'u' => {
                        if i + 4 >= b.len() {
                            return None;
                        }
                        let mut v = u32::from_str_radix(&src[i + 1..i + 5], 16).ok()?;
                        i += 4;
                        if (0xD800..0xDC00).contains(&v) {
                            if i + 7 > b.len() || b[i + 1] != b'\\' || b[i + 2] != b'u' {
                                return None;
                            }
                            let lo = u32::from_str_radix(&src[i + 3..i + 7], 16).ok()?;
                            if !(0xDC00..0xE000).contains(&lo) {
                                return None;
                            }
                            v = 0x10000 + ((v - 0xD800) << 10) + (lo - 0xDC00);
                            i += 6;
                        }
                        out.push(char::from_u32(v)?);
                    }
                    _ => return None,
                }
                i += 1;
            }
            0x00..=0x1F => return None,
            _ => {
                let ch = src[i..].chars().next()?;
                out.push(ch);
                i += ch.len_utf8();
            }
        }
    }
    None
}

fn json_field(src: &str, key: &str) -> Option<String> {
    let pat = format!("\"{key}\"");
    let mut rest = src;
    loop {
        let pos = match rest.find(&pat) {
            Some(p) => p,
            None => return None,
        };
        rest = &rest[pos + pat.len()..];
        let cur = rest.trim_start();
        let cur = match cur.strip_prefix(':') {
            Some(c) => c,
            None => continue,
        };
        let cur = cur.trim_start();
        let lit = match cur.strip_prefix('"') {
            Some(c) => c,
            None => continue,
        };
        return parse_str_lit(lit).map(|(s, _)| s);
    }
}

/// Persist tool paths as JSON (creates parent dirs). Mirrors `save_config`.
pub fn save_tool_config(path: &std::path::Path, cfg: &ToolPaths) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }
    std::fs::write(path, cfg.to_json())
}

/// Load tool paths; `None` when missing or unparsable (never fails hard).
pub fn load_tool_config(path: &std::path::Path) -> Option<ToolPaths> {
    match std::fs::read_to_string(path) {
        Ok(s) => ToolPaths::parse(&s),
        Err(_) => None,
    }
}

// ------------------------------------------------- compat profile discovery

fn compat_subdir(data_dir: &std::path::Path) -> PathBuf {
    data_dir
        .join("compatibility")
        .join("huawei")
        .join("p10")
}

/// Profile file for a model id over an explicit data dir:
/// `data/compatibility/huawei/p10/<MODEL>.json`. Mirrors `compat_file`.
pub fn compat_profile_path(data_dir: &std::path::Path, model: &str) -> PathBuf {
    compat_subdir(data_dir).join(format!("{}.json", model.trim()))
}

/// Resolved profile path, `None` when the file does not exist.
pub fn resolve_compat_profile(data_dir: &std::path::Path, model: &str) -> Option<PathBuf> {
    let p = compat_profile_path(data_dir, model);
    if p.is_file() {
        Some(p)
    } else {
        None
    }
}

/// Available profile ids (sorted `*.json` stems in the compat subdir).
pub fn available_profiles(data_dir: &std::path::Path) -> Vec<String> {
    let mut out = Vec::new();
    let entries = match std::fs::read_dir(compat_subdir(data_dir)) {
        Ok(e) => e,
        Err(_) => return out,
    };
    for entry in entries {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        let path = entry.path();
        let is_json = match path.extension().and_then(|e| e.to_str()) {
            Some(e) => e.eq_ignore_ascii_case("json"),
            None => false,
        };
        if !is_json {
            continue;
        }
        if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
            out.push(stem.to_string());
        }
    }
    out.sort();
    out.dedup();
    out
}

// ------------------------------------------------- image magic + size policy

/// Recovery/boot size policy: 1 MiB ..= 100 MiB.
/// Mirrors `Test-RecoveryImageFile` (`1MB..100MB`, PS `MB` = MiB) and
/// `test_image` (bash `1048576..104857600`).
pub const RECOVERY_MIN_BYTES: u64 = 1_048_576;
/// See [`RECOVERY_MIN_BYTES`].
pub const RECOVERY_MAX_BYTES: u64 = 104_857_600;
/// System image size policy: >= 500 MiB. Mirrors `Test-SystemImageFile`
/// (`500MB`) and `test_system_image` (bash `524288000`).
pub const SYSTEM_MIN_BYTES: u64 = 524_288_000;

/// Image kind by magic bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageKind {
    /// `ANDROID!` boot image magic at offset 0.
    Boot,
    /// Android sparse magic at 0 or ext4 magic (`53 EF`) at offset 1080.
    System,
    /// Anything else (or unreadable).
    Unknown,
}

fn read_at(path: &std::path::Path, offset: u64, len: usize) -> Option<Vec<u8>> {
    use std::io::{Read, Seek, SeekFrom};
    let mut f = std::fs::File::open(path).ok()?;
    if f.seek(SeekFrom::Start(offset)).is_err() {
        return None;
    }
    let mut buf = vec![0u8; len];
    let mut n = 0;
    while n < len {
        match f.read(&mut buf[n..]) {
            Ok(0) => break,
            Ok(r) => n += r,
            Err(_) => return None,
        }
    }
    buf.truncate(n);
    Some(buf)
}

/// Minimal magic check, reimplemented here on purpose: `gsi-image` only knows
/// gzip/sparse-container detection and has no `ANDROID!` boot-magic or
/// ext4-at-1080 check, so no extra dep is pulled in.
pub fn detect_image_kind(path: &std::path::Path) -> ImageKind {
    if let Some(h) = read_at(path, 0, 8) {
        if h.starts_with(b"ANDROID!") {
            return ImageKind::Boot;
        }
        if h.starts_with(&[0x3A, 0xFF, 0x26, 0xED]) {
            return ImageKind::System;
        }
    }
    if let Some(h) = read_at(path, 1080, 2) {
        if h.len() == 2 && h[0] == 0x53 && h[1] == 0xEF {
            return ImageKind::System;
        }
    }
    ImageKind::Unknown
}

/// Result of an image-file check: existence, size policy, magic kind.
/// Verdict `pass` = exists + size policy (header only informs `kind`,
/// mirroring the scripts where unspecific headers do not fail the file).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageCheck {
    pub exists: bool,
    pub size_bytes: u64,
    pub size_ok: bool,
    pub kind: ImageKind,
    pub pass: bool,
}

fn check_image(path: &std::path::Path, min: u64, max: Option<u64>) -> ImageCheck {
    let (exists, size) = match std::fs::metadata(path) {
        Ok(m) => (true, m.len()),
        Err(_) => (false, 0),
    };
    let in_max = match max {
        Some(x) => size <= x,
        None => true,
    };
    let size_ok = exists && size >= min && in_max;
    let kind = if exists {
        detect_image_kind(path)
    } else {
        ImageKind::Unknown
    };
    ImageCheck {
        exists,
        size_bytes: size,
        size_ok,
        kind,
        pass: exists && size_ok,
    }
}

/// Recovery/boot image check (1 MiB ..= 100 MiB).
/// Mirrors `Test-RecoveryImageFile` / `test_image`.
pub fn test_recovery_image(path: &std::path::Path) -> ImageCheck {
    check_image(path, RECOVERY_MIN_BYTES, Some(RECOVERY_MAX_BYTES))
}

/// System/GSI image check (>= 500 MiB). Mirrors `Test-SystemImageFile` /
/// `test_system_image` size verdict (filename hints are advisory, see
/// [`check_system_file_name`]).
pub fn test_system_image(path: &std::path::Path) -> ImageCheck {
    check_image(path, SYSTEM_MIN_BYTES, None)
}

/// Advisory filename hints for a system image (P10 needs arm64 A-only).
/// Bash `test_system_image` fails these; PowerShell only warns — here they
/// stay advisory next to the size verdict.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SystemNameCheck {
    pub looks_arm64: bool,
    pub looks_ab: bool,
}

/// Advisory filename hints (pure, case-insensitive).
pub fn check_system_file_name(name: &str) -> SystemNameCheck {
    let n = name.to_ascii_lowercase();
    SystemNameCheck {
        looks_arm64: n.contains("arm64"),
        looks_ab: n.contains("_ab") || n.contains("a/b"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEST_SEQ: AtomicU64 = AtomicU64::new(0);

    fn fixture_dir(name: &str) -> PathBuf {
        let n = TEST_SEQ.fetch_add(1, Ordering::SeqCst);
        let mut d = std::env::temp_dir();
        d.push(format!("gsi-config-test-{}-{name}-{n}", std::process::id()));
        let _ = std::fs::create_dir_all(&d);
        d
    }

    fn write_file(dir: &std::path::Path, name: &str, bytes: &[u8]) -> PathBuf {
        let p = dir.join(name);
        let _ = std::fs::write(&p, bytes);
        p
    }

    fn sparse_file(dir: &std::path::Path, name: &str, head: &[u8], len: u64) -> PathBuf {
        let p = write_file(dir, name, head);
        if let Ok(f) = std::fs::File::options().write(true).open(&p) {
            let _ = f.set_len(len);
        }
        p
    }

    fn set_mtime(p: &std::path::Path, secs: u64) {
        let t = std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(secs);
        if let Ok(f) = std::fs::File::options().write(true).open(p) {
            let _ = f.set_modified(t);
        }
    }

    fn cleanup(d: &std::path::Path) {
        let _ = std::fs::remove_dir_all(d);
    }

    #[test]
    fn subdir_joins() {
        let base = PathBuf::from("/tmp/x");
        assert_eq!(subdir(&base, "downloads"), PathBuf::from("/tmp/x/downloads"));
    }

    #[test]
    fn dirs_resolve_or_none_without_panic() {
        // Must never fail hard, even with empty environment.
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

    #[test]
    fn newest_img_wins_with_tiebreak() {
        let d = fixture_dir("newest");
        let a = write_file(&d, "a.img", b"data-a");
        let b = write_file(&d, "b.img", b"data-b");
        set_mtime(&a, 1000);
        set_mtime(&b, 2000);
        assert_eq!(newest_matching_file(&d, is_img_name), Some(b.clone()));
        assert_eq!(find_rom_base_image(&[d.clone()]), Some(b.clone()));
        set_mtime(&b, 1000);
        assert_eq!(newest_matching_file(&d, is_img_name), Some(a.clone()));
        assert_eq!(newest_matching_file(&d, is_system_image_name), None);
        cleanup(&d);
    }

    #[test]
    fn recovery_finder_exact_first_fallback_extra() {
        let d = fixture_dir("recovery");
        let fw = d.join("firmware");
        let data = d.join("data");
        let _ = std::fs::create_dir_all(&fw);
        let _ = std::fs::create_dir_all(&data);
        let exact = write_file(&fw, "RECOVERY_RAMDISK.img", b"stock");
        let extra = write_file(&data, "my-RECOVERY_RAMDIS-copy.img", b"copy");
        let _ = write_file(&data, "boot.img", b"boot");
        let names = stock_recovery_names();
        let found = find_recovery_images(&[fw.clone(), data.clone()], &names);
        assert_eq!(found.len(), 2);
        assert_eq!(found[0], exact);
        assert_eq!(found[1], extra);
        assert!(is_stock_recovery_name("recovery_ramdis.img", &names));
        assert!(!is_stock_recovery_name("boot.img", &names));
        cleanup(&d);
    }

    #[test]
    fn system_finder_exact_then_arm64_fallback() {
        let d = fixture_dir("system");
        let roms = d.join("roms");
        let _ = std::fs::create_dir_all(&roms);
        let arm64 = write_file(&roms, "lineage-21-arm64_bgN.img", b"gsi");
        set_mtime(&arm64, 3000);
        let other = write_file(&roms, "other.img", b"other");
        set_mtime(&other, 4000);
        assert!(is_system_image_name("lineage-21-arm64_bgN.img"));
        assert!(!is_system_image_name("other.img"));
        // Fallback picks newest arm64 match, not newest img overall.
        assert_eq!(
            find_system_image(&[roms.clone()], None),
            Some(arm64.clone())
        );
        // Exact registry name wins even when older than the fallback.
        let exact = write_file(&roms, "wanted-system.img", b"w");
        set_mtime(&exact, 1000);
        assert_eq!(
            find_system_image(&[roms.clone()], Some("wanted-system.img")),
            Some(exact)
        );
        cleanup(&d);
    }

    #[test]
    fn magisk_and_update_app_patterns() {
        let d = fixture_dir("patterns");
        let mag = d.join("magisk");
        let _ = std::fs::create_dir_all(&mag);
        let p1 = write_file(&mag, "magisk_patched-abc123.img", b"p1");
        set_mtime(&p1, 5000);
        let p2 = write_file(&mag, "MAGISK-PATCHED-xyz.img", b"p2");
        set_mtime(&p2, 6000);
        let _ = write_file(&mag, "stock.img", b"s");
        assert!(is_magisk_patched_name("magisk_patched-abc123.img"));
        assert!(!is_magisk_patched_name("stock.img"));
        assert_eq!(find_magisk_patched(&[mag.clone()]), Some(p2));
        let app = write_file(&d, "UPDATE.APP", b"app");
        let _ = write_file(&d, "ota.zip", b"zip");
        assert!(is_update_app_name("update.app"));
        assert!(!is_update_app_name("ota.zip"));
        assert_eq!(find_update_apps(&[d.clone()]), vec![app]);
        cleanup(&d);
    }

    #[test]
    fn size_policy_matrix() {
        let d = fixture_dir("sizes");
        // Recovery/boot: pass window 1 MiB ..= 100 MiB.
        let ok = sparse_file(&d, "rec-ok.img", b"ANDROID!", RECOVERY_MIN_BYTES);
        let chk = test_recovery_image(&ok);
        assert!(chk.pass && chk.size_ok && chk.exists);
        assert_eq!(chk.kind, ImageKind::Boot);
        let tiny = write_file(&d, "rec-tiny.img", b"ANDROID!");
        let chk = test_recovery_image(&tiny);
        assert!(!chk.pass && !chk.size_ok && chk.exists);
        let huge = sparse_file(&d, "rec-huge.img", b"ANDROID!", RECOVERY_MAX_BYTES + 1);
        assert!(!test_recovery_image(&huge).pass);
        let missing = d.join("nope.img");
        let chk = test_recovery_image(&missing);
        assert!(!chk.exists && !chk.pass);
        // System: >= 500 MiB, sparse magic classifies as System.
        let sys = sparse_file(&d, "sys.img", &[0x3A, 0xFF, 0x26, 0xED], SYSTEM_MIN_BYTES);
        let chk = test_system_image(&sys);
        assert!(chk.pass && chk.size_ok);
        assert_eq!(chk.kind, ImageKind::System);
        let small = sparse_file(&d, "sys-small.img", &[0x3A, 0xFF, 0x26, 0xED], 100 << 20);
        assert!(!test_system_image(&small).pass);
        // ext4 magic at offset 1080 classifies as System.
        let mut ext4 = vec![0u8; 1082];
        ext4[1080] = 0x53;
        ext4[1081] = 0xEF;
        let e = write_file(&d, "ext4.img", &ext4);
        assert_eq!(detect_image_kind(&e), ImageKind::System);
        assert_eq!(detect_image_kind(&missing), ImageKind::Unknown);
        cleanup(&d);
    }

    #[test]
    fn system_filename_hints_are_advisory() {
        let good = check_system_file_name("lineage-21-arm64_bgN.img");
        assert!(good.looks_arm64 && !good.looks_ab);
        let ab = check_system_file_name("treble_arm64_ab-vanilla.img");
        assert!(ab.looks_arm64 && ab.looks_ab);
        let bad = check_system_file_name("system.img");
        assert!(!bad.looks_arm64 && !bad.looks_ab);
    }

    #[test]
    fn tool_config_json_roundtrip() {
        let cfg = ToolPaths::new(
            "C:\\tools\\adb.exe",
            "/usr/bin/fastboot",
            "",
            "2026-10-07 08:00:00",
        );
        let s = cfg.to_json();
        assert_eq!(ToolPaths::parse(&s), Some(cfg.clone()));
        // Extra keys (e.g. Setup-Windows installDir) are tolerated.
        let extra = "{\"adb\":\"a\",\"fastboot\":\"b\",\"scrcpy\":\"\",\"updated\":\"u\",\"installDir\":\"C:\\\\x\"}";
        assert_eq!(
            ToolPaths::parse(extra),
            Some(ToolPaths::new("a", "b", "", "u"))
        );
        // Escapes survive the roundtrip.
        let tricky = ToolPaths::new("q\"q", "back\\slash", "a\nb", "u");
        assert_eq!(ToolPaths::parse(&tricky.to_json()), Some(tricky));
        // Missing keys and garbage fail soft.
        assert_eq!(ToolPaths::parse("{\"adb\":\"a\"}"), None);
        assert_eq!(ToolPaths::parse("not json"), None);
        // Save/load roundtrip incl. parent-dir creation.
        let d = fixture_dir("cfg");
        let f = d.join("sub").join("config.json");
        assert!(save_tool_config(&f, &cfg).is_ok());
        assert_eq!(load_tool_config(&f), Some(cfg));
        assert_eq!(load_tool_config(&d.join("absent.json")), None);
        cleanup(&d);
    }

    #[test]
    fn compat_discovery_over_fixture_dir() {
        let d = fixture_dir("compat");
        let sub = d
            .join("compatibility")
            .join("huawei")
            .join("p10");
        let _ = std::fs::create_dir_all(&sub);
        let _ = std::fs::write(sub.join("VTR-L29.json"), "{}");
        let _ = std::fs::write(sub.join("VKY-L29.json"), "{}");
        let _ = std::fs::write(sub.join("notes.txt"), "x");
        let p = compat_profile_path(&d, "VTR-L29");
        assert_eq!(p.file_name().and_then(|n| n.to_str()), Some("VTR-L29.json"));
        assert_eq!(resolve_compat_profile(&d, "VTR-L29"), Some(p));
        assert_eq!(resolve_compat_profile(&d, "UNKNOWN"), None);
        assert_eq!(
            available_profiles(&d),
            vec!["VKY-L29".to_string(), "VTR-L29".to_string()]
        );
        assert!(available_profiles(&d.join("empty")).is_empty());
        cleanup(&d);
    }

    #[test]
    fn roots_and_install_rules() {
        assert_eq!(
            script_root(Some(std::path::Path::new("/x/scripts"))),
            PathBuf::from("/x/scripts")
        );
        assert_eq!(
            tool_root(std::path::Path::new("/x/scripts")),
            PathBuf::from("/x")
        );
        assert_eq!(
            tool_root(std::path::Path::new("/x/other")),
            PathBuf::from("/x/other")
        );
        let prev = std::env::var_os("TT_SCRIPT_ROOT");
        std::env::set_var("TT_SCRIPT_ROOT", "/tmp/tt-script-root-test");
        let got = script_root(None);
        match prev {
            Some(v) => std::env::set_var("TT_SCRIPT_ROOT", v),
            None => std::env::remove_var("TT_SCRIPT_ROOT"),
        }
        assert_eq!(got, PathBuf::from("/tmp/tt-script-root-test"));
        assert_eq!(
            install_base_dir(
                std::path::Path::new("/repo/data/tools"),
                true,
                std::path::Path::new("/home/u")
            ),
            PathBuf::from("/repo/data/tools")
        );
        assert_eq!(
            install_base_dir(
                std::path::Path::new("/repo/data/tools"),
                false,
                std::path::Path::new("/home/u")
            ),
            PathBuf::from("/home/u/.local/share/trebleManager/tools")
        );
        let plan = tool_link_plan(
            std::path::Path::new("/src"),
            std::path::Path::new("/base"),
        );
        assert_eq!(plan.len(), 6);
        assert_eq!(
            plan[0],
            (PathBuf::from("/src/adb"), PathBuf::from("/base/adb"))
        );
    }

    #[test]
    fn tool_scan_finds_executables() {
        let d = fixture_dir("scan");
        let adb = write_file(&d, "adb", b"x");
        let fb = write_file(&d, "fastboot.exe", b"x");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            for p in [&adb, &fb] {
                if let Ok(m) = std::fs::metadata(p) {
                    let mut perm = m.permissions();
                    perm.set_mode(0o755);
                    let _ = std::fs::set_permissions(p, perm);
                }
            }
        }
        let hits = scan_dir_for_tools(&d);
        assert_eq!(hits.len(), 2);
        assert_eq!(hits[0], ("adb", adb));
        assert_eq!(hits[1], ("fastboot", fb));
        cleanup(&d);
    }
}
