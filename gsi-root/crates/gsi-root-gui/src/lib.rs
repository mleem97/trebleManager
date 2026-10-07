//! Slint GUI for gsi-root: same core API as the CLI, no own logic.
//!
//! Pages Dashboard/GSI/Detect/Tools/Updates/Logs/Settings plus the Phase-9
//! screen cluster (flash/backup/media/flows render modules). Patch/verify
//! surface the core's honest EXPERIMENTAL refusal instead of faking success.

mod pages_backup;
mod pages_flash;
mod pages_flows;
mod pages_media;

use gsi_image::{detect_container, parse_sparse_header, Container};
use gsi_root_core::{resolve_engine, GsiProfile};

slint::include_modules!();

/// Analyze an image into a human-readable, multi-line report (testable).
pub fn analyze_text(path: &str) -> String {
    let p = std::path::PathBuf::from(path);
    if path.trim().is_empty() {
        return "no file given".to_string();
    }
    let container = detect_container(&p);
    let sparse = parse_sparse_header(&p);
    let profile = GsiProfile {
        container: Some(container),
        sparse,
        android_api: None,
        arch: None,
    };
    let engine = resolve_engine(&profile);
    let mut out = format!("file: {path}\ncontainer: {}\n", match container {
        Container::Gzip => "gzip",
        Container::AndroidSparse => "android-sparse",
        Container::Raw => "raw",
    });
    match sparse {
        Some(h) => out.push_str(&format!(
            "sparse: blocks={} block_size={} chunks={}\n",
            h.total_blocks, h.block_size, h.total_chunks
        )),
        None => out.push_str("sparse: no\n"),
    }
    out.push_str(&format!(
        "engine: {:?} ({:?})\n",
        engine.engine, engine.maturity
    ));
    for r in &engine.reasons {
        out.push_str(&format!("reason: {r}\n"));
    }
    out
}

/// Patch request: honest refusal until a hardware POC exists.
pub fn patch_text(path: &str) -> String {
    format!(
        "EXPERIMENTAL — refused.\nNo hardware POC for {path}: nothing written, nothing faked.\nSee devices/huawei-p10 for the POC plan."
    )
}

/// Update check against a GitHub repo (`owner/name`).
pub fn update_text(repo: &str) -> String {
    match gsi_update::latest_tag(repo) {
        Ok(t) => format!("latest: {t}"),
        Err(e) => format!("check failed: {e}"),
    }
}

/// Locate a managed external tool (PATH plus known trebleManager tool dirs).
fn find_managed(name: &str) -> Option<gsi_tool::Tool> {
    let mut extra = Vec::new();
    for cand in [
        "tools/platform-tools",
        "data/tools",
        "platform-tools",
        "tools",
    ] {
        let p = std::path::PathBuf::from(cand);
        if p.is_dir() {
            extra.push(p);
        }
    }
    gsi_tool::locate(name, &extra)
}

/// Detect attached devices via the managed adb/fastboot binding.
///
/// Read-only queries (`adb devices`, `fastboot devices`), parsed with
/// `gsi-device`. Renders human-readable text for the Detect page.
/// Missing tools or devices are reported honestly, never faked.
pub fn detect_text() -> String {
    use std::time::Duration;
    let mut out = String::new();
    let mut any = false;
    match find_managed("adb") {
        Some(adb) => {
            out.push_str(&format!("adb: {}\n", adb.path.display()));
            if let Some(v) = gsi_tool::probe_version(&adb) {
                if !v.trim().is_empty() {
                    out.push_str(&format!("adb version: {v}\n"));
                }
            }
            let r = gsi_tool::run(&adb, &["devices"], Duration::from_secs(20));
            let lines: Vec<&str> = r.stdout.lines().collect();
            for d in gsi_device::parse::adb_devices(&lines) {
                out.push_str(&format!("adb {} {}\n", d.serial, d.state));
                any = true;
            }
            if !r.success {
                let err = r.stderr.trim();
                if err.is_empty() {
                    out.push_str("adb devices failed\n");
                } else {
                    out.push_str(&format!("adb devices failed: {err}\n"));
                }
            }
        }
        None => {
            out.push_str("adb: not found (managed binding needs adb on PATH)\n");
        }
    }
    match find_managed("fastboot") {
        Some(fb) => {
            out.push_str(&format!("fastboot: {}\n", fb.path.display()));
            if let Some(v) = gsi_tool::probe_version(&fb) {
                if !v.trim().is_empty() {
                    out.push_str(&format!("fastboot version: {v}\n"));
                }
            }
            let r = gsi_tool::run(&fb, &["devices"], Duration::from_secs(20));
            let lines: Vec<&str> = r.stdout.lines().collect();
            for d in gsi_device::parse::fastboot_devices(&lines) {
                out.push_str(&format!("fastboot {} {}\n", d.serial, d.state));
                any = true;
            }
            if !r.success {
                let err = r.stderr.trim();
                if err.is_empty() {
                    out.push_str("fastboot devices failed\n");
                } else {
                    out.push_str(&format!("fastboot devices failed: {err}\n"));
                }
            }
        }
        None => {
            out.push_str("fastboot: not found (managed binding needs fastboot on PATH)\n");
        }
    }
    if !any {
        out.push_str("no devices (or no tools)\n");
    }
    out
}

/// List managed external tools (adb/fastboot plus extractor programs).
///
/// Same sources as the CLI `tools` command: PATH plus known dirs, version
/// probe when present, honest MISSING notes otherwise. Renders text for
/// the Tools page.
pub fn tools_text() -> String {
    let mut out = String::new();
    for name in ["adb", "fastboot"] {
        match find_managed(name) {
            Some(t) => {
                let v = match gsi_tool::probe_version(&t) {
                    Some(s) => s,
                    None => String::new(),
                };
                if v.trim().is_empty() {
                    out.push_str(&format!("{name}: {}\n", t.path.display()));
                } else {
                    out.push_str(&format!("{name}: {} {v}\n", t.path.display()));
                }
            }
            None => {
                out.push_str(&format!("{name}: MISSING (native protocol: Phase 8)\n"));
            }
        }
    }
    let mut found_extractor = false;
    for exe in [
        "huawei_firmware_extractor.py",
        "payload-dumper-go",
        "payload-dumper-go.exe",
        "splitupdate",
        "split_updata.pl",
    ] {
        let bare = exe
            .trim_end_matches(".py")
            .trim_end_matches(".exe");
        let present = find_managed(exe).is_some()
            || find_managed(bare).is_some()
            || std::path::Path::new(exe).is_file();
        if present {
            out.push_str(&format!("extractor: {exe} present\n"));
            found_extractor = true;
        }
    }
    if !found_extractor {
        out.push_str(
            "extractor: none found (place HuaweiFirmwareExtractor/payload-dumper-go into data/tools/)\n",
        );
    }
    out
}

/// Start the GUI. Returns process exit code.
pub fn run() -> i32 {
    let app = match App::new() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("gui init failed: {e}");
            return 1;
        }
    };
    app.set_version_text(format!("gsi-root {}", env!("CARGO_PKG_VERSION")).into());
    let log = std::sync::Arc::new(std::sync::Mutex::new(String::new()));
    let weak = app.as_weak();
    {
        let log = log.clone();
        app.on_append_log(move |line| {
            if let Ok(mut l) = log.lock() {
                if !l.is_empty() {
                    l.push('\n');
                }
                l.push_str(line.as_str());
                let text: slint::SharedString = l.clone().into();
                drop(l);
                if let Some(a) = weak.upgrade() {
                    a.set_log_text(text);
                }
            }
        });
    }
    app.on_analyze_image(|p| analyze_text(p.as_str()).into());
    app.on_patch_image(|p| patch_text(p.as_str()).into());
    app.on_detect_devices(|| detect_text().into());
    app.on_list_tools(|| tools_text().into());
    app.on_check_update(|r| update_text(r.as_str()).into());
    // Phase-9 screen cluster: static previews (empty inputs); live values
    // arrive with the exec layer (Phase 8). Pages never execute.
    app.on_flash_refresh(|| pages_flash::flash_text("recovery_ramdisk", "", "", "", "", false, false, false, false, false, false, false, false, false, false, false, false, "UNCLEAR", 0, "", &[]).into());
    app.on_flash_system_refresh(|| pages_flash::flash_system_text("", "", "", false, false, false, false, false, false, "UNCLEAR", 0, "", &[]).into());
    app.on_wipe_refresh(|| pages_flash::wipe_text(false, false, false).into());
    app.on_unlock_refresh(|| pages_flash::unlock_text().into());
    app.on_verify_refresh(|| pages_flash::verify_text("", "", "", &[]).into());
    app.on_preflight_refresh(|| pages_flash::preflight_blocked_text("", "", true, &[], &[], &[], "", "").into());
    app.on_backup_refresh(|| pages_backup::backup_text("recovery_ramdisk", "backups/VTR-L29/recovery_ramdisk", "VTR-L29", "").into());
    app.on_restore_refresh(|| pages_backup::restore_text("", "recovery_ramdisk", false).into());
    app.on_reinstall_refresh(|| pages_backup::reinstall_text("VTR-L29", "recovery_ramdisk").into());
    app.on_resume_refresh(|| pages_backup::resume_text("", "").into());
    app.on_bootkeys_refresh(|| pages_backup::bootkeys_text("").into());
    app.on_extract_refresh(|| pages_media::extract_text("", &[], false, "").into());
    app.on_download_refresh(|| pages_media::download_text(&[], &[], "", "data/firmware").into());
    app.on_firmware_refresh(|| pages_media::firmware_text("VTR-L29", "", "", "FAIL", &[]).into());
    app.on_export_refresh(|| pages_media::export_text("", "unknown", "open", &[]).into());
    app.on_kernel_refresh(|| pages_media::kernel_text(&[], &[]).into());
    app.on_twrp_refresh(|| pages_media::twrp_text("", "", false).into());
    app.on_compat_refresh(|| pages_flows::compat_text("", "", "", "", &[], "", &[], &[], "").into());
    app.on_goals_refresh(|| pages_flows::goals_text("", &[], &[]).into());
    app.on_rootmethods_refresh(|| pages_flows::rootmethods_text(&[], "").into());
    app.on_patch_refresh(|| pages_flows::patch_text("", "", "", "").into());
    app.on_persist_refresh(|| pages_flows::persist_text("", &[], "", false).into());
    app.on_romselect_refresh(|| pages_flows::romselect_text(&[], "", "", &[]).into());
    app.on_help_refresh(|| pages_flows::help_text(env!("CARGO_PKG_VERSION")).into());
    app.on_status_refresh(|| pages_flows::status_text("", "", "", "", "", "", "", "", "", "", "", "").into());
    app.on_wizard_refresh(|| pages_flows::wizard_text("", "", &[], &[], "").into());
    if let Err(e) = app.run() {
        eprintln!("gui error: {e}");
        return 1;
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn analyze_empty_is_clear() {
        assert_eq!(analyze_text(""), "no file given");
    }

    #[test]
    fn analyze_missing_file_is_unsupported() {
        let t = analyze_text("/no/such/file.img");
        assert!(t.contains("container: raw"));
    }

    #[test]
    fn patch_refuses_with_path() {
        let t = patch_text("x.img");
        assert!(t.contains("EXPERIMENTAL"));
        assert!(t.contains("x.img"));
    }
}
