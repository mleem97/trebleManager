//! Pure gate/rule layer ported from `scripts/Treble-Toolkit.ps1` and
//! `scripts/treble-toolkit.sh`.
//!
//! Covers: `Test-StepGate`/`step_gate`, `Test-TTFlashReadiness`/
//! `check_readiness` aggregation, `Invoke-Preflight`/`preflight` pure
//! subset, `Get-PatchBase`/`patch_base`, `Test-FirmwareCompatibility`/
//! `firmware_compat` verdict, `Invoke-FailureFlow` outcomes,
//! `Get-PreferredRootMethod`/`root_method_ids`/`root_method_name`,
//! `Get-TTFirmwareBaseline` normalize, `target_partition` mapping,
//! `Get-OSClassification`/`os_classify`, `Test-TTAdmin` pure core.
//!
//! All inputs are explicit (strings, bools, lists). No process spawning,
//! no device access. Live execution is always refused; exec is Phase 8.

use gsi_registry::{CompatVerdict, RomEntry, TargetConfig};

// ------------------------------------------------------------ step gate

/// Explicit device states (caller queries live state; this crate only rules).
#[derive(Debug, Clone, Default)]
pub struct DeviceStates {
    pub overall: String,
    pub adb: String,
    pub fastboot: String,
}

/// Explicit inputs for one step-gate evaluation.
#[derive(Debug, Clone, Default)]
pub struct GateInput {
    pub step: String,
    pub states: DeviceStates,
    pub profile_verified: bool,
    pub patched_present: bool,
    pub patch_base_present: bool,
    pub patch_base_source: String,
}

/// Step-gate verdict: pass with no reasons, or block with reasons.
#[derive(Debug, Clone)]
pub struct StepGate {
    pub pass: bool,
    pub reasons: Vec<String>,
}

fn norm_step(s: &str) -> String {
    s.trim().to_lowercase()
}

fn needs_device(step: &str) -> bool {
    matches!(
        step,
        "analyze"
            | "verify"
            | "patch"
            | "backup"
            | "flash"
            | "restore"
            | "flash-system"
            | "twrp"
            | "validate"
            | "export"
            | "diagnostic"
    )
}

fn needs_fastboot(step: &str) -> bool {
    matches!(step, "flash" | "restore" | "flash-system" | "twrp")
}

fn needs_android(step: &str) -> bool {
    matches!(step, "analyze" | "verify" | "validate")
}

fn needs_verified_profile(step: &str) -> bool {
    matches!(step, "flash" | "twrp" | "flash-system")
}

/// Evaluate a step gate over explicit inputs (mirrors `Test-StepGate`).
pub fn step_gate(input: &GateInput) -> StepGate {
    let step = norm_step(&input.step);
    let mut reasons: Vec<String> = Vec::new();
    let overall = input.states.overall.trim().to_uppercase();
    if needs_device(step.as_str()) && overall == "NO_DEVICE" {
        reasons.push("no device connected".to_string());
    }
    if needs_android(step.as_str()) && input.states.adb.trim() != "ADB_READY" {
        reasons.push(format!(
            "need Android/ADB (now: {})",
            input.states.adb.trim()
        ));
    }
    if needs_fastboot(step.as_str()) && input.states.fastboot.trim() != "FASTBOOT_READY" {
        reasons.push(format!(
            "need fastboot (now: {})",
            input.states.fastboot.trim()
        ));
    }
    if needs_verified_profile(step.as_str()) && !input.profile_verified {
        reasons.push("profile unverified".to_string());
    }
    if step == "flash" && !input.patched_present {
        reasons.push("no patched image registered".to_string());
    }
    if step == "backup" && !input.patch_base_present {
        if input.patch_base_source.trim() == "rom" {
            reasons.push("no ROM base image (recovery export first)".to_string());
        } else {
            reasons.push("no stock image (extract first)".to_string());
        }
    }
    let pass = reasons.is_empty();
    StepGate { pass, reasons }
}

// ------------------------------------------------------------ readiness

/// Pure parts for flash-readiness aggregation (caller measures each part).
#[derive(Debug, Clone, Default)]
pub struct ReadinessParts {
    pub model_ok: bool,
    pub profile_verified: bool,
    pub partition_ok: bool,
    pub image_exists: bool,
    pub hash_known: bool,
    pub size_ok: bool,
    pub firmware_ok: bool,
    pub backup_ok: bool,
    pub fastboot_ok: bool,
    pub differs_from_stock: bool,
}

/// One named readiness check.
#[derive(Debug, Clone)]
pub struct ReadinessCheck {
    pub name: String,
    pub pass: bool,
    pub detail: String,
}

/// Aggregated flash-readiness verdict.
#[derive(Debug, Clone)]
pub struct FlashReadiness {
    pub go: bool,
    pub checks: Vec<ReadinessCheck>,
}

fn check(name: &str, pass: bool) -> ReadinessCheck {
    let detail = if pass {
        "ok".to_string()
    } else {
        "missing".to_string()
    };
    ReadinessCheck {
        name: name.to_string(),
        pass,
        detail,
    }
}

/// Aggregate readiness parts (mirrors `Test-TTFlashReadiness` table logic).
pub fn check_readiness(parts: &ReadinessParts) -> FlashReadiness {
    let checks = vec![
        check("Model matches (VTR/VKY)", parts.model_ok),
        check("Profile verified (flash allowed)", parts.profile_verified),
        check("Partition exists", parts.partition_ok),
        check("Image exists (patched)", parts.image_exists),
        check("Image hash known (SHA-256)", parts.hash_known),
        check("Image size plausible (1-100 MB)", parts.size_ok),
        check("Firmware compatibility established", parts.firmware_ok),
        check("Backup available (original.img)", parts.backup_ok),
        check("Fastboot device connected", parts.fastboot_ok),
        check("Patched != stock (no fake patch)", parts.differs_from_stock),
    ];
    let mut go = true;
    for c in checks.iter() {
        if !c.pass {
            go = false;
            break;
        }
    }
    FlashReadiness { go, checks }
}

// ------------------------------------------------------------ preflight

/// Explicit preflight inputs (no spawning; caller passes paths and lists).
#[derive(Debug, Clone, Default)]
pub struct PreflightInput {
    pub adb_path: String,
    pub fastboot_path: String,
    pub shell_ok: bool,
    pub writables: Vec<(String, bool)>,
    pub files: Vec<(String, bool)>,
    pub hashes: Vec<(String, String, String)>,
}

/// Preflight verdict: go with no blocks, or blocked with reasons.
#[derive(Debug, Clone)]
pub struct PreflightReport {
    pub go: bool,
    pub blocks: Vec<String>,
}

/// Pure preflight over explicit inputs (mirrors `Invoke-Preflight` checks).
pub fn preflight(input: &PreflightInput) -> PreflightReport {
    let mut blocks: Vec<String> = Vec::new();
    if !input.shell_ok {
        blocks.push("unsupported shell (need PowerShell 5.1+/7+ or POSIX sh)".to_string());
    }
    if input.adb_path.trim().is_empty() {
        blocks.push("adb missing (install via Setup or platform-tools on PATH)".to_string());
    }
    if input.fastboot_path.trim().is_empty() {
        blocks.push("fastboot missing (install via Setup or platform-tools on PATH)".to_string());
    }
    for (name, ok) in input.writables.iter() {
        if !ok {
            blocks.push(format!("directory not writable: {name}"));
        }
    }
    for (name, exists) in input.files.iter() {
        if !exists {
            blocks.push(format!("file missing: {name}"));
        }
    }
    for (name, expected, actual) in input.hashes.iter() {
        let e = expected.trim().to_lowercase();
        let a = actual.trim().to_lowercase();
        if e.is_empty() || a.is_empty() {
            blocks.push(format!("hash missing: {name}"));
        } else if e != a {
            blocks.push(format!("hash mismatch: {name} (expected != actual)"));
        }
    }
    let go = blocks.is_empty();
    PreflightReport { go, blocks }
}

// ------------------------------------------------------------ patch base

/// Resolved patch base (which image Magisk patches).
#[derive(Debug, Clone)]
pub struct PatchBase {
    pub source: String,
    pub image: String,
    pub label: String,
}

/// Short display name for an installed-ROM id (mirrors `rom_label`).
pub fn rom_label(id: &str) -> String {
    let t = id.trim();
    if t.is_empty() {
        return "?".to_string();
    }
    if t == "stock" {
        return "Stock EMUI".to_string();
    }
    if t == "other" {
        return "Other custom ROM".to_string();
    }
    if let Some(rest) = t.strip_prefix("rom:") {
        return rest.to_string();
    }
    t.to_string()
}

/// GSI field of a registry entry (reuse point for `rom_entry_gsi` logic).
pub fn entry_gsi(entry: &RomEntry) -> &str {
    entry.gsi.as_str()
}

/// Patch-base rule over explicit inputs (mirrors `Get-PatchBase`).
///
/// - empty/`stock` installed ROM -> stock recovery.
/// - GSI/system-only ROM (`entry_gsi` non-empty) -> stock recovery.
/// - full device ROM -> its package image.
/// - `base_kind == "system"` is honestly refused in every branch.
pub fn patch_base(
    installed_rom: &str,
    stock_image: &str,
    rom_image: &str,
    entry_gsi_value: &str,
    base_kind: &str,
) -> Result<PatchBase, String> {
    if base_kind.trim().to_lowercase() == "system" {
        return Err("honest refusal: system image as patch base. Magisk needs boot/recovery; a GSI system image contains no recovery - use the stock UPDATE.APP recovery as patch base.".to_string());
    }
    let rom = installed_rom.trim();
    if rom.is_empty() || rom == "stock" {
        if stock_image.trim().is_empty() {
            return Err("no stock image (extract first)".to_string());
        }
        return Ok(PatchBase {
            source: "stock".to_string(),
            image: stock_image.trim().to_string(),
            label: "Stock EMUI".to_string(),
        });
    }
    if !entry_gsi_value.trim().is_empty() {
        if stock_image.trim().is_empty() {
            return Err(
                "no stock image for GSI patch base (GSI keeps stock recovery; extract first)"
                    .to_string(),
            );
        }
        return Ok(PatchBase {
            source: "stock-gsi".to_string(),
            image: stock_image.trim().to_string(),
            label: rom_label(rom),
        });
    }
    if rom_image.trim().is_empty() {
        return Err("no ROM base image (recovery export first)".to_string());
    }
    Ok(PatchBase {
        source: "rom".to_string(),
        image: rom_image.trim().to_string(),
        label: rom_label(rom),
    })
}

// ------------------------------------------------------------ compat reuse

/// Firmware compatibility verdict (delegates to `gsi_registry`).
pub fn firmware_compat(model: &str, firmware: &str, region: &str) -> CompatVerdict {
    gsi_registry::firmware_compat(model, firmware, region)
}

/// Vendor advice (delegates to `gsi_registry`).
pub fn vendor_advice(emui: &str, target_android: &str) -> String {
    gsi_registry::vendor_advice(emui, target_android)
}

/// Resolved target config (delegates to `gsi_registry`).
pub fn target_config(entry: &RomEntry, firmware_base: &str) -> TargetConfig {
    gsi_registry::target_config(entry, firmware_base)
}

// ------------------------------------------------------------ failure flow

/// Controlled failure-flow outcomes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FailureOutcome {
    Diagnostic,
    RestoreRequired,
    Restored,
    Aborted,
}

/// Map an explicit choice to an outcome (mirrors `Invoke-FailureFlow` menu).
pub fn failure_outcome_from_choice(choice: &str) -> FailureOutcome {
    let c = choice.trim().to_lowercase();
    if c == "1" || c == "diagnostic" || c == "diag" {
        FailureOutcome::Diagnostic
    } else if c == "2" || c == "restore" || c == "restored" {
        FailureOutcome::Restored
    } else if c == "restore-required" || c == "restore_required" {
        FailureOutcome::RestoreRequired
    } else {
        FailureOutcome::Aborted
    }
}

/// Human message for an outcome + step context.
pub fn failure_message(outcome: &FailureOutcome, step: &str, detail: &str) -> String {
    let s = step.trim();
    let d = detail.trim();
    match outcome {
        FailureOutcome::Diagnostic => {
            format!("Step '{s}' blocked/failed -> DIAGNOSTIC collected. Detail: {d}")
        }
        FailureOutcome::RestoreRequired => {
            format!(
                "Step '{s}' failed -> RESTORE_REQUIRED. Restore original before retry. Detail: {d}"
            )
        }
        FailureOutcome::Restored => {
            format!("Step '{s}' recovered -> RESTORED from backup. Detail: {d}")
        }
        FailureOutcome::Aborted => {
            format!("Step '{s}' ended -> ABORTED by user. Nothing flashed after abort. Detail: {d}")
        }
    }
}

// ------------------------------------------------------------ root methods

/// One root method row of the static table.
#[derive(Debug, Clone)]
pub struct RootMethod {
    pub id: String,
    pub name: String,
    pub preferred: bool,
}

fn root_table() -> Vec<RootMethod> {
    vec![
        RootMethod {
            id: "magisk-recovery".to_string(),
            name: "Magisk patched recovery_ramdisk (PREFERRED)".to_string(),
            preferred: true,
        },
        RootMethod {
            id: "magisk-twrp".to_string(),
            name: "Magisk via TWRP zip (alternative)".to_string(),
            preferred: false,
        },
        RootMethod {
            id: "phh-su".to_string(),
            name: "phh superuser (legacy, no modules)".to_string(),
            preferred: false,
        },
        RootMethod {
            id: "kernelsu".to_string(),
            name: "KernelSU (experimental, v0.9.2 only)".to_string(),
            preferred: false,
        },
    ]
}

/// Static root-method ids in priority order (mirrors `root_method_ids`).
pub fn root_method_ids() -> Vec<String> {
    let mut out = Vec::new();
    for m in root_table() {
        out.push(m.id);
    }
    out
}

/// Static root-method display name (mirrors `root_method_name`).
pub fn root_method_name(id: &str) -> Result<String, String> {
    let want = id.trim();
    for m in root_table() {
        if m.id == want {
            return Ok(m.name);
        }
    }
    Err(format!(
        "unknown root method: '{id}' (known: magisk-recovery, magisk-twrp, phh-su, kernelsu)"
    ))
}

/// Methods with preferred first (mirrors `Get-PreferredRootMethod`).
pub fn preferred_root_methods() -> Vec<RootMethod> {
    let mut pref = Vec::new();
    let mut rest = Vec::new();
    for m in root_table() {
        if m.preferred {
            pref.push(m);
        } else {
            rest.push(m);
        }
    }
    pref.append(&mut rest);
    pref
}

// ------------------------------------------------------------ baseline

/// Collapse whitespace and trim (baseline normalize).
pub fn normalize_firmware_baseline(raw: &str) -> String {
    let mut parts: Vec<&str> = Vec::new();
    for w in raw.split_whitespace() {
        parts.push(w);
    }
    parts.join(" ")
}

/// Baseline from explicit parts (mirrors `Get-TTFirmwareBaseline` pure core).
///
/// Override wins when non-empty; else `display + incremental`, normalized.
pub fn firmware_baseline_from_parts(
    display: &str,
    incremental: &str,
    override_baseline: &str,
) -> String {
    if !override_baseline.trim().is_empty() {
        return normalize_firmware_baseline(override_baseline);
    }
    let mut joined = String::new();
    joined.push_str(display.trim());
    if !display.trim().is_empty() && !incremental.trim().is_empty() {
        joined.push(' ');
    }
    joined.push_str(incremental.trim());
    normalize_firmware_baseline(&joined)
}

// ------------------------------------------------------------ partitions

/// Target partition for a profile (`VTR-*`/`VKY-*` -> `recovery_ramdisk`).
pub fn target_partition(profile: &str) -> String {
    let p = profile.trim().to_uppercase();
    if p.starts_with("VTR-") || p.starts_with("VKY-") {
        return "recovery_ramdisk".to_string();
    }
    String::new()
}

/// Forbidden flash targets (never flash these directly).
pub fn is_forbidden_partition(part: &str) -> bool {
    matches!(
        part.trim().to_lowercase().as_str(),
        "boot" | "recovery" | "system" | "vendor" | "userdata"
    )
}

// ------------------------------------------------------------ os classify

/// OS classification result (mirrors `Get-OSClassification`).
#[derive(Debug, Clone)]
pub struct OsClassification {
    pub kind: String,
    pub detail: String,
    pub android_release: String,
    pub gsi_variant: String,
    pub is_stock: bool,
    pub is_gsi: bool,
    pub supported_android: bool,
    pub emui: String,
    pub notes: Vec<String>,
}

fn blob_has(blob: &str, kw: &str) -> bool {
    blob.contains(kw)
}

fn gsi_variant_of(blob: &str) -> String {
    let marker = "arm64_";
    let mut out = String::new();
    if let Some(pos) = blob.find(marker) {
        let tail = &blob[pos + marker.len()..];
        for ch in tail.chars() {
            if ch.is_ascii_lowercase() || ch.is_ascii_digit() {
                out.push(ch);
            } else {
                break;
            }
        }
    }
    out
}

fn display_marks_stock(display: &str) -> bool {
    let up = display.to_uppercase();
    if up.contains("EMUI") {
        return true;
    }
    if let Some(pos) = up.find("VTR-") {
        let tail = &up[pos + 4..];
        let bytes: Vec<char> = tail.chars().collect();
        let mut i = 0;
        while i < bytes.len() {
            if bytes[i] == 'C' {
                if let Some(n) = bytes.get(i + 1) {
                    if n.is_ascii_digit() {
                        return true;
                    }
                }
            }
            i += 1;
        }
    }
    false
}

/// Classify OS from explicit strings (mirrors `Get-OSClassification` rules).
pub fn os_classify(
    model: &str,
    pname: &str,
    display: &str,
    release: &str,
    emui: &str,
) -> OsClassification {
    let blob = format!("{model} {pname} {display}").to_lowercase();
    let mut is_gsi = false;
    for kw in [
        "lineage_",
        "trebledroid",
        "treble",
        "phh",
        "aosp",
        "pixel",
        "superior",
        "havoc",
        "crdroid",
        "gsi",
        "arm64_b",
    ] {
        if blob_has(&blob, kw) {
            is_gsi = true;
            break;
        }
    }
    let mut is_stock = false;
    if !emui.trim().is_empty() || display_marks_stock(display) {
        is_stock = true;
    }
    if is_gsi {
        is_stock = false;
    }
    let mut kind = "Unknown".to_string();
    if blob_has(&blob, "trebledroid") {
        kind = "TrebleDroid-GSI".to_string();
    } else if blob_has(&blob, "lineage_") {
        kind = "Lineage-GSI".to_string();
    } else if blob_has(&blob, "pixel") {
        kind = "PixelExperience-GSI".to_string();
    } else if blob_has(&blob, "superior") {
        kind = "SuperiorOS-GSI".to_string();
    } else if is_gsi {
        kind = "AOSP/GSI-Custom".to_string();
    } else if is_stock && emui.contains("9.1") {
        kind = "Stock-EMUI-9.1".to_string();
    } else if is_stock && emui.contains("9.0") {
        kind = "Stock-EMUI-9.0".to_string();
    } else if is_stock && emui.contains("8.") {
        kind = "Stock-EMUI-8".to_string();
    } else if is_stock {
        kind = "Stock-EMUI/Harmony-Basis".to_string();
    } else if blob_has(&blob, "havoc") || blob_has(&blob, "crdroid") || blob_has(&blob, "arrow") {
        kind = "Custom-ROM (non-GSI)".to_string();
    }
    let variant = gsi_variant_of(&blob);
    let rel = release.trim().to_string();
    let mut supported = false;
    for v in ["8", "9", "10", "11", "12", "13", "14"] {
        if rel == v || rel.starts_with(&format!("{v}.")) || rel.starts_with(v) && v.len() > 1 {
            supported = true;
            break;
        }
    }
    if rel == "8" || rel == "9" || rel.starts_with("8.") || rel.starts_with("9.") {
        supported = true;
    }
    let mut notes: Vec<String> = Vec::new();
    if kind.contains("GSI") {
        notes.push("GSI hides Huawei firmware base -> determine baseline assisted.".to_string());
    }
    if kind.starts_with("Stock") {
        notes.push("Stock: EMUI version directly readable, baseline verifiable.".to_string());
    }
    if !supported {
        notes.push(format!(
            "Android release '{rel}' untested (8-14 supported)."
        ));
    }
    let detail = format!("{model} / {pname} / Android {rel} ({display})");
    OsClassification {
        kind,
        detail,
        android_release: rel,
        gsi_variant: variant,
        is_stock,
        is_gsi,
        supported_android: supported,
        emui: emui.to_string(),
        notes,
    }
}

// ------------------------------------------------------------ admin core

/// Parse injected `/proc/self/status` content; `true` when Uid is 0.
///
/// Returns `Err` when no `Uid:` line or value is unreadable (honest, no guess).
pub fn admin_from_proc_status(content: &str) -> Result<bool, String> {
    for line in content.lines() {
        if let Some(rest) = line.trim_start().strip_prefix("Uid:") {
            let rest = rest.trim();
            let mut first = String::new();
            for ch in rest.chars() {
                if ch.is_ascii_digit() {
                    first.push(ch);
                } else {
                    break;
                }
            }
            if first.is_empty() {
                return Err("honest: Uid line unreadable; cannot determine admin".to_string());
            }
            match first.parse::<u32>() {
                Ok(n) => return Ok(n == 0),
                Err(_) => {
                    return Err("honest: Uid value not numeric; cannot determine admin".to_string());
                }
            }
        }
    }
    Err(
        "honest: no Uid line in injected status; cannot determine admin without OS query"
            .to_string(),
    )
}

/// Windows guidance (UAC stays OS-side; this crate never elevates).
pub fn windows_admin_guidance() -> String {
    "Windows admin (UAC) stays OS-side: re-run elevated if fastboot drivers need it; this crate never elevates."
        .to_string()
}

// ------------------------------------------------------------ live refusal

/// Refuse live/device execution with a clear message (exec = Phase 8).
pub fn refuse_live(op: &str) -> Result<(), String> {
    let name = op.trim();
    if name.is_empty() {
        return Err(
            "live execution refused: device/tool operation needs Phase 8 executor, not this gate crate."
                .to_string(),
        );
    }
    Err(format!(
        "live execution refused: '{name}' needs device/tools; exec = Phase 8, not this crate."
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gate_input(step: &str) -> GateInput {
        GateInput {
            step: step.to_string(),
            states: DeviceStates {
                overall: "READY".to_string(),
                adb: "ADB_READY".to_string(),
                fastboot: "FASTBOOT_READY".to_string(),
            },
            profile_verified: true,
            patched_present: true,
            patch_base_present: true,
            patch_base_source: "stock".to_string(),
        }
    }

    #[test]
    fn temp_dir_image_paths() {
        let mut dir = std::env::temp_dir();
        dir.push("gsi-gates-test");
        let _ = std::fs::create_dir_all(&dir);
        let mut stock = dir.clone();
        stock.push("stock.img");
        let mut rom = dir.clone();
        rom.push("rom-boot.img");
        let _ = std::fs::write(&stock, b"boot");
        let _ = std::fs::write(&rom, b"boot");
        let stock_s = stock.to_string_lossy().to_string();
        let rom_s = rom.to_string_lossy().to_string();
        let a = patch_base("stock", &stock_s, "", "", "boot");
        assert!(a.is_ok());
        let b = patch_base("rom:Custom Device Build", "", &rom_s, "", "boot");
        assert!(b.is_ok());
        let _ = std::fs::remove_file(&stock);
        let _ = std::fs::remove_file(&rom);
        let _ = std::fs::remove_dir(&dir);
    }

    #[test]
    fn gate_pass_and_block() {
        let ok = step_gate(&gate_input("analyze"));
        assert!(ok.pass);
        assert!(ok.reasons.is_empty());
        let mut bad = gate_input("flash");
        bad.states.fastboot = "FASTBOOT_NO_DEVICE".to_string();
        bad.profile_verified = false;
        bad.patched_present = false;
        let r = step_gate(&bad);
        assert!(!r.pass);
        assert!(r.reasons.len() >= 3);
        let mut no_dev = gate_input("verify");
        no_dev.states.overall = "NO_DEVICE".to_string();
        no_dev.states.adb = "NO_DEVICE".to_string();
        let r2 = step_gate(&no_dev);
        assert!(!r2.pass);
        let mut bak = gate_input("backup");
        bak.patch_base_present = false;
        bak.patch_base_source = "rom".to_string();
        let r3 = step_gate(&bak);
        assert!(!r3.pass);
        let mut bak2 = gate_input("backup");
        bak2.patch_base_present = false;
        bak2.patch_base_source = "stock".to_string();
        let r4 = step_gate(&bak2);
        assert!(!r4.pass);
    }

    #[test]
    fn readiness_aggregation() {
        let all = ReadinessParts {
            model_ok: true,
            profile_verified: true,
            partition_ok: true,
            image_exists: true,
            hash_known: true,
            size_ok: true,
            firmware_ok: true,
            backup_ok: true,
            fastboot_ok: true,
            differs_from_stock: true,
        };
        let r = check_readiness(&all);
        assert!(r.go);
        assert!(r.checks.len() == 10);
        let mut one = all.clone();
        one.backup_ok = false;
        let r2 = check_readiness(&one);
        assert!(!r2.go);
        let mut count_fail = 0;
        for c in r2.checks.iter() {
            if !c.pass {
                count_fail += 1;
            }
        }
        assert!(count_fail == 1);
    }

    #[test]
    fn patch_base_branches_and_refusal() {
        let stock = patch_base("stock", "/tmp/a/stock.img", "", "", "boot");
        assert!(stock.is_ok());
        if let Ok(pb) = stock {
            assert!(pb.source == "stock");
        }
        let gsi = patch_base(
            "rom:Lineage 20",
            "/tmp/a/stock.img",
            "",
            "arm64_bgN",
            "boot",
        );
        assert!(gsi.is_ok());
        if let Ok(pb) = gsi {
            assert!(pb.source == "stock-gsi");
        }
        let rom = patch_base(
            "rom:Custom Device Build",
            "",
            "/tmp/a/rom-boot.img",
            "",
            "boot",
        );
        assert!(rom.is_ok());
        if let Ok(pb) = rom {
            assert!(pb.source == "rom");
        }
        let refused = patch_base("stock", "/tmp/a/stock.img", "", "", "system");
        assert!(refused.is_err());
        let refused2 = patch_base(
            "rom:Lineage 20",
            "/tmp/a/stock.img",
            "",
            "arm64_bgN",
            "system",
        );
        assert!(refused2.is_err());
        let missing_stock = patch_base("stock", "", "", "", "boot");
        assert!(missing_stock.is_err());
        let missing_rom = patch_base("rom:Custom Device Build", "", "", "", "boot");
        assert!(missing_rom.is_err());
    }

    #[test]
    fn firmware_verdict_matrix() {
        let ok = firmware_compat("VTR-L29", "VTR-L29 9.1.0.297(C432E5R1P9)", "C432");
        assert!(ok.status == "PASS");
        let wrong = firmware_compat("VTR-L29", "VKY-L29 9.1.0.297(C432E5R1P9)", "C432");
        assert!(wrong.status == "FAIL");
        let sub = firmware_compat("VTR-L29", "VTR-L09 9.1.0.297(C432E5R1P9)", "C432");
        assert!(sub.status == "WARN");
        let empty = firmware_compat("VTR-L29", "", "");
        assert!(empty.status == "FAIL");
        let region = firmware_compat("VTR-L29", "VTR-L29 9.1.0.297(C432E5R1P9)", "C185");
        assert!(region.status == "WARN");
    }

    #[test]
    fn root_table_lookups() {
        let ids = root_method_ids();
        assert!(ids.len() == 4);
        assert!(ids.first().map(|s| s.as_str()) == Some("magisk-recovery"));
        let n = root_method_name("magisk-recovery");
        assert!(n.is_ok());
        if let Ok(name) = n {
            assert!(name.contains("PREFERRED"));
        }
        let bad = root_method_name("nope");
        assert!(bad.is_err());
        let ordered = preferred_root_methods();
        assert!(ordered.len() == 4);
        if let Some(first) = ordered.first() {
            assert!(first.preferred);
            assert!(first.id == "magisk-recovery");
        }
        let adv = vendor_advice("9.1", "13");
        assert!(adv.contains("expected to boot"));
    }

    #[test]
    fn os_classify_samples() {
        let s = os_classify(
            "VTR-L29",
            "VTR-L29",
            "VTR-L29 9.1.0.297(C432E5R1P9)",
            "9",
            "EmotionUI_9.1",
        );
        assert!(s.kind == "Stock-EMUI-9.1");
        assert!(s.is_stock);
        assert!(!s.is_gsi);
        let g = os_classify(
            "lineage_vtr",
            "lineage_vtr",
            "lineage_vtr-userdebug 13 arm64_bgN",
            "13",
            "",
        );
        assert!(g.is_gsi);
        assert!(g.kind == "Lineage-GSI");
        let t = os_classify(
            "trebledroid",
            "trebledroid",
            "trebledroid 13 arm64_bvN",
            "13",
            "",
        );
        assert!(t.kind == "TrebleDroid-GSI");
        let u = os_classify("", "", "", "", "");
        assert!(u.kind == "Unknown");
        assert!(!u.supported_android);
    }

    #[test]
    fn admin_parse_fixtures() {
        let root = "Name:\ttool\nUid:\t0\t0\t0\t0\nGid:\t0\t0\t0\t0\n";
        let r = admin_from_proc_status(root);
        assert!(r.is_ok());
        if let Ok(v) = r {
            assert!(v);
        }
        let user = "Name:\ttool\nUid:\t1000\t1000\t1000\t1000\n";
        let r2 = admin_from_proc_status(user);
        assert!(r2.is_ok());
        if let Ok(v) = r2 {
            assert!(!v);
        }
        let missing = "Name:\ttool\nGid:\t0\n";
        assert!(admin_from_proc_status(missing).is_err());
        let g = windows_admin_guidance();
        assert!(g.contains("UAC"));
    }

    #[test]
    fn preflight_blocks_and_baseline_partition() {
        let good = PreflightInput {
            adb_path: "/usr/bin/adb".to_string(),
            fastboot_path: "/usr/bin/fastboot".to_string(),
            shell_ok: true,
            writables: vec![("logs".to_string(), true)],
            files: vec![("x.img".to_string(), true)],
            hashes: vec![("x".to_string(), "ab".to_string(), "ab".to_string())],
        };
        let r = preflight(&good);
        assert!(r.go);
        let mut bad = good.clone();
        bad.adb_path = String::new();
        bad.hashes = vec![("x".to_string(), "ab".to_string(), "cd".to_string())];
        let r2 = preflight(&bad);
        assert!(!r2.go);
        assert!(r2.blocks.len() >= 2);
        assert!(
            normalize_firmware_baseline("  VTR-L29   9.1.0.297(C432E5R1P9)  ")
                == "VTR-L29 9.1.0.297(C432E5R1P9)"
        );
        assert!(!firmware_baseline_from_parts("VTR-L29 9.1.0.275", "C432", "").is_empty());
        assert!(firmware_baseline_from_parts("a", "b", "  custom   base ") == "custom base");
        assert!(target_partition("VTR-L29") == "recovery_ramdisk");
        assert!(target_partition("VKY-L29") == "recovery_ramdisk");
        assert!(target_partition("GENERIC-TREBLE").is_empty());
        assert!(is_forbidden_partition("system"));
        assert!(!is_forbidden_partition("recovery_ramdisk"));
    }

    #[test]
    fn failure_flow_and_refusal() {
        assert!(failure_outcome_from_choice("1") == FailureOutcome::Diagnostic);
        assert!(failure_outcome_from_choice("2") == FailureOutcome::Restored);
        assert!(failure_outcome_from_choice("x") == FailureOutcome::Aborted);
        let m = failure_message(&FailureOutcome::Diagnostic, "flash", "blocked");
        assert!(m.contains("DIAGNOSTIC"));
        let m2 = failure_message(&FailureOutcome::Aborted, "flash", "user abort");
        assert!(m2.contains("ABORTED"));
        assert!(refuse_live("fastboot flash").is_err());
        let e = entry_gsi_label_check();
        assert!(e);
    }

    fn entry_gsi_label_check() -> bool {
        let v: serde_json::Value = serde_json::json!({"name": "LineageOS", "gsi": "arm64_bgN"});
        let mut ok = false;
        if let Ok(entry) = serde_json::from_value::<RomEntry>(v) {
            if entry_gsi(&entry) == "arm64_bgN" {
                let cfg = target_config(&entry, "EMUI 9.1");
                if cfg.firmware_base == "EMUI 9.1" {
                    ok = true;
                }
            }
            if rom_label("stock") != "Stock EMUI" {
                ok = false;
            }
            if rom_label("rom:Lineage 20") != "Lineage 20" {
                ok = false;
            }
        }
        ok
    }
}
