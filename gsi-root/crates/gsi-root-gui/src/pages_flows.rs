//! Flows cluster GUI renders: compat, goals, root methods, patch, persist,
//! installed-ROM select, help, status, wizard.
//!
//! Read-only renders plus honest refusal. Nothing here touches a device,
//! writes a file, or starts a download. Buttons in the Slint side only call
//! the render callbacks and append a log line.
//!
//! All inputs are explicit (plain strings and string lists). The caller
//! computes live values with the core crates and passes the results in:
//! - compat lists and verdicts: future wiring via `gsi-registry`
//!   (`rom_options`, `target_androids`, `systems_for`, `variants_for`,
//!   `firmware_compat`, `vendor_advice`).
//! - goal steps and plans: future wiring via `gsi-state`
//!   (`goal_steps`, `new_workflow_plan`).
//! - root methods and patch base: future wiring via `gsi-gates`
//!   (`preferred_root_methods`, `patch_base`).
//! - patch refusal and persist plan: future wiring via `gsi-workflow`
//!   (`run_patch` refusal, `plan_persist_fixes`).
//! - bilingual strings: future wiring via `gsi-i18n` (`lookup`).
//!
//! This module uses only `std`, so it adds no new crate dependency to
//! `gsi-root-gui` (which today uses `gsi-image`, `gsi-root-core`,
//! `gsi-config`, `gsi-update`, `gsi-device`, `gsi-tool`, `slint`).
//!
//! Note on `patch_text`: the crate root already exposes
//! `crate::patch_text(path)`. This module exposes
//! `pages_flows::patch_text(source, image, label, partition)` with explicit
//! patch-base inputs. The full paths differ, so both can coexist.

/// Render device and ROM compatibility plus firmware and vendor verdicts.
///
/// Mirrors `Screen-Compatibility` / `screen_compat`: recommended builds,
/// researched-broken builds, firmware base note, firmware compat verdict
/// (`PASS` / `WARN` / `FAIL` with reasons), and vendor advice.
///
/// Empty `recommended` means no tested builds are listed; empty
/// `firmware_reasons` falls back to a neutral line. Nothing is guessed.
pub fn compat_text(
    model: &str,
    firmware_baseline: &str,
    region: &str,
    firmware_status: &str,
    firmware_reasons: &[String],
    vendor_advice: &str,
    recommended: &[String],
    not_recommended: &[String],
    firmware_base_note: &str,
) -> String {
    let mut out = String::new();
    out.push_str("Compatibility registry (researched ROM/firmware matrix)\n");
    let m = model.trim();
    if m.is_empty() {
        out.push_str("model: unknown\n");
    } else {
        out.push_str(&format!("model: {m}\n"));
    }
    let base = firmware_baseline.trim();
    if base.is_empty() {
        out.push_str("firmware baseline: (empty)\n");
    } else {
        out.push_str(&format!("firmware baseline: {base}\n"));
    }
    let reg = region.trim();
    if reg.is_empty() {
        out.push_str("region: unknown\n");
    } else {
        out.push_str(&format!("region: {reg}\n"));
    }
    out.push_str("\nRecommended (tested builds only):\n");
    if recommended.is_empty() {
        out.push_str(" none listed (submit device data first)\n");
    } else {
        for r in recommended {
            out.push_str(&format!(" [+] {r}\n"));
        }
    }
    out.push_str("\nNOT recommended (researched):\n");
    if not_recommended.is_empty() {
        out.push_str(" none listed\n");
    } else {
        for r in not_recommended {
            out.push_str(&format!(" [X] {r}\n"));
        }
    }
    out.push_str("\nFirmware base:\n");
    let note = firmware_base_note.trim();
    if note.is_empty() {
        out.push_str(" (no firmware note)\n");
    } else {
        out.push_str(&format!(" {note}\n"));
    }
    out.push_str("\nCompatibility:\n");
    let status = firmware_status.trim();
    if status.is_empty() {
        out.push_str(" status: UNKNOWN\n");
    } else {
        out.push_str(&format!(" status: {status}\n"));
    }
    if firmware_reasons.is_empty() {
        out.push_str(" - no reasons given\n");
    } else {
        for r in firmware_reasons {
            out.push_str(&format!(" - {r}\n"));
        }
    }
    out.push_str("\nVendor advice:\n");
    let adv = vendor_advice.trim();
    if adv.is_empty() {
        out.push_str(" unknown\n");
    } else {
        out.push_str(&format!(" {adv}\n"));
    }
    out
}

/// Render a workflow goal table plus its plan, with an exec-refusal note.
///
/// Mirrors `Screen-GoalSelect` / `screen_goals` and the planner
/// `New-WorkflowPlan` / `goal_steps`: goal id, ordered steps with per-step
/// status, then a GUI refusal line (read-only render, no run).
/// `statuses` runs parallel to `steps`; a missing entry renders as
/// `pending`. Unknown or empty goal renders honestly.
pub fn goals_text(goal: &str, steps: &[String], statuses: &[String]) -> String {
    let mut out = String::new();
    let g = goal.trim();
    if g.is_empty() {
        out.push_str("Workflow goal: (none)\n");
    } else {
        out.push_str(&format!("Plan for goal: {g}\n"));
    }
    if steps.is_empty() {
        out.push_str(" no steps (unknown goal)\n");
    } else {
        for (i, s) in steps.iter().enumerate() {
            let st = if i < statuses.len() {
                let t = statuses[i].trim();
                if t.is_empty() {
                    "pending"
                } else {
                    t
                }
            } else {
                "pending"
            };
            out.push_str(&format!(" - {s} [{st}]\n"));
        }
    }
    out.push_str("\nGUI is read-only: plan shown, workflow NOT started here.\n");
    out.push_str("Run stays in the script flow with explicit confirmation.\n");
    out
}

/// Render the root-method table.
///
/// Mirrors `Screen-RootMethods` / `screen_rootmethods`: ordered methods as
/// `(id, name, preferred)` triples plus the current selection. The first
/// preferred entry carries the `[PREFERRED]` tag, matching the script order
/// (Magisk patched recovery first).
pub fn rootmethods_text(methods: &[(String, String, bool)], current: &str) -> String {
    let mut out = String::new();
    out.push_str("Root methods (Magisk preferred + compatible alternatives)\n");
    if methods.is_empty() {
        out.push_str(" no methods listed\n");
    } else {
        for (id, name, preferred) in methods {
            let mark = if id.trim() == current.trim() && !current.trim().is_empty() {
                "*"
            } else {
                " "
            };
            let tag = if *preferred { " [PREFERRED]" } else { "" };
            out.push_str(&format!(" [{mark}] {id}: {name}{tag}\n"));
        }
    }
    let cur = current.trim();
    if cur.is_empty() {
        out.push_str("Current: (none)\n");
    } else {
        out.push_str(&format!("Current: {cur}\n"));
    }
    out.push_str("Note: TWRP and Magisk-recovery share the recovery_ramdisk slot.\n");
    out
}

/// Render the patch-base rule plus the honest EXPERIMENTAL refusal.
///
/// Mirrors `Screen-Patch` / `screen_patch` and `Get-PatchBase` /
/// `patch_base`: `stock` means the stock UPDATE.APP recovery image,
/// `stock-gsi` means stock recovery is still correct for a system-only GSI,
/// `rom` means the base must come from the installed ROM package.
/// The GUI never patches; it renders the rule and refuses live work,
/// matching the `gsi-workflow` `run_patch` refusal.
pub fn patch_text(
    base_source: &str,
    base_image: &str,
    base_label: &str,
    target_partition: &str,
) -> String {
    let mut out = String::new();
    out.push_str("Magisk patch (compatible, real patch lives on the phone)\n");
    let src = base_source.trim();
    let label = base_label.trim();
    let img = base_image.trim();
    let part = target_partition.trim();
    if src.is_empty() {
        out.push_str("Source: unknown (no patch base resolved)\n");
    } else if src == "rom" {
        out.push_str(&format!(
            "Phone runs: {label} (full device ROM)\nRULE: patch base MUST come from this ROM package. NOT from stock firmware.\n"
        ));
    } else if src == "stock-gsi" {
        out.push_str(&format!(
            "Phone runs: {label} (GSI, system-only)\nA GSI never touches recovery: recovery_ramdisk is still stock, so the patch base IS the stock recovery. Correct, not a workaround.\n"
        ));
    } else if src == "stock" {
        out.push_str("Phone runs: Stock EMUI\nRule: patch base = stock UPDATE.APP recovery image. Nothing else.\n");
    } else {
        out.push_str(&format!("Source: {src}\n"));
    }
    if img.is_empty() {
        out.push_str("Input: (missing - extract or export first)\n");
    } else {
        out.push_str(&format!("Input: {img}\n"));
    }
    if part.is_empty() {
        out.push_str("Target partition: (unknown)\n");
    } else {
        out.push_str(&format!("Target partition: {part}\n"));
    }
    out.push_str("EXPERIMENTAL - refused in GUI.\nNo hardware POC: nothing written, nothing faked. See devices/huawei-p10 for the POC plan.\n");
    out
}

/// Render the persist-fix plan (service.d boot scripts).
///
/// Mirrors `Screen-PersistFixes` / `screen_persist` and
/// `plan_persist_fixes`: target dir, file list, removability note, and the
/// live-root gate. Honest scope is kept: on P10 every rooted boot still
/// needs the Vol-Up key trick; only the aptouch and speaker fixes persist.
pub fn persist_text(
    target_dir: &str,
    files: &[String],
    removable_note: &str,
    needs_live_root: bool,
) -> String {
    let mut out = String::new();
    out.push_str("Persist root fixes (service.d, survives reboot)\n");
    out.push_str("Honest scope: on P10, Magisk lives in recovery_ramdisk - every ROOTED boot needs Vol-Up + Power. No safe way around that.\n");
    out.push_str("What persists instead: aptouch + speaker fixes as Magisk service.d boot scripts, plus the verified root state.\n");
    let dir = target_dir.trim();
    if dir.is_empty() {
        out.push_str("Target: (unknown)\n");
    } else {
        out.push_str(&format!("Target: {dir}\n"));
    }
    if files.is_empty() {
        out.push_str("Files: (none listed)\n");
    } else {
        out.push_str("Files:\n");
        for f in files {
            out.push_str(&format!(" - {f}\n"));
        }
    }
    let note = removable_note.trim();
    if !note.is_empty() {
        out.push_str(&format!("{note}\n"));
    }
    if needs_live_root {
        out.push_str("Install needs live root (uid=0). GUI only renders the plan.\n");
    } else {
        out.push_str("No live-root gate reported.\n");
    }
    out
}

/// Render the installed-ROM select list.
///
/// Mirrors `Select-InstalledRom` / `select_rom`: plain-word options,
/// current and suggested markers, researched-broken entries (not
/// selectable), and the patch-source rule that follows the choice.
pub fn romselect_text(
    options: &[String],
    current: &str,
    suggested: &str,
    broken: &[String],
) -> String {
    let mut out = String::new();
    out.push_str("Which system is on your phone right now?\n");
    out.push_str("This decides everything: patch source, skipped steps, blocked ROMs.\n");
    if options.is_empty() {
        out.push_str("Options: (none)\n");
    } else {
        for (i, o) in options.iter().enumerate() {
            let mut mark = String::new();
            if !current.trim().is_empty() && o.trim() == current.trim() {
                mark.push_str("  <-- current");
            } else if current.trim().is_empty()
                && !suggested.trim().is_empty()
                && o.trim() == suggested.trim()
            {
                mark.push_str("  <-- detected");
            }
            out.push_str(&format!(" [{}] {o}{mark}\n", i + 1));
        }
    }
    if !broken.is_empty() {
        out.push_str("\nNOT supported (researched broken - cannot be selected):\n");
        for b in broken {
            out.push_str(&format!("  X {b}\n"));
        }
    }
    let cur = current.trim();
    if cur.is_empty() {
        out.push_str("\nPhone runs: (unknown - pick one)\n");
    } else {
        out.push_str(&format!("\nPhone runs: {cur}\n"));
    }
    out.push_str("Rule: full device ROM -> base from its package; GSI or stock -> stock recovery.\n");
    out
}

/// Render the CLI help text.
///
/// Mirrors `Show-TTHelp` / `show_help`: tool name plus version and the
/// command list. Read-only; the GUI never runs these commands itself.
pub fn help_text(version: &str) -> String {
    let mut out = String::new();
    let v = version.trim();
    if v.is_empty() {
        out.push_str("Huawei P10 Root Manager\n");
    } else {
        out.push_str(&format!("Huawei P10 Root Manager v{v}\n"));
    }
    out.push_str("Usage: treble-toolkit [detect|devices|analyze|firmware|download|extract|export|patch|backup|flash|flash-system|twrp|root-methods|compat|persist|validate|verify|restore|wipe|reinstall|rom|download-rom|diagnostic|preflight|recon|status|workflow|resume|root|wizard|help]\n");
    out.push_str("No args: TUI. Download/flash/restore need explicit confirmation.\n");
    out
}

/// Render the device status overview.
///
/// Mirrors `Show-TTStatus` / `status_screen`: profile, OS class and detail,
/// Android release, mode, firmware baseline, installed system, stock and
/// patched images, backup, shared slot, and recovery_ramdisk state.
/// Missing values render as `missing` or `unknown`, never guessed.
pub fn status_text(
    profile: &str,
    os_kind: &str,
    os_detail: &str,
    android: &str,
    mode: &str,
    firmware: &str,
    installed_label: &str,
    stock_image: &str,
    patched_image: &str,
    backup: &str,
    slot: &str,
    recovery: &str,
) -> String {
    let mut out = String::new();
    out.push_str("Status (any OS is detected, nothing assumed)\n");
    let mut line = String::from("Device profile : ");
    if profile.trim().is_empty() {
        line.push_str("unknown");
    } else {
        line.push_str(profile.trim());
    }
    out.push_str(&line);
    out.push('\n');
    if os_kind.trim().is_empty() {
        out.push_str("OS class       : (no analysis yet)\n");
    } else {
        out.push_str(&format!("OS class       : {}\n", os_kind.trim()));
    }
    if !os_detail.trim().is_empty() {
        out.push_str(&format!("Detail         : {}\n", os_detail.trim()));
    }
    if android.trim().is_empty() {
        out.push_str("Android        : unknown\n");
    } else {
        out.push_str(&format!("Android        : {}\n", android.trim()));
    }
    if mode.trim().is_empty() {
        out.push_str("Mode           : none\n");
    } else {
        out.push_str(&format!("Mode           : {}\n", mode.trim()));
    }
    if recovery.trim().is_empty() {
        out.push_str("RecoveryRamdisk: UNKNOWN/NOT DETECTED (run analysis)\n");
    } else {
        out.push_str(&format!("RecoveryRamdisk: {}\n", recovery.trim()));
    }
    if firmware.trim().is_empty() {
        out.push_str("Firmware       : UNKNOWN\n");
    } else {
        out.push_str(&format!("Firmware       : {}\n", firmware.trim()));
    }
    if installed_label.trim().is_empty() {
        out.push_str("Phone runs     : unknown\n");
    } else {
        out.push_str(&format!("Phone runs     : {}\n", installed_label.trim()));
    }
    if stock_image.trim().is_empty() {
        out.push_str("Stock image    : missing\n");
    } else {
        out.push_str(&format!("Stock image    : {}\n", stock_image.trim()));
    }
    if patched_image.trim().is_empty() {
        out.push_str("Patched image  : missing\n");
    } else {
        out.push_str(&format!("Patched image  : {}\n", patched_image.trim()));
    }
    if backup.trim().is_empty() {
        out.push_str("Backup         : missing\n");
    } else {
        out.push_str(&format!("Backup         : {}\n", backup.trim()));
    }
    if slot.trim().is_empty() {
        out.push_str("Slot (recovery_ramdisk): unknown (TWRP/Magisk share it, last flashed wins)\n");
    } else {
        out.push_str(&format!(
            "Slot (recovery_ramdisk): {}  (TWRP/Magisk share it, last flashed wins)\n",
            slot.trim()
        ));
    }
    out.push_str("Bootloader: Unknown/Unlocked/Locked is NEVER guessed.\n");
    out.push_str("Magisk: only report verified (uid=0), never from boot alone.\n");
    out
}

/// Render the guided-wizard path as text.
///
/// Mirrors `Start-TTWizard` / `wizard`: installed system, chosen goal,
/// ordered steps with visible `SKIP` lines, and the patch-source note.
/// Text only; the GUI never runs the steps.
pub fn wizard_text(
    installed_label: &str,
    goal_label: &str,
    steps: &[String],
    skipped: &[String],
    patch_note: &str,
) -> String {
    let mut out = String::new();
    out.push_str("Guided run (asks system + goal, runs step by step)\n");
    let sys = installed_label.trim();
    if sys.is_empty() {
        out.push_str("Your system: (unknown)\n");
    } else {
        out.push_str(&format!("Your system: {sys}\n"));
    }
    let goal = goal_label.trim();
    if goal.is_empty() {
        out.push_str("Goal: (none chosen)\n");
    } else {
        out.push_str(&format!("Goal: {goal}\n"));
    }
    let note = patch_note.trim();
    if !note.is_empty() {
        out.push_str(&format!("{note}\n"));
    }
    if steps.is_empty() {
        out.push_str("Steps: (none)\n");
    } else {
        out.push_str("Steps:\n");
        for (i, s) in steps.iter().enumerate() {
            out.push_str(&format!(" [{}] {s}\n", i + 1));
        }
    }
    if !skipped.is_empty() {
        for s in skipped {
            out.push_str(&format!(" [SKIP] {s}\n"));
        }
    }
    out.push_str("GUI is read-only: path shown, steps NOT started here.\n");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strings(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn compat_golden_lists_both_groups_and_verdict() {
        let t = compat_text(
            "VTR-L29",
            "VTR-L29 9.1.0.297(C432E5R1P9)",
            "C432",
            "PASS",
            &strings(&["EMUI 9.1 base ok (recovery_ramdisk method)."]),
            "Pie vendor: Q/R/S boot. Target Android 13 wanted to boot.",
            &strings(&["LineageOS 20 A13"]),
            &strings(&["LineageOS 20 Light - non-booting"]),
            "required_base EMUI 9.1",
        );
        assert!(t.contains("model: VTR-L29"));
        assert!(t.contains("[+] LineageOS 20 A13"));
        assert!(t.contains("[X] LineageOS 20 Light"));
        assert!(t.contains("status: PASS"));
        assert!(t.contains("Vendor advice:"));
    }

    #[test]
    fn compat_golden_empty_registry_is_honest() {
        let t = compat_text("", "", "", "", &[], "", &[], &[], "");
        assert!(t.contains("model: unknown"));
        assert!(t.contains("none listed (submit device data first)"));
        assert!(t.contains("status: UNKNOWN"));
    }

    #[test]
    fn goals_golden_plan_and_refusal() {
        let t = goals_text(
            "root",
            &strings(&["reconnaissance", "compatibility", "magisk_patch"]),
            &strings(&["pending", "pending", "pending"]),
        );
        assert!(t.contains("Plan for goal: root"));
        assert!(t.contains("reconnaissance [pending]"));
        assert!(t.contains("GUI is read-only"));
    }

    #[test]
    fn goals_golden_unknown_goal_is_honest() {
        let t = goals_text("nope", &[], &[]);
        assert!(t.contains("no steps (unknown goal)"));
        assert!(t.contains("GUI is read-only"));
    }

    #[test]
    fn rootmethods_golden_marks_preferred_and_current() {
        let methods = vec![
            (
                "magisk-recovery".to_string(),
                "Magisk patched recovery_ramdisk (PREFERRED)".to_string(),
                true,
            ),
            (
                "phh-su".to_string(),
                "phh superuser (legacy, no modules)".to_string(),
                false,
            ),
        ];
        let t = rootmethods_text(&methods, "magisk-recovery");
        assert!(t.contains("[PREFERRED]"));
        assert!(t.contains("Current: magisk-recovery"));
        assert!(t.contains("share the recovery_ramdisk slot"));
    }

    #[test]
    fn patch_golden_rom_rule_and_refusal() {
        let t = patch_text("rom", "/data/recovery/rom-boot.img", "Custom Device Build", "recovery_ramdisk");
        assert!(t.contains("MUST come from this ROM package"));
        assert!(t.contains("/data/recovery/rom-boot.img"));
        assert!(t.contains("EXPERIMENTAL - refused in GUI"));
    }

    #[test]
    fn patch_golden_gsi_rule() {
        let t = patch_text("stock-gsi", "/data/stock.img", "Lineage 20", "recovery_ramdisk");
        assert!(t.contains("GSI never touches recovery"));
        assert!(t.contains("EXPERIMENTAL - refused in GUI"));
    }

    #[test]
    fn persist_golden_plan() {
        let t = persist_text(
            "/data/adb/service.d",
            &strings(&["000-treblemanager-aptouch.sh", "000-treblemanager-smartpa.sh"]),
            "Removable: delete the two files.",
            true,
        );
        assert!(t.contains("/data/adb/service.d"));
        assert!(t.contains("aptouch"));
        assert!(t.contains("uid=0"));
    }

    #[test]
    fn romselect_golden_options_and_broken() {
        let t = romselect_text(
            &strings(&["Stock EMUI", "LineageOS 20"]),
            "Stock EMUI",
            "Stock EMUI",
            &strings(&["LineageOS 20 Light - non-booting"]),
        );
        assert!(t.contains("[1] Stock EMUI  <-- current"));
        assert!(t.contains("X LineageOS 20 Light"));
        assert!(t.contains("decides everything"));
    }

    #[test]
    fn help_golden_usage() {
        let t = help_text("0.1.0");
        assert!(t.contains("Huawei P10 Root Manager v0.1.0"));
        assert!(t.contains("wizard"));
    }

    #[test]
    fn status_golden_full() {
        let t = status_text(
            "VTR-L29",
            "Stock-EMUI-9.1",
            "VTR-L29 / VTR-L29 / Android 9",
            "9",
            "android",
            "VTR-L29 9.1.0.297(C432E5R1P9)",
            "Stock EMUI",
            "/data/stock.img",
            "",
            "",
            "magisk",
            "DETECTED",
        );
        assert!(t.contains("Device profile : VTR-L29"));
        assert!(t.contains("OS class"));
        assert!(t.contains("Patched image  : missing"));
        assert!(t.contains("NEVER guessed"));
    }

    #[test]
    fn wizard_golden_steps_and_skips() {
        let t = wizard_text(
            "Stock EMUI",
            "Root only",
            &strings(&["Stock firmware (find + download)", "Magisk patch (on your phone)"]),
            &strings(&["stock UPDATE.APP extract - not needed"]),
            "GSI detected: recovery is untouched stock.",
        );
        assert!(t.contains("Your system: Stock EMUI"));
        assert!(t.contains("[1] Stock firmware"));
        assert!(t.contains("[SKIP] stock UPDATE.APP"));
        assert!(t.contains("read-only"));
    }
}
