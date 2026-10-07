//! Backup GUI center renders (read-only, never executing).
//!
//! Covers the Backup center cluster: `BackupCenter` (saved-backup tree),
//! `CreateBackupCenter` (create plan + requirements), `RestoreViewCenter`
//! (verify-first restore flow), `ReinstallCenter` (checklist) and
//! `ResumeCenter` (saved workflow state).
//!
//! Reuse, never duplicate:
//! - the plan shape mirrors `gsi-diag::plan_backup` and the metadata
//!   preview mirrors `gsi-diag::backup_metadata_json` (live values are
//!   caller-measured; this module only renders text).
//! - the device-dump step renders the `gsi-exec::backup_dd_cmd` command
//!   shape as text; execution is Phase 8 shell work, refused here.
//! - shared plan bodies delegate to `crate::pages_backup`
//!   (`backup_text`, `restore_text`, `reinstall_text`, `resume_text`).
//! - future live wiring (shell-owned): newest backup with `original.img`
//!   via the `live` scan, dest preview under the `gsi-config` data dir.
//!
//! Slint side lives in `ui/center_backup.slint` (`BackupCenter`,
//! `CreateBackupCenter`, `RestoreViewCenter`, `ReinstallCenter`,
//! `ResumeCenter`) with the same visual pattern as the Detect page (baked
//! title, note, Refresh button, read-only log view) and the component
//! contract `in-out property <string> result; callback refresh() ->
//! string;` plus an `append-log` passthrough. No device, no network, no
//! file access, no `unwrap` / `expect` / `panic` outside tests.

/// Saved-backup tree grouped by model and date, with verified states.
///
/// `backups` entries are `(dir, date, state, sha256)` tuples where `state`
/// is `verified`, `unverified`, `corrupt`, or empty (rendered as
/// `unverified`) and `sha256` is the sidecar hash or empty (rendered as
/// `unknown`). An empty list renders the honest `No backups yet` empty
/// state plus the Create CTA hint. Nothing is read or written here.
pub fn backup_center_text(
    model: &str,
    dest_preview: &str,
    backups: &[(String, String, String, String)],
) -> String {
    let mut out = String::new();
    out.push_str("Backup center (read-only, nothing executed)\n");
    let mdl = model.trim();
    if mdl.is_empty() {
        out.push_str("model: unknown\n");
    } else {
        out.push_str(&format!("model: {mdl}\n"));
    }
    let dest = dest_preview.trim();
    if dest.is_empty() {
        out.push_str("dest: (unknown)\n");
    } else {
        out.push_str(&format!("dest: {dest}\n"));
    }
    if backups.is_empty() {
        out.push_str("No backups yet\n");
        out.push_str("Create CTA: open Create Backup to plan the first backup (back up the base before any flash)\n");
        out.push_str("note: without a stock base image there is no backup; nothing is faked\n");
        return out;
    }
    out.push_str("backups (model/date tree):\n");
    let mut shown: usize = 0;
    for (dir, date, state, sha) in backups {
        let d = dir.trim();
        let dt = date.trim();
        if d.is_empty() && dt.is_empty() {
            continue;
        }
        if dt.is_empty() {
            out.push_str(&format!(" - {d}\n"));
        } else if d.is_empty() {
            out.push_str(&format!(" - {dt}\n"));
        } else {
            out.push_str(&format!(" - {dt} {d}\n"));
        }
        let st = state.trim();
        if st.is_empty() {
            out.push_str("   state: unverified\n");
        } else {
            out.push_str(&format!("   state: {st}\n"));
        }
        let h = sha.trim();
        if h.is_empty() {
            out.push_str("   sha256: unknown\n");
        } else {
            out.push_str(&format!("   sha256: {h}\n"));
        }
        shown += 1;
    }
    if shown == 0 {
        out.push_str("No backups yet\n");
        out.push_str("Create CTA: open Create Backup to plan the first backup (back up the base before any flash)\n");
        return out;
    }
    out.push_str("states: verified = hash sidecar matches; unverified = not checked yet; corrupt = mismatch, do not flash\n");
    out
}

/// Create-backup plan plus requirements.
///
/// Delegates the plan body to `crate::pages_backup::backup_text` (which
/// mirrors `New-TTBackup` / `do_backup` and the `gsi-diag::plan_backup`
/// shape) and adds the requirements block. The live `dd` dump
/// (`gsi-exec::backup_dd_cmd`) is always refused here.
pub fn create_backup_center_text(
    partition: &str,
    dest_dir: &str,
    model: &str,
    firmware: &str,
) -> String {
    let mut out = String::new();
    out.push_str("Create backup (read-only plan, nothing executed)\n");
    out.push_str(&crate::pages_backup::backup_text(
        partition, dest_dir, model, firmware,
    ));
    out.push_str("requirements:\n");
    out.push_str(" - dest dir exists and is writable (resolved via the gsi-config data dir)\n");
    out.push_str(" - enough free space for the full partition dump\n");
    out.push_str(" - live device in adb/fastboot mode for the dump step (gsi-exec::backup_dd_cmd, Phase 8)\n");
    out.push_str("refused: the dump step needs a live device; this view only renders the plan\n");
    out
}

/// Verify-first restore flow display.
///
/// Stages render as `select -> verify -> target -> preflight -> review ->
/// typed RESTORE -> execute -> verify`. The plan body delegates to
/// `crate::pages_backup::restore_text` (which mirrors
/// `Invoke-TTRestoreFlow` / `do_restore`). `confirm_restore` is the first
/// typed word (`RESTORE`), `confirm_yes` the second (`YES`). Execution is
/// refused here; the post-restore hash and slot check stay shell-owned.
pub fn restore_view_center_text(
    backup_dir: &str,
    partition: &str,
    backup_has_original: bool,
    confirm_restore: bool,
    confirm_yes: bool,
) -> String {
    let mut out = String::new();
    out.push_str("Restore (verify-first flow, read-only, nothing executed)\n");
    out.push_str("flow: select -> verify -> target -> preflight -> review -> typed RESTORE -> execute -> verify\n");
    if backup_dir.trim().is_empty() {
        out.push_str("[select] no backup dir selected\n");
    } else {
        out.push_str(&format!("[select] {}\n", backup_dir.trim()));
    }
    if backup_has_original {
        out.push_str("[verify] original.img present, sha256 sidecar checked by shell\n");
    } else {
        out.push_str("[verify] blocked: no original.img, nothing to verify\n");
    }
    if partition.trim().is_empty() {
        out.push_str("[target] no target partition\n");
    } else {
        out.push_str(&format!("[target] {}\n", partition.trim()));
    }
    out.push_str("[preflight] fastboot mode + backup-first gates (shell-owned)\n");
    out.push_str(&crate::pages_backup::restore_text(
        backup_dir,
        partition,
        backup_has_original,
    ));
    out.push_str("[review] plan above; execution stays in the guided runner\n");
    if confirm_restore {
        out.push_str("[typed RESTORE] given (1/2)\n");
    } else {
        out.push_str("[typed RESTORE] missing (1/2)\n");
    }
    if confirm_yes {
        out.push_str("[typed YES] given (2/2)\n");
    } else {
        out.push_str("[typed YES] missing (2/2)\n");
    }
    out.push_str("[execute] refused here: live fastboot flash is Phase 8 work\n");
    out.push_str("[verify] post-restore hash + slot check (shell-owned)\n");
    out
}

/// Full-reinstall checklist preview.
///
/// Delegates to `crate::pages_backup::reinstall_text` (which mirrors
/// `Screen-Reinstall` / `screen_reinstall`). Empty model falls back to
/// `VTR-L29`, empty partition to `recovery_ramdisk`, exactly like the
/// shared body. Wipe and flash stay refused without a live device.
pub fn reinstall_center_text(model: &str, partition: &str) -> String {
    let mut out = String::new();
    out.push_str("Reinstall center (read-only, nothing executed)\n");
    out.push_str(&crate::pages_backup::reinstall_text(model, partition));
    out
}

/// Saved workflow-state resume preview.
///
/// Delegates to `crate::pages_backup::resume_text` (which mirrors
/// `Screen-Resume` / `screen_resume`). `goal` is the saved goal id,
/// `steps_text` carries preformatted per-step lines. Execution stays in
/// the guided runner; this view only renders the saved plan.
pub fn resume_center_text(goal: &str, steps_text: &str) -> String {
    let mut out = String::new();
    out.push_str("Resume center (read-only, nothing executed)\n");
    out.push_str(&crate::pages_backup::resume_text(goal, steps_text));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(dir: &str, date: &str, state: &str, sha: &str) -> (String, String, String, String) {
        (
            dir.to_string(),
            date.to_string(),
            state.to_string(),
            sha.to_string(),
        )
    }

    #[test]
    fn backup_empty_renders_empty_state_and_cta() {
        let t = backup_center_text("VTR-L29", "backups/VTR-L29", &[]);
        assert!(t.contains("Backup center"));
        assert!(t.contains("No backups yet"));
        assert!(t.contains("Create CTA"));
        assert!(t.contains("nothing is faked"));
    }

    #[test]
    fn backup_tree_renders_states_and_hashes() {
        let rows = vec![
            entry(
                "backups/VTR-L29/recovery_ramdisk/s1",
                "2026-10-07",
                "verified",
                "ab12",
            ),
            entry("backups/VTR-L29/recovery_ramdisk/s0", "2026-10-06", "", ""),
        ];
        let t = backup_center_text("VTR-L29", "backups/VTR-L29", &rows);
        assert!(t.contains("model: VTR-L29"));
        assert!(t.contains("2026-10-07 backups/VTR-L29/recovery_ramdisk/s1"));
        assert!(t.contains("state: verified"));
        assert!(t.contains("sha256: ab12"));
        assert!(t.contains("state: unverified"));
        assert!(t.contains("sha256: unknown"));
        assert!(t.contains("corrupt = mismatch, do not flash"));
    }

    #[test]
    fn backup_blank_rows_fall_back_to_empty_state() {
        let rows = vec![entry("  ", "", "", "")];
        let t = backup_center_text("", "", &rows);
        assert!(t.contains("model: unknown"));
        assert!(t.contains("No backups yet"));
    }

    #[test]
    fn create_plan_delegates_and_lists_requirements() {
        let t = create_backup_center_text(
            "recovery_ramdisk",
            "backups/VTR-L29/recovery_ramdisk/20261007-080000",
            "VTR-L29",
            "VTR-L29C432BXXX",
        );
        assert!(t.contains("Create backup"));
        assert!(t.contains("Backup plan"));
        assert!(t.contains("partition: recovery_ramdisk"));
        assert!(t.contains("requirements:"));
        assert!(t.contains("gsi-exec::backup_dd_cmd"));
        assert!(t.contains("refused: the dump step needs a live device"));
    }

    #[test]
    fn create_empty_partition_is_refused() {
        let t = create_backup_center_text("", "backups/dir", "", "");
        assert!(t.contains("refused: empty partition"));
        assert!(t.contains("requirements:"));
    }

    #[test]
    fn restore_flow_shows_stages_and_confirm_state() {
        let t = restore_view_center_text(
            "backups/VTR-L29/recovery_ramdisk/20261007-080000",
            "recovery_ramdisk",
            true,
            true,
            false,
        );
        assert!(t.contains("verify-first flow"));
        assert!(t.contains("select -> verify -> target -> preflight -> review -> typed RESTORE -> execute -> verify"));
        assert!(t.contains("[select] backups/VTR-L29/recovery_ramdisk/20261007-080000"));
        assert!(t.contains("[verify] original.img present"));
        assert!(t.contains("[target] recovery_ramdisk"));
        assert!(t.contains("Restore plan"));
        assert!(t.contains("[typed RESTORE] given (1/2)"));
        assert!(t.contains("[typed YES] missing (2/2)"));
        assert!(t.contains("[execute] refused here"));
        assert!(t.contains("[verify] post-restore hash + slot check"));
    }

    #[test]
    fn restore_missing_original_blocks_verify() {
        let t = restore_view_center_text("", "", false, false, false);
        assert!(t.contains("[select] no backup dir selected"));
        assert!(t.contains("[verify] blocked: no original.img"));
        assert!(t.contains("[target] no target partition"));
        assert!(t.contains("no backup with original.img found"));
    }

    #[test]
    fn reinstall_delegates_checklist() {
        let t = reinstall_center_text("VTR-L29", "recovery_ramdisk");
        assert!(t.contains("Reinstall center"));
        assert!(t.contains("Full reinstall checklist"));
        assert!(t.contains(
            "ROM choice -> backup reminder -> optional wipe -> flash -> reboot -> verify"
        ));
    }

    #[test]
    fn reinstall_empty_inputs_fall_back() {
        let t = reinstall_center_text("", "");
        assert!(t.contains("model: VTR-L29"));
        assert!(t.contains("slot partition: recovery_ramdisk"));
    }

    #[test]
    fn resume_delegates_goal_and_steps() {
        let t = resume_center_text("full_reinstall", "reconnaissance [done]\nbackup [active]");
        assert!(t.contains("Resume center"));
        assert!(t.contains("goal: full_reinstall"));
        assert!(t.contains("reconnaissance [done]"));
        assert!(t.contains("guided runner"));
    }

    #[test]
    fn resume_empty_goal_reports_none() {
        let t = resume_center_text("  ", "");
        assert!(t.contains("no saved workflow"));
    }
}
