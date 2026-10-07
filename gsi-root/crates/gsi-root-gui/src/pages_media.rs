//! Media/download cluster GUI text renders (read-only, never executing).
//!
//! Covers the TUI screens `Screen-Extract`, `Screen-DownloadFirmware`,
//! `Screen-Firmware`, `Screen-ExportRecovery`, `Screen-KernelFixes`,
//! `Screen-Twrp` (`scripts/Treble-Toolkit.ps1`) and `screen_extract`,
//! `screen_download`, `screen_firmware`, `screen_export`,
//! `screen_kernelfixes`, `screen_twrp` (`scripts/treble-toolkit.sh`).
//!
//! All functions here are pure `String` renders with explicit inputs: the
//! GUI passes already-known values in, this module never touches the
//! filesystem, the network, or a device, and never starts a download,
//! extraction, or flash. Live behaviour stays behind the core crates
//! (`gsi-archive` classify/extract/probe, `gsi-update` ROM/Magisk/firmware
//! helpers, `gsi-diag` export planning, `gsi-registry` target/compat data)
//! and behind explicit user confirmation in the app shell.
//!
//! The TUI loop itself (`Start-TTTui` / `main_menu`) is replaced by the GUI
//! app shell; these pages only replace the individual screens listed above.

/// Archive kind label by file name (case-insensitive, script order).
///
/// Mirrors `gsi-archive::classify` without taking that dependency so this
/// GUI module builds on the crates `gsi-root-gui` already uses.
fn archive_kind_label(name: &str) -> &'static str {
    let lower = name.to_lowercase();
    if lower.ends_with(".tar.gz") {
        "tar.gz"
    } else if lower.ends_with(".tar.xz") {
        "tar.xz"
    } else if lower.ends_with(".tgz") {
        "tgz"
    } else if lower.ends_with(".tar") {
        "tar"
    } else if lower.ends_with(".zip") {
        "zip"
    } else if lower.ends_with(".gz") {
        "gzip-single"
    } else if lower.ends_with(".xz") {
        "xz-single"
    } else if lower.ends_with(".app") {
        "update-app"
    } else if lower.ends_with(".img") {
        "plain-img"
    } else {
        "plain"
    }
}

/// Extract page preview: archive classify plus content-list preview.
///
/// `archive_name` is the file name only (used for kind detection).
/// `entry_names` are already-listed container entries (zip/tar) for preview.
/// `is_update_app` flags a proprietary Huawei UPDATE.APP container.
/// `header_hex` is the first bytes as upper hex (may be empty).
/// Render only; extraction is never executed here.
pub fn extract_text(
    archive_name: &str,
    entry_names: &[&str],
    is_update_app: bool,
    header_hex: &str,
) -> String {
    let mut out = String::new();
    if archive_name.trim().is_empty() {
        out.push_str("no archive given\n");
        out.push_str("expected: UPDATE.APP (full firmware) in data/firmware/\n");
    } else {
        out.push_str("file: ");
        out.push_str(archive_name);
        out.push('\n');
        out.push_str("kind: ");
        out.push_str(archive_kind_label(archive_name));
        out.push('\n');
    }
    if is_update_app {
        out.push_str("container: UPDATE.APP (proprietary Huawei format)\n");
    }
    if header_hex.trim().is_empty() {
        out.push_str("header: unknown\n");
    } else {
        out.push_str("header: ");
        out.push_str(header_hex);
        out.push('\n');
    }
    if entry_names.is_empty() {
        out.push_str("entries: none listed\n");
    } else {
        out.push_str("entries:\n");
        let mut shown: usize = 0;
        for e in entry_names.iter() {
            if shown >= 8 {
                break;
            }
            out.push_str(" - ");
            out.push_str(e);
            out.push('\n');
            shown += 1;
        }
        if entry_names.len() > shown {
            out.push_str(" - ... (truncated, full list in extractor)\n");
        }
    }
    out.push_str("preview only: nothing extracted, nothing written\n");
    out.push_str("extractors (manual, into data/tools/): huawei_firmware_extractor.py / splitupdate\n");
    out.push_str("keep exact name RECOVERY_RAMDIS.img vs RECOVERY_RAMDISK.img, do NOT rename\n");
    out
}

/// Download page: firmware/Magisk/ROM download options plus cache notes.
///
/// `rom_options` are registry labels with verified links (may be empty).
/// `firmware_sources` are trusted source descriptions/URLs (may be empty).
/// `magisk_status` is a one-line Magisk APK/channel state (may be empty).
/// `cache_dir` is the offline cache dir shown to the user.
/// Lists options only; no download is started here.
pub fn download_text(
    rom_options: &[&str],
    firmware_sources: &[&str],
    magisk_status: &str,
    cache_dir: &str,
) -> String {
    let mut out = String::new();
    if cache_dir.trim().is_empty() {
        out.push_str("cache: data/firmware/\n");
    } else {
        out.push_str("cache: ");
        out.push_str(cache_dir);
        out.push('\n');
    }
    out.push_str("policy: full packages only (no deltas), offline cache, no re-download\n");
    if rom_options.is_empty() {
        out.push_str("roms: none listed (drop package into data/roms/ manually)\n");
    } else {
        out.push_str("roms (verified links only):\n");
        let mut shown: usize = 0;
        for r in rom_options.iter() {
            if shown >= 8 {
                break;
            }
            out.push_str(" - ");
            out.push_str(r);
            out.push('\n');
            shown += 1;
        }
        if rom_options.len() > shown {
            out.push_str(" - ... (truncated)\n");
        }
    }
    if firmware_sources.is_empty() {
        out.push_str("firmware sources: HiSuite (official) / consumer search / FIRM FINDER V2 (historic) / androidhost.ru (community archive)\n");
    } else {
        out.push_str("firmware sources:\n");
        let mut shown: usize = 0;
        for s in firmware_sources.iter() {
            if shown >= 6 {
                break;
            }
            out.push_str(" - ");
            out.push_str(s);
            out.push('\n');
            shown += 1;
        }
        if firmware_sources.len() > shown {
            out.push_str(" - ... (truncated)\n");
        }
    }
    if magisk_status.trim().is_empty() {
        out.push_str("magisk: no APK in data/magisk/ (official GitHub releases only)\n");
    } else {
        out.push_str("magisk: ");
        out.push_str(magisk_status);
        out.push('\n');
    }
    out.push_str("hash: SHA-256 saved as <file>.sha256 sidecar, verify before use\n");
    out.push_str("size: plan approx. 2-4 GB per full firmware\n");
    out.push_str("refused: no download started here (paste link + YES confirmation lives in the TUI/CLI path)\n");
    out
}

/// Firmware page: baseline plus compatibility verdict.
///
/// `model` is the device profile (e.g. VTR-L29). `baseline` is the firmware
/// string. `region` is the CUST region (e.g. C432, may be empty).
/// `compat_status` is PASS/WARN/FAIL. `reasons` are verdict reasons.
/// Read-only check; nothing is downloaded or flashed here.
pub fn firmware_text(
    model: &str,
    baseline: &str,
    region: &str,
    compat_status: &str,
    reasons: &[&str],
) -> String {
    let mut out = String::new();
    if model.trim().is_empty() {
        out.push_str("model: VTR-L29\n");
    } else {
        out.push_str("model: ");
        out.push_str(model);
        out.push('\n');
    }
    if baseline.trim().is_empty() {
        out.push_str("baseline: (empty)\n");
    } else {
        out.push_str("baseline: ");
        out.push_str(baseline);
        out.push('\n');
    }
    if region.trim().is_empty() {
        out.push_str("region: unknown\n");
    } else {
        out.push_str("region: ");
        out.push_str(region);
        out.push('\n');
    }
    if compat_status.trim().is_empty() {
        out.push_str("compatibility: FAIL\n");
    } else {
        out.push_str("compatibility: ");
        out.push_str(compat_status);
        out.push('\n');
    }
    if reasons.is_empty() {
        out.push_str("reasons: none given (pick a firmware baseline first)\n");
    } else {
        for r in reasons.iter() {
            out.push_str(" - ");
            out.push_str(r);
            out.push('\n');
        }
    }
    out.push_str("sources (no dubious auto-download): professorjtj FIRM FINDER V2 / discussion #2542 example VTR-L29 9.1.0.297(C432E5R1P9) / P10 wiki EMUI 9.1 base\n");
    out.push_str("FAIL means do NOT continue, pick another firmware\n");
    out
}

/// Export page: recovery export plan/refusal.
///
/// `rom_file` is the ROM package name. `image_kind` is boot/system/unknown.
/// `plan_kind` is the planner branch (direct-img/rom-zip/rom-tar/wrapper).
/// `notes` are planner hints. GSI/system images are refused honestly;
/// repack/write stays open and is never executed here.
pub fn export_text(
    rom_file: &str,
    image_kind: &str,
    plan_kind: &str,
    notes: &[&str],
) -> String {
    let mut out = String::new();
    if rom_file.trim().is_empty() {
        out.push_str("rom: none (drop .zip/.tar.gz/.img.gz/.img.xz/.img into data/roms/)\n");
    } else {
        out.push_str("rom: ");
        out.push_str(rom_file);
        out.push('\n');
    }
    if image_kind.trim().is_empty() {
        out.push_str("image: unknown\n");
    } else {
        out.push_str("image: ");
        out.push_str(image_kind);
        out.push('\n');
    }
    if plan_kind.trim().is_empty() {
        out.push_str("plan: open\n");
    } else {
        out.push_str("plan: ");
        out.push_str(plan_kind);
        out.push('\n');
    }
    if image_kind.trim().to_lowercase() == "system" {
        out.push_str("refused: SYSTEM image (filesystem) cannot be a Magisk patch base; GSI leaves recovery stock, use the stock UPDATE.APP recovery\n");
    }
    if notes.is_empty() {
        out.push_str("notes: extract *recovery*.img or boot.img, validate ANDROID! magic per file; payload.bin needs an external dumper\n");
    } else {
        for n in notes.iter() {
            out.push_str(" - ");
            out.push_str(n);
            out.push('\n');
        }
    }
    out.push_str("repack/write: open, stated here, not executed\n");
    out
}

/// Kernel page: kernel-fix options from the scripts (informational).
///
/// `kernel_notes` and `fix_notes` override the built-in P10 wiki defaults
/// when non-empty. Needs verified root on device; nothing runs here.
pub fn kernel_text(kernel_notes: &[&str], fix_notes: &[&str]) -> String {
    let mut out = String::new();
    out.push_str("kernels (permissive SELinux for some GSIs):\n");
    if kernel_notes.is_empty() {
        out.push_str(" - Some GSIs need permissive SELinux, custom kernel required\n");
        out.push_str(" - EMUI 8: Proto8 (all P10) or HyperPlus (EU/Global, or CN only with UFS chip)\n");
        out.push_str(" - EMUI 9 CN: Pangu kernel\n");
        out.push_str(" - KernelSU: v0.9.2 only, NOT v0.9.5+\n");
    } else {
        for k in kernel_notes.iter() {
            out.push_str(" - ");
            out.push_str(k);
            out.push('\n');
        }
    }
    out.push_str("fixes (need root, verify uid=0 first):\n");
    if fix_notes.is_empty() {
        out.push_str(" - Speakers: chown root:audio + chmod 0660 /dev/nxp_smartpa_dev\n");
        out.push_str(" - Touchscreen edges: stop aptouch\n");
        out.push_str(" - Decrypt: needs EMUI 9.0 vendor fstab (9.1 is read-only erofs)\n");
    } else {
        for f in fix_notes.iter() {
            out.push_str(" - ");
            out.push_str(f);
            out.push('\n');
        }
    }
    out.push_str("TWRP rule: factory reset ONLY via stock recovery, TWRP wipe breaks userdata\n");
    out.push_str("GApps: MindTheGapps (OpenGApps fails on Oreo GSIs, Pie+ works)\n");
    out
}

/// TWRP page: plan plus shared-slot warning.
///
/// `twrp_image` is the device-exact TWRP image path (may be empty).
/// `slot_state` is the current recovery_ramdisk slot holder (may be empty).
/// `profile_verified` gates flashing; false renders the blocked note.
/// Planning only; flashing stays behind the safety gate and never runs here.
pub fn twrp_text(twrp_image: &str, slot_state: &str, profile_verified: bool) -> String {
    let mut out = String::new();
    if twrp_image.trim().is_empty() {
        out.push_str("twrp: no image (use a build for YOUR exact model, VTR vs VKY differ)\n");
    } else {
        out.push_str("twrp: ");
        out.push_str(twrp_image);
        out.push('\n');
    }
    if slot_state.trim().is_empty() {
        out.push_str("slot: recovery_ramdisk (holder unknown)\n");
    } else {
        out.push_str("slot: ");
        out.push_str(slot_state);
        out.push('\n');
    }
    out.push_str("sources (device-exact only): XDA P10 Plus TWRP 3.2.1-0 (oreo)\n");
    out.push_str("WARNING: TWRP and Magisk-recovery SHARE the recovery_ramdisk slot (mutual overwrite)\n");
    out.push_str("backup: back up the current slot first\n");
    out.push_str("NEVER wipe userdata in TWRP (use stock recovery); boot TWRP with Vol-Up held\n");
    if profile_verified {
        out.push_str("profile: verified (flash still needs fastboot + double confirmation, not executed here)\n");
    } else {
        out.push_str("profile: unverified, DO NOT FLASH\n");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_golden_zip_preview() {
        let t = extract_text(
            "rom.zip",
            &["boot.img", "recovery.img"],
            false,
            "50 4B 03 04",
        );
        assert!(t.contains("file: rom.zip"));
        assert!(t.contains("kind: zip"));
        assert!(t.contains(" - boot.img"));
        assert!(t.contains("preview only"));
        assert!(t.contains("do NOT rename"));
    }

    #[test]
    fn extract_golden_empty_update_app() {
        let t = extract_text("", &[], true, "");
        assert!(t.contains("no archive given"));
        assert!(t.contains("UPDATE.APP"));
        assert!(t.contains("header: unknown"));
        assert!(t.contains("entries: none listed"));
    }

    #[test]
    fn download_golden_options_and_cache() {
        let t = download_text(
            &["LineageOS 20 UNOFFICIAL (20251021)"],
            &["HiSuite https://consumer.huawei.com/de/support/hisuite/"],
            "Magisk-v28.1.apk cached",
            "data/firmware",
        );
        assert!(t.contains("cache: data/firmware"));
        assert!(t.contains("LineageOS 20"));
        assert!(t.contains("HiSuite"));
        assert!(t.contains(".sha256 sidecar"));
        assert!(t.contains("no download started here"));
    }

    #[test]
    fn download_golden_empty() {
        let t = download_text(&[], &[], "", "");
        assert!(t.contains("cache: data/firmware/"));
        assert!(t.contains("drop package into data/roms/"));
        assert!(t.contains("no APK in data/magisk/"));
    }

    #[test]
    fn firmware_golden_pass() {
        let t = firmware_text(
            "VTR-L29",
            "VTR-L29 9.1.0.297(C432E5R1P9)",
            "C432",
            "PASS",
            &["EMUI 9.1 base ok (recovery_ramdisk method)."],
        );
        assert!(t.contains("model: VTR-L29"));
        assert!(t.contains("baseline: VTR-L29 9.1.0.297(C432E5R1P9)"));
        assert!(t.contains("compatibility: PASS"));
        assert!(t.contains("EMUI 9.1 base ok"));
    }

    #[test]
    fn firmware_golden_empty_is_fail() {
        let t = firmware_text("", "", "", "", &[]);
        assert!(t.contains("model: VTR-L29"));
        assert!(t.contains("baseline: (empty)"));
        assert!(t.contains("compatibility: FAIL"));
        assert!(t.contains("do NOT continue"));
    }

    #[test]
    fn export_golden_system_refused() {
        let t = export_text("lineage.img", "system", "direct-img", &[]);
        assert!(t.contains("rom: lineage.img"));
        assert!(t.contains("refused: SYSTEM image"));
        assert!(t.contains("repack/write: open"));
    }

    #[test]
    fn export_golden_zip_plan() {
        let t = export_text(
            "custom-rom.zip",
            "unknown",
            "rom-zip",
            &["extract entries matching *recovery*.img"],
        );
        assert!(t.contains("plan: rom-zip"));
        assert!(t.contains("*recovery*.img"));
        assert!(t.contains("not executed"));
    }

    #[test]
    fn kernel_golden_defaults() {
        let t = kernel_text(&[], &[]);
        assert!(t.contains("Proto8"));
        assert!(t.contains("Pangu"));
        assert!(t.contains("v0.9.2 only"));
        assert!(t.contains("stop aptouch"));
        assert!(t.contains("ONLY via stock recovery"));
    }

    #[test]
    fn twrp_golden_warns_shared_slot() {
        let t = twrp_text("", "magisk", false);
        assert!(t.contains("no image"));
        assert!(t.contains("SHARE the recovery_ramdisk slot"));
        assert!(t.contains("NEVER wipe userdata in TWRP"));
        assert!(t.contains("DO NOT FLASH"));
    }

    #[test]
    fn twrp_golden_verified() {
        let t = twrp_text("twrp-3.2.1-0.img", "twrp", true);
        assert!(t.contains("twrp: twrp-3.2.1-0.img"));
        assert!(t.contains("slot: twrp"));
        assert!(t.contains("verified"));
    }
}
