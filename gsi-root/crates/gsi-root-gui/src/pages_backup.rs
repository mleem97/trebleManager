//! Read-only renders for the backup/restore/reinstall/resume/bootkeys cluster.
//!
//! Mirrors the scripts without executing anything:
//! `Screen-Backup` / `screen_backup` (`do_backup`),
//! `Screen-Restore` (`Invoke-TTRestoreFlow`) / `do_restore` (bash has no
//! `screen_restore` wrapper),
//! `Screen-Reinstall` / `screen_reinstall` (plus `Invoke-GuidedWipe` /
//! `guided_wipe`),
//! `Screen-Resume` / `screen_resume`,
//! `Screen-Bootkeys` / `screen_bootkeys`.
//!
//! Live integration would need crates that `gsi-root-gui` does not depend
//! on yet (see `gsi-root-gui/Cargo.toml`): `gsi-diag` (`plan_backup`,
//! `backup_metadata_json`), `gsi-workflow` (`plan_restore`,
//! `plan_guided_wipe`), `gsi-state` (workflow-state and slot read),
//! `gsi-gates` (`target_partition`). This module stays dependency-free on
//! purpose: every function below is pure over explicit `&str` / `bool`
//! inputs, renders text, and refuses execution honestly. No device, no
//! network, no file access.
//!
//! Slint side lives in `ui/pages_backup.slint` with the same visual pattern
//! as the Detect page (title, note, Refresh button, read-only log view).

/// Backup plan preview plus metadata preview and device-dump refusal.
///
/// Mirrors `New-TTBackup` / `do_backup` and `gsi-diag::plan_backup` shape:
/// empty partition or dest dir is refused, empty model falls back to
/// `VTR-L29` like the scripts. The live `dd` off the device is always
/// refused here (needs a live device, Phase 8 work).
pub fn backup_text(partition: &str, dest_dir: &str, model: &str, firmware: &str) -> String {
    let part = partition.trim();
    let dest = dest_dir.trim();
    let mdl = if model.trim().is_empty() {
        "VTR-L29"
    } else {
        model.trim()
    };
    let fw = firmware.trim();
    let mut out = String::new();
    out.push_str("Backup plan (read-only, nothing executed)\n");
    if part.is_empty() {
        out.push_str("refused: empty partition\n");
        return out;
    }
    if dest.is_empty() {
        out.push_str("refused: empty dest dir\n");
        return out;
    }
    out.push_str(&format!("model: {mdl}\n"));
    out.push_str(&format!("partition: {part}\n"));
    out.push_str(&format!("dest dir: {dest}\n"));
    if fw.is_empty() {
        out.push_str("firmware: unknown\n");
    } else {
        out.push_str(&format!("firmware: {fw}\n"));
    }
    out.push_str("source: firmware-extracted\n");
    out.push_str("contains: original.img + metadata.json + sha256.txt (+partition-probe)\n");
    out.push_str("mandatory before flash: back up the base first\n");
    out.push_str(&format!(
        "metadata preview: {{\"model\":\"{mdl}\",\"partition\":\"{part}\",\"dest_dir\":\"{dest}\",\"firmware\":\"{fw}\",\"source\":\"firmware-extracted\",\"tool\":\"gsi-diag\"}}\n"
    ));
    out.push_str("refused: device dump (dd) off the live partition needs a live device; plan only, nothing read, nothing written\n");
    out.push_str("note: without a stock base image there is no backup; nothing is faked\n");
    out
}

/// Restore plan preview with the `original.img` gate.
///
/// Mirrors `Invoke-TTRestoreFlow` / `do_restore` and
/// `gsi-workflow::plan_restore` steps. `backup_has_original` is the
/// caller-measured `original.img` file check. Fastboot readiness and the
/// `RESTORE` + `YES` double confirmation are rendered as still needed
/// (this view never confirms and never flashes).
pub fn restore_text(backup_dir: &str, partition: &str, backup_has_original: bool) -> String {
    let dir = backup_dir.trim();
    let part = partition.trim();
    let mut out = String::new();
    out.push_str("Restore plan (read-only, nothing executed)\n");
    if dir.is_empty() {
        out.push_str("gate: no backup dir\n");
    } else {
        out.push_str(&format!("backup dir: {dir}\n"));
        out.push_str(&format!("image: {dir}/original.img\n"));
    }
    if part.is_empty() {
        out.push_str("gate: no target partition\n");
    } else {
        out.push_str(&format!("partition: {part}\n"));
        out.push_str(&format!("flash detail: fastboot flash {part} original.img\n"));
    }
    if backup_has_original {
        out.push_str("gate: original.img present\n");
    } else {
        out.push_str("gate: no backup with original.img found\n");
    }
    out.push_str("gate: need fastboot (reboot to fastboot first)\n");
    out.push_str("gate: missing first confirmation (RESTORE)\n");
    out.push_str("gate: missing second confirmation (YES)\n");
    out.push_str("steps:\n");
    out.push_str(" - find-backup: newest backup dir with original.img\n");
    out.push_str(" - verify-hash: restore hash SHA-256 recorded\n");
    out.push_str(" - require-fastboot: abort unless fastboot mode\n");
    out.push_str(" - warn-restore: restore overwrites slot with stock image\n");
    out.push_str(" - confirm-restore: type RESTORE (1/2)\n");
    out.push_str(" - confirm-yes: type YES (2/2)\n");
    out.push_str(" - flash: destructive, Phase 8 only\n");
    out.push_str(" - record-slot: slot holds stock after restore\n");
    out.push_str("warning: never flash without a live fastboot device\n");
    out.push_str("refused: live fastboot flash is Phase 8 work; plan only\n");
    out
}

/// Full-reinstall checklist preview.
///
/// Mirrors `Screen-Reinstall` / `screen_reinstall`: the flow chains
/// ROM choice, backup reminder, optional wipe, flash, reboot, verify.
/// `model` and `partition` only label the target; empty model falls back
/// to `VTR-L29`, empty partition falls back to `recovery_ramdisk`.
pub fn reinstall_text(model: &str, partition: &str) -> String {
    let mdl = if model.trim().is_empty() {
        "VTR-L29"
    } else {
        model.trim()
    };
    let part = if partition.trim().is_empty() {
        "recovery_ramdisk"
    } else {
        partition.trim()
    };
    let mut out = String::new();
    out.push_str("Full reinstall checklist (read-only, nothing executed)\n");
    out.push_str(&format!("model: {mdl}\n"));
    out.push_str(&format!("slot partition: {part}\n"));
    out.push_str("chain: ROM choice -> backup reminder -> optional wipe -> flash -> reboot -> verify\n");
    out.push_str("goal steps (full_reinstall): reconnaissance, compatibility, firmware, rom_validation, backup, wipe, flash_system, reboot, validate\n");
    out.push_str("[1] Custom ROM / GSI image: wipe userdata first is optional (double-confirmed), then flash-system path\n");
    out.push_str("[2] Stock full firmware path: full firmware for exact model and region, flash via HiSuite (official, recommended) or service flow\n");
    out.push_str("stock scope: this tool cannot unpack UPDATE.APP system images itself; an already extracted SYSTEM.img can go through Install ROM\n");
    out.push_str("wipe: double-confirmed WIPE + YES, userdata only, system stays; fallback is stock eRecovery (Vol-Up 3s) wipe on the phone\n");
    out.push_str("after flash: eRecovery wipe data/factory reset, then first boot and verify\n");
    out.push_str("refused: wipe and flash need a live device and explicit on-device confirms; plan only\n");
    out
}

/// Workflow-state resume preview.
///
/// Mirrors `Screen-Resume` / `screen_resume`: `goal` is the saved goal id,
/// `steps_text` carries preformatted per-step lines such as
/// `reconnaissance [done]`. Empty goal means no saved workflow.
pub fn resume_text(goal: &str, steps_text: &str) -> String {
    let g = goal.trim();
    let mut out = String::new();
    out.push_str("Resume workflow (read-only, nothing executed)\n");
    if g.is_empty() {
        out.push_str("no saved workflow state to resume\n");
        return out;
    }
    out.push_str(&format!("goal: {g}\n"));
    let body = steps_text.trim();
    if body.is_empty() {
        out.push_str("steps: none recorded\n");
    } else {
        out.push_str("steps:\n");
        for line in body.lines() {
            let t = line.trim();
            if t.is_empty() {
                continue;
            }
            out.push_str(&format!(" - {t}\n"));
        }
    }
    out.push_str("note: resume re-runs the workflow from its plan (completed screens run again)\n");
    out.push_str("refused: resume execution needs the guided runner; this view only renders the saved plan\n");
    out
}

/// Huawei boot keycombo table.
///
/// Mirrors `Screen-Bootkeys` / `screen_bootkeys` (from issue #2542, exact).
/// `boot_mode` is the persisted `last_root.boot_mode` value; empty means
/// no persisted value is shown.
pub fn bootkeys_text(boot_mode: &str) -> String {
    let mut out = String::new();
    out.push_str("Huawei boot mechanism (read-only guide, no device change)\n");
    out.push_str("- Magisk boot: Vol-Up + Power until Huawei logo, then release (Magisk boot cheat)\n");
    out.push_str("- Without trick: stock boot (no root); behavior NOT persistent\n");
    out.push_str("- Set persistent byte: warning screen -> eRecovery (Vol-Up 3s) -> confirm Wipe/Factory Reset + reboot -> boots Magisk root (wipe is NOT executed)\n");
    out.push_str("- Clear byte: remove /dload, power off -> Vol-Up + Vol-Down + Power until logo -> EMUI OS Upgrade not successful -> reboot -> clean\n");
    out.push_str("- /dload must NOT be on storage, else EMUI updater starts instead of recovery\n");
    out.push_str("- Never use Pixel / A-B guides on this device\n");
    out.push_str("- SET/CLEAR/verify are manual on-device steps with double confirmation in the scripts\n");
    let bm = boot_mode.trim();
    if bm.is_empty() {
        out.push_str("persisted boot mode: unknown (no saved state)\n");
    } else {
        out.push_str(&format!("persisted boot mode: {bm}\n"));
    }
    out.push_str("refused: boot-mode change is a manual on-device act; this view only documents the combos\n");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backup_renders_plan_and_refusal() {
        let t = backup_text(
            "recovery_ramdisk",
            "backups/VTR-L29/recovery_ramdisk/20261007-080000",
            "VTR-L29",
            "VTR-L29C432BXXX",
        );
        assert!(t.contains("Backup plan"));
        assert!(t.contains("partition: recovery_ramdisk"));
        assert!(t.contains("original.img + metadata.json + sha256.txt"));
        assert!(t.contains("metadata preview"));
        assert!(t.contains("refused: device dump"));
    }

    #[test]
    fn backup_empty_partition_is_refused() {
        let t = backup_text("", "backups/dir", "VTR-L29", "");
        assert!(t.contains("refused: empty partition"));
    }

    #[test]
    fn backup_empty_dest_is_refused() {
        let t = backup_text("recovery_ramdisk", "  ", "", "");
        assert!(t.contains("refused: empty dest dir"));
    }

    #[test]
    fn backup_empty_model_falls_back() {
        let t = backup_text("recovery_ramdisk", "backups/dir", "", "");
        assert!(t.contains("model: VTR-L29"));
    }

    #[test]
    fn restore_renders_gate_and_steps() {
        let t = restore_text("backups/VTR-L29/recovery_ramdisk/20261007-080000", "recovery_ramdisk", true);
        assert!(t.contains("Restore plan"));
        assert!(t.contains("original.img present"));
        assert!(t.contains("fastboot flash recovery_ramdisk original.img"));
        assert!(t.contains("type RESTORE (1/2)"));
        assert!(t.contains("never flash without a live fastboot device"));
        assert!(t.contains("Phase 8"));
    }

    #[test]
    fn restore_missing_original_is_gate() {
        let t = restore_text("", "", false);
        assert!(t.contains("no backup dir"));
        assert!(t.contains("no target partition"));
        assert!(t.contains("no backup with original.img found"));
    }

    #[test]
    fn reinstall_renders_chain() {
        let t = reinstall_text("VTR-L29", "recovery_ramdisk");
        assert!(t.contains("Full reinstall checklist"));
        assert!(t.contains("ROM choice -> backup reminder -> optional wipe -> flash -> reboot -> verify"));
        assert!(t.contains("Custom ROM / GSI"));
        assert!(t.contains("HiSuite"));
        assert!(t.contains("UPDATE.APP"));
        assert!(t.contains("WIPE + YES"));
    }

    #[test]
    fn reinstall_empty_inputs_fall_back() {
        let t = reinstall_text("", "");
        assert!(t.contains("model: VTR-L29"));
        assert!(t.contains("slot partition: recovery_ramdisk"));
    }

    #[test]
    fn resume_renders_goal_and_steps() {
        let t = resume_text("full_reinstall", "reconnaissance [done]\nbackup [active]");
        assert!(t.contains("goal: full_reinstall"));
        assert!(t.contains("reconnaissance [done]"));
        assert!(t.contains("backup [active]"));
        assert!(t.contains("re-runs the workflow from its plan"));
    }

    #[test]
    fn resume_empty_goal_reports_none() {
        let t = resume_text("  ", "");
        assert!(t.contains("no saved workflow"));
    }

    #[test]
    fn bootkeys_renders_table() {
        let t = bootkeys_text("cheat");
        assert!(t.contains("Vol-Up + Power until Huawei logo"));
        assert!(t.contains("/dload must NOT be on storage"));
        assert!(t.contains("Never use Pixel"));
        assert!(t.contains("persisted boot mode: cheat"));
    }

    #[test]
    fn bootkeys_empty_mode_is_unknown() {
        let t = bootkeys_text("");
        assert!(t.contains("persisted boot mode: unknown"));
    }
}
