//! Typed native workflows: plan once, run everywhere (CLI/GUI/scripts).
//!
//! GUI, CLI and scripts share these definitions — no duplicate logic.
//! Destructive steps never run without explicit confirmation (the runner
//! refuses them unless `allow_destructive` is set, i.e. CLI `--yes`).

use gsi_image::{detect_container, Container};
use gsi_root_core::Maturity;
use sha2::{Digest, Sha256, Sha512};
use std::path::{Path, PathBuf};

/// One workflow step (spec §11).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkflowStep {
    Download { url: String, dest: PathBuf },
    VerifyHash { file: PathBuf, sha256: String },
    Analyze { file: PathBuf },
    Extract,
    Patch,
    Repack,
    VerifyOutput,
    Backup,
    DeviceDetect,
    Flash,
    Reboot,
    Test,
    Cleanup,
}

/// A named, inspectable workflow.
#[derive(Debug, Clone)]
pub struct Workflow {
    pub id: String,
    pub name: String,
    pub steps: Vec<WorkflowStep>,
}

/// Progress events for CLI bars and GUI status (spec §31).
#[derive(Debug, Clone)]
pub enum ProgressEvent {
    Started { operation: String },
    Progress { current: u64, total: u64 },
    Message { message: String },
    Warning { message: String },
    Completed,
}

/// Step outcome (honest: unimplemented engines refuse, never fake).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StepOutcome {
    Done,
    Skipped(String),
    RefusedExperimental(String),
    Failed(String),
}

/// P10 → LineageOS 20 reference workflow (spec §34 pipeline, device part
/// still manual until the hardware POC exists).
pub fn p10_lineage20(input: &Path, _output: &Path) -> Workflow {
    Workflow {
        id: "p10-lineage20".to_string(),
        name: "Huawei P10 → LineageOS 20 Root".to_string(),
        steps: vec![
            WorkflowStep::Analyze {
                file: input.to_path_buf(),
            },
            WorkflowStep::Patch,
            WorkflowStep::Repack,
            WorkflowStep::VerifyOutput,
            WorkflowStep::Backup,
            WorkflowStep::Flash,
        ],
    }
}

/// SHA-256 of a file, lowercase hex.
pub fn sha256_file(path: &Path) -> Result<String, String> {
    let data = std::fs::read(path).map_err(|e| format!("read: {e}"))?;
    let mut h = Sha256::new();
    h.update(&data);
    Ok(format!("{:x}", h.finalize()))
}

/// SHA-512 of a file, lowercase hex (mirrors Get-FileHashInfo/file_hash dual output).
pub fn sha512_file(path: &Path) -> Result<String, String> {
    let data = std::fs::read(path).map_err(|e| format!("read: {e}"))?;
    let mut h = Sha512::new();
    h.update(&data);
    Ok(format!("{:x}", h.finalize()))
}

/// Analyze step: real container detection + maturity verdict.
pub fn run_analyze(file: &Path) -> (Vec<ProgressEvent>, StepOutcome) {
    let mut ev = vec![ProgressEvent::Started {
        operation: format!("analyze {}", file.display()),
    }];
    let container = detect_container(file);
    ev.push(ProgressEvent::Message {
        message: format!(
            "container: {}",
            match container {
                Container::Gzip => "gzip",
                Container::AndroidSparse => "android-sparse",
                Container::Raw => "raw",
            }
        ),
    });
    ev.push(ProgressEvent::Completed);
    (ev, StepOutcome::Done)
}

/// Patch step: refuses until a hardware POC exists (never faked).
pub fn run_patch(_plan: &str) -> (Vec<ProgressEvent>, StepOutcome) {
    (
        vec![ProgressEvent::Started {
            operation: "patch".to_string(),
        }],
        StepOutcome::RefusedExperimental(
            "no hardware POC: patching would be unproven. See devices/huawei-p10.".to_string(),
        ),
    )
}

/// Verify a file hash against an expected SHA-256.
pub fn run_verify_hash(file: &Path, expected: &str) -> (Vec<ProgressEvent>, StepOutcome) {
    let mut ev = vec![ProgressEvent::Started {
        operation: format!("verify {}", file.display()),
    }];
    match sha256_file(file) {
        Ok(got) => {
            if got.eq_ignore_ascii_case(expected) {
                ev.push(ProgressEvent::Completed);
                (ev, StepOutcome::Done)
            } else {
                (
                    ev,
                    StepOutcome::Failed(format!("hash mismatch: got {got}, want {expected}")),
                )
            }
        }
        Err(e) => (ev, StepOutcome::Failed(e)),
    }
}

/// Maturity of the whole workflow path (worst link decides).
pub fn workflow_maturity() -> Maturity {
    // Patch engine unproven on hardware → whole root path is Experimental.
    Maturity::Experimental
}

// ------------------------------------------------------------ flash plans
//
// Pure plan builders mirroring `Invoke-TTSafeFlash` / `safe_flash`,
// `Invoke-SystemFlash` / `system_flash`, `Invoke-TwrpFlash` / `twrp_flash`,
// `Invoke-TTRestoreFlow` / `do_restore` and `Invoke-GuidedWipe` /
// `guided_wipe`. Plans are ordered, backup-first and carry the
// double-confirm words. Live fastboot exec is Phase 8 and always refused.

/// One ordered plan step.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanStep {
    /// Stable step id (e.g. `backup`, `confirm-flash`, `flash`).
    pub id: String,
    /// Human detail (command, warning or gate text).
    pub detail: String,
    /// True only for the real flash/erase step.
    pub destructive: bool,
}

/// Pure destructive-operation plan (no I/O, no spawn).
#[derive(Debug, Clone)]
pub struct DestructivePlan {
    /// Op id: `safe-flash`, `system-flash`, `twrp-flash`, `restore`, `wipe`.
    pub op: String,
    /// Target partition (`recovery_ramdisk`, `system`, `userdata`).
    pub partition: String,
    /// Image path (empty for wipe).
    pub image: String,
    /// Ordered safe steps, backup-first, confirm-gated.
    pub steps: Vec<PlanStep>,
    /// True when every gate + both confirmations hold.
    pub ready: bool,
    /// Blocking reasons (empty when ready).
    pub blocks: Vec<String>,
    /// First confirmation given (FLASH / RESTORE / WIPE).
    pub confirm_first: bool,
    /// Second confirmation given (YES).
    pub confirm_second: bool,
    /// Required first word.
    pub first_word: String,
    /// Required second word.
    pub second_word: String,
    /// Warnings shown before confirm (shared slot, boot chain, fallback).
    pub warnings: Vec<String>,
}

fn plan_step(id: &str, detail: &str, destructive: bool) -> PlanStep {
    PlanStep {
        id: id.to_string(),
        detail: detail.to_string(),
        destructive,
    }
}

fn refused_exec(op: &str) -> StepOutcome {
    StepOutcome::RefusedExperimental(format!(
        "live {op} needs device fastboot; exec is Phase 8, plan only."
    ))
}

fn refuse_events(op: &str) -> Vec<ProgressEvent> {
    vec![
        ProgressEvent::Started {
            operation: op.to_string(),
        },
        ProgressEvent::Message {
            message: "refused: destructive exec is Phase 8 (plan only)".to_string(),
        },
    ]
}

/// Safe-flash plan (`Invoke-TTSafeFlash` / `safe_flash`).
///
/// Targets `recovery_ramdisk` only; requires backup, verified profile,
/// fastboot readiness and both confirmations (FLASH + YES).
pub fn plan_safe_flash(
    image: &str,
    partition: &str,
    backup_available: bool,
    profile_verified: bool,
    fastboot_ready: bool,
    confirm_flash: bool,
    confirm_yes: bool,
) -> DestructivePlan {
    let part = partition.trim().to_string();
    let img = image.trim().to_string();
    let mut blocks: Vec<String> = Vec::new();
    if img.is_empty() {
        blocks.push("no patched image registered".to_string());
    }
    if part != "recovery_ramdisk" {
        blocks.push("refuse: safe flash only targets recovery_ramdisk".to_string());
    }
    if !backup_available {
        blocks.push("no backup (original.img) - backup first".to_string());
    }
    if !profile_verified {
        blocks.push("profile unverified".to_string());
    }
    if !fastboot_ready {
        blocks.push("need fastboot (FASTBOOT_READY)".to_string());
    }
    if !confirm_flash {
        blocks.push("missing first confirmation (FLASH)".to_string());
    }
    if !confirm_yes {
        blocks.push("missing second confirmation (YES)".to_string());
    }
    let detail_flash = format!("fastboot flash {part} <patched>");
    let steps = vec![
        plan_step(
            "safety-check",
            "readiness gate (10 checks) must pass",
            false,
        ),
        plan_step("backup", "mandatory backup-first: original.img", false),
        plan_step(
            "warn-bootchain",
            "WARNING: modifies boot chain; system/vendor untouched; userdata NEVER wiped",
            false,
        ),
        plan_step("confirm-flash", "type FLASH (1/2)", false),
        plan_step("confirm-yes", "type YES (2/2)", false),
        plan_step("flash", detail_flash.as_str(), true),
        plan_step("record-slot", "slot holds magisk after OK", false),
    ];
    let ready = blocks.is_empty();
    DestructivePlan {
        op: "safe-flash".to_string(),
        partition: part,
        image: img,
        steps,
        ready,
        blocks,
        confirm_first: confirm_flash,
        confirm_second: confirm_yes,
        first_word: "FLASH".to_string(),
        second_word: "YES".to_string(),
        warnings: vec![
            "GSI (system/vendor) stays untouched. userdata is NEVER wiped.".to_string(),
            "TWRP and Magisk share the recovery_ramdisk slot (mutual overwrite).".to_string(),
        ],
    }
}

/// Destructive safe-flash exec (always refused; Phase 8 owns fastboot).
pub fn run_safe_flash(plan: &DestructivePlan) -> (Vec<ProgressEvent>, StepOutcome) {
    let _ = plan;
    (refuse_events("safe-flash"), refused_exec("fastboot flash"))
}

/// System-flash plan (`Invoke-SystemFlash` / `system_flash`).
///
/// `image_ok` is the caller-measured system-image check; `registry_blocked`
/// is true when the filename matches a researched-broken registry marker.
pub fn plan_system_flash(
    image: &str,
    profile_verified: bool,
    fastboot_ready: bool,
    image_ok: bool,
    registry_blocked: bool,
    confirm_flash: bool,
    confirm_yes: bool,
) -> DestructivePlan {
    let img = image.trim().to_string();
    let mut blocks: Vec<String> = Vec::new();
    if img.is_empty() {
        blocks.push("no system image path".to_string());
    }
    if !image_ok {
        blocks.push("system image check FAIL".to_string());
    }
    if registry_blocked {
        blocks.push("registry BLOCKED (researched broken)".to_string());
    }
    if !profile_verified {
        blocks.push("profile unverified".to_string());
    }
    if !fastboot_ready {
        blocks.push("need fastboot (FASTBOOT_READY)".to_string());
    }
    if !confirm_flash {
        blocks.push("missing first confirmation (FLASH)".to_string());
    }
    if !confirm_yes {
        blocks.push("missing second confirmation (YES)".to_string());
    }
    let steps = vec![
        plan_step("system-image-check", "test_system_image must PASS", false),
        plan_step("registry-check", "researched-broken builds block hard", false),
        plan_step("backup-reminder", "back up internal storage first", false),
        plan_step(
            "warn-replace-system",
            "WARNING: replaces Android system (fastboot flash system); afterwards eRecovery wipe + first boot",
            false,
        ),
        plan_step("confirm-flash", "type FLASH (1/2)", false),
        plan_step("confirm-yes", "type YES (2/2)", false),
        plan_step("require-fastboot", "abort unless fastboot mode", false),
        plan_step("flash-system", "fastboot flash system <gsi>", true),
    ];
    let ready = blocks.is_empty();
    DestructivePlan {
        op: "system-flash".to_string(),
        partition: "system".to_string(),
        image: img,
        steps,
        ready,
        blocks,
        confirm_first: confirm_flash,
        confirm_second: confirm_yes,
        first_word: "FLASH".to_string(),
        second_word: "YES".to_string(),
        warnings: vec![
            "Back up internal storage first. userdata is NOT wiped automatically.".to_string(),
            "Afterwards: eRecovery wipe data/factory reset, then first boot.".to_string(),
        ],
    }
}

/// Destructive system-flash exec (always refused; Phase 8 owns fastboot).
pub fn run_system_flash(plan: &DestructivePlan) -> (Vec<ProgressEvent>, StepOutcome) {
    let _ = plan;
    (
        refuse_events("system-flash"),
        refused_exec("fastboot flash system"),
    )
}

/// TWRP-flash plan (`Invoke-TwrpFlash` / `twrp_flash`).
pub fn plan_twrp_flash(
    image: &str,
    partition: &str,
    backup_available: bool,
    profile_verified: bool,
    fastboot_ready: bool,
    confirm_flash: bool,
    confirm_yes: bool,
) -> DestructivePlan {
    let part = partition.trim().to_string();
    let img = image.trim().to_string();
    let mut blocks: Vec<String> = Vec::new();
    if img.is_empty() {
        blocks.push("no TWRP image path".to_string());
    }
    if part != "recovery_ramdisk" {
        blocks.push("refuse: TWRP only targets recovery_ramdisk".to_string());
    }
    if !backup_available {
        blocks.push("no backup (original.img) - backup first".to_string());
    }
    if !profile_verified {
        blocks.push("profile unverified".to_string());
    }
    if !fastboot_ready {
        blocks.push("need fastboot (FASTBOOT_READY)".to_string());
    }
    if !confirm_flash {
        blocks.push("missing first confirmation (FLASH)".to_string());
    }
    if !confirm_yes {
        blocks.push("missing second confirmation (YES)".to_string());
    }
    let detail_flash = format!("fastboot flash {part} <twrp>");
    let steps = vec![
        plan_step("twrp-image-check", "recovery image check must PASS", false),
        plan_step(
            "backup",
            "mandatory backup-first: current slot image",
            false,
        ),
        plan_step(
            "warn-shared-slot",
            "TWRP and Magisk-recovery SHARE the recovery_ramdisk slot (mutual overwrite)",
            false,
        ),
        plan_step("confirm-flash", "type FLASH (1/2)", false),
        plan_step("confirm-yes", "type YES (2/2)", false),
        plan_step("require-fastboot", "abort unless fastboot mode", false),
        plan_step("flash", detail_flash.as_str(), true),
        plan_step(
            "record-slot",
            "slot holds twrp after OK; boot with Vol-Up",
            false,
        ),
    ];
    let ready = blocks.is_empty();
    DestructivePlan {
        op: "twrp-flash".to_string(),
        partition: part,
        image: img,
        steps,
        ready,
        blocks,
        confirm_first: confirm_flash,
        confirm_second: confirm_yes,
        first_word: "FLASH".to_string(),
        second_word: "YES".to_string(),
        warnings: vec![
            "TWRP and Magisk-recovery SHARE the recovery_ramdisk slot.".to_string(),
            "Boot TWRP: hold Vol-Up. NEVER wipe userdata in TWRP.".to_string(),
        ],
    }
}

/// Destructive TWRP-flash exec (always refused; Phase 8 owns fastboot).
pub fn run_twrp_flash(plan: &DestructivePlan) -> (Vec<ProgressEvent>, StepOutcome) {
    let _ = plan;
    (
        refuse_events("twrp-flash"),
        refused_exec("fastboot flash twrp"),
    )
}

/// Restore plan (`Invoke-TTRestoreFlow` / `do_restore`).
///
/// `backup_dir` holds `original.img`; `backup_has_original` is the
/// caller-measured file check.
pub fn plan_restore(
    backup_dir: &str,
    partition: &str,
    backup_has_original: bool,
    fastboot_ready: bool,
    confirm_restore: bool,
    confirm_yes: bool,
) -> DestructivePlan {
    let dir = backup_dir.trim().to_string();
    let part = partition.trim().to_string();
    let mut blocks: Vec<String> = Vec::new();
    if dir.is_empty() {
        blocks.push("no backup dir".to_string());
    }
    if !backup_has_original {
        blocks.push("no backup with original.img found".to_string());
    }
    if part.is_empty() {
        blocks.push("no target partition".to_string());
    }
    if !fastboot_ready {
        blocks.push("need fastboot (reboot to fastboot first)".to_string());
    }
    if !confirm_restore {
        blocks.push("missing first confirmation (RESTORE)".to_string());
    }
    if !confirm_yes {
        blocks.push("missing second confirmation (YES)".to_string());
    }
    let img = if dir.is_empty() {
        String::new()
    } else {
        format!("{dir}/original.img")
    };
    let detail_flash = format!("fastboot flash {part} original.img");
    let steps = vec![
        plan_step("find-backup", "newest backup dir with original.img", false),
        plan_step("verify-hash", "restore hash SHA-256 recorded", false),
        plan_step("require-fastboot", "abort unless fastboot mode", false),
        plan_step(
            "warn-restore",
            "restore overwrites slot with stock image",
            false,
        ),
        plan_step("confirm-restore", "type RESTORE (1/2)", false),
        plan_step("confirm-yes", "type YES (2/2)", false),
        plan_step("flash", detail_flash.as_str(), true),
        plan_step("record-slot", "slot holds stock after restore", false),
    ];
    let ready = blocks.is_empty();
    DestructivePlan {
        op: "restore".to_string(),
        partition: part,
        image: img,
        steps,
        ready,
        blocks,
        confirm_first: confirm_restore,
        confirm_second: confirm_yes,
        first_word: "RESTORE".to_string(),
        second_word: "YES".to_string(),
        warnings: vec!["Never flash without a live fastboot device.".to_string()],
    }
}

/// Destructive restore exec (always refused; Phase 8 owns fastboot).
pub fn run_restore(plan: &DestructivePlan) -> (Vec<ProgressEvent>, StepOutcome) {
    let _ = plan;
    (
        refuse_events("restore"),
        refused_exec("fastboot flash restore"),
    )
}

/// Guided-wipe plan (`Invoke-GuidedWipe` / `guided_wipe`).
///
/// Erases userdata only; never automatic; eRecovery fallback on refusal.
pub fn plan_guided_wipe(
    fastboot_ready: bool,
    confirm_wipe: bool,
    confirm_yes: bool,
) -> DestructivePlan {
    let mut blocks: Vec<String> = Vec::new();
    if !fastboot_ready {
        blocks.push("wipe needs fastboot mode".to_string());
    }
    if !confirm_wipe {
        blocks.push("missing first confirmation (WIPE)".to_string());
    }
    if !confirm_yes {
        blocks.push("missing second confirmation (YES)".to_string());
    }
    let steps = vec![
        plan_step("require-fastboot", "abort unless fastboot mode", false),
        plan_step(
            "warn-erase",
            "WARNING: erases ALL user data (apps, photos, settings). System stays.",
            false,
        ),
        plan_step("confirm-wipe", "type WIPE (1/2)", false),
        plan_step("confirm-yes", "type YES (2/2)", false),
        plan_step("erase-userdata", "fastboot erase userdata", true),
        plan_step(
            "note-fallback",
            "fallback if refused: stock eRecovery (Vol-Up 3s) -> wipe data/factory reset on phone",
            false,
        ),
    ];
    let ready = blocks.is_empty();
    DestructivePlan {
        op: "wipe".to_string(),
        partition: "userdata".to_string(),
        image: String::new(),
        steps,
        ready,
        blocks,
        confirm_first: confirm_wipe,
        confirm_second: confirm_yes,
        first_word: "WIPE".to_string(),
        second_word: "YES".to_string(),
        warnings: vec![
            "This erases ALL user data. System stays.".to_string(),
            "Fallback: stock eRecovery wipe on the phone.".to_string(),
        ],
    }
}

/// Destructive wipe exec (always refused; Phase 8 owns fastboot).
pub fn run_guided_wipe(plan: &DestructivePlan) -> (Vec<ProgressEvent>, StepOutcome) {
    let _ = plan;
    (
        refuse_events("wipe"),
        refused_exec("fastboot erase userdata"),
    )
}

// ------------------------------------------------------------ persist fixes
//
// `Install-PersistFixes` / `install_persist_fixes`: Magisk service.d boot
// scripts (aptouch + smartpa) so fixes survive reboots.

/// aptouch service.d script content (stops edge-touch service each boot).
pub const APTOUCH_SERVICE_SCRIPT: &str = "#!/system/bin/sh\n# trebleManager: stop aptouch (touchscreen edges) - runs every boot via Magisk service.d\nstop aptouch\n";

/// smartpa service.d script content (speaker fix each boot).
pub const SMARTPA_SERVICE_SCRIPT: &str = "#!/system/bin/sh\n# trebleManager: smartpa speaker fix - runs every boot via Magisk service.d\nchown root:audio /dev/nxp_smartpa_dev\nchmod 0660 /dev/nxp_smartpa_dev\n";

/// Target dir for persist scripts on device.
pub const PERSIST_TARGET_DIR: &str = "/data/adb/service.d";

/// Removability note (non-destructive, removable).
pub const PERSIST_REMOVABLE_NOTE: &str =
    "Removable: delete the two files from /data/adb/service.d; non-destructive, no system touch.";

/// One persist file (name + exact content).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersistFile {
    /// File name (e.g. `000-treblemanager-aptouch.sh`).
    pub name: String,
    /// Exact script content.
    pub content: String,
}

/// Persist plan (target dir + files + removability note).
#[derive(Debug, Clone)]
pub struct PersistPlan {
    /// Device target dir.
    pub target_dir: String,
    /// Files to install.
    pub files: Vec<PersistFile>,
    /// Removability note.
    pub removable_note: String,
    /// True: needs live root (uid=0).
    pub needs_live_root: bool,
}

/// Persist plan (`Install-PersistFixes` file set, no I/O).
pub fn plan_persist_fixes() -> PersistPlan {
    PersistPlan {
        target_dir: PERSIST_TARGET_DIR.to_string(),
        files: vec![
            PersistFile {
                name: "000-treblemanager-aptouch.sh".to_string(),
                content: APTOUCH_SERVICE_SCRIPT.to_string(),
            },
            PersistFile {
                name: "000-treblemanager-smartpa.sh".to_string(),
                content: SMARTPA_SERVICE_SCRIPT.to_string(),
            },
        ],
        removable_note: PERSIST_REMOVABLE_NOTE.to_string(),
        needs_live_root: true,
    }
}

/// Live persist install (always refused; needs device adb, Phase 8).
///
/// `has_live_root` mirrors the `su -c id` uid=0 gate: without it the
/// refusal names the missing root, with it the refusal names Phase 8.
pub fn run_persist_install(has_live_root: bool) -> (Vec<ProgressEvent>, StepOutcome) {
    let ev = vec![
        ProgressEvent::Started {
            operation: "persist-install".to_string(),
        },
        ProgressEvent::Message {
            message: "refused: live adb install is Phase 8 (plan only)".to_string(),
        },
    ];
    if !has_live_root {
        (
            ev,
            StepOutcome::RefusedExperimental(
                "no live root (need uid=0); boot rooted first, grant Magisk, retry. Exec is Phase 8."
                    .to_string(),
            ),
        )
    } else {
        (
            ev,
            StepOutcome::RefusedExperimental(
                "live adb install needs Phase 8 executor; plan only (service.d set ready)."
                    .to_string(),
            ),
        )
    }
}

// ------------------------------------------------------------ analysis
//
// Pure sequences over injected transcripts, reusing `gsi-device` parsers.
// No spawn, no device access.

fn text_lines(text: &str) -> Vec<&str> {
    let mut out: Vec<&str> = Vec::new();
    for line in text.lines() {
        out.push(line);
    }
    out
}

fn prop_value(map: &std::collections::BTreeMap<String, String>, key: &str) -> String {
    match map.get(key) {
        Some(v) => v.clone(),
        None => String::new(),
    }
}

fn profile_for_model(model: &str) -> (String, bool) {
    let m = model;
    if m.contains("VTR-L29") {
        return ("VTR-L29".to_string(), false);
    }
    if m.contains("VTR-L09") {
        return ("VTR-L09".to_string(), false);
    }
    if m.contains("VKY-L29") {
        return ("VKY-L29".to_string(), false);
    }
    if m.contains("VTR-AL00") {
        return ("VTR-AL00".to_string(), false);
    }
    if m.contains("VTR-TL00") {
        return ("VTR-TL00".to_string(), false);
    }
    if m.contains("VKY-L09") {
        return ("VKY-L09".to_string(), false);
    }
    if m.contains("VKY-AL00") {
        return ("VKY-AL00".to_string(), false);
    }
    if m.contains("VKY-TL00") {
        return ("VKY-TL00".to_string(), false);
    }
    ("VTR-L29".to_string(), true)
}

/// Android findings over injected transcripts.
#[derive(Debug, Clone)]
pub struct AndroidFindings {
    /// Count of parsed adb devices.
    pub device_count: usize,
    /// First serial (empty when none).
    pub first_serial: String,
    /// `ro.product.model`.
    pub model: String,
    /// `ro.product.name`.
    pub pname: String,
    /// `ro.build.display.id`.
    pub display: String,
    /// `ro.build.version.release`.
    pub release: String,
    /// EMUI (`ro.build.version.emui` else `ro.emui.version`).
    pub emui: String,
    /// OS kind (mirrors `os_classify`).
    pub os_kind: String,
    /// OS detail (`model / pname / Android release (display)`).
    pub os_detail: String,
    /// Derived profile (GSI defaults to VTR-L29 with warning).
    pub profile_id: String,
    /// True when by-name lists `recovery_ramdisk`.
    pub has_recovery_ramdisk: bool,
    /// Count of by-name entries.
    pub byname_count: usize,
    /// Notes (GSI default, missing device, missing slot).
    pub warnings: Vec<String>,
}

/// Android analysis (`Invoke-TTAndroidAnalysis` / `android_analysis`).
///
/// Pure over `adb devices`, `getprop` and `by-name` transcripts.
pub fn android_analysis(
    adb_devices_text: &str,
    getprop_text: &str,
    byname_text: &str,
) -> AndroidFindings {
    let adb_lines = text_lines(adb_devices_text);
    let adb_devs = gsi_device::parse::adb_devices(&adb_lines);
    let prop_lines = text_lines(getprop_text);
    let props = gsi_device::parse::getprop(&prop_lines);
    let byname = gsi_device::parse::byname(byname_text);
    let model = prop_value(&props, "ro.product.model");
    let pname = prop_value(&props, "ro.product.name");
    let display = prop_value(&props, "ro.build.display.id");
    let release = prop_value(&props, "ro.build.version.release");
    let mut emui = prop_value(&props, "ro.build.version.emui");
    if emui.trim().is_empty() {
        emui = prop_value(&props, "ro.emui.version");
    }
    let os = gsi_gates::os_classify(&model, &pname, &display, &release, &emui);
    let (profile_id, defaulted) = profile_for_model(model.as_str());
    let mut has_recovery_ramdisk = false;
    for e in byname.iter() {
        if e.name == "recovery_ramdisk" {
            has_recovery_ramdisk = true;
            break;
        }
    }
    let mut warnings: Vec<String> = Vec::new();
    if adb_devs.is_empty() {
        warnings.push("no adb device in transcript".to_string());
    }
    if defaulted {
        warnings.push(
            "Model string is GSI (no VTR/VKY); profile default VTR-L29, verification before flash mandatory."
                .to_string(),
        );
    }
    if !has_recovery_ramdisk {
        warnings.push(
            "recovery_ramdisk NOT in by-name -> fastboot analysis + firmware path needed."
                .to_string(),
        );
    }
    let mut first_serial = String::new();
    if let Some(d) = adb_devs.first() {
        first_serial = d.serial.clone();
    }
    AndroidFindings {
        device_count: adb_devs.len(),
        first_serial,
        model,
        pname,
        display,
        release,
        emui,
        os_kind: os.kind,
        os_detail: os.detail,
        profile_id,
        has_recovery_ramdisk,
        byname_count: byname.len(),
        warnings,
    }
}

/// Fastboot findings over injected transcripts.
#[derive(Debug, Clone)]
pub struct FastbootFindings {
    /// Count of parsed fastboot devices.
    pub device_count: usize,
    /// First serial (empty when none).
    pub first_serial: String,
    /// Parsed getvar values.
    pub vars: std::collections::BTreeMap<String, String>,
    /// True on Huawei `Command not allowed` (quirk, not lock proof).
    pub command_denied: bool,
    /// Last FAILED remote reason.
    pub last_failed_reason: String,
    /// Notes (no device, denial quirk).
    pub warnings: Vec<String>,
}

/// Fastboot analysis (`Invoke-TTFastbootAnalysis` / `fastboot_analysis`).
///
/// Pure over `fastboot devices` and `getvar` transcripts.
pub fn fastboot_analysis(fastboot_devices_text: &str, getvar_text: &str) -> FastbootFindings {
    let fb_lines = text_lines(fastboot_devices_text);
    let fb_devs = gsi_device::parse::fastboot_devices(&fb_lines);
    let getvar_lines = text_lines(getvar_text);
    let gv = gsi_device::parse::getvar(&getvar_lines);
    let mut warnings: Vec<String> = Vec::new();
    if fb_devs.is_empty() {
        warnings.push("no fastboot device in transcript".to_string());
    }
    if gv.command_denied {
        warnings
            .push("Huawei refuses getvar (Command not allowed). NOT proof of lock.".to_string());
    }
    let mut first_serial = String::new();
    if let Some(d) = fb_devs.first() {
        first_serial = d.serial.clone();
    }
    FastbootFindings {
        device_count: fb_devs.len(),
        first_serial,
        vars: gv.vars,
        command_denied: gv.command_denied,
        last_failed_reason: gv.last_failed_reason,
        warnings,
    }
}

// ------------------------------------------------------------ device states
//
// Pure state machines over injected transcripts (`Get-DeviceStates` /
// `device_states`, `Update-TTMode` / `detect_mode`).

/// Device mode (Android adb takes precedence over fastboot).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeviceMode {
    /// adb device present.
    Android,
    /// fastboot device present (no adb device).
    Fastboot,
    /// Neither present.
    NoDevice,
}

impl DeviceMode {
    /// Lowercase id (`android`, `fastboot`, `none`).
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Android => "android",
            Self::Fastboot => "fastboot",
            Self::NoDevice => "none",
        }
    }
}

/// Mode report with winning serials.
#[derive(Debug, Clone)]
pub struct ModeReport {
    /// Winning mode.
    pub mode: DeviceMode,
    /// adb serial (empty unless Android).
    pub adb_serial: String,
    /// fastboot serial (empty unless Fastboot).
    pub fastboot_serial: String,
}

/// Device states (`Get-DeviceStates` / `device_states`).
///
/// Pure over injected `adb devices` / `fastboot devices` transcripts plus
/// tool-availability flags. Mirrors the script state names exactly.
pub fn device_states(
    adb_devices_text: &str,
    adb_available: bool,
    fastboot_devices_text: &str,
    fastboot_available: bool,
) -> gsi_gates::DeviceStates {
    let mut adb = "ADB_NOT_FOUND".to_string();
    if adb_available {
        let lines = text_lines(adb_devices_text);
        let devs = gsi_device::parse::adb_devices(&lines);
        let mut ready: usize = 0;
        let mut unauth: usize = 0;
        let mut off: usize = 0;
        for d in devs.iter() {
            if d.state == "device" {
                ready += 1;
            } else if d.state == "unauthorized" {
                unauth += 1;
            } else if d.state == "offline" {
                off += 1;
            }
        }
        if ready > 1 {
            adb = "ADB_MULTIPLE_DEVICES".to_string();
        } else if ready == 1 {
            adb = "ADB_READY".to_string();
        } else if unauth > 0 {
            adb = "ADB_UNAUTHORIZED".to_string();
        } else if off > 0 {
            adb = "ADB_OFFLINE".to_string();
        } else {
            adb = "NO_DEVICE".to_string();
        }
    }
    let mut fastboot = "FASTBOOT_NOT_FOUND".to_string();
    if fastboot_available {
        let lines = text_lines(fastboot_devices_text);
        let devs = gsi_device::parse::fastboot_devices(&lines);
        if devs.len() > 1 {
            fastboot = "FASTBOOT_MULTIPLE_DEVICES".to_string();
        } else if devs.len() == 1 {
            fastboot = "FASTBOOT_READY".to_string();
        } else {
            fastboot = "FASTBOOT_NO_DEVICE".to_string();
        }
    }
    let mut overall = "UNKNOWN_DEVICE_STATE".to_string();
    let adb_gone = adb == "ADB_NOT_FOUND" || adb == "NO_DEVICE";
    let fb_gone = fastboot == "FASTBOOT_NOT_FOUND" || fastboot == "FASTBOOT_NO_DEVICE";
    if adb_gone && fb_gone {
        overall = "NO_DEVICE".to_string();
    }
    if adb == "ADB_READY" || fastboot == "FASTBOOT_READY" {
        overall = "READY".to_string();
    }
    gsi_gates::DeviceStates {
        overall,
        adb,
        fastboot,
    }
}

/// Mode detection (`Update-TTMode` / `detect_mode`).
///
/// Pure: first adb `device` wins, else first fastboot device, else none.
pub fn detect_mode(
    adb_devices_text: &str,
    adb_available: bool,
    fastboot_devices_text: &str,
    fastboot_available: bool,
) -> ModeReport {
    if adb_available {
        let lines = text_lines(adb_devices_text);
        let devs = gsi_device::parse::adb_devices(&lines);
        for d in devs.iter() {
            if d.state == "device" {
                return ModeReport {
                    mode: DeviceMode::Android,
                    adb_serial: d.serial.clone(),
                    fastboot_serial: String::new(),
                };
            }
        }
    }
    if fastboot_available {
        let lines = text_lines(fastboot_devices_text);
        let devs = gsi_device::parse::fastboot_devices(&lines);
        for d in devs.iter() {
            if d.state == "fastboot" {
                return ModeReport {
                    mode: DeviceMode::Fastboot,
                    adb_serial: String::new(),
                    fastboot_serial: d.serial.clone(),
                };
            }
        }
    }
    ModeReport {
        mode: DeviceMode::NoDevice,
        adb_serial: String::new(),
        fastboot_serial: String::new(),
    }
}

// ------------------------------------------------------------ validate/verify
//
// `Invoke-TTValidate` / `validate_device` / `validate_checked` and
// `verify_root`: check-list plan types plus live-adb refusals (Phase 8).

/// One validation check result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationCheck {
    /// Check name (e.g. `ROOT`, `SELinux`).
    pub name: String,
    /// True = pass.
    pub pass: bool,
    /// Detail text.
    pub detail: String,
}

/// Validation check-list plan (read-only post-flash checks).
#[derive(Debug, Clone)]
pub struct ValidationPlan {
    /// Ordered check names.
    pub checks: Vec<String>,
    /// Note (read-only, needs live adb).
    pub note: String,
}

/// Check names of `validate_device` (ADB, OS, SELinux, ROOT, mounts,
/// WIFI, BLUETOOTH, BATTERY, SENSORS).
pub fn validation_check_names() -> Vec<String> {
    vec![
        "ADB".to_string(),
        "OS".to_string(),
        "SELinux".to_string(),
        "ROOT".to_string(),
        "MOUNTS-system".to_string(),
        "MOUNTS-vendor".to_string(),
        "WIFI".to_string(),
        "BLUETOOTH".to_string(),
        "BATTERY".to_string(),
        "SENSORS".to_string(),
    ]
}

/// Validate plan (`Invoke-TTValidate` check list, no I/O).
pub fn plan_validate_device() -> ValidationPlan {
    ValidationPlan {
        checks: validation_check_names(),
        note: "read-only post-flash checks; live adb is Phase 8.".to_string(),
    }
}

/// Aggregate check results (`validate_checked`: false on any FAIL).
pub fn validate_checked(checks: &[ValidationCheck]) -> bool {
    for c in checks.iter() {
        if !c.pass {
            return false;
        }
    }
    true
}

/// Live validate exec (always refused; needs device adb, Phase 8).
pub fn run_validate_device() -> (Vec<ProgressEvent>, StepOutcome) {
    (
        vec![
            ProgressEvent::Started {
                operation: "validate".to_string(),
            },
            ProgressEvent::Message {
                message: "refused: live adb validation is Phase 8 (plan only)".to_string(),
            },
        ],
        StepOutcome::RefusedExperimental(
            "live adb validation needs Phase 8 executor; plan only.".to_string(),
        ),
    )
}

/// Root-verify verdict over injected `which su` / `su -c id` transcripts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VerifyVerdict {
    /// `su -c id` contains `uid=0`.
    Rooted,
    /// su present but no uid=0.
    Inconclusive,
    /// No root verifiable.
    NotRooted,
}

impl VerifyVerdict {
    /// Uppercase id (`ROOTED`, `INCONCLUSIVE`, `NOT_ROOTED`).
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Rooted => "ROOTED",
            Self::Inconclusive => "INCONCLUSIVE",
            Self::NotRooted => "NOT_ROOTED",
        }
    }
}

/// Root-verify plan (boot cheat + adb sequence, no I/O).
#[derive(Debug, Clone)]
pub struct VerifyRootPlan {
    /// Ordered steps.
    pub steps: Vec<String>,
    /// Note (only uid=0 counts; boot alone is not root).
    pub note: String,
}

/// Verify-root plan (`verify_root` sequence, no I/O).
pub fn plan_verify_root() -> VerifyRootPlan {
    VerifyRootPlan {
        steps: vec![
            "reboot-to-android".to_string(),
            "vol-up-power-boot-cheat".to_string(),
            "wait-for-adb".to_string(),
            "which-su".to_string(),
            "su-c-id-uid0".to_string(),
            "magisk-v".to_string(),
        ],
        note: "only uid=0 counts; boot alone is not root.".to_string(),
    }
}

/// Pure classifier for `verify_root` transcripts.
pub fn classify_verify_root(which_su: &str, su_id: &str) -> VerifyVerdict {
    if su_id.contains("uid=0") {
        return VerifyVerdict::Rooted;
    }
    let w = which_su.trim();
    if !w.is_empty() && !w.contains("not found") && !w.contains("No such") {
        return VerifyVerdict::Inconclusive;
    }
    VerifyVerdict::NotRooted
}

/// Live verify-root exec (always refused; needs device adb, Phase 8).
pub fn run_verify_root() -> (Vec<ProgressEvent>, StepOutcome) {
    (
        vec![
            ProgressEvent::Started {
                operation: "verify-root".to_string(),
            },
            ProgressEvent::Message {
                message: "refused: live adb root-verify is Phase 8 (plan only)".to_string(),
            },
        ],
        StepOutcome::RefusedExperimental(
            "live adb root-verify needs Phase 8 executor; plan only.".to_string(),
        ),
    )
}

// ------------------------------------------------------------ downloaded firmware
//
// `Test-DownloadedFirmware`: size heuristic plus UPDATE.APP probe.

/// Below this size a firmware package is implausible (delta/short package).
pub const FIRMWARE_MIN_SIZE: u64 = 100 * 1024 * 1024;

/// Downloaded-firmware check (mirrors `Test-DownloadedFirmware`).
#[derive(Debug, Clone)]
pub struct DownloadedFirmwareCheck {
    /// Lowercase hex SHA-256 of the file.
    pub sha256: String,
    /// File size in bytes.
    pub size: u64,
    /// False when implausibly small.
    pub ok: bool,
    /// Size, hash and container notes.
    pub notes: Vec<String>,
}

/// Check a downloaded firmware file: size heuristic plus UPDATE.APP probe.
///
/// Reads the file hash plus at most 32 header bytes for
/// `gsi_archive::probe_update_app`; surfaces its verdict note for
/// UPDATE.APP containers. No network, no extraction.
pub fn verify_downloaded_firmware(path: &Path) -> Result<DownloadedFirmwareCheck, String> {
    if !path.is_file() {
        return Err(format!("file missing: {}", path.display()));
    }
    let size = std::fs::metadata(path)
        .map_err(|e| format!("meta: {e}"))?
        .len();
    let sha256 = sha256_file(path)?;
    let mut header = [0u8; 32];
    let header_len = match std::fs::File::open(path) {
        Ok(mut f) => {
            use std::io::Read as _;
            match f.read(&mut header) {
                Ok(n) => n,
                Err(e) => return Err(format!("read header: {e}")),
            }
        }
        Err(e) => return Err(format!("open: {e}")),
    };
    let probe = gsi_archive::probe_update_app(&header[..header_len]);
    let mb = size as f64 / (1024.0 * 1024.0);
    let mut notes = vec![format!("Size: {mb:.1} MB"), format!("SHA-256: {sha256}")];
    let mut ok = true;
    if size < FIRMWARE_MIN_SIZE {
        ok = false;
        notes.push(
            "WARN: < 100 MB - implausibly small for full firmware (maybe delta/short package)."
                .to_string(),
        );
    }
    if probe.is_update_app {
        notes.push(format!(
            "UPDATE.APP container detected (size {size}, header {}): {}",
            probe.header_hex, probe.note
        ));
    } else if probe.is_zip {
        notes.push("ZIP container: check entries for UPDATE.APP manually.".to_string());
    } else if probe.is_gzip {
        notes.push("gzip container: not a firmware ZIP.".to_string());
    } else {
        notes.push("Not a ZIP - unpack / UPDATE.APP search manually.".to_string());
    }
    Ok(DownloadedFirmwareCheck {
        sha256,
        size,
        ok,
        notes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn tmp(name: &str, bytes: &[u8]) -> PathBuf {
        let mut p = std::env::temp_dir();
        p.push(format!("gsi-workflow-test-{name}"));
        let mut f = std::fs::File::create(&p).unwrap();
        f.write_all(bytes).unwrap();
        p
    }

    #[test]
    fn p10_workflow_lists_steps_in_order() {
        let w = p10_lineage20(Path::new("in.img"), Path::new("out.img"));
        assert_eq!(w.id, "p10-lineage20");
        assert!(matches!(w.steps[0], WorkflowStep::Analyze { .. }));
        assert!(matches!(w.steps[1], WorkflowStep::Patch));
        assert!(w.steps.len() >= 5);
    }

    #[test]
    fn analyze_is_real_and_reports_container() {
        let p = tmp("a.gz", &[0x1F, 0x8B, 0x08, 0x00]);
        let (ev, out) = run_analyze(&p);
        assert_eq!(out, StepOutcome::Done);
        assert!(ev.iter().any(|e| matches!(
            e,
            ProgressEvent::Message { message } if message.contains("gzip")
        )));
        std::fs::remove_file(p).ok();
    }

    #[test]
    fn patch_refuses_honestly() {
        let (_, out) = run_patch("anything");
        assert!(matches!(out, StepOutcome::RefusedExperimental(_)));
    }

    #[test]
    fn hash_verify_roundtrip() {
        let p = tmp("h.bin", b"hello treble");
        let h = sha256_file(&p).unwrap();
        let (_, out) = run_verify_hash(&p, &h);
        assert_eq!(out, StepOutcome::Done);
        let (_, out2) = run_verify_hash(&p, "deadbeef");
        assert!(matches!(out2, StepOutcome::Failed(_)));
        std::fs::remove_file(p).ok();
    }

    #[test]
    fn sha512_known_answer() {
        // sha512("abc") well-known prefix; full length 128 hex chars.
        let p = tmp("h512.bin", b"abc");
        let h = sha512_file(&p).unwrap();
        assert_eq!(h.len(), 128);
        assert!(h.starts_with("ddaf35a193617abacc417349ae20413112"));
        assert!(sha512_file(Path::new("/nonexistent-tt-sha512")).is_err());
        std::fs::remove_file(p).ok();
    }

    #[test]
    fn destructive_steps_are_distinct_variants() {
        // The type system (not a boolean) separates destructive steps.
        let w = p10_lineage20(Path::new("i"), Path::new("o"));
        assert!(w.steps.contains(&WorkflowStep::Flash));
        assert!(w.steps.contains(&WorkflowStep::Backup));
    }

    // ------------------------- android/fastboot analysis fixtures

    const ADB_ONE: &str = "List of devices attached\n6PQ0217B08003446\tdevice\n";
    const GETPROP_STOCK: &str = "[ro.product.model]: [VTR-L29]\n[ro.product.name]: [VTR-L29]\n[ro.build.display.id]: [VTR-L29 9.1.0.297(C432E5R1P9)]\n[ro.build.version.release]: [9]\n[ro.build.version.emui]: [EmotionUI_9.1]\n";
    const GETPROP_GSI: &str = "[ro.product.model]: [trebledroid]\n[ro.product.name]: [trebledroid]\n[ro.build.display.id]: [trebledroid 13 arm64_bvN]\n[ro.build.version.release]: [13]\n";
    const BYNAME_OK: &str =
        "recovery_ramdisk -> /dev/block/mmcblk0p30\nboot -> /dev/block/mmcblk0p28\n";
    const FB_ONE: &str = "6PQ0217B08003446\tfastboot\n";
    const GETVAR_DENIED: &str =
        "getvar:unlocked FAILED (remote: Command not allowed)\nfinished. total time: 0.016s\n";
    const GETVAR_OK: &str = "product: VTR-L29\nsecure: yes\nfinished.\n";

    #[test]
    fn android_analysis_stock_profile() {
        let f = android_analysis(ADB_ONE, GETPROP_STOCK, BYNAME_OK);
        assert_eq!(f.device_count, 1);
        assert_eq!(f.first_serial, "6PQ0217B08003446");
        assert_eq!(f.model, "VTR-L29");
        assert_eq!(f.profile_id, "VTR-L29");
        assert!(f.has_recovery_ramdisk);
        assert_eq!(f.byname_count, 2);
        assert_eq!(f.emui, "EmotionUI_9.1");
        assert!(f.os_kind.contains("Stock"));
        assert!(f.os_detail.contains("VTR-L29"));
    }

    #[test]
    fn android_analysis_gsi_defaults_profile_with_warning() {
        let f = android_analysis(ADB_ONE, GETPROP_GSI, "boot -> /dev/block/x\n");
        assert_eq!(f.profile_id, "VTR-L29");
        assert!(!f.has_recovery_ramdisk);
        assert!(f.warnings.iter().any(|w| w.contains("GSI")));
        assert!(f.warnings.iter().any(|w| w.contains("recovery_ramdisk")));
        assert!(f.os_kind.contains("GSI"));
    }

    #[test]
    fn android_analysis_no_device_warns() {
        let f = android_analysis("List of devices attached\n", GETPROP_STOCK, BYNAME_OK);
        assert_eq!(f.device_count, 0);
        assert!(f.warnings.iter().any(|w| w.contains("no adb device")));
    }

    #[test]
    fn fastboot_analysis_denied_is_quirk() {
        let f = fastboot_analysis(FB_ONE, GETVAR_DENIED);
        assert_eq!(f.device_count, 1);
        assert!(f.command_denied);
        assert_eq!(f.last_failed_reason, "Command not allowed");
        assert!(f.warnings.iter().any(|w| w.contains("NOT proof of lock")));
        let ok = fastboot_analysis(FB_ONE, GETVAR_OK);
        assert!(!ok.command_denied);
        assert_eq!(ok.vars.get("product").map(String::as_str), Some("VTR-L29"));
        let none = fastboot_analysis("", "");
        assert_eq!(none.device_count, 0);
        assert!(none
            .warnings
            .iter()
            .any(|w| w.contains("no fastboot device")));
    }

    // ------------------------- states / mode

    #[test]
    fn device_states_matrix() {
        let r = device_states(ADB_ONE, true, "", true);
        assert_eq!(r.adb, "ADB_READY");
        assert_eq!(r.overall, "READY");
        let n = device_states("List of devices attached\n", true, "", true);
        assert_eq!(n.overall, "NO_DEVICE");
        let u = device_states(
            "List of devices attached\nX\tunauthorized\n",
            true,
            "",
            true,
        );
        assert_eq!(u.adb, "ADB_UNAUTHORIZED");
        assert_eq!(u.overall, "UNKNOWN_DEVICE_STATE");
        let m = device_states(
            "List of devices attached\nA\tdevice\nB\tdevice\n",
            true,
            "",
            false,
        );
        assert_eq!(m.adb, "ADB_MULTIPLE_DEVICES");
        let f = device_states("", false, FB_ONE, true);
        assert_eq!(f.fastboot, "FASTBOOT_READY");
        assert_eq!(f.overall, "READY");
        let gone = device_states("", false, "", false);
        assert_eq!(gone.adb, "ADB_NOT_FOUND");
        assert_eq!(gone.overall, "NO_DEVICE");
    }

    #[test]
    fn detect_mode_matrix() {
        let a = detect_mode(ADB_ONE, true, FB_ONE, true);
        assert_eq!(a.mode, DeviceMode::Android);
        assert_eq!(a.adb_serial, "6PQ0217B08003446");
        assert_eq!(a.mode.as_str(), "android");
        let f = detect_mode("List of devices attached\n", true, FB_ONE, true);
        assert_eq!(f.mode, DeviceMode::Fastboot);
        assert_eq!(f.fastboot_serial, "6PQ0217B08003446");
        let n = detect_mode("", false, "", false);
        assert_eq!(n.mode, DeviceMode::NoDevice);
        assert_eq!(n.mode.as_str(), "none");
    }

    // ------------------------- flash / restore / wipe plans

    fn only_destructive(plan: &DestructivePlan) -> Vec<&PlanStep> {
        plan.steps.iter().filter(|s| s.destructive).collect()
    }

    #[test]
    fn safe_flash_plan_matrix() {
        let ok = plan_safe_flash("m.img", "recovery_ramdisk", true, true, true, true, true);
        assert!(ok.ready);
        assert!(ok.blocks.is_empty());
        assert_eq!(ok.first_word, "FLASH");
        assert_eq!(ok.steps[0].id, "safety-check");
        assert_eq!(ok.steps[1].id, "backup");
        assert!(ok.steps.iter().any(|s| s.id == "confirm-flash"));
        assert!(ok.steps.iter().any(|s| s.id == "confirm-yes"));
        assert_eq!(only_destructive(&ok).len(), 1);
        // Missing confirmations -> refused plan.
        let no = plan_safe_flash("m.img", "recovery_ramdisk", true, true, true, false, false);
        assert!(!no.ready);
        assert!(no.blocks.iter().any(|b| b.contains("FLASH")));
        assert!(no.blocks.iter().any(|b| b.contains("YES")));
        assert!(!no.confirm_first && !no.confirm_second);
        // Backup-first mandatory.
        let nb = plan_safe_flash("m.img", "recovery_ramdisk", false, true, true, true, true);
        assert!(!nb.ready);
        assert!(nb.blocks.iter().any(|b| b.contains("backup")));
        // Wrong partition refused.
        let wp = plan_safe_flash("m.img", "system", true, true, true, true, true);
        assert!(!wp.ready);
        // Refusal exec.
        let (_, out) = run_safe_flash(&ok);
        match out {
            StepOutcome::RefusedExperimental(m) => assert!(m.contains("Phase 8")),
            _ => panic!("safe flash must refuse"),
        }
    }

    #[test]
    fn system_flash_plan_matrix() {
        let ok = plan_system_flash("gsi.img", true, true, true, false, true, true);
        assert!(ok.ready);
        assert_eq!(ok.partition, "system");
        assert_eq!(only_destructive(&ok).len(), 1);
        let blocked = plan_system_flash("gsi.img", true, true, true, true, true, true);
        assert!(!blocked.ready);
        assert!(blocked.blocks.iter().any(|b| b.contains("BLOCKED")));
        let bad = plan_system_flash("gsi.img", true, true, false, false, true, true);
        assert!(!bad.ready);
        let noconf = plan_system_flash("gsi.img", true, true, true, false, false, true);
        assert!(!noconf.ready);
        let (_, out) = run_system_flash(&ok);
        assert!(matches!(out, StepOutcome::RefusedExperimental(_)));
    }

    #[test]
    fn twrp_flash_plan_matrix() {
        let ok = plan_twrp_flash("twrp.img", "recovery_ramdisk", true, true, true, true, true);
        assert!(ok.ready);
        assert!(ok.warnings.iter().any(|w| w.contains("SHARE")));
        assert!(ok.warnings.iter().any(|w| w.contains("NEVER wipe")));
        assert_eq!(only_destructive(&ok).len(), 1);
        let no = plan_twrp_flash(
            "twrp.img",
            "recovery_ramdisk",
            true,
            true,
            true,
            true,
            false,
        );
        assert!(!no.ready);
        let nb = plan_twrp_flash(
            "twrp.img",
            "recovery_ramdisk",
            false,
            true,
            true,
            true,
            true,
        );
        assert!(!nb.ready);
        let (_, out) = run_twrp_flash(&ok);
        assert!(matches!(out, StepOutcome::RefusedExperimental(_)));
    }

    #[test]
    fn restore_plan_matrix() {
        let ok = plan_restore("/b/2026", "recovery_ramdisk", true, true, true, true);
        assert!(ok.ready);
        assert_eq!(ok.first_word, "RESTORE");
        assert!(ok.image.contains("original.img"));
        assert_eq!(only_destructive(&ok).len(), 1);
        let noback = plan_restore("/b/2026", "recovery_ramdisk", false, true, true, true);
        assert!(!noback.ready);
        assert!(noback.blocks.iter().any(|b| b.contains("original.img")));
        let noconf = plan_restore("/b/2026", "recovery_ramdisk", true, true, false, true);
        assert!(!noconf.ready);
        let nofb = plan_restore("/b/2026", "recovery_ramdisk", true, false, true, true);
        assert!(!nofb.ready);
        let (_, out) = run_restore(&ok);
        assert!(matches!(out, StepOutcome::RefusedExperimental(_)));
    }

    #[test]
    fn wipe_plan_matrix_and_fallback() {
        let ok = plan_guided_wipe(true, true, true);
        assert!(ok.ready);
        assert_eq!(ok.partition, "userdata");
        assert_eq!(ok.first_word, "WIPE");
        assert!(ok.steps.iter().any(|s| s.id == "note-fallback"));
        assert_eq!(only_destructive(&ok).len(), 1);
        let no = plan_guided_wipe(true, false, false);
        assert!(!no.ready);
        assert!(no.blocks.iter().any(|b| b.contains("WIPE")));
        let nofb = plan_guided_wipe(false, true, true);
        assert!(!nofb.ready);
        let (_, out) = run_guided_wipe(&ok);
        assert!(matches!(out, StepOutcome::RefusedExperimental(_)));
    }

    // ------------------------- persist
    // Key-line asserts kept: no standalone service.d files exist in the repo
    // (scripts embed them inline in Treble-Toolkit.ps1/treble-toolkit.sh),
    // so no stable relative file path is available for byte-match.

    #[test]
    fn persist_contents_and_plan() {
        assert!(APTOUCH_SERVICE_SCRIPT.contains("#!/system/bin/sh"));
        assert!(APTOUCH_SERVICE_SCRIPT.contains("stop aptouch"));
        assert!(SMARTPA_SERVICE_SCRIPT.contains("chown root:audio /dev/nxp_smartpa_dev"));
        assert!(SMARTPA_SERVICE_SCRIPT.contains("chmod 0660 /dev/nxp_smartpa_dev"));
        let p = plan_persist_fixes();
        assert_eq!(p.target_dir, "/data/adb/service.d");
        assert_eq!(p.files.len(), 2);
        assert!(p.files.iter().any(|f| f.name.contains("aptouch")));
        assert!(p.files.iter().any(|f| f.name.contains("smartpa")));
        assert!(p.removable_note.contains("Remov"));
        assert!(p.needs_live_root);
        let (_, noroot) = run_persist_install(false);
        match noroot {
            StepOutcome::RefusedExperimental(m) => assert!(m.contains("uid=0")),
            _ => panic!("persist without root must refuse"),
        }
        let (_, withroot) = run_persist_install(true);
        match withroot {
            StepOutcome::RefusedExperimental(m) => assert!(m.contains("Phase 8")),
            _ => panic!("persist with root must still refuse (Phase 8)"),
        }
    }

    // ------------------------- validate / verify

    #[test]
    fn validate_plan_and_checked() {
        let plan = plan_validate_device();
        assert_eq!(plan.checks.len(), 10);
        assert!(plan.checks.contains(&"ROOT".to_string()));
        assert!(plan.checks.contains(&"SELinux".to_string()));
        assert_eq!(validation_check_names().len(), 10);
        let all_pass = vec![
            ValidationCheck {
                name: "ROOT".to_string(),
                pass: true,
                detail: "uid=0".to_string(),
            },
            ValidationCheck {
                name: "OS".to_string(),
                pass: true,
                detail: "9".to_string(),
            },
        ];
        assert!(validate_checked(&all_pass));
        let one_fail = vec![
            ValidationCheck {
                name: "ROOT".to_string(),
                pass: true,
                detail: "uid=0".to_string(),
            },
            ValidationCheck {
                name: "WIFI".to_string(),
                pass: false,
                detail: "empty".to_string(),
            },
        ];
        assert!(!validate_checked(&one_fail));
        assert!(validate_checked(&[]));
        let (_, out) = run_validate_device();
        assert!(matches!(out, StepOutcome::RefusedExperimental(_)));
    }

    #[test]
    fn verify_root_plan_and_classifier() {
        let plan = plan_verify_root();
        assert!(plan.steps.contains(&"su-c-id-uid0".to_string()));
        assert!(plan.note.contains("uid=0"));
        assert_eq!(
            classify_verify_root("/system/xbin/su", "uid=0(root) gid=0"),
            VerifyVerdict::Rooted
        );
        assert_eq!(
            classify_verify_root("/system/xbin/su", "no output"),
            VerifyVerdict::Inconclusive
        );
        assert_eq!(classify_verify_root("", ""), VerifyVerdict::NotRooted);
        assert_eq!(
            classify_verify_root("not found", "denied"),
            VerifyVerdict::NotRooted
        );
        assert_eq!(VerifyVerdict::Rooted.as_str(), "ROOTED");
        let (_, out) = run_verify_root();
        assert!(matches!(out, StepOutcome::RefusedExperimental(_)));
    }

    // ------------------------- downloaded firmware (hermetic temp dirs)

    fn fw_tmp_dir(tag: &str) -> PathBuf {
        let mut p = std::env::temp_dir();
        p.push(format!("gsi-workflow-fw-{tag}-{}", std::process::id()));
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn downloaded_firmware_probes_update_app() {
        let d = fw_tmp_dir("app");
        let f = d.join("UPDATE.APP");
        let mut bytes = vec![0x55, 0xAA, 0x5A, 0xA5];
        bytes.extend_from_slice(b"firmware-bytes");
        std::fs::write(&f, &bytes).unwrap();
        let chk = verify_downloaded_firmware(&f).unwrap();
        assert_eq!(chk.size, bytes.len() as u64);
        assert!(!chk.ok);
        assert_eq!(chk.sha256, sha256_file(&f).unwrap());
        assert!(chk
            .notes
            .iter()
            .any(|n| n.contains("UPDATE.APP container detected")));
        assert!(chk
            .notes
            .iter()
            .any(|n| n.contains(gsi_archive::probe_update_app(&bytes).note)));
        assert!(verify_downloaded_firmware(&d.join("missing.APP")).is_err());
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn downloaded_firmware_plain_gets_manual_note() {
        let d = fw_tmp_dir("plain");
        let f = d.join("fw.bin");
        std::fs::write(&f, b"tiny-plain").unwrap();
        let chk = verify_downloaded_firmware(&f).unwrap();
        assert!(!chk.ok);
        assert!(chk.notes.iter().any(|n| n.contains("Not a ZIP")));
        assert!(!chk
            .notes
            .iter()
            .any(|n| n.contains("UPDATE.APP container detected")));
        std::fs::remove_dir_all(&d).ok();
    }
}
