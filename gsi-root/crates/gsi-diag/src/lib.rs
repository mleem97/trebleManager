//! Host-side diagnostic, backup, export, validation, and developer-dump planning.
//!
//! Pure planning plus ZIP bundle assembly. No device access, no network,
//! no process spawning. Device-bound execution paths return honest refusal
//! errors that name the missing live-device step (Phase 8).
//!
//! Script sources: `scripts/Treble-Toolkit.ps1` (`New-TTDiagnostic`,
//! `New-TTBackup`, `Export-RecoveryFromRom`, `Write-ValidationReport`,
//! `Invoke-DeveloperDump`) and `scripts/treble-toolkit.sh` (`do_diagnostic`,
//! `do_backup`, `export_recovery`, `developer_dump`, `validate_device`).
//! Archive kinds come from `gsi_archive::classify`; image kinds (`boot`,
//! `system`, `unknown`) match `Test-ImageKind` / `image_kind` (ANDROID!
//! magic vs sparse/ext4 filesystem, see the `gsi-fs` extents reader).

use std::io::{Cursor, Write};
use std::path::{Component, Path, PathBuf};

/// Upper bound for total bundled bytes (texts plus staged files).
pub const MAX_BUNDLE_BYTES: u64 = 64 * 1024 * 1024;

/// Marker inserted for redacted values.
pub const REDACTED: &str = "<redacted>";

// ------------------------------------------------------------- redaction

/// Redact device identifiers in free text.
///
/// Heuristic and documented: keyword values (`serial`, `serialno`, `imei`,
/// `imsi`, `meid`, `uuid`, `android_id`), the first column of
/// `adb`/`fastboot devices`-style lines, and long pure-digit runs (14 or
/// more digits, e.g. IMEI/IMSI). Hash lines without keywords pass through
/// unchanged.
pub fn redact(text: &str) -> String {
    let mut lines: Vec<String> = Vec::new();
    for line in text.split('\n') {
        lines.push(redact_line(line));
    }
    lines.join("\n")
}

fn redact_line(line: &str) -> String {
    let step1 = redact_device_column(line);
    let step2 = redact_keywords(&step1);
    redact_digit_runs(&step2)
}

/// Replace the first column when the second column is a known device state
/// (`device`, `unauthorized`, `offline`, `recovery`, `sideload`, `fastboot`,
/// `bootloader`). Separators after the first token are preserved.
fn redact_device_column(line: &str) -> String {
    const STATES: &[&str] = &[
        "device",
        "unauthorized",
        "offline",
        "recovery",
        "sideload",
        "fastboot",
        "bootloader",
    ];
    let bytes = line.as_bytes();
    let mut start = 0;
    while start < bytes.len() && (bytes[start] == b' ' || bytes[start] == b'\t') {
        start += 1;
    }
    let mut end = start;
    while end < bytes.len() && bytes[end] != b' ' && bytes[end] != b'\t' && bytes[end] != b'\r' {
        end += 1;
    }
    if start >= end {
        return line.to_string();
    }
    let mut sep = end;
    while sep < bytes.len() && (bytes[sep] == b' ' || bytes[sep] == b'\t') {
        sep += 1;
    }
    let mut tok_end = sep;
    while tok_end < bytes.len()
        && bytes[tok_end] != b' '
        && bytes[tok_end] != b'\t'
        && bytes[tok_end] != b'\r'
    {
        tok_end += 1;
    }
    if sep >= tok_end {
        return line.to_string();
    }
    let second = match line.get(sep..tok_end) {
        Some(s) => s,
        None => "",
    };
    let low = second.to_ascii_lowercase();
    let known = STATES.iter().any(|s| low == *s);
    if !known {
        return line.to_string();
    }
    let first = match line.get(start..end) {
        Some(s) => s,
        None => "",
    };
    if first == REDACTED || first.is_empty() {
        return line.to_string();
    }
    // Header lines such as "List of devices attached" have more than two
    // leading words; only redact single-token first columns.
    if first.contains(':') {
        return line.to_string();
    }
    let mut out = String::new();
    out.push_str(match line.get(..start) {
        Some(s) => s,
        None => "",
    });
    out.push_str(REDACTED);
    out.push_str(match line.get(end..) {
        Some(s) => s,
        None => "",
    });
    out
}

fn is_word_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

/// Replace values after identifier keywords with [`REDACTED`].
/// Longer keywords win so `serialno` is not split as `serial` + `no`.
fn redact_keywords(s: &str) -> String {
    const KEYS: &[&str] = &[
        "android_id",
        "android-id",
        "serial_no",
        "serialno",
        "serial",
        "imei",
        "imsi",
        "meid",
        "uuid",
    ];
    let bytes = s.as_bytes();
    let lower: Vec<u8> = bytes.iter().map(|b| b.to_ascii_lowercase()).collect();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        let mut hit_len = 0;
        for key in KEYS {
            let kb = key.as_bytes();
            if i + kb.len() > lower.len() {
                continue;
            }
            if lower[i..i + kb.len()] != kb[..] {
                continue;
            }
            let before_ok = i == 0 || !is_word_byte(lower[i - 1]);
            let after_ok = i + kb.len() >= lower.len() || !is_word_byte(lower[i + kb.len()]);
            if before_ok && after_ok {
                hit_len = kb.len();
                break;
            }
        }
        if hit_len == 0 {
            out.push(bytes[i]);
            i += 1;
            continue;
        }
        out.extend_from_slice(&bytes[i..i + hit_len]);
        let mut j = i + hit_len;
        while j < bytes.len()
            && (bytes[j] == b' '
                || bytes[j] == b'\t'
                || bytes[j] == b':'
                || bytes[j] == b'='
                || bytes[j] == b'"'
                || bytes[j] == b'\'')
        {
            out.push(bytes[j]);
            j += 1;
        }
        let mut v = j;
        while v < bytes.len()
            && bytes[v] != b' '
            && bytes[v] != b'\t'
            && bytes[v] != b'\r'
            && bytes[v] != b'\n'
            && bytes[v] != b','
            && bytes[v] != b';'
            && bytes[v] != b'"'
            && bytes[v] != b'\''
        {
            v += 1;
        }
        if v > j {
            let val = &bytes[j..v];
            if val != REDACTED.as_bytes() {
                out.extend_from_slice(REDACTED.as_bytes());
            } else {
                out.extend_from_slice(val);
            }
            i = v;
        } else {
            i = j;
        }
    }
    match String::from_utf8(out) {
        Ok(s2) => s2,
        Err(_) => s.to_string(),
    }
}

/// Replace pure-digit runs of length 14 or more with [`REDACTED`].
fn redact_digit_runs(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if !bytes[i].is_ascii_digit() {
            out.push(bytes[i]);
            i += 1;
            continue;
        }
        let mut j = i;
        while j < bytes.len() && bytes[j].is_ascii_digit() {
            j += 1;
        }
        if j - i >= 14 {
            out.extend_from_slice(REDACTED.as_bytes());
        } else {
            out.extend_from_slice(&bytes[i..j]);
        }
        i = j;
    }
    match String::from_utf8(out) {
        Ok(s2) => s2,
        Err(_) => s.to_string(),
    }
}

// ------------------------------------------------- diagnostic bundle

fn check_entry_name(name: &str) -> Result<(), String> {
    if name.is_empty() {
        return Err("refused: empty entry name".to_string());
    }
    if name.len() > 200 {
        return Err(format!("refused: entry name too long: {name}"));
    }
    if name.starts_with('/') || name.starts_with('\\') {
        return Err(format!("refused: absolute entry name: {name}"));
    }
    if name.contains('\\') || name.contains(':') {
        return Err(format!("refused: bad entry name: {name}"));
    }
    let path = Path::new(name);
    for comp in path.components() {
        match comp {
            Component::ParentDir => {
                return Err(format!("refused: entry escapes dir: {name}"));
            }
            Component::RootDir | Component::Prefix(_) => {
                return Err(format!("refused: absolute entry name: {name}"));
            }
            Component::CurDir | Component::Normal(_) => {}
        }
    }
    Ok(())
}

fn collect_files(
    base: &Path,
    dir: &Path,
    staged: &mut Vec<(String, Vec<u8>)>,
    total: &mut u64,
) -> Result<(), String> {
    let entries = std::fs::read_dir(dir).map_err(|e| format!("read dir: {e}"))?;
    for entry in entries {
        let entry = entry.map_err(|e| format!("dir entry: {e}"))?;
        let ftype = entry.file_type().map_err(|e| format!("file type: {e}"))?;
        if ftype.is_dir() {
            collect_files(base, &entry.path(), staged, total)?;
        } else if ftype.is_file() {
            let rel = entry
                .path()
                .strip_prefix(base)
                .map_err(|e| format!("rel path: {e}"))?
                .to_path_buf();
            let name = rel.to_string_lossy().replace('\\', "/");
            check_entry_name(&name)?;
            let data = std::fs::read(entry.path()).map_err(|e| format!("read file: {e}"))?;
            *total = total.saturating_add(data.len() as u64);
            if *total > MAX_BUNDLE_BYTES {
                return Err("refused: bundle exceeds 64 MiB cap".to_string());
            }
            staged.push((name, data));
        }
        // Skip symlinks and other special files: never follow off-dir links.
    }
    Ok(())
}

/// Build a diagnostic ZIP archive.
///
/// `texts` are named text entries (`device.json`, `adb.txt`, ... like
/// `New-TTDiagnostic` / `do_diagnostic`). With `anonymize` set, each text
/// passes through [`redact`]. `files_dir`, when given, contributes its
/// regular files under relative names (raw bytes, no redaction).
/// Returns the ZIP bytes.
pub fn build_diagnostic_zip(
    texts: &[(&str, &str)],
    files_dir: Option<&Path>,
    anonymize: bool,
) -> Result<Vec<u8>, String> {
    let mut total: u64 = 0;
    for (name, body) in texts {
        check_entry_name(name)?;
        total = total.saturating_add(body.len() as u64);
    }
    let mut staged: Vec<(String, Vec<u8>)> = Vec::new();
    if let Some(dir) = files_dir {
        collect_files(dir, dir, &mut staged, &mut total)?;
    }
    if total > MAX_BUNDLE_BYTES {
        return Err("refused: bundle exceeds 64 MiB cap".to_string());
    }
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let opts = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    for (name, body) in texts {
        let text = if anonymize {
            redact(body)
        } else {
            body.to_string()
        };
        writer
            .start_file(*name, opts)
            .map_err(|e| format!("zip start: {e}"))?;
        writer
            .write_all(text.as_bytes())
            .map_err(|e| format!("zip write: {e}"))?;
    }
    for (name, data) in &staged {
        writer
            .start_file(name.as_str(), opts)
            .map_err(|e| format!("zip start: {e}"))?;
        writer
            .write_all(data)
            .map_err(|e| format!("zip write: {e}"))?;
    }
    let cursor = writer.finish().map_err(|e| format!("zip finish: {e}"))?;
    Ok(cursor.into_inner())
}

// ------------------------------------------------------- backup plan

/// Planned stock-partition backup.
///
/// Mirrors the fields `New-TTBackup` / `do_backup` capture: target
/// partition, destination dir, device model, firmware baseline, and the
/// source note. Planning only; running the dump needs a live device.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackupPlan {
    pub partition: String,
    pub dest_dir: String,
    pub model: String,
    pub firmware: String,
    pub source: String,
}

/// Build a backup plan. An empty model falls back to `VTR-L29` like the
/// scripts; empty partition or dest dir is refused.
pub fn plan_backup(
    partition: &str,
    dest_dir: &str,
    model: &str,
    firmware: &str,
) -> Result<BackupPlan, String> {
    if partition.trim().is_empty() {
        return Err("refused: empty partition".to_string());
    }
    if dest_dir.trim().is_empty() {
        return Err("refused: empty dest dir".to_string());
    }
    let model_out = if model.trim().is_empty() {
        "VTR-L29".to_string()
    } else {
        model.to_string()
    };
    Ok(BackupPlan {
        partition: partition.to_string(),
        dest_dir: dest_dir.to_string(),
        model: model_out,
        firmware: firmware.to_string(),
        source: "firmware-extracted".to_string(),
    })
}

/// Refuse backup execution. The device dump path (`dd` off the live
/// partition) needs a live device and stays Phase 8 work; the plan type
/// above is the portable part.
pub fn exec_backup(plan: &BackupPlan) -> Result<String, String> {
    Err(format!(
        "refused: device dump of '{}' needs a live device (Phase 8); plan at '{}' recorded, nothing executed",
        plan.partition, plan.dest_dir
    ))
}

fn quote_json(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
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
    out.push('"');
    out
}

/// Render backup metadata JSON (`metadata.json` next to `original.img`).
pub fn backup_metadata_json(plan: &BackupPlan) -> String {
    let mut s = String::from("{");
    s.push_str("\"model\":");
    s.push_str(&quote_json(&plan.model));
    s.push_str(",\"partition\":");
    s.push_str(&quote_json(&plan.partition));
    s.push_str(",\"dest_dir\":");
    s.push_str(&quote_json(&plan.dest_dir));
    s.push_str(",\"firmware\":");
    s.push_str(&quote_json(&plan.firmware));
    s.push_str(",\"source\":");
    s.push_str(&quote_json(&plan.source));
    s.push_str(",\"tool\":\"gsi-diag\"}");
    s
}

// ------------------------------------------------- export planner

/// Planned recovery export (inputs only; extraction stays with the caller).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportPlan {
    /// `direct-img` | `rom-zip` | `rom-tar` | `wrapper-gz` | `wrapper-xz`.
    pub kind: String,
    /// ROM base name with the archive suffix stripped.
    pub base_name: String,
    /// Ordered hints; repack/write is always listed as open, never done.
    pub notes: Vec<String>,
}

fn system_refusal() -> String {
    "refused: SYSTEM image (filesystem with folders) cannot be a Magisk patch base; Magisk needs boot/recovery; a GSI leaves recovery stock, use the stock UPDATE.APP recovery; repack/write open, not executed here".to_string()
}

/// Strip one archive suffix in script order (`.tar.gz` first, like
/// `Export-RecoveryFromRom`); fall back to `rom` on empty stems.
fn export_base_name(rom_file_name: &str) -> String {
    const SUFFIXES: &[&str] = &[
        ".tar.gz", ".tar.xz", ".tgz", ".gz", ".xz", ".zip", ".img", ".tar", ".app",
    ];
    let mut base = match rom_file_name.rsplit(['/', '\\']).next() {
        Some(b) => b,
        None => rom_file_name,
    };
    if base.is_empty() {
        base = "rom";
    }
    let lower = base.to_lowercase();
    for sfx in SUFFIXES {
        if lower.ends_with(sfx) && base.len() >= sfx.len() {
            let cut = base.len() - sfx.len();
            match base.get(..cut) {
                Some(prefix) => {
                    base = prefix;
                    break;
                }
                None => continue,
            }
        }
    }
    if base.is_empty() {
        "rom".to_string()
    } else {
        base.to_string()
    }
}

/// Plan a recovery export from a ROM package name plus an image kind.
///
/// `rom_file_name` is classified with `gsi_archive::classify`;
/// `image_kind` is `boot` (ANDROID! magic), `system` (sparse/ext4, see the
/// `gsi-fs` extents reader), or `unknown` (mirrors `Test-ImageKind`).
/// A system image as patch base is refused like the scripts; repack/write
/// stays open and is stated in the plan notes.
pub fn plan_export(rom_file_name: &str, image_kind: &str) -> Result<ExportPlan, String> {
    let base = export_base_name(rom_file_name);
    let kind = gsi_archive::classify(rom_file_name);
    let img = image_kind.trim().to_lowercase();
    let open_note = "repack/write: open, stated here, not executed".to_string();
    match kind {
        gsi_archive::ArchiveKind::Zip => Ok(ExportPlan {
            kind: "rom-zip".to_string(),
            base_name: base,
            notes: vec![
                "extract entries matching *recovery*.img or boot.img; validate ANDROID! magic per file"
                    .to_string(),
                "payload.bin path needs an external payload dumper (not bundled)".to_string(),
                "empty candidate set means a GSI system package: refuse as no-recovery".to_string(),
                open_note,
            ],
        }),
        gsi_archive::ArchiveKind::Tar
        | gsi_archive::ArchiveKind::TarGz
        | gsi_archive::ArchiveKind::TarXz
        | gsi_archive::ArchiveKind::Tgz => Ok(ExportPlan {
            kind: "rom-tar".to_string(),
            base_name: base,
            notes: vec![
                "extract tar container, then copy *recovery*.img / boot.img; validate ANDROID! magic per file"
                    .to_string(),
                "payload.bin inside needs an external payload dumper (not bundled)".to_string(),
                "empty candidate set means a GSI system package: refuse as no-recovery".to_string(),
                open_note,
            ],
        }),
        gsi_archive::ArchiveKind::GzipSingle => {
            if img == "system" {
                Err(system_refusal())
            } else {
                Ok(ExportPlan {
                    kind: "wrapper-gz".to_string(),
                    base_name: base,
                    notes: vec![
                        "decompress the wrapper, then dispatch the inner file as .img".to_string(),
                        "validate ANDROID! magic after decompression".to_string(),
                        open_note,
                    ],
                })
            }
        }
        gsi_archive::ArchiveKind::XzSingle => {
            if img == "system" {
                Err(system_refusal())
            } else {
                Ok(ExportPlan {
                    kind: "wrapper-xz".to_string(),
                    base_name: base,
                    notes: vec![
                        "decompress the wrapper, then dispatch the inner file as .img".to_string(),
                        "validate ANDROID! magic after decompression".to_string(),
                        open_note,
                    ],
                })
            }
        }
        gsi_archive::ArchiveKind::Plain => {
            let lower = rom_file_name.to_lowercase();
            if lower.ends_with(".img") {
                if img == "boot" {
                    Ok(ExportPlan {
                        kind: "direct-img".to_string(),
                        base_name: base,
                        notes: vec![
                            "copy the image, record sha256 plus metadata.json".to_string(),
                            "validate the target partition from the device profile before patch/flash"
                                .to_string(),
                            open_note,
                        ],
                    })
                } else if img == "system" {
                    Err(system_refusal())
                } else {
                    Err("refused: not an Android boot image (no ANDROID! magic); GSI system images contain no recovery, use the stock UPDATE.APP path".to_string())
                }
            } else {
                Err("refused: unsupported format (want .img, .img.gz/.img.xz, ROM .zip, .tar/.tar.gz/.tgz)".to_string())
            }
        }
    }
}

// ------------------------------------------------ validation report

/// One named validation check (mirrors `Invoke-TTValidate` items and
/// `validate_device` rows: ADB, OS, SELinux, ROOT, mounts, WIFI,
/// BLUETOOTH, BATTERY, SENSORS).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Check {
    pub name: String,
    pub pass: bool,
    pub detail: String,
}

/// Build one check row.
pub fn new_check(name: &str, pass: bool, detail: &str) -> Check {
    Check {
        name: name.to_string(),
        pass,
        detail: detail.to_string(),
    }
}

/// Overall verdict: true only when every check passes (empty set passes).
pub fn report_ok(checks: &[Check]) -> bool {
    checks.iter().all(|c| c.pass)
}

/// Render checks as markdown with an `Overall` trailer.
pub fn render_markdown(checks: &[Check]) -> String {
    let mut s = String::from("# Validation report\n\n");
    if checks.is_empty() {
        s.push_str("Overall: PASS\n");
        return s;
    }
    for c in checks {
        s.push_str("- [");
        s.push_str(if c.pass { "PASS" } else { "FAIL" });
        s.push_str("] ");
        s.push_str(&c.name);
        s.push_str(" -- ");
        s.push_str(&c.detail);
        s.push('\n');
    }
    s.push('\n');
    s.push_str("Overall: ");
    s.push_str(if report_ok(checks) { "PASS" } else { "FAIL" });
    s.push('\n');
    s
}

/// Render checks as a compact JSON string (manual escaping, no new deps).
pub fn render_json(checks: &[Check], tool: &str) -> String {
    let mut s = String::from("{\"tool\":");
    s.push_str(&quote_json(tool));
    s.push_str(",\"results\":[");
    for (i, c) in checks.iter().enumerate() {
        if i > 0 {
            s.push(',');
        }
        s.push_str("{\"name\":");
        s.push_str(&quote_json(&c.name));
        s.push_str(",\"pass\":");
        s.push_str(if c.pass { "true" } else { "false" });
        s.push_str(",\"detail\":");
        s.push_str(&quote_json(&c.detail));
        s.push('}');
    }
    s.push_str("]}");
    s
}

// ------------------------------------------------- developer dump

/// One caller-supplied tool version (name plus version string).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolVersion {
    pub tool: String,
    pub version: String,
}

/// Pure host info. All values are passed in by the caller; this crate never
/// spawns processes (mirrors the read-only `Invoke-DeveloperDump` /
/// `developer_dump` kinds without touching a device).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostInfo {
    pub os: String,
    pub arch: String,
    pub tool_versions: Vec<ToolVersion>,
}

/// Render host info as the `host.txt` bundle entry.
pub fn host_bundle_entry(info: &HostInfo) -> (String, String) {
    let mut text = String::new();
    text.push_str("os: ");
    text.push_str(&info.os);
    text.push('\n');
    text.push_str("arch: ");
    text.push_str(&info.arch);
    text.push('\n');
    for tv in &info.tool_versions {
        text.push_str(&tv.tool);
        text.push_str(": ");
        text.push_str(&tv.version);
        text.push('\n');
    }
    (String::from("host.txt"), text)
}

/// Build the dump file name for a developer-dump kind. Only the script
/// kinds are valid (`dump-partitions`, `dump-properties`, `dump-vendor`,
/// `dump-logs`); anything else is refused. `stamp` must look like
/// `yyyyMMdd-HHmmss`.
pub fn developer_dump_filename(kind: &str, stamp: &str) -> Result<String, String> {
    const KINDS: &[&str] = &[
        "dump-partitions",
        "dump-properties",
        "dump-vendor",
        "dump-logs",
    ];
    if !KINDS.contains(&kind) {
        return Err(format!(
            "refused: unknown dump kind '{kind}' (want dump-partitions|dump-properties|dump-vendor|dump-logs)"
        ));
    }
    if stamp.is_empty()
        || !stamp
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err("refused: bad stamp (want yyyyMMdd-HHmmss)".to_string());
    }
    Ok(format!("{kind}-{stamp}.txt"))
}

// ------------------------------------------------- log file list

/// One log file entry (name plus size in bytes, no content).
///
/// Mirrors `Screen-Logs` file listing: names plus sizes only, never a
/// content dump. Sorted by name for deterministic output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogFile {
    /// File name only (no directory part).
    pub name: String,
    /// File size in bytes (0 when unreadable).
    pub size: u64,
}

/// List regular log files directly under `log_dir`.
///
/// Pure over an explicit dir: a missing or unreadable dir yields an empty
/// list, subdirectories and symlinks are skipped, entries are sorted by
/// name. Never panics, never touches the network, a device, or file
/// content.
pub fn list_log_files(log_dir: &Path) -> Vec<LogFile> {
    let mut out: Vec<LogFile> = Vec::new();
    let entries = match std::fs::read_dir(log_dir) {
        Ok(e) => e,
        Err(_) => return out,
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
        if !ftype.is_file() {
            continue;
        }
        let name = match entry.file_name().into_string() {
            Ok(s) => s,
            Err(_) => continue,
        };
        if name.trim().is_empty() {
            continue;
        }
        let size = match entry.metadata() {
            Ok(m) => m.len(),
            Err(_) => 0,
        };
        out.push(LogFile { name, size });
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    out
}

// ------------------------------------------------- log filename + line

/// Build a log file name (`toolkit-<stamp>.log`).
///
/// Pure builder mirroring `Write-TTLog`/`log` (`toolkit-$STAMP.log`).
/// `stamp` is caller-provided (`yyyyMMdd-HHmmss`); no clock reads.
pub fn log_filename(stamp: &str) -> String {
    format!("toolkit-{stamp}.log")
}

/// Append one line as `[timestamp] line` plus newline.
///
/// All inputs are caller-provided; no clock reads. Creates `log_dir` when
/// missing. `name` must be a plain file name; `timestamp` must be a
/// single non-empty line (`yyyy-MM-dd HH:mm:ss` like the scripts).
/// Returns the file path.
pub fn write_log_line(
    log_dir: &Path,
    name: &str,
    timestamp: &str,
    line: &str,
) -> Result<PathBuf, String> {
    if name.is_empty() {
        return Err("refused: empty log name".to_string());
    }
    if name.len() > 200 {
        return Err(format!("refused: log name too long: {name}"));
    }
    if name.contains('/') || name.contains('\\') || name.contains(':') {
        return Err(format!("refused: bad log name: {name}"));
    }
    if name == "." || name == ".." {
        return Err(format!("refused: bad log name: {name}"));
    }
    if timestamp.is_empty() {
        return Err("refused: empty timestamp".to_string());
    }
    if timestamp.contains('\n') || timestamp.contains('\r') {
        return Err("refused: multi-line timestamp".to_string());
    }
    if line.contains('\n') || line.contains('\r') {
        return Err("refused: multi-line log line".to_string());
    }
    std::fs::create_dir_all(log_dir).map_err(|e| format!("mkdir: {e}"))?;
    let path = log_dir.join(name);
    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .map_err(|e| format!("open log: {e}"))?;
    let text = format!("[{timestamp}] {line}\n");
    f.write_all(text.as_bytes())
        .map_err(|e| format!("write log: {e}"))?;
    Ok(path)
}

// ------------------------------------------------- stamped report

/// Render checks as JSON with a caller-provided timestamp.
///
/// Mirrors `Write-ValidationReport` (`tool` + `timestamp` + `results`).
/// `stamp` is `yyyy-MM-dd HH:mm:ss`; no clock reads.
pub fn stamped_report(checks: &[Check], stamp: &str) -> String {
    let mut s = String::from("{\"tool\":");
    s.push_str(&quote_json("gsi-diag"));
    s.push_str(",\"timestamp\":");
    s.push_str(&quote_json(stamp));
    s.push_str(",\"results\":[");
    for (i, c) in checks.iter().enumerate() {
        if i > 0 {
            s.push(',');
        }
        s.push_str("{\"name\":");
        s.push_str(&quote_json(&c.name));
        s.push_str(",\"pass\":");
        s.push_str(if c.pass { "true" } else { "false" });
        s.push_str(",\"detail\":");
        s.push_str(&quote_json(&c.detail));
        s.push('}');
    }
    s.push_str("]}");
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static N: AtomicUsize = AtomicUsize::new(0);

    fn unique_dir(tag: &str) -> std::path::PathBuf {
        let n = N.fetch_add(1, Ordering::SeqCst);
        let mut p = std::env::temp_dir();
        p.push(format!("gsi-diag-test-{tag}-{}-{n}", std::process::id()));
        match std::fs::create_dir_all(&p) {
            Ok(()) => {}
            Err(e) => {
                assert!(false, "mkdir: {e}");
            }
        }
        p
    }

    fn read_zip_entry(bytes: &[u8], name: &str) -> Vec<u8> {
        let cursor = Cursor::new(bytes.to_vec());
        match zip::ZipArchive::new(cursor) {
            Ok(mut archive) => match archive.by_name(name) {
                Ok(mut f) => {
                    let mut buf = Vec::new();
                    match f.read_to_end(&mut buf) {
                        Ok(_) => buf,
                        Err(e) => {
                            assert!(false, "read entry: {e}");
                            Vec::new()
                        }
                    }
                }
                Err(e) => {
                    assert!(false, "missing entry {name}: {e}");
                    Vec::new()
                }
            },
            Err(e) => {
                assert!(false, "open zip: {e}");
                Vec::new()
            }
        }
    }

    fn zip_names(bytes: &[u8]) -> Vec<String> {
        let cursor = Cursor::new(bytes.to_vec());
        match zip::ZipArchive::new(cursor) {
            Ok(archive) => archive.file_names().map(|s| s.to_string()).collect(),
            Err(e) => {
                assert!(false, "open zip: {e}");
                Vec::new()
            }
        }
    }

    #[test]
    fn zip_roundtrip_texts_only() {
        let texts = [
            ("device.json", "{\"mode\":\"fastboot\"}"),
            ("adb.txt", "ok"),
        ];
        let bytes = match build_diagnostic_zip(&texts, None, false) {
            Ok(b) => b,
            Err(e) => {
                assert!(false, "build: {e}");
                Vec::new()
            }
        };
        assert!(!bytes.is_empty());
        assert_eq!(
            read_zip_entry(&bytes, "device.json"),
            b"{\"mode\":\"fastboot\"}".to_vec()
        );
        assert_eq!(read_zip_entry(&bytes, "adb.txt"), b"ok".to_vec());
        let mut names = zip_names(&bytes);
        names.sort();
        assert_eq!(
            names,
            vec!["adb.txt".to_string(), "device.json".to_string()]
        );
    }

    #[test]
    fn zip_applies_redaction_when_anonymized() {
        let texts = [("adb.txt", "ABCD1234\tdevice"), ("note.txt", "plain")];
        let bytes = match build_diagnostic_zip(&texts, None, true) {
            Ok(b) => b,
            Err(e) => {
                assert!(false, "build: {e}");
                Vec::new()
            }
        };
        let adb = read_zip_entry(&bytes, "adb.txt");
        let adb_s = match String::from_utf8(adb) {
            Ok(s) => s,
            Err(e) => {
                assert!(false, "utf8: {e}");
                String::new()
            }
        };
        assert!(adb_s.contains(REDACTED));
        assert!(!adb_s.contains("ABCD1234"));
        let raw = match build_diagnostic_zip(&texts, None, false) {
            Ok(b) => b,
            Err(e) => {
                assert!(false, "build: {e}");
                Vec::new()
            }
        };
        let adb_raw = read_zip_entry(&raw, "adb.txt");
        assert_eq!(adb_raw, b"ABCD1234\tdevice".to_vec());
    }

    #[test]
    fn zip_includes_files_from_dir() {
        let dir = unique_dir("files");
        match std::fs::write(dir.join("log.txt"), b"log line") {
            Ok(()) => {}
            Err(e) => {
                assert!(false, "write: {e}");
            }
        }
        match std::fs::create_dir_all(dir.join("sub")) {
            Ok(()) => {}
            Err(e) => {
                assert!(false, "mkdir sub: {e}");
            }
        }
        match std::fs::write(dir.join("sub").join("inner.txt"), b"inner") {
            Ok(()) => {}
            Err(e) => {
                assert!(false, "write: {e}");
            }
        }
        let texts = [("device.json", "{}")];
        let bytes = match build_diagnostic_zip(&texts, Some(dir.as_path()), false) {
            Ok(b) => b,
            Err(e) => {
                assert!(false, "build: {e}");
                Vec::new()
            }
        };
        assert_eq!(read_zip_entry(&bytes, "log.txt"), b"log line".to_vec());
        assert_eq!(read_zip_entry(&bytes, "sub/inner.txt"), b"inner".to_vec());
        match std::fs::remove_dir_all(&dir) {
            Ok(()) => {}
            Err(_) => {}
        }
    }

    #[test]
    fn zip_refuses_bad_entry_names() {
        for bad in ["", "../evil.txt", "/abs.txt", "a\\b.txt", "c:d.txt"] {
            let texts = [(bad, "x")];
            assert!(build_diagnostic_zip(&texts, None, false).is_err());
        }
    }

    #[test]
    fn redaction_cases() {
        assert_eq!(redact("serial: ABC123XYZ"), "serial: <redacted>");
        assert_eq!(redact("IMEI=490154203237518"), "IMEI=<redacted>");
        assert_eq!(redact("my serialno: XYZ987ABC"), "my serialno: <redacted>");
        assert_eq!(redact("ABCD1234\tdevice"), "<redacted>\tdevice");
        assert_eq!(redact("ABCD1234\tfastboot"), "<redacted>\tfastboot");
        assert_eq!(redact("call 490154203237518 now"), "call <redacted> now");
        // Hash and version lines without keywords pass through.
        let hash = "SHA256: 9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08";
        assert_eq!(redact(hash), hash);
        assert_eq!(
            redact("recovery_ramdisk present, version 9.1.0"),
            "recovery_ramdisk present, version 9.1.0"
        );
        assert_eq!(
            redact("List of devices attached"),
            "List of devices attached"
        );
    }

    #[test]
    fn backup_plan_and_refusal() {
        let plan = match plan_backup("recovery_ramdisk", "/tmp/d", "", "9.1.0") {
            Ok(p) => p,
            Err(e) => {
                assert!(false, "plan: {e}");
                BackupPlan {
                    partition: String::new(),
                    dest_dir: String::new(),
                    model: String::new(),
                    firmware: String::new(),
                    source: String::new(),
                }
            }
        };
        assert_eq!(plan.model, "VTR-L29");
        assert_eq!(plan.source, "firmware-extracted");
        let meta = backup_metadata_json(&plan);
        assert!(meta.contains("\"partition\":\"recovery_ramdisk\""));
        assert!(meta.contains("\"model\":\"VTR-L29\""));
        let err = match exec_backup(&plan) {
            Ok(_) => {
                assert!(false, "exec must refuse");
                String::new()
            }
            Err(e) => e,
        };
        assert!(err.contains("live device"));
        assert!(err.contains("Phase 8"));
        assert!(plan_backup("", "/tmp/d", "VTR-L29", "").is_err());
        assert!(plan_backup("recovery_ramdisk", "", "VTR-L29", "").is_err());
    }

    #[test]
    fn planner_branch_matrix() {
        let ok = match plan_export("boot.img", "boot") {
            Ok(p) => p,
            Err(e) => {
                assert!(false, "plan: {e}");
                ExportPlan {
                    kind: String::new(),
                    base_name: String::new(),
                    notes: Vec::new(),
                }
            }
        };
        assert_eq!(ok.kind, "direct-img");
        assert_eq!(ok.base_name, "boot");
        assert!(plan_export("lineage.img", "system").is_err());
        let sys_err = match plan_export("lineage.img", "system") {
            Ok(_) => {
                assert!(false, "system must refuse");
                String::new()
            }
            Err(e) => e,
        };
        assert!(sys_err.contains("SYSTEM image"));
        assert!(plan_export("blob.img", "unknown").is_err());
        let zip_plan = match plan_export("custom-rom.zip", "unknown") {
            Ok(p) => p,
            Err(e) => {
                assert!(false, "plan: {e}");
                ExportPlan {
                    kind: String::new(),
                    base_name: String::new(),
                    notes: Vec::new(),
                }
            }
        };
        assert_eq!(zip_plan.kind, "rom-zip");
        assert_eq!(zip_plan.base_name, "custom-rom");
        for name in ["rom.tar", "rom.tar.gz", "rom.tgz", "rom.tar.xz"] {
            let p = match plan_export(name, "unknown") {
                Ok(p) => p,
                Err(e) => {
                    assert!(false, "plan {name}: {e}");
                    ExportPlan {
                        kind: String::new(),
                        base_name: String::new(),
                        notes: Vec::new(),
                    }
                }
            };
            assert_eq!(p.kind, "rom-tar");
        }
        let gz = match plan_export("sys.img.gz", "boot") {
            Ok(p) => p,
            Err(e) => {
                assert!(false, "plan: {e}");
                ExportPlan {
                    kind: String::new(),
                    base_name: String::new(),
                    notes: Vec::new(),
                }
            }
        };
        assert_eq!(gz.kind, "wrapper-gz");
        let xz = match plan_export("sys.img.xz", "boot") {
            Ok(p) => p,
            Err(e) => {
                assert!(false, "plan: {e}");
                ExportPlan {
                    kind: String::new(),
                    base_name: String::new(),
                    notes: Vec::new(),
                }
            }
        };
        assert_eq!(xz.kind, "wrapper-xz");
        assert!(plan_export("sys.img.gz", "system").is_err());
        assert!(plan_export("fw.app", "boot").is_err());
        assert!(plan_export("notes.txt", "unknown").is_err());
    }

    #[test]
    fn report_builder_golden_strings() {
        let checks = vec![
            new_check("ADB", true, "ABCD1234"),
            new_check("ROOT", false, "uid!=0"),
        ];
        assert!(!report_ok(&checks));
        assert!(report_ok(&[]));
        assert_eq!(
            render_markdown(&checks),
            "# Validation report\n\n- [PASS] ADB -- ABCD1234\n- [FAIL] ROOT -- uid!=0\n\nOverall: FAIL\n"
        );
        assert_eq!(
            render_markdown(&[]),
            "# Validation report\n\nOverall: PASS\n"
        );
        assert_eq!(
            render_json(&checks, "gsi-diag"),
            "{\"tool\":\"gsi-diag\",\"results\":[{\"name\":\"ADB\",\"pass\":true,\"detail\":\"ABCD1234\"},{\"name\":\"ROOT\",\"pass\":false,\"detail\":\"uid!=0\"}]}"
        );
        assert_eq!(
            render_json(&[], "gsi-diag"),
            "{\"tool\":\"gsi-diag\",\"results\":[]}"
        );
        let tricky = vec![new_check("A\"B", true, "x\ny\\z")];
        assert_eq!(
            render_json(&tricky, "t"),
            "{\"tool\":\"t\",\"results\":[{\"name\":\"A\\\"B\",\"pass\":true,\"detail\":\"x\\ny\\\\z\"}]}"
        );
    }

    #[test]
    fn developer_dump_kinds_and_host_entry() {
        let name = match developer_dump_filename("dump-partitions", "20240101-120000") {
            Ok(n) => n,
            Err(e) => {
                assert!(false, "filename: {e}");
                String::new()
            }
        };
        assert_eq!(name, "dump-partitions-20240101-120000.txt");
        assert!(developer_dump_filename("dump-bogus", "20240101-120000").is_err());
        assert!(developer_dump_filename("dump-logs", "bad stamp!").is_err());
        assert!(developer_dump_filename("dump-logs", "").is_err());
        let info = HostInfo {
            os: "windows".to_string(),
            arch: "x86_64".to_string(),
            tool_versions: vec![ToolVersion {
                tool: "adb".to_string(),
                version: "1.0.41".to_string(),
            }],
        };
        let (entry_name, entry_text) = host_bundle_entry(&info);
        assert_eq!(entry_name, "host.txt");
        assert!(entry_text.contains("os: windows"));
        assert!(entry_text.contains("arch: x86_64"));
        assert!(entry_text.contains("adb: 1.0.41"));
        // Host entry composes into a bundle.
        let texts = [(entry_name.as_str(), entry_text.as_str())];
        let bytes = match build_diagnostic_zip(&texts, None, false) {
            Ok(b) => b,
            Err(e) => {
                assert!(false, "build: {e}");
                Vec::new()
            }
        };
        assert!(zip_names(&bytes).contains(&"host.txt".to_string()));
    }

    #[test]
    fn log_files_list_names_and_sizes() {
        let dir = unique_dir("logs");
        match std::fs::write(dir.join("b.log"), b"12345") {
            Ok(()) => {}
            Err(e) => {
                assert!(false, "write: {e}");
            }
        }
        match std::fs::write(dir.join("a.log"), b"12") {
            Ok(()) => {}
            Err(e) => {
                assert!(false, "write: {e}");
            }
        }
        match std::fs::create_dir_all(dir.join("sub")) {
            Ok(()) => {}
            Err(e) => {
                assert!(false, "mkdir sub: {e}");
            }
        }
        let files = list_log_files(dir.as_path());
        assert_eq!(files.len(), 2);
        assert_eq!(files[0].name, "a.log");
        assert_eq!(files[0].size, 2);
        assert_eq!(files[1].name, "b.log");
        assert_eq!(files[1].size, 5);
        // Missing dir yields empty list, never fails.
        let missing = dir.join("nope");
        let empty = list_log_files(missing.as_path());
        assert!(empty.is_empty());
        match std::fs::remove_dir_all(&dir) {
            Ok(()) => {}
            Err(_) => {}
        }
    }

    #[test]
    fn log_filename_golden() {
        assert_eq!(log_filename("20240101-120000"), "toolkit-20240101-120000.log");
        assert_eq!(log_filename("20261231-235959"), "toolkit-20261231-235959.log");
    }

    #[test]
    fn write_log_line_golden_bytes() {
        let dir = unique_dir("writelog");
        let name = log_filename("20240101-120000");
        assert_eq!(name, "toolkit-20240101-120000.log");
        let stamp = "2024-01-01 12:00:00";
        match write_log_line(dir.as_path(), &name, stamp, "INFO hello") {
            Ok(p) => assert_eq!(p, dir.join(&name)),
            Err(e) => assert!(false, "write: {e}"),
        }
        match write_log_line(dir.as_path(), &name, "2024-01-01 12:00:01", "ERROR boom") {
            Ok(_) => {}
            Err(e) => assert!(false, "append: {e}"),
        }
        let raw = match std::fs::read(dir.join(&name)) {
            Ok(b) => b,
            Err(e) => {
                assert!(false, "read: {e}");
                Vec::new()
            }
        };
        assert_eq!(
            raw,
            b"[2024-01-01 12:00:00] INFO hello\n[2024-01-01 12:00:01] ERROR boom\n".to_vec()
        );
        // Bad inputs refused, no clock reads anywhere.
        assert!(write_log_line(dir.as_path(), "../evil.log", stamp, "x").is_err());
        assert!(write_log_line(dir.as_path(), "a/b.log", stamp, "x").is_err());
        assert!(write_log_line(dir.as_path(), &name, "", "x").is_err());
        assert!(write_log_line(dir.as_path(), &name, stamp, "a\nb").is_err());
        match std::fs::remove_dir_all(&dir) {
            Ok(()) => {}
            Err(_) => {}
        }
    }

    #[test]
    fn stamped_report_golden() {
        let checks = vec![
            new_check("ADB", true, "ok"),
            new_check("ROOT", false, "uid!=0"),
        ];
        assert_eq!(
            stamped_report(&checks, "2024-01-01 12:00:00"),
            "{\"tool\":\"gsi-diag\",\"timestamp\":\"2024-01-01 12:00:00\",\"results\":[{\"name\":\"ADB\",\"pass\":true,\"detail\":\"ok\"},{\"name\":\"ROOT\",\"pass\":false,\"detail\":\"uid!=0\"}]}"
        );
        assert_eq!(
            stamped_report(&[], "2024-01-01 12:00:00"),
            "{\"tool\":\"gsi-diag\",\"timestamp\":\"2024-01-01 12:00:00\",\"results\":[]}"
        );
        let tricky = vec![new_check("A\"B", true, "x\ny\\z")];
        assert_eq!(
            stamped_report(&tricky, "2024-01-02 03:04:05"),
            "{\"tool\":\"gsi-diag\",\"timestamp\":\"2024-01-02 03:04:05\",\"results\":[{\"name\":\"A\\\"B\",\"pass\":true,\"detail\":\"x\\ny\\\\z\"}]}"
        );
    }
}
