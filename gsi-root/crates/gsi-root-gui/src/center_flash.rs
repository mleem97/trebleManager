//! Flash cluster GUI renders: 10-step flow, progress, system flash,
//! TWRP, wipe.
//!
//! Read-only renders plus honest refusal. Nothing here touches a device,
//! writes a file, spawns a process, or uses the network. All inputs are
//! explicit (plain strings, bools, and string lists); the caller measures
//! live values and passes them in.
//!
//! Reuse (never duplicated):
//! - `crate::pages_flash::flash_system_text` for the system-flash plan.
//! - `crate::pages_flash::wipe_text` for the wipe step list.
//! - `crate::pages_media::twrp_text` for the TWRP image + slot text.
//!
//! Future wiring (needs new dep, NOT added here):
//! - `gsi-workflow::plan_safe_flash`, `plan_system_flash`,
//!   `plan_twrp_flash`, `plan_guided_wipe`, `plan_restore` for live plans.
//! - `gsi-gates::check_readiness`, `failure_message` for live gates.
//! - `gsi-exec::run_plan`, `flash_verdict`, `GoalConfirms` /
//!   `SystemOverride` for live exec + typed-confirm semantics.
//! - `gsi-state::read_slot`, `new_workflow_plan` for live slot + goals.
//!
//! Target rule: only `recovery_ramdisk` or `system` from the explicit list.
//! Any other target renders the honest unknown form; free text is refused.
//!
//! Slint side lives in `ui/center_flash.slint` with the same visual pattern
//! as the Detect page (baked title, desc, Refresh button, read-only
//! TextEdit, append-log passthrough; target uses a ComboBox, never a
//! free-text field).
//!
//! This module uses only `std` plus sibling page modules. No new crate
//! dependency is introduced. No `unwrap` / `expect` / `panic` outside tests.

// ---------------------------------------------------------------- 10-step flow

/// Allowed flash targets (explicit list only, never free text).
const ALLOWED_TARGETS: &[&str] = &["recovery_ramdisk", "system"];

/// True only for the two explicit targets.
fn is_allowed_target(target: &str) -> bool {
    let t = target.trim();
    t == ALLOWED_TARGETS[0] || t == ALLOWED_TARGETS[1]
}

/// Render the 10-step flash flow display.
///
/// Steps in order: device / image / analyze / target / compat / backup /
/// safety / confirm / execute / verify. Every step renders from its
/// explicit input; empty input renders the honest pending form.
/// `target` must be `recovery_ramdisk` or `system`; anything else renders
/// the unknown-target form (free text refused). `confirm_flash` /
/// `confirm_yes` are the typed FLASH + YES states (text only).
/// Destructive execute is always refused here (Phase 8 work, plan only).
#[allow(clippy::too_many_arguments)]
pub fn flash_center_text(
    device_state: &str,
    image_path: &str,
    image_note: &str,
    target: &str,
    compat_note: &str,
    backup_note: &str,
    safety_note: &str,
    confirm_flash: bool,
    confirm_yes: bool,
    verify_note: &str,
) -> String {
    let mut out = String::new();
    out.push_str("Flash flow (read-only, nothing executed)\n");
    out.push_str("10 steps: device / image / analyze / target / compat / backup / safety / confirm / execute / verify.\n");
    let dev = device_state.trim();
    if dev.is_empty() {
        out.push_str("[1/10] device: unknown (no live query in GUI)\n");
    } else {
        out.push_str(&format!("[1/10] device: {dev}\n"));
    }
    let img = image_path.trim();
    if img.is_empty() {
        out.push_str("[2/10] image: (missing - no patched image registered)\n");
    } else {
        out.push_str(&format!("[2/10] image: {img}\n"));
    }
    let note = image_note.trim();
    if note.is_empty() {
        out.push_str("[3/10] analyze: no image check recorded\n");
    } else {
        out.push_str(&format!("[3/10] analyze: {note}\n"));
    }
    let tgt = target.trim();
    if is_allowed_target(tgt) {
        out.push_str(&format!("[4/10] target: {tgt} (explicit list only)\n"));
    } else if tgt.is_empty() {
        out.push_str("[4/10] target: unknown (choose recovery_ramdisk or system)\n");
    } else {
        out.push_str(&format!(
            "[4/10] target: unknown ({tgt} refused - choose recovery_ramdisk or system)\n"
        ));
    }
    let compat = compat_note.trim();
    if compat.is_empty() {
        out.push_str("[5/10] compat: unchecked (registry verdict pending)\n");
    } else {
        out.push_str(&format!("[5/10] compat: {compat}\n"));
    }
    let backup = backup_note.trim();
    if backup.is_empty() {
        out.push_str("[6/10] backup: missing (backup first, original.img required)\n");
    } else {
        out.push_str(&format!("[6/10] backup: {backup}\n"));
    }
    let safety = safety_note.trim();
    if safety.is_empty() {
        out.push_str("[7/10] safety: readiness gate must pass (model, profile, partition, hash, size, firmware, backup, fastboot, differs-from-stock)\n");
    } else {
        out.push_str(&format!("[7/10] safety: {safety}\n"));
    }
    if confirm_flash {
        out.push_str("[8/10] confirm: type FLASH (1/2): given\n");
    } else {
        out.push_str("[8/10] confirm: type FLASH (1/2): missing (typed-confirm required)\n");
    }
    if confirm_yes {
        out.push_str("[8/10] confirm: type YES (2/2): given\n");
    } else {
        out.push_str("[8/10] confirm: type YES (2/2): missing (typed-confirm required)\n");
    }
    out.push_str("[9/10] execute: fastboot flash <target> <image> (destructive, Phase 8 only - refused here)\n");
    let verify = verify_note.trim();
    if verify.is_empty() {
        out.push_str("[10/10] verify: pending (reboot + uid=0 checklist after flash)\n");
    } else {
        out.push_str(&format!("[10/10] verify: {verify}\n"));
    }
    out.push_str("refused: live fastboot flash is Phase 8 work; plan only, nothing written\n");
    out
}

// ---------------------------------------------------------------- progress

/// Render one progress phase line (`phase [status]`).
fn phase_line(phase: &str, status: &str) -> String {
    let p = phase.trim();
    let name = if p.is_empty() { "(unnamed)" } else { p };
    let s = status.trim();
    let st = if s.is_empty() { "pending" } else { s };
    format!(" - {name} [{st}]\n")
}

/// Render flash progress: phases plus stdout/stderr plus safety notes.
///
/// Mirrors the `gsi-workflow` / `gsi-exec` run phases: `phases` carries
/// `(phase, status)` pairs in order (expected: preflight, backup, verify,
/// flash, reboot, verification). Empty input renders the six pending
/// phases honestly. `stdout_text` / `stderr_text` are the last captured
/// outputs (may be empty). Cancel during flash is unsafe (partial write);
/// the failure panel lists View Details / Restore Backup / Save Report as
/// navigation hints only (no action here).
pub fn flash_progress_center_text(
    phases: &[(String, String)],
    stdout_text: &str,
    stderr_text: &str,
    failure_detail: &str,
) -> String {
    let mut out = String::new();
    out.push_str("Flash progress (read-only monitor, nothing executed)\n");
    out.push_str("Phases:\n");
    if phases.is_empty() {
        for p in [
            "preflight",
            "backup",
            "verify",
            "flash",
            "reboot",
            "verification",
        ] {
            out.push_str(&phase_line(p, "pending"));
        }
    } else {
        for (phase, status) in phases {
            out.push_str(&phase_line(phase, status));
        }
    }
    out.push_str("stdout:\n");
    let so = stdout_text.trim();
    if so.is_empty() {
        out.push_str(" (empty)\n");
    } else {
        for line in so.lines() {
            out.push_str(&format!(" | {line}\n"));
        }
    }
    out.push_str("stderr:\n");
    let se = stderr_text.trim();
    if se.is_empty() {
        out.push_str(" (empty)\n");
    } else {
        for line in se.lines() {
            out.push_str(&format!(" | {line}\n"));
        }
    }
    out.push_str("Cancel note: cancel is UNSAFE once flash starts (partial write possible); power loss or cable pull can brick the slot. Restore path only.\n");
    let fail = failure_detail.trim();
    if fail.is_empty() {
        out.push_str("Failure panel: no failure recorded.\n");
    } else {
        out.push_str(&format!("Failure: {fail}\n"));
    }
    out.push_str("On failure (navigation hints, no action here): View Details (open Logs page) / Restore Backup (open Restore plan) / Save Report (use diagnostic bundle).\n");
    out
}

// ---------------------------------------------------------------- system flash

/// Render the system-flash plan.
///
/// Reuses `crate::pages_flash::flash_system_text` (same gates as
/// `gsi-workflow::plan_system_flash`): local image check, researched-broken
/// registry block, profile + fastboot gates, typed FLASH + YES, verdict
/// preview. Nothing is flashed here.
#[allow(clippy::too_many_arguments)]
pub fn system_flash_center_text(
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
    out.push_str("System flash center (read-only, nothing executed)\n");
    out.push_str(&crate::pages_flash::flash_system_text(
        image,
        image_note,
        registry_note,
        profile_verified,
        fastboot_ready,
        image_ok,
        registry_blocked,
        confirm_flash,
        confirm_yes,
        verdict,
        okay,
        total_time,
        failed,
    ));
    out
}

// ---------------------------------------------------------------- twrp

/// Render the TWRP plan with the slot-share warning.
///
/// Reuses `crate::pages_media::twrp_text`: `twrp_image` is the local TWRP
/// image path (empty when none), `slot_state` the shared-slot occupant,
/// `profile_verified` the flash gate. TWRP and Magisk share the
/// recovery_ramdisk slot (mutual overwrite); typed FLASH + YES still
/// required for live work (text only here).
pub fn twrp_center_text(twrp_image: &str, slot_state: &str, profile_verified: bool) -> String {
    let mut out = String::new();
    out.push_str("TWRP center (read-only, nothing executed)\n");
    out.push_str(&crate::pages_media::twrp_text(
        twrp_image,
        slot_state,
        profile_verified,
    ));
    out.push_str(
        "WARNING: TWRP and Magisk-recovery SHARE the recovery_ramdisk slot (last flashed wins).\n",
    );
    out.push_str(
        "Typed-confirm requirement: live flash needs typed FLASH (1/2) then typed YES (2/2).\n",
    );
    out
}

// ---------------------------------------------------------------- wipe

/// Render the wipe plan with the typed WIPE gate plus backup requirement.
///
/// Reuses `crate::pages_flash::wipe_text` step list (userdata only, never
/// automatic, eRecovery fallback always shown). `backup_ok` /
/// `backup_note` add the mandatory backup gate: without a backup the plan
/// renders NOT READY. Typed WIPE (1/2) then YES (2/2) required (text only).
pub fn wipe_center_text(
    fastboot_ready: bool,
    confirm_wipe: bool,
    confirm_yes: bool,
    backup_ok: bool,
    backup_note: &str,
) -> String {
    let mut out = String::new();
    out.push_str("Wipe center (read-only, nothing executed)\n");
    out.push_str(&crate::pages_flash::wipe_text(
        fastboot_ready,
        confirm_wipe,
        confirm_yes,
    ));
    if backup_ok {
        let note = backup_note.trim();
        if note.is_empty() {
            out.push_str("Backup gate: backup present (original.img).\n");
        } else {
            out.push_str(&format!("Backup gate: backup present ({note}).\n"));
        }
    } else {
        out.push_str("Backup gate: NO backup recorded - back up first (typed WIPE still refused without it).\n");
    }
    out.push_str("Typed-confirm display: type WIPE (1/2) then type YES (2/2) on a live fastboot device only.\n");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strings(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn flash_center_golden_ten_steps() {
        let t = flash_center_text(
            "FASTBOOT_READY",
            "data/magisk/magisk_patched.img",
            "PASS|size ok",
            "recovery_ramdisk",
            "firmware PASS",
            "backups/dir (original.img present)",
            "all gates pass",
            true,
            true,
            "reboot + uid=0 checklist",
        );
        assert!(t.contains("Flash flow"));
        assert!(t.contains("[1/10] device: FASTBOOT_READY"));
        assert!(t.contains("[2/10] image: data/magisk/magisk_patched.img"));
        assert!(t.contains("[3/10] analyze: PASS|size ok"));
        assert!(t.contains("[4/10] target: recovery_ramdisk"));
        assert!(t.contains("[5/10] compat: firmware PASS"));
        assert!(t.contains("[6/10] backup:"));
        assert!(t.contains("[7/10] safety:"));
        assert!(t.contains("type FLASH (1/2): given"));
        assert!(t.contains("type YES (2/2): given"));
        assert!(t.contains("[9/10] execute:"));
        assert!(t.contains("[10/10] verify:"));
        assert!(t.contains("Phase 8"));
    }

    #[test]
    fn flash_center_golden_system_target_allowed() {
        let t = flash_center_text("", "", "", "system", "", "", "", false, false, "");
        assert!(t.contains("[4/10] target: system"));
        assert!(t.contains("typed-confirm required"));
    }

    #[test]
    fn flash_center_golden_free_text_refused() {
        let t = flash_center_text("", "", "", "boot", "", "", "", false, false, "");
        assert!(t.contains("refused - choose recovery_ramdisk or system"));
        let empty = flash_center_text("", "", "", "", "", "", "", false, false, "");
        assert!(empty.contains("choose recovery_ramdisk or system"));
    }

    #[test]
    fn flash_progress_golden_phases_and_streams() {
        let phases = vec![
            ("preflight".to_string(), "done".to_string()),
            ("backup".to_string(), "done".to_string()),
            ("verify".to_string(), "active".to_string()),
            ("flash".to_string(), "pending".to_string()),
            ("reboot".to_string(), "pending".to_string()),
            ("verification".to_string(), "pending".to_string()),
        ];
        let t = flash_progress_center_text(&phases, "Sending system", "error: none", "");
        assert!(t.contains("Flash progress"));
        assert!(t.contains("preflight [done]"));
        assert!(t.contains("verify [active]"));
        assert!(t.contains("stdout:"));
        assert!(t.contains("| Sending system"));
        assert!(t.contains("| error: none"));
        assert!(t.contains("UNSAFE once flash starts"));
        assert!(t.contains("View Details (open Logs page)"));
        assert!(t.contains("Restore Backup (open Restore plan)"));
        assert!(t.contains("Save Report (use diagnostic bundle)"));
    }

    #[test]
    fn flash_progress_golden_empty_renders_pending() {
        let t =
            flash_progress_center_text(&[], "", "", "flash FAILED (remote: Command not allowed)");
        assert!(t.contains("preflight [pending]"));
        assert!(t.contains("verification [pending]"));
        assert!(t.contains("stdout:\n (empty)"));
        assert!(t.contains("stderr:\n (empty)"));
        assert!(t.contains("Failure: flash FAILED"));
    }

    #[test]
    fn system_flash_center_golden_ready_and_blocked() {
        let ok = system_flash_center_text(
            "lineage-20-arm64_bgN.img",
            "PASS|header=3A FF 26 ED",
            "",
            true,
            true,
            true,
            false,
            true,
            true,
            "UNCLEAR",
            0,
            "",
            &[],
        );
        assert!(ok.contains("System flash center"));
        assert!(ok.contains("System flash plan"));
        assert!(ok.contains("READY: all gates pass"));
        let blocked = system_flash_center_text(
            "bad.img",
            "FAIL|too small",
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
    }

    #[test]
    fn twrp_center_golden_slot_share_warning() {
        let t = twrp_center_text("data/twrp.img", "magisk", false);
        assert!(t.contains("TWRP center"));
        assert!(t.contains("twrp: data/twrp.img"));
        assert!(t.contains("SHARE the recovery_ramdisk slot"));
        assert!(t.contains("typed FLASH"));
        assert!(t.contains("DO NOT FLASH"));
    }

    #[test]
    fn wipe_center_golden_typed_wipe_and_backup() {
        let t = wipe_center_text(true, true, true, true, "backups/dir");
        assert!(t.contains("Wipe center"));
        assert!(t.contains("Wipe plan"));
        assert!(t.contains("type WIPE (1/2): given"));
        assert!(t.contains("type YES (2/2): given"));
        assert!(t.contains("Backup gate: backup present"));
        assert!(t.contains("type WIPE (1/2) then type YES (2/2)"));
    }

    #[test]
    fn wipe_center_golden_no_backup_is_gate() {
        let t = wipe_center_text(false, false, false, false, "");
        assert!(t.contains("wipe needs fastboot mode"));
        assert!(t.contains("NO backup recorded"));
        assert!(t.contains("NOT READY"));
    }
}
