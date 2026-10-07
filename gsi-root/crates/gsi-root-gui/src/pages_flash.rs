//! Read-only renders for the flash/wipe/unlock/verify cluster.
//!
//! Mirrors the scripts without executing anything:
//! `Screen-Flash` / `screen_flash` (`Invoke-TTSafeFlash` / `safe_flash`),
//! `Screen-FlashSystem` / `screen_flashsystem` (`Invoke-SystemFlash` /
//! `system_flash`),
//! `Screen-Wipe` / `screen_wipe` (`Invoke-GuidedWipe` / `guided_wipe`),
//! `Screen-Unlock` / `screen_unlock` (PotatoNV guided-only),
//! `Screen-RebootVerify` / `screen_verify` (`verify_root` plus
//! `validate_device` checklist),
//! `Show-PreflightBlocked` / `preflight` (global gate).
//!
//! Live integration would need crates that `gsi-root-gui` does not depend
//! on yet (see `gsi-root-gui/Cargo.toml`): `gsi-workflow`
//! (`plan_safe_flash`, `plan_system_flash`, `plan_guided_wipe`,
//! `plan_verify_root`, `plan_validate_device`, `classify_verify_root`),
//! `gsi-gates` (`check_readiness`, `preflight`), `gsi-state` (slot read,
//! `new_workflow_plan`). The CLI text helpers (`render_flash_verdict`,
//! `render_readiness` in `gsi-root-cli`) are copied here as pure text
//! logic; this module never depends on the CLI binary crate.
//!
//! This module stays dependency-free on purpose: every function below is
//! pure over explicit `&str` / `bool` / `u32` / `&[String]` inputs,
//! renders text, and refuses execution honestly. No device, no network,
//! no file access, no `unwrap` / `expect` / `panic` outside tests.
//!
//! Slint side lives in `ui/pages_flash.slint` with the same visual pattern
//! as the Detect page (title, note, Refresh button, read-only log view).

/// Verdict preview text copied from `render_flash_verdict` / `Show-FlashVerdict`.
///
/// `verdict` is `OK`, `FAILED`, or anything else (rendered as `UNCLEAR`).
/// `okay` counts `OKAY` lines, `total_time` is the `Total time:` value,
/// `failed` carries trimmed `FAILED` / `remote:` / `error` lines (at most
/// the first three are shown). `what` labels the operation.
fn verdict_text(verdict: &str, what: &str, okay: u32, total_time: &str, failed: &[String]) -> String {
    let w = if what.trim().is_empty() {
        "flash".to_string()
    } else {
        what.trim().to_string()
    };
    let v = verdict.trim();
    if v == "OK" {
        let mut s = format!("FLASH RESULT: OK ({okay} OKAY");
        if !total_time.trim().is_empty() {
            s.push_str(&format!(", {}", total_time.trim()));
        }
        s.push_str(&format!(") [{w}]\n"));
        s
    } else if v == "FAILED" {
        let mut s = String::from("FLASH RESULT: FAILED - nothing claimed as done.\n");
        let mut n: u32 = 0;
        for f in failed.iter() {
            if n >= 3 {
                break;
            }
            s.push_str(&format!(" ! {f}\n"));
            n += 1;
        }
        s.push_str("Hints: 'Command not allowed' = Huawei refused (retry, cable, TROUBLESHOOTING). 'too large' = image bigger than partition. Full output is in the log.\n");
        s
    } else {
        String::from(
            "FLASH RESULT: UNCLEAR - no FAILED, but no Finished either. Verify manually before rebooting.\n",
        )
    }
}

/// One readiness line copied from `render_readiness` / `Test-TTFlashReadiness`.
fn readiness_line(name: &str, pass: bool, detail_ok: &str, detail_missing: &str) -> String {
    let mark = if pass { "[OK]" } else { "[FAIL]" };
    let detail = if pass { detail_ok } else { detail_missing };
    format!("{mark} {name}: {detail}\n")
}

/// Flash verdict preview: readiness render plus selected plan summary plus
/// double-confirm representation as text.
///
/// Mirrors `Screen-Flash` / `screen_flash` with `Invoke-TTSafeFlash` /
/// `safe_flash`, `Test-TTFlashReadiness` / `check_readiness`, and
/// `Get-FlashVerdict` / `flash_verdict`. The ten `*_ok` flags are the
/// caller-measured readiness parts (model, profile, partition, image,
/// hash, size, firmware, backup, fastboot, differs-from-stock).
/// `confirm_flash` is the first word (`FLASH`), `confirm_yes` the second
/// (`YES`). Verdict inputs (`verdict`, `okay`, `total_time`, `failed`)
/// preview the last flash output; empty verdict renders the `UNCLEAR`
/// form. This view never confirms and never flashes.
#[allow(clippy::too_many_arguments)]
pub fn flash_text(
    partition: &str,
    image: &str,
    sha256: &str,
    backup_dir: &str,
    slot: &str,
    model_ok: bool,
    profile_verified: bool,
    partition_ok: bool,
    image_exists: bool,
    hash_known: bool,
    size_ok: bool,
    firmware_ok: bool,
    backup_ok: bool,
    fastboot_ok: bool,
    differs_from_stock: bool,
    confirm_flash: bool,
    confirm_yes: bool,
    verdict: &str,
    okay: u32,
    total_time: &str,
    failed: &[String],
) -> String {
    let part = if partition.trim().is_empty() {
        "recovery_ramdisk".to_string()
    } else {
        partition.trim().to_string()
    };
    let mut out = String::new();
    out.push_str("Flash plan (read-only, nothing executed)\n");
    out.push_str("=== Safety check before flash ===\n");
    out.push_str(&readiness_line(
        "Model matches (VTR/VKY)",
        model_ok,
        "ok",
        "missing",
    ));
    out.push_str(&readiness_line(
        "Profile verified (flash allowed)",
        profile_verified,
        "ok",
        "missing",
    ));
    out.push_str(&readiness_line("Partition exists", partition_ok, "ok", "missing"));
    out.push_str(&readiness_line(
        "Image exists (patched)",
        image_exists,
        "ok",
        "missing",
    ));
    out.push_str(&readiness_line(
        "Image hash known (SHA-256)",
        hash_known,
        "ok",
        "missing",
    ));
    out.push_str(&readiness_line(
        "Image size plausible (1-100 MB)",
        size_ok,
        "ok",
        "missing",
    ));
    out.push_str(&readiness_line(
        "Firmware compatibility established",
        firmware_ok,
        "ok",
        "missing",
    ));
    out.push_str(&readiness_line(
        "Backup available (original.img)",
        backup_ok,
        "ok",
        "missing",
    ));
    out.push_str(&readiness_line(
        "Fastboot device connected",
        fastboot_ok,
        "ok",
        "missing",
    ));
    out.push_str(&readiness_line(
        "Patched != stock (no fake patch)",
        differs_from_stock,
        "ok",
        "missing",
    ));
    let go = model_ok
        && profile_verified
        && partition_ok
        && image_exists
        && hash_known
        && size_ok
        && firmware_ok
        && backup_ok
        && fastboot_ok
        && differs_from_stock;
    if go {
        out.push_str("READY: all checks pass.\n");
    } else {
        out.push_str("BLOCKED: fix FAILED checks first.\n");
    }
    out.push_str("WARNING\n");
    out.push_str(&format!("You are about to modify:\n  {part}\n"));
    if image.trim().is_empty() {
        out.push_str("Image: (missing - no patched image registered)\n");
    } else {
        out.push_str(&format!("Image: {}\n", image.trim()));
    }
    if sha256.trim().is_empty() {
        out.push_str("SHA-256: unknown\n");
    } else {
        out.push_str(&format!("SHA-256: {}\n", sha256.trim()));
    }
    if backup_dir.trim().is_empty() {
        out.push_str("Original backup: missing (backup first)\n");
    } else {
        out.push_str(&format!("Original backup: {}\n", backup_dir.trim()));
    }
    if slot.trim().is_empty() {
        out.push_str("Slot (recovery_ramdisk): unknown\n");
    } else {
        out.push_str(&format!("Slot (recovery_ramdisk): {}\n", slot.trim()));
    }
    out.push_str("This operation modifies the boot chain.\n");
    out.push_str("GSI (system/vendor) stays untouched. userdata is NEVER wiped.\n");
    out.push_str("TWRP and Magisk share the recovery_ramdisk slot (mutual overwrite).\n");
    out.push_str("steps:\n");
    out.push_str(" - safety-check: readiness gate (10 checks) must pass\n");
    out.push_str(" - backup: mandatory backup-first: original.img\n");
    out.push_str(" - warn-bootchain: modifies boot chain; system/vendor untouched; userdata NEVER wiped\n");
    if confirm_flash {
        out.push_str(" - confirm-flash: type FLASH (1/2): given\n");
    } else {
        out.push_str(" - confirm-flash: type FLASH (1/2): missing\n");
    }
    if confirm_yes {
        out.push_str(" - confirm-yes: type YES (2/2): given\n");
    } else {
        out.push_str(" - confirm-yes: type YES (2/2): missing\n");
    }
    out.push_str(&format!(" - flash: fastboot flash {part} <patched> (destructive, Phase 8 only)\n"));
    out.push_str(" - record-slot: slot holds magisk after OK\n");
    if !confirm_flash {
        out.push_str("gate: missing first confirmation (FLASH)\n");
    }
    if !confirm_yes {
        out.push_str("gate: missing second confirmation (YES)\n");
    }
    if !fastboot_ok {
        out.push_str("gate: need fastboot (FASTBOOT_READY)\n");
    }
    if !profile_verified {
        out.push_str("gate: profile unverified\n");
    }
    if !backup_ok {
        out.push_str("gate: no backup (original.img) - backup first\n");
    }
    let what = format!("flash {part}");
    out.push_str(&verdict_text(verdict, &what, okay, total_time, failed));
    out.push_str("refused: live fastboot flash is Phase 8 work; plan only, nothing written\n");
    out
}

/// System-flash plan render.
///
/// Mirrors `Screen-FlashSystem` / `screen_flashsystem` with
/// `Invoke-SystemFlash` / `system_flash`, `Test-SystemImageFile` /
/// `test_system_image`, and the researched-broken registry cross-check.
/// `image_note` is the caller-measured system-image check line (for
/// example `PASS|header=3A FF 26 ED`), `registry_note` names the blocking
/// marker when `registry_blocked` is true. Confirmations render as text
/// only. Verdict inputs preview the last output. Nothing is flashed here.
#[allow(clippy::too_many_arguments)]
pub fn flash_system_text(
    image: &str,
    image_note: &str,
    registry_note: &str,
    profile_verified: bool,
    fastboot_ready: bool,
    image_ok: bool,
    registry_blocked: bool,
    confirm_flash: bool,
    confirm_yes: bool,
    verdict: &str,
    okay: u32,
    total_time: &str,
    failed: &[String],
) -> String {
    let mut out = String::new();
    out.push_str("System flash plan (read-only, nothing executed)\n");
    out.push_str("=== System image check (ROM install) ===\n");
    if image.trim().is_empty() {
        out.push_str("image: (missing - no system image path)\n");
    } else {
        out.push_str(&format!("image: {}\n", image.trim()));
    }
    if image_note.trim().is_empty() {
        out.push_str("check: no system image check recorded\n");
    } else {
        out.push_str(&format!("check: {}\n", image_note.trim()));
    }
    if image_ok {
        out.push_str("Result: PASS\n");
    } else {
        out.push_str("Result: FAIL\n");
    }
    if registry_blocked {
        if registry_note.trim().is_empty() {
            out.push_str("REGISTRY BLOCKED: researched BROKEN build\n");
        } else {
            out.push_str(&format!(
                "REGISTRY BLOCKED: marker \"{}\" is researched BROKEN.\n",
                registry_note.trim()
            ));
        }
    } else {
        out.push_str("registry: no researched-broken marker matched\n");
    }
    if !profile_verified {
        out.push_str("gate: profile unverified\n");
    }
    if !fastboot_ready {
        out.push_str("gate: need fastboot (FASTBOOT_READY)\n");
    }
    if !image_ok {
        out.push_str("gate: system image check FAIL\n");
    }
    if registry_blocked {
        out.push_str("gate: registry BLOCKED (researched broken)\n");
    }
    if !confirm_flash {
        out.push_str("gate: missing first confirmation (FLASH)\n");
    }
    if !confirm_yes {
        out.push_str("gate: missing second confirmation (YES)\n");
    }
    let ready =
        !image.trim().is_empty() && image_ok && !registry_blocked && profile_verified && fastboot_ready && confirm_flash && confirm_yes;
    if ready {
        out.push_str("READY: all gates pass (flash still not executed here).\n");
    } else {
        out.push_str("DO NOT FLASH\n");
    }
    out.push_str("WARNING\n");
    out.push_str("You are about to REPLACE the Android system (fastboot flash system).\n");
    out.push_str("Back up internal storage first. userdata is NOT wiped automatically.\n");
    out.push_str("Afterwards: eRecovery wipe data/factory reset, then first boot (takes a while).\n");
    out.push_str("steps:\n");
    out.push_str(" - system-image-check: test_system_image must PASS\n");
    out.push_str(" - registry-check: researched-broken builds block hard\n");
    out.push_str(" - backup-reminder: back up internal storage first\n");
    out.push_str(" - warn-replace-system: replaces Android system (fastboot flash system); afterwards eRecovery wipe + first boot\n");
    if confirm_flash {
        out.push_str(" - confirm-flash: type FLASH (1/2): given\n");
    } else {
        out.push_str(" - confirm-flash: type FLASH (1/2): missing\n");
    }
    if confirm_yes {
        out.push_str(" - confirm-yes: type YES (2/2): given\n");
    } else {
        out.push_str(" - confirm-yes: type YES (2/2): missing\n");
    }
    out.push_str(" - require-fastboot: abort unless fastboot mode\n");
    out.push_str(" - flash-system: fastboot flash system <gsi> (destructive, Phase 8 only)\n");
    out.push_str("Next on OK: fastboot reboot -> eRecovery (Vol-Up 3s) -> wipe data/factory reset -> first setup.\n");
    out.push_str(&verdict_text(verdict, "flash system", okay, total_time, failed));
    out.push_str("refused: live fastboot flash is Phase 8 work; plan only, nothing written\n");
    out
}

/// Guided-wipe plan render with the eRecovery fallback note.
///
/// Mirrors `Screen-Wipe` / `screen_wipe` with `Invoke-GuidedWipe` /
/// `guided_wipe` and `gsi-workflow::plan_guided_wipe`. Erases userdata
/// only; never automatic. Confirmations render as text only; the stock
/// eRecovery fallback is always shown. Nothing is erased here.
pub fn wipe_text(fastboot_ready: bool, confirm_wipe: bool, confirm_yes: bool) -> String {
    let mut out = String::new();
    out.push_str("Wipe plan (read-only, nothing executed)\n");
    out.push_str("WARNING\n");
    out.push_str("This erases ALL user data (apps, photos, settings). System stays.\n");
    if !fastboot_ready {
        out.push_str("gate: wipe needs fastboot mode\n");
    }
    if !confirm_wipe {
        out.push_str("gate: missing first confirmation (WIPE)\n");
    }
    if !confirm_yes {
        out.push_str("gate: missing second confirmation (YES)\n");
    }
    let ready = fastboot_ready && confirm_wipe && confirm_yes;
    if ready {
        out.push_str("READY: all gates pass (erase still not executed here).\n");
    } else {
        out.push_str("NOT READY: confirm on a live fastboot device only.\n");
    }
    out.push_str("steps:\n");
    out.push_str(" - require-fastboot: abort unless fastboot mode\n");
    out.push_str(" - warn-erase: erases ALL user data (apps, photos, settings). System stays.\n");
    if confirm_wipe {
        out.push_str(" - confirm-wipe: type WIPE (1/2): given\n");
    } else {
        out.push_str(" - confirm-wipe: type WIPE (1/2): missing\n");
    }
    if confirm_yes {
        out.push_str(" - confirm-yes: type YES (2/2): given\n");
    } else {
        out.push_str(" - confirm-yes: type YES (2/2): missing\n");
    }
    out.push_str(" - erase-userdata: fastboot erase userdata (destructive, Phase 8 only)\n");
    out.push_str(" - note-fallback: fallback if refused: stock eRecovery (Vol-Up 3s) -> wipe data/factory reset on phone\n");
    out.push_str("fallback (manual, safe): reboot to stock eRecovery (Vol-Up 3s) -> Wipe data / factory reset -> confirm on phone.\n");
    out.push_str("refused: live fastboot erase is Phase 8 work; plan only, nothing erased\n");
    out
}

/// Bootloader-unlock guidance text (guided only, never executed).
///
/// Mirrors `Screen-Unlock` / `screen_unlock` and `$WikiKnowledge.UnlockSteps`:
/// the tool never unlocks anything itself; PotatoNV testpoint is the wiki
/// method. Static text; no inputs needed.
pub fn unlock_text() -> String {
    let mut out = String::new();
    out.push_str("Bootloader unlock (PotatoNV, wiki method - guided only)\n");
    out.push_str("The tool NEVER unlocks anything itself.\n");
    out.push_str(" - Huawei locks TWO levels: USER LOCK (kernel/ODM/product) and BL LOCK (system/boot/recovery/userdata).\n");
    out.push_str(" - 1. PotatoNV testpoint method per its official tutorial: https://github.com/mashed-potatoes/PotatoNV\n");
    out.push_str(" - 2. Boot the engineering image to special fastboot, choose 'Disable FBLock' (unlocks USER LOCK).\n");
    out.push_str(" - 3. Reboot to normal fastboot (shows unlocked, but NOT fully). PotatoNV shows a random 16-digit code.\n");
    out.push_str(" - 4. Run: fastboot oem unlock XXXXXXXXXXXXXXXX  (your code) - now fully unlocked.\n");
    out.push_str(" - This tool NEVER runs unlock commands itself.\n");
    out.push_str("Already booting a GSI? Then unlock is done - continue with step 2.\n");
    out.push_str("refused: unlock is a manual on-device act; this view only documents the wiki steps\n");
    out
}

/// Root-verify verdict over explicit transcripts plus the validate checklist.
///
/// Mirrors `Screen-RebootVerify` / `screen_verify` with `verify_root` /
/// `Invoke-TTRootVerification` and `validate_device` / `Invoke-TTValidate`.
/// `which_su` is the `which su` output, `su_id` the `su -c id` output,
/// `magisk_v` the `magisk -v` output. The classifier copies
/// `classify_verify_root`: `uid=0` wins, else a present `su` without
/// denial is `INCONCLUSIVE`, else `NOT_ROOTED`. `checks` carries
/// preformatted validate lines such as `ROOT|1|uid=0`. Read-only; live
/// adb work stays Phase 8.
pub fn verify_text(which_su: &str, su_id: &str, magisk_v: &str, checks: &[String]) -> String {
    let mut out = String::new();
    out.push_str("Reboot + verify plan (read-only, nothing executed)\n");
    out.push_str("Huawei boot procedure (mandatory, otherwise no root):\n");
    out.push_str(" - Vol-Up + Power until Huawei logo, then release (Magisk boot cheat).\n");
    out.push_str(" - Without trick it boots stock (no root). If needed open Magisk app and grant root.\n");
    out.push_str("steps:\n");
    out.push_str(" - reboot-to-android\n");
    out.push_str(" - vol-up-power-boot-cheat\n");
    out.push_str(" - wait-for-adb\n");
    out.push_str(" - which-su\n");
    out.push_str(" - su-c-id-uid0\n");
    out.push_str(" - magisk-v\n");
    let w = which_su.trim();
    let id = su_id.trim();
    let mv = magisk_v.trim();
    if w.is_empty() {
        out.push_str("which su: (empty)\n");
    } else {
        out.push_str(&format!("which su: {w}\n"));
    }
    if id.is_empty() {
        out.push_str("su -c id: (empty)\n");
    } else {
        out.push_str(&format!("su -c id: {id}\n"));
    }
    if mv.is_empty() {
        out.push_str("magisk -v: (empty)\n");
    } else {
        out.push_str(&format!("magisk -v: {mv}\n"));
    }
    let verdict = if id.contains("uid=0") {
        "ROOTED"
    } else if !w.is_empty() && !w.contains("not found") && !w.contains("No such") {
        "INCONCLUSIVE"
    } else {
        "NOT_ROOTED"
    };
    out.push_str(&format!("Result: {verdict}\n"));
    if verdict == "ROOTED" {
        out.push_str("ROOT DETECTED (uid=0).\n");
    } else if verdict == "INCONCLUSIVE" {
        out.push_str("su present but no uid=0 (approval / Magisk app / first boot?). INCONCLUSIVE.\n");
    } else {
        out.push_str("No root verifiable. Repeat boot trick + check Magisk app.\n");
    }
    out.push_str("Note: boot alone != root. Only uid=0 counts.\n");
    out.push_str("validate (read-only post-flash checks; live adb is Phase 8):\n");
    if checks.is_empty() {
        out.push_str(" - ADB, OS, SELinux, ROOT, MOUNTS-system, MOUNTS-vendor, WIFI, BLUETOOTH, BATTERY, SENSORS\n");
    } else {
        for c in checks.iter() {
            let t = c.trim();
            if t.is_empty() {
                continue;
            }
            out.push_str(&format!(" - {t}\n"));
        }
    }
    out.push_str("refused: live adb validation is Phase 8 work; plan only\n");
    out
}

/// Blocked-prerequisites render from explicit preflight inputs.
///
/// Mirrors `Show-PreflightBlocked` / `preflight` and
/// `gsi-gates::preflight`: `adb_path` / `fastboot_path` empty means the
/// tool is missing, `shell_ok` false means an unsupported shell,
/// `unwritable_dirs` / `missing_files` / `hash_problems` list the failing
/// entries. `adb_state` / `fastboot_state` are the current device states
/// shown for context. Empty blocks render the READY form.
pub fn preflight_blocked_text(
    adb_path: &str,
    fastboot_path: &str,
    shell_ok: bool,
    unwritable_dirs: &[String],
    missing_files: &[String],
    hash_problems: &[String],
    adb_state: &str,
    fastboot_state: &str,
) -> String {
    let mut blocks: Vec<String> = Vec::new();
    if !shell_ok {
        blocks.push("unsupported shell (need PowerShell 5.1+/7+ or POSIX sh)".to_string());
    }
    if adb_path.trim().is_empty() {
        blocks.push("adb missing (install via Setup or platform-tools on PATH)".to_string());
    }
    if fastboot_path.trim().is_empty() {
        blocks.push("fastboot missing (install via Setup or platform-tools on PATH)".to_string());
    }
    for d in unwritable_dirs.iter() {
        let t = d.trim();
        if t.is_empty() {
            continue;
        }
        blocks.push(format!("directory not writable: {t}"));
    }
    for f in missing_files.iter() {
        let t = f.trim();
        if t.is_empty() {
            continue;
        }
        blocks.push(format!("file missing: {t}"));
    }
    for h in hash_problems.iter() {
        let t = h.trim();
        if t.is_empty() {
            continue;
        }
        blocks.push(format!("hash mismatch: {t} (expected != actual)"));
    }
    let mut out = String::new();
    if blocks.is_empty() {
        out.push_str("PREFLIGHT READY (all green)\n");
    } else {
        out.push_str("Preflight blocked - fix first, no main menu\n");
        out.push_str("Missing global prerequisites:\n");
        for b in blocks.iter() {
            out.push_str(&format!(" [NOT READY] {b}\n"));
        }
        out.push_str("Fix: run the setup now? It asks for paths or installs into user PATH (scrcpy optional).\n");
        out.push_str("No local setup file (remote run): place adb/fastboot on PATH, then restart.\n");
    }
    let adb = if adb_state.trim().is_empty() {
        "unknown".to_string()
    } else {
        adb_state.trim().to_string()
    };
    let fb = if fastboot_state.trim().is_empty() {
        "unknown".to_string()
    } else {
        fastboot_state.trim().to_string()
    };
    out.push_str(&format!("Device states right now:\n ADB: {adb} | Fastboot: {fb}\n"));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strings(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn flash_ready_renders_plan_and_confirm_state() {
        let t = flash_text(
            "recovery_ramdisk",
            "magisk-patched.img",
            "ab12",
            "backups/VTR-L29/recovery_ramdisk/20261007-080000",
            "magisk",
            true,
            true,
            true,
            true,
            true,
            true,
            true,
            true,
            true,
            true,
            true,
            true,
            "UNCLEAR",
            0,
            "",
            &[],
        );
        assert!(t.contains("Flash plan"));
        assert!(t.contains("=== Safety check before flash ==="));
        assert!(t.contains("[OK] Backup available (original.img)"));
        assert!(t.contains("READY: all checks pass."));
        assert!(t.contains("fastboot flash recovery_ramdisk <patched>"));
        assert!(t.contains("type FLASH (1/2): given"));
        assert!(t.contains("type YES (2/2): given"));
        assert!(t.contains("FLASH RESULT: UNCLEAR"));
        assert!(t.contains("Phase 8"));
    }

    #[test]
    fn flash_blocked_lists_gates() {
        let t = flash_text(
            "recovery_ramdisk",
            "",
            "",
            "",
            "",
            false,
            false,
            false,
            false,
            false,
            false,
            false,
            false,
            false,
            false,
            false,
            false,
            "FAILED",
            0,
            "",
            &strings(&["FAILED (remote: Command not allowed)"]),
        );
        assert!(t.contains("BLOCKED: fix FAILED checks first."));
        assert!(t.contains("missing first confirmation (FLASH)"));
        assert!(t.contains("missing second confirmation (YES)"));
        assert!(t.contains("need fastboot (FASTBOOT_READY)"));
        assert!(t.contains("profile unverified"));
        assert!(t.contains("FLASH RESULT: FAILED"));
        assert!(t.contains("nothing claimed as done"));
    }

    #[test]
    fn flash_ok_verdict_renders_counts() {
        let t = flash_text(
            "recovery_ramdisk",
            "m.img",
            "aa",
            "backups/dir",
            "magisk",
            true,
            true,
            true,
            true,
            true,
            true,
            true,
            true,
            true,
            true,
            true,
            true,
            "OK",
            2,
            "0.500s",
            &[],
        );
        assert!(t.contains("FLASH RESULT: OK (2 OKAY, 0.500s)"));
    }

    #[test]
    fn flash_empty_partition_falls_back() {
        let t = flash_text(
            "",
            "m.img",
            "",
            "backups/dir",
            "",
            true,
            true,
            true,
            true,
            true,
            true,
            true,
            true,
            true,
            true,
            true,
            true,
            "",
            0,
            "",
            &[],
        );
        assert!(t.contains("fastboot flash recovery_ramdisk <patched>"));
    }

    #[test]
    fn flash_system_ready_and_blocked() {
        let ok = flash_system_text(
            "lineage-20-arm64_bgN.img",
            "PASS|header=3A FF 26 ED",
            "",
            true,
            true,
            true,
            false,
            true,
            true,
            "OK",
            2,
            "1.2s",
            &[],
        );
        assert!(ok.contains("System flash plan"));
        assert!(ok.contains("Result: PASS"));
        assert!(ok.contains("fastboot flash system <gsi>"));
        assert!(ok.contains("eRecovery wipe data/factory reset"));
        assert!(ok.contains("READY: all gates pass"));
        let blocked = flash_system_text(
            "bad.img",
            "FAIL|too small (< 500 MB), full GSI expected",
            "lineage-light",
            true,
            true,
            false,
            true,
            false,
            true,
            "FAILED",
            0,
            "",
            &strings(&["FAILED (remote: too large)"]),
        );
        assert!(blocked.contains("DO NOT FLASH"));
        assert!(blocked.contains("registry BLOCKED"));
        assert!(blocked.contains("system image check FAIL"));
        assert!(blocked.contains("missing first confirmation (FLASH)"));
        assert!(blocked.contains("FLASH RESULT: FAILED"));
    }

    #[test]
    fn wipe_renders_steps_and_fallback() {
        let t = wipe_text(true, true, true);
        assert!(t.contains("Wipe plan"));
        assert!(t.contains("erases ALL user data"));
        assert!(t.contains("type WIPE (1/2): given"));
        assert!(t.contains("type YES (2/2): given"));
        assert!(t.contains("fastboot erase userdata"));
        assert!(t.contains("stock eRecovery (Vol-Up 3s)"));
        assert!(t.contains("Phase 8"));
        let blocked = wipe_text(false, false, false);
        assert!(blocked.contains("wipe needs fastboot mode"));
        assert!(blocked.contains("missing first confirmation (WIPE)"));
        assert!(blocked.contains("missing second confirmation (YES)"));
        assert!(blocked.contains("NOT READY"));
    }

    #[test]
    fn unlock_renders_potatonv_steps() {
        let t = unlock_text();
        assert!(t.contains("NEVER unlocks anything itself"));
        assert!(t.contains("PotatoNV"));
        assert!(t.contains("Disable FBLock"));
        assert!(t.contains("16-digit code"));
        assert!(t.contains("fastboot oem unlock XXXXXXXXXXXXXXXX"));
        assert!(t.contains("GSI? Then unlock is done"));
    }

    #[test]
    fn verify_rooted_and_checks() {
        let t = verify_text(
            "/system/xbin/su",
            "uid=0(root) gid=0(root)",
            "27.0:MAGISK",
            &strings(&["ROOT|1|uid=0", "ADB|1|device"]),
        );
        assert!(t.contains("Vol-Up + Power until Huawei logo"));
        assert!(t.contains("Result: ROOTED"));
        assert!(t.contains("ROOT DETECTED (uid=0)"));
        assert!(t.contains("Only uid=0 counts"));
        assert!(t.contains("ROOT|1|uid=0"));
    }

    #[test]
    fn verify_inconclusive_and_not_rooted() {
        let inc = verify_text("/system/xbin/su", "Permission denied", "", &[]);
        assert!(inc.contains("Result: INCONCLUSIVE"));
        assert!(inc.contains("no uid=0"));
        let none = verify_text("", "", "", &[]);
        assert!(none.contains("Result: NOT_ROOTED"));
        assert!(none.contains("No root verifiable"));
        assert!(none.contains("ADB, OS, SELinux"));
    }

    #[test]
    fn preflight_ready_and_blocked() {
        let ready = preflight_blocked_text(
            "/usr/bin/adb",
            "/usr/bin/fastboot",
            true,
            &[],
            &[],
            &[],
            "ADB_READY",
            "FASTBOOT_READY",
        );
        assert!(ready.contains("PREFLIGHT READY"));
        assert!(ready.contains("ADB: ADB_READY | Fastboot: FASTBOOT_READY"));
        let blocked = preflight_blocked_text(
            "",
            "",
            false,
            &strings(&["logs"]),
            &strings(&["data/stock.img"]),
            &strings(&["x.img"]),
            "NO_DEVICE",
            "FASTBOOT_NO_DEVICE",
        );
        assert!(blocked.contains("Preflight blocked - fix first"));
        assert!(blocked.contains("[NOT READY] adb missing"));
        assert!(blocked.contains("[NOT READY] fastboot missing"));
        assert!(blocked.contains("unsupported shell"));
        assert!(blocked.contains("directory not writable: logs"));
        assert!(blocked.contains("file missing: data/stock.img"));
        assert!(blocked.contains("hash mismatch: x.img"));
        assert!(blocked.contains("ADB: NO_DEVICE | Fastboot: FASTBOOT_NO_DEVICE"));
    }
}
