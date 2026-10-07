//! Root cluster GUI renders: current state, methods, Magisk, patch,
//! verify-root, persist.
//!
//! Read-only renders plus honest refusal. Nothing here touches a device,
//! writes a file, spawns a process, or uses the network. All inputs are
//! explicit (plain strings and string lists); the caller measures live
//! values and passes them in.
//!
//! Reuse (never duplicated):
//! - `crate::pages_flows::patch_text` for the patch-base rule text.
//! - `crate::pages_flows::persist_text` for the persist plan text.
//! - `crate::pages_flash::verify_text` for the uid=0 checklist text.
//! - `gsi_workflow::classify_verify_root`, `plan_verify_root`,
//!   `validation_check_names`, `workflow_maturity` for verify honesty.
//! - `gsi_update::magisk_version_from_filename` for the APK version guess.
//! - slot occupant string is caller-measured via `gsi_state::read_slot`
//!   (passthrough, no filesystem here).
//!
//! Future wiring (needs new dep, NOT added here):
//! - `gsi-gates::preferred_root_methods` for the live method table.
//! - `gsi-gates::patch_base` for the live patch-base resolution.
//! - `gsi-exec::prepare_magisk_patch`, `magisk_patch_instructions`,
//!   `GoalConfirms` / `SystemOverride` for live Magisk + confirm semantics.
//!
//! Slint side lives in `ui/center_root.slint` with the same visual pattern
//! as the Detect page (baked title, desc, Refresh button, read-only
//! TextEdit, append-log passthrough).
//!
//! This module uses only `std` plus crates `gsi-root-gui` already depends
//! on (`gsi-workflow`, `gsi-update`). No new dependency is introduced.
//! No `unwrap` / `expect` / `panic` outside tests.

// ---------------------------------------------------------------- root state

/// Render current root state plus the honesty badge.
///
/// Mirrors the Status screen slot occupant plus the Vol-Up boot-cheat note:
/// `installed_label` is the saved system display name, `slot` the shared
/// `recovery_ramdisk` occupant (`unknown` when unrecorded), `boot_mode` the
/// persisted boot mode, `root_state` the last verified state (`ROOTED`
/// only when `uid=0` was seen), `device_note` caller context (may be empty).
/// Never claims Ready: the whole root path is EXPERIMENTAL until a hardware
/// POC exists (see `gsi_workflow::workflow_maturity`).
pub fn root_center_text(
    installed_label: &str,
    slot: &str,
    boot_mode: &str,
    root_state: &str,
    device_note: &str,
) -> String {
    let mut out = String::new();
    out.push_str("Root status (read-only, nothing executed)\n");
    out.push_str("EXPERIMENTAL — no hardware POC. Nothing here means Ready.\n");
    let label = installed_label.trim();
    if label.is_empty() {
        out.push_str("Phone runs: unknown (pick a system)\n");
    } else {
        out.push_str(&format!("Phone runs: {label}\n"));
    }
    let s = slot.trim();
    if s.is_empty() {
        out.push_str(
            "Slot (recovery_ramdisk): unknown (TWRP/Magisk share it, last flashed wins)\n",
        );
    } else {
        out.push_str(&format!(
            "Slot (recovery_ramdisk): {s} (TWRP/Magisk share it, last flashed wins)\n"
        ));
    }
    let bm = boot_mode.trim();
    if bm.is_empty() {
        out.push_str("Boot mode: unknown (no saved state)\n");
    } else {
        out.push_str(&format!("Boot mode: {bm}\n"));
    }
    out.push_str("Rooted boot needs the key trick every time: Vol-Up + Power until Huawei logo, then release.\n");
    let rs = root_state.trim();
    if rs.is_empty() {
        out.push_str("Verified root: unknown (only uid=0 counts, boot alone is not root)\n");
    } else {
        out.push_str(&format!("Verified root: {rs} (only uid=0 counts)\n"));
    }
    let note = device_note.trim();
    if !note.is_empty() {
        out.push_str(&format!("Note: {note}\n"));
    }
    out.push_str("refused: live root checks need a device (Phase 8 work); render only\n");
    out
}

// ---------------------------------------------------------------- methods

/// Risk line for one known gates method id (no invented methods).
///
/// Unknown ids render an honest unknown-risk line, never a guessed claim.
fn method_risk_line(id: &str) -> &'static str {
    match id.trim() {
        "magisk-recovery" => {
            "risk: modifies boot chain (recovery_ramdisk); every rooted boot needs Vol-Up + Power."
        }
        "magisk-twrp" => {
            "risk: shares recovery_ramdisk slot (mutual overwrite); needs a TWRP build for the exact model."
        }
        "phh-su" => "risk: legacy, no modules.",
        "kernelsu" => "risk: EXPERIMENTAL (v0.9.2 only), never Ready.",
        _ => "risk: unknown method (not in gates table), no claim.",
    }
}

/// Render the root-method cards from an explicit gates table.
///
/// Mirrors `Screen-RootMethods` / `screen_rootmethods` and
/// `gsi-gates::preferred_root_methods`: `methods` carries
/// `(id, name, preferred)` triples in priority order (Magisk patched
/// recovery first). Only the passed ids are rendered; nothing is invented.
/// `current` is the saved selection (may be empty). `compat_note` is caller
/// context (may be empty). Selection is display-only here.
pub fn methods_center_text(
    methods: &[(String, String, bool)],
    current: &str,
    compat_note: &str,
) -> String {
    let mut out = String::new();
    out.push_str("Root methods (read-only cards, nothing executed)\n");
    out.push_str("Source: gates table (preferred first). No method invented here.\n");
    if methods.is_empty() {
        out.push_str(" no methods listed (registry/scripts own the table)\n");
    } else {
        for (id, name, preferred) in methods {
            let mark = if !current.trim().is_empty() && id.trim() == current.trim() {
                "*"
            } else {
                " "
            };
            let tag = if *preferred { " [PREFERRED]" } else { "" };
            out.push_str(&format!(" [{mark}] {id}: {name}{tag}\n"));
            out.push_str(&format!("     {}\n", method_risk_line(id)));
        }
    }
    let cur = current.trim();
    if cur.is_empty() {
        out.push_str("Current: (none)\n");
    } else {
        out.push_str(&format!("Current: {cur}\n"));
    }
    out.push_str("Note: TWRP and Magisk-recovery share the recovery_ramdisk slot.\n");
    out.push_str(
        "Select note: selection is display-only here; execution stays in the guided runner.\n",
    );
    let note = compat_note.trim();
    if !note.is_empty() {
        out.push_str(&format!("Compat: {note}\n"));
    }
    out
}

// ---------------------------------------------------------------- magisk

/// Render Magisk prepare / verify / hash / plan flows.
///
/// Mirrors `gsi-exec::prepare_magisk_patch` staging plus
/// `magisk_patch_instructions` on-device steps and the hash/size gate
/// (patched must differ from stock; identical hash is rejected, no fake
/// patch). `staged_path` is the staged copy, `base_image` the patch base,
/// `apk_name` the local APK file name (version guessed via
/// `gsi_update::magisk_version_from_filename`), `sha_expected` /
/// `sha_actual` the recorded hashes, `size_note` caller size context.
/// Plan-only: EXPERIMENTAL banner, never Ready. Destructive flash needs
/// typed FLASH + YES (shown as requirement text, never executed).
pub fn magisk_center_text(
    staged_path: &str,
    base_image: &str,
    apk_name: &str,
    sha_expected: &str,
    sha_actual: &str,
    size_note: &str,
) -> String {
    let mut out = String::new();
    out.push_str("Magisk (read-only plan, nothing executed)\n");
    out.push_str("EXPERIMENTAL — plan only until hardware POC. Never Ready.\n");
    let base = base_image.trim();
    if base.is_empty() {
        out.push_str("Patch base: (missing - extract or export first)\n");
    } else {
        out.push_str(&format!("Patch base: {base}\n"));
    }
    let staged = staged_path.trim();
    if staged.is_empty() {
        out.push_str("Staged copy: (none - stage the base first)\n");
    } else {
        out.push_str(&format!("Staged copy: {staged}\n"));
    }
    let apk = apk_name.trim();
    if apk.is_empty() {
        out.push_str("Magisk APK: (none cached)\n");
    } else {
        let ver = gsi_update::magisk_version_from_filename(apk);
        out.push_str(&format!("Magisk APK: {apk} (version guess: {ver})\n"));
    }
    out.push_str("On-device patch (manual, on the phone):\n");
    out.push_str(" 1. Install Magisk APK from the official release source.\n");
    out.push_str(" 2. adb push <staged> /sdcard/Download/\n");
    out.push_str(" 3. Magisk app -> Install -> Select and Patch a File -> select RECOVERY_RAMDISK image (NOT boot.img).\n");
    out.push_str(" 4. adb pull /sdcard/Download/magisk_patched-*.img data/magisk/\n");
    let exp = sha_expected.trim();
    let got = sha_actual.trim();
    if exp.is_empty() && got.is_empty() {
        out.push_str("Hash: unknown (no hashes recorded)\n");
    } else {
        if exp.is_empty() {
            out.push_str("Expected hash: (unknown)\n");
        } else {
            out.push_str(&format!("Expected hash: {exp}\n"));
        }
        if got.is_empty() {
            out.push_str("Actual hash: (unknown)\n");
        } else {
            out.push_str(&format!("Actual hash: {got}\n"));
        }
        if !exp.is_empty() && !got.is_empty() {
            if exp.eq_ignore_ascii_case(got) {
                out.push_str(
                    "Hash verdict: REJECTED (identical hash means no real patch, no fake patch).\n",
                );
            } else {
                out.push_str("Hash verdict: differ (patched must differ from stock).\n");
            }
        }
    }
    let size = size_note.trim();
    if size.is_empty() {
        out.push_str("Size: unknown (plausibility unchecked)\n");
    } else {
        out.push_str(&format!("Size: {size}\n"));
    }
    out.push_str(
        "Typed-confirm requirement: live flash needs typed FLASH (1/2) then typed YES (2/2).\n",
    );
    out.push_str("refused: live patch + flash need a device and Phase 8 executor; plan only\n");
    out
}

// ---------------------------------------------------------------- patch

/// Render the patch-base rule plus the honest EXPERIMENTAL refusal.
///
/// Reuses `crate::pages_flows::patch_text` (same patch-base rule shape as
/// `gsi-gates::patch_base`): `stock` means stock UPDATE.APP recovery,
/// `stock-gsi` means stock recovery is still correct for a system-only GSI,
/// `rom` means the base must come from the installed ROM package. The GUI
/// never patches; live work is refused like `gsi-workflow::run_patch`.
pub fn patch_center_text(
    base_source: &str,
    base_image: &str,
    base_label: &str,
    target_partition: &str,
) -> String {
    let mut out = String::new();
    out.push_str("Patch center (read-only, nothing executed)\n");
    out.push_str(&crate::pages_flows::patch_text(
        base_source,
        base_image,
        base_label,
        target_partition,
    ));
    out.push_str("Typed-confirm requirement: live patch + flash need typed FLASH (1/2) then typed YES (2/2).\n");
    out
}

// ---------------------------------------------------------------- verify

/// Render the uid=0 verify checklist.
///
/// Reuses `crate::pages_flash::verify_text` for the transcript classifier
/// (same rules as `gsi_workflow::classify_verify_root`: `uid=0` wins, else
/// su-without-denial is INCONCLUSIVE, else NOT_ROOTED) plus the
/// `plan_verify_root` step order and the `validation_check_names` checklist
/// context. `which_su` / `su_id` / `magisk_v` are explicit transcripts;
/// `checks` carries preformatted validate lines such as `ROOT|1|uid=0`.
/// Read-only; live adb work stays Phase 8.
pub fn verify_root_center_text(
    which_su: &str,
    su_id: &str,
    magisk_v: &str,
    checks: &[String],
) -> String {
    let mut out = String::new();
    out.push_str("Verify root center (read-only, nothing executed)\n");
    let plan = gsi_workflow::plan_verify_root();
    out.push_str("Plan steps:\n");
    for s in &plan.steps {
        out.push_str(&format!(" - {s}\n"));
    }
    out.push_str(&crate::pages_flash::verify_text(
        which_su, su_id, magisk_v, checks,
    ));
    let names = gsi_workflow::validation_check_names();
    if checks.is_empty() {
        out.push_str("Validate checklist (pending live adb):\n");
        for n in &names {
            out.push_str(&format!(" - {n}\n"));
        }
    }
    out.push_str(&format!("Checklist note: {}\n", plan.note));
    out
}

// ---------------------------------------------------------------- persist

/// Render the persist-fix plan plus removability.
///
/// Reuses `crate::pages_flows::persist_text` (same shape as
/// `gsi_workflow::plan_persist_fixes`): `target_dir` is the service.d dir,
/// `files` the file list, `removable_note` the removability line,
/// `needs_live_root` the uid=0 install gate. Honest scope is kept: on P10
/// every rooted boot still needs the Vol-Up key trick; only the aptouch
/// and speaker fixes persist. Install needs live root (uid=0); the GUI
/// only renders the plan.
pub fn persist_center_text(
    target_dir: &str,
    files: &[String],
    removable_note: &str,
    needs_live_root: bool,
) -> String {
    let mut out = String::new();
    out.push_str("Persist center (read-only, nothing executed)\n");
    out.push_str(&crate::pages_flows::persist_text(
        target_dir,
        files,
        removable_note,
        needs_live_root,
    ));
    out.push_str("Typed-confirm requirement: live install runs only with uid=0 verified first.\n");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strings(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn root_center_golden_state_and_badge() {
        let t = root_center_text("Stock EMUI", "magisk", "cheat", "ROOTED (uid=0)", "");
        assert!(t.contains("Root status"));
        assert!(t.contains("EXPERIMENTAL"));
        assert!(t.contains("Phone runs: Stock EMUI"));
        assert!(t.contains("Slot (recovery_ramdisk): magisk"));
        assert!(t.contains("Vol-Up + Power"));
        assert!(t.contains("only uid=0 counts"));
        assert!(!t.contains("READY"));
    }

    #[test]
    fn root_center_golden_unknown_is_honest() {
        let t = root_center_text("", "", "", "", "");
        assert!(t.contains("Phone runs: unknown"));
        assert!(t.contains("unknown (TWRP/Magisk share it"));
        assert!(t.contains("unknown (no saved state)"));
        assert!(t.contains("unknown (only uid=0 counts"));
    }

    #[test]
    fn methods_center_golden_gates_table() {
        let methods = vec![
            (
                "magisk-recovery".to_string(),
                "Magisk patched recovery_ramdisk (PREFERRED)".to_string(),
                true,
            ),
            (
                "magisk-twrp".to_string(),
                "Magisk via TWRP zip (alternative)".to_string(),
                false,
            ),
            (
                "phh-su".to_string(),
                "phh superuser (legacy, no modules)".to_string(),
                false,
            ),
            (
                "kernelsu".to_string(),
                "KernelSU (experimental, v0.9.2 only)".to_string(),
                false,
            ),
        ];
        let t = methods_center_text(&methods, "magisk-recovery", "");
        assert!(t.contains("Root methods"));
        assert!(t.contains("magisk-recovery"));
        assert!(t.contains("magisk-twrp"));
        assert!(t.contains("phh-su"));
        assert!(t.contains("kernelsu"));
        assert!(t.contains("[PREFERRED]"));
        assert!(t.contains("Current: magisk-recovery"));
        assert!(t.contains("share the recovery_ramdisk slot"));
        assert!(t.contains("display-only"));
    }

    #[test]
    fn methods_center_golden_empty_is_honest() {
        let t = methods_center_text(&[], "", "");
        assert!(t.contains("no methods listed"));
        assert!(t.contains("Current: (none)"));
    }

    #[test]
    fn methods_center_golden_unknown_id_has_no_claim() {
        let methods = vec![("custom-xyz".to_string(), "Custom XYZ".to_string(), false)];
        let t = methods_center_text(&methods, "", "");
        assert!(t.contains("custom-xyz"));
        assert!(t.contains("unknown method"));
    }

    #[test]
    fn magisk_center_golden_flows_and_experimental() {
        let t = magisk_center_text(
            "data/staging/recovery.img",
            "data/stock.img",
            "Magisk-v27.0.apk",
            "aa11",
            "bb22",
            "plausible (1-100 MB)",
        );
        assert!(t.contains("Magisk (read-only plan"));
        assert!(t.contains("EXPERIMENTAL"));
        assert!(t.contains("Patch base: data/stock.img"));
        assert!(t.contains("Staged copy: data/staging/recovery.img"));
        assert!(t.contains("Select and Patch a File"));
        assert!(t.contains("differ (patched must differ"));
        assert!(t.contains("typed FLASH"));
        assert!(!t.contains("READY"));
    }

    #[test]
    fn magisk_center_golden_identical_hash_rejected() {
        let t = magisk_center_text("", "", "", "aa11", "aa11", "");
        assert!(t.contains("REJECTED (identical hash"));
    }

    #[test]
    fn patch_center_golden_rule_and_refusal() {
        let t = patch_center_text(
            "rom",
            "/data/recovery/rom-boot.img",
            "Custom Device Build",
            "recovery_ramdisk",
        );
        assert!(t.contains("Patch center"));
        assert!(t.contains("MUST come from this ROM package"));
        assert!(t.contains("EXPERIMENTAL - refused"));
        assert!(t.contains("typed FLASH"));
    }

    #[test]
    fn verify_root_center_golden_uid0() {
        let t = verify_root_center_text(
            "/system/xbin/su",
            "uid=0(root) gid=0(root)",
            "27.0:MAGISK",
            &strings(&["ROOT|1|uid=0"]),
        );
        assert!(t.contains("Verify root center"));
        assert!(t.contains("reboot-to-android"));
        assert!(t.contains("Result: ROOTED"));
        assert!(t.contains("Only uid=0 counts"));
    }

    #[test]
    fn verify_root_center_golden_empty_checklist() {
        let t = verify_root_center_text("", "", "", &[]);
        assert!(t.contains("Result: NOT_ROOTED"));
        assert!(t.contains("Validate checklist (pending live adb)"));
        assert!(t.contains("MOUNTS-system"));
    }

    #[test]
    fn persist_center_golden_plan_and_removable() {
        let t = persist_center_text(
            "/data/adb/service.d",
            &strings(&[
                "000-treblemanager-aptouch.sh",
                "000-treblemanager-smartpa.sh",
            ]),
            "Removable: delete the two files.",
            true,
        );
        assert!(t.contains("Persist center"));
        assert!(t.contains("/data/adb/service.d"));
        assert!(t.contains("aptouch"));
        assert!(t.contains("uid=0"));
        assert!(t.contains("Vol-Up"));
    }
}
