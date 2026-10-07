//! Firmware GUI center renders (read-only, never executing).
//!
//! Covers the Firmware center cluster: `FirmwareCenter` (baseline plus
//! local discovery/analysis/metadata/hashes), `DownloadCenter` (options
//! plus progress display), `ExtractCenter` (archive preview plus extractor
//! tool status table) and `ExportCenter` (recovery export plan).
//!
//! Reuse, never duplicate:
//! - baseline/compat bodies delegate to `crate::pages_media`
//!   (`firmware_text`, `download_text`, `extract_text`, `export_text`).
//! - archive kind + container probing stays behind `gsi-archive`
//!   (`classify`, `probe_update_app`); this module renders the
//!   caller-measured name/header only, never reimplements classification.
//! - download hashing and firmware checks stay behind `gsi-update`
//!   (`verify_hash_sidecar`, `verify_downloaded_firmware`,
//!   `download_to_with_progress`, `download_file_with_progress`,
//!   `bootstrap_install`); progress lines below are caller-measured.
//! - extractor lookup stays behind `gsi-tool::locate` plus the
//!   `gsi-config` tool-dir helpers; this module only renders the table.
//!
//! Execution honesty: the real download runs on a worker thread. The
//! render below only accepts already-measured progress lines; the thread
//! pump is shell-owned (see `DownloadCenter` and `download_center_text`).
//!
//! Slint side lives in `ui/center_firmware.slint` (`FirmwareCenter`,
//! `DownloadCenter`, `ExtractCenter`, `ExportCenter`) with the same visual
//! pattern as the Detect page (baked title, note, Refresh button,
//! read-only log view) and the component contract `in-out property
//! <string> result; callback refresh() -> string;` plus an `append-log`
//! passthrough. No device, no network, no file access, no `unwrap` /
//! `expect` / `panic` outside tests.

/// Firmware baseline plus local discovery, analysis, metadata, hashes.
///
/// `model`/`baseline`/`region`/`compat_status`/`reasons` feed the shared
/// `crate::pages_media::firmware_text` body. `local_image` is the
/// caller-found local file name (may be empty), `header_hex` its first
/// bytes as upper hex (may be empty), `sha256` its sidecar hash (may be
/// empty). Kind classification is intentionally not repeated here; it
/// stays behind `gsi-archive::classify` in the shell.
#[allow(clippy::too_many_arguments)] // explicit inputs by design: no hidden state, all caller-measured and tested
pub fn firmware_center_text(
    model: &str,
    baseline: &str,
    region: &str,
    compat_status: &str,
    reasons: &[String],
    local_image: &str,
    header_hex: &str,
    sha256: &str,
) -> String {
    let mut out = String::new();
    out.push_str("Firmware center (read-only, nothing executed)\n");
    let refs: Vec<&str> = reasons.iter().map(|s| s.as_str()).collect();
    out.push_str(&crate::pages_media::firmware_text(
        model,
        baseline,
        region,
        compat_status,
        &refs,
    ));
    out.push_str("local discovery:\n");
    if local_image.trim().is_empty() {
        out.push_str(" image: none found locally\n");
    } else {
        out.push_str(&format!(" image: {}\n", local_image.trim()));
    }
    out.push_str("analysis:\n");
    if header_hex.trim().is_empty() {
        out.push_str(" header: unknown\n");
    } else {
        out.push_str(&format!(" header: {}\n", header_hex.trim()));
    }
    out.push_str("metadata:\n");
    out.push_str(
        " kind + container probe via gsi-archive::classify / probe_update_app (shell-owned)\n",
    );
    out.push_str("hashes:\n");
    if sha256.trim().is_empty() {
        out.push_str(" sha256: unknown (verify before use)\n");
    } else {
        out.push_str(&format!(" sha256: {}\n", sha256.trim()));
    }
    out.push_str("hash note: SHA-256 saved as <file>.sha256 sidecar (gsi-update::verify_hash_sidecar shell-side)\n");
    out
}

/// Download options plus progress display.
///
/// `rom_options`/`firmware_sources`/`magisk_status`/`cache_dir` feed the
/// shared `crate::pages_media::download_text` body. `progress_lines` are
/// caller-measured progress events (for example `12% 1.2/4.0 GB`); empty
/// renders the honest idle form. The real download runs on a worker
/// thread: this render only displays lines, the thread pump is
/// shell-owned (`gsi-update::download_to_with_progress` /
/// `download_file_with_progress`). No download is started here.
pub fn download_center_text(
    rom_options: &[String],
    firmware_sources: &[String],
    magisk_status: &str,
    cache_dir: &str,
    progress_lines: &[String],
) -> String {
    let mut out = String::new();
    out.push_str("Download center (options preview; execution note below)\n");
    let roms: Vec<&str> = rom_options.iter().map(|s| s.as_str()).collect();
    let fws: Vec<&str> = firmware_sources.iter().map(|s| s.as_str()).collect();
    out.push_str(&crate::pages_media::download_text(
        &roms,
        &fws,
        magisk_status,
        cache_dir,
    ));
    out.push_str("progress:\n");
    let mut shown: usize = 0;
    for line in progress_lines {
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        out.push_str(&format!(" - {t}\n"));
        shown += 1;
    }
    if shown == 0 {
        out.push_str(" idle (no download running)\n");
    }
    out.push_str("execution note: the real download runs on a worker thread; progress lines render here as they arrive, the thread pump is shell-owned (gsi-update::download_to_with_progress / download_file_with_progress)\n");
    out.push_str("refused: no download started here\n");
    out
}

/// Archive preview plus extractor tool status table.
///
/// `archive_name`/`entry_names`/`is_update_app`/`header_hex` feed the
/// shared `crate::pages_media::extract_text` body. `tools` entries are
/// `(name, detail, status)` triples where `status` is one of `Installed`,
/// `Found-incompatible`, `Missing` (empty renders as status unknown).
/// Lookup stays behind `gsi-tool::locate` in the shell; extraction itself
/// stays manual via the extractor tools.
pub fn extract_center_text(
    archive_name: &str,
    entry_names: &[String],
    is_update_app: bool,
    header_hex: &str,
    tools: &[(String, String, String)],
) -> String {
    let mut out = String::new();
    out.push_str("Extract center (read-only, nothing executed)\n");
    let entries: Vec<&str> = entry_names.iter().map(|s| s.as_str()).collect();
    out.push_str(&crate::pages_media::extract_text(
        archive_name,
        &entries,
        is_update_app,
        header_hex,
    ));
    out.push_str("extractor tools:\n");
    if tools.is_empty() {
        out.push_str(
            " none probed (place HuaweiFirmwareExtractor / payload-dumper-go into data/tools/)\n",
        );
    } else {
        for (name, detail, status) in tools {
            let n = name.trim();
            if n.is_empty() {
                continue;
            }
            let d = detail.trim();
            let s = status.trim();
            if d.is_empty() && s.is_empty() {
                out.push_str(&format!(" - {n}: [status unknown]\n"));
            } else if d.is_empty() {
                out.push_str(&format!(" - {n}: [{s}]\n"));
            } else if s.is_empty() {
                out.push_str(&format!(" - {n}: {d} [status unknown]\n"));
            } else {
                out.push_str(&format!(" - {n}: {d} [{s}]\n"));
            }
        }
    }
    out.push_str("lookup note: gsi-tool::locate over PATH plus data/tools (shell-owned)\n");
    out
}

/// Recovery export plan preview.
///
/// Delegates to `crate::pages_media::export_text` (which mirrors the
/// `gsi-diag::plan_export` branches). GSI/system images stay refused as a
/// patch base; repack/write stays open and is never executed here.
pub fn export_center_text(
    rom_file: &str,
    image_kind: &str,
    plan_kind: &str,
    notes: &[String],
) -> String {
    let mut out = String::new();
    out.push_str("Export center (read-only, nothing executed)\n");
    let refs: Vec<&str> = notes.iter().map(|s| s.as_str()).collect();
    out.push_str(&crate::pages_media::export_text(
        rom_file, image_kind, plan_kind, &refs,
    ));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strings(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn firmware_full_renders_baseline_and_local() {
        let t = firmware_center_text(
            "VTR-L29",
            "VTR-L29 9.1.0.297(C432E5R1P9)",
            "C432",
            "PASS",
            &strings(&["EMUI 9.1 base ok (recovery_ramdisk method)."]),
            "UPDATE.APP",
            "55 50 44 41",
            "cd90",
        );
        assert!(t.contains("Firmware center"));
        assert!(t.contains("baseline: VTR-L29 9.1.0.297(C432E5R1P9)"));
        assert!(t.contains("compatibility: PASS"));
        assert!(t.contains("image: UPDATE.APP"));
        assert!(t.contains("header: 55 50 44 41"));
        assert!(t.contains("sha256: cd90"));
        assert!(t.contains("gsi-archive::classify"));
    }

    #[test]
    fn firmware_empty_is_honest() {
        let t = firmware_center_text("", "", "", "", &[], "", "", "");
        assert!(t.contains("model: VTR-L29"));
        assert!(t.contains("baseline: (empty)"));
        assert!(t.contains("compatibility: FAIL"));
        assert!(t.contains("image: none found locally"));
        assert!(t.contains("header: unknown"));
        assert!(t.contains("sha256: unknown"));
    }

    #[test]
    fn download_options_and_idle_progress() {
        let t = download_center_text(
            &strings(&["LineageOS 20 UNOFFICIAL (20251021)"]),
            &strings(&["HiSuite (official)"]),
            "Magisk-v28.1.apk cached",
            "data/firmware",
            &[],
        );
        assert!(t.contains("Download center"));
        assert!(t.contains("LineageOS 20"));
        assert!(t.contains("HiSuite"));
        assert!(t.contains("idle (no download running)"));
        assert!(t.contains("worker thread"));
        assert!(t.contains("thread pump is shell-owned"));
        assert!(t.contains("no download started here"));
    }

    #[test]
    fn download_progress_lines_render() {
        let t = download_center_text(&[], &[], "", "", &strings(&["12% 1.2/4.0 GB", "sha256 ok"]));
        assert!(t.contains(" - 12% 1.2/4.0 GB"));
        assert!(t.contains(" - sha256 ok"));
        assert!(!t.contains("idle (no download running)"));
    }

    #[test]
    fn extract_table_statuses() {
        let tools = vec![
            (
                "huawei_firmware_extractor.py".to_string(),
                "data/tools/huawei_firmware_extractor.py".to_string(),
                "Installed".to_string(),
            ),
            (
                "payload-dumper-go".to_string(),
                "found 0.0.0, need >= 1.0".to_string(),
                "Found-incompatible".to_string(),
            ),
            (
                "splitupdate".to_string(),
                "".to_string(),
                "Missing".to_string(),
            ),
        ];
        let t = extract_center_text(
            "UPDATE.APP",
            &strings(&["RECOVERY_RAMDIS.img"]),
            true,
            "55 50",
            &tools,
        );
        assert!(t.contains("Extract center"));
        assert!(t.contains("container: UPDATE.APP"));
        assert!(t.contains(
            "huawei_firmware_extractor.py: data/tools/huawei_firmware_extractor.py [Installed]"
        ));
        assert!(t.contains("payload-dumper-go: found 0.0.0, need >= 1.0 [Found-incompatible]"));
        assert!(t.contains("splitupdate: [Missing]"));
        assert!(t.contains("gsi-tool::locate"));
    }

    #[test]
    fn extract_no_tools_is_honest() {
        let t = extract_center_text("", &[], false, "", &[]);
        assert!(t.contains("no archive given"));
        assert!(t.contains("none probed"));
    }

    #[test]
    fn export_system_refused() {
        let t = export_center_text("lineage.img", "system", "direct-img", &[]);
        assert!(t.contains("Export center"));
        assert!(t.contains("rom: lineage.img"));
        assert!(t.contains("refused: SYSTEM image"));
        assert!(t.contains("repack/write: open"));
    }

    #[test]
    fn export_zip_plan() {
        let t = export_center_text(
            "custom-rom.zip",
            "unknown",
            "rom-zip",
            &strings(&["extract entries matching *recovery*.img"]),
        );
        assert!(t.contains("plan: rom-zip"));
        assert!(t.contains("*recovery*.img"));
    }
}
