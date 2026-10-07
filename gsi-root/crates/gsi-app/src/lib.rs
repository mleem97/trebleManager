//! Application service layer: one shared API for GUI, TUI and CLI.
//!
//! Facade only: every service delegates to the existing domain crates and
//! maps errors honestly. No logic is duplicated here.
//! UI -> service (this crate) -> domain -> infra.
//!
//! Safety rule: the service layer never invents confirmations. Every
//! destructive function takes [`SafetyConfirm`] collected by the caller
//! (GUI/TUI/CLI) and refuses before any call when it does not match.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

// ------------------------------------------------------------ AppState

/// Device snapshot (pure data, no logic).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeviceSnapshot {
    /// Product model (e.g. `VTR-L29`).
    #[serde(default)]
    pub model: String,
    /// Android release (e.g. `9`).
    #[serde(default)]
    pub android: String,
    /// Mode id (`android`, `fastboot`, `none`).
    #[serde(default)]
    pub mode: String,
    /// Masked serial (see [`mask_serial`], never the raw serial).
    #[serde(default)]
    pub serial_masked: String,
}

/// Tool snapshot (pure data, no logic).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolSnapshot {
    /// Resolved adb path (empty when missing).
    #[serde(default)]
    pub adb: String,
    /// Resolved fastboot path (empty when missing).
    #[serde(default)]
    pub fastboot: String,
    /// Probed version lines.
    #[serde(default)]
    pub versions: Vec<String>,
}

/// Compatibility snapshot (pure data, no logic).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompatSnapshot {
    /// Active profile id (e.g. `VTR-L29`).
    #[serde(default)]
    pub profile: String,
    /// Verdict lines (`PASS`/`WARN`/`FAIL` plus reasons).
    #[serde(default)]
    pub verdicts: Vec<String>,
}

/// Image snapshot (pure data, no logic).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImageSnapshot {
    /// Image path.
    #[serde(default)]
    pub path: String,
    /// Image kind (`boot`, `system`, `unknown`).
    #[serde(default)]
    pub kind: String,
    /// Container (`gzip`, `android-sparse`, `raw`).
    #[serde(default)]
    pub container: String,
}

/// Workflow snapshot (pure data, no logic).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkflowSnapshot {
    /// Active goal id (empty = none).
    #[serde(default)]
    pub goal: String,
    /// Ordered step ids.
    #[serde(default)]
    pub steps: Vec<String>,
    /// Step states (`id: status` lines).
    #[serde(default)]
    pub states: Vec<String>,
}

/// Backup snapshot (pure data, no logic).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackupSnapshot {
    /// Known backup entries (dirs or `original.img` paths).
    #[serde(default)]
    pub list: Vec<String>,
}

/// Firmware snapshot (pure data, no logic).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FirmwareSnapshot {
    /// Normalized firmware baseline.
    #[serde(default)]
    pub baseline: String,
}

/// Log snapshot (pure data, no logic).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LogSnapshot {
    /// Buffered entry count.
    #[serde(default)]
    pub count: usize,
}

/// Settings snapshot (pure data, no logic).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SettingsState {
    /// UI language code (`en` / `de`).
    #[serde(default)]
    pub language: String,
    /// Search paths (dirs).
    #[serde(default)]
    pub paths: Vec<String>,
    /// Safety master switch (destructive ops still need per-op confirm).
    #[serde(default)]
    pub safety: bool,
}

/// Update snapshot (pure data, no logic).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdateSnapshot {
    /// Current version tag.
    #[serde(default)]
    pub current: String,
    /// Latest known version tag.
    #[serde(default)]
    pub latest: String,
}

/// Whole-app snapshot (pure data, no logic).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppState {
    /// Device state.
    #[serde(default)]
    pub device: DeviceSnapshot,
    /// Tool state.
    #[serde(default)]
    pub tools: ToolSnapshot,
    /// Compatibility state.
    #[serde(default)]
    pub compatibility: CompatSnapshot,
    /// Image state.
    #[serde(default)]
    pub image: ImageSnapshot,
    /// Workflow state.
    #[serde(default)]
    pub workflow: WorkflowSnapshot,
    /// Backup state.
    #[serde(default)]
    pub backup: BackupSnapshot,
    /// Firmware state.
    #[serde(default)]
    pub firmware: FirmwareSnapshot,
    /// Log state.
    #[serde(default)]
    pub logs: LogSnapshot,
    /// Settings state.
    #[serde(default)]
    pub settings: SettingsState,
    /// Update state.
    #[serde(default)]
    pub update: UpdateSnapshot,
}

// ------------------------------------------------------------ events

/// UI log event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AppEvent {
    /// Neutral info line.
    Info(String),
    /// Success line.
    Success(String),
    /// Warning line.
    Warning(String),
    /// Error line.
    Error(String),
    /// Debug line.
    Debug(String),
    /// Progress tick.
    Progress { done: u64, total: u64 },
    /// Operation started.
    Started(String),
    /// Operation finished.
    Finished(String),
    /// Operation cancelled.
    Cancelled(String),
    /// Operation refused (safety gate).
    Refused(String),
}

impl AppEvent {
    /// Level id (`info`, `success`, `warning`, `error`, `debug`,
    /// `progress`, `started`, `finished`, `cancelled`, `refused`).
    pub fn level(&self) -> &'static str {
        match self {
            Self::Info(_) => "info",
            Self::Success(_) => "success",
            Self::Warning(_) => "warning",
            Self::Error(_) => "error",
            Self::Debug(_) => "debug",
            Self::Progress { .. } => "progress",
            Self::Started(_) => "started",
            Self::Finished(_) => "finished",
            Self::Cancelled(_) => "cancelled",
            Self::Refused(_) => "refused",
        }
    }

    /// Text payload (empty for progress ticks).
    pub fn text(&self) -> String {
        match self {
            Self::Info(s)
            | Self::Success(s)
            | Self::Warning(s)
            | Self::Error(s)
            | Self::Debug(s)
            | Self::Started(s)
            | Self::Finished(s)
            | Self::Cancelled(s)
            | Self::Refused(s) => s.clone(),
            Self::Progress { done, total } => {
                let mut s = done.to_string();
                s.push('/');
                s.push_str(&total.to_string());
                s
            }
        }
    }
}

/// One buffered log entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LogEntry {
    /// Monotonic sequence number.
    pub seq: u64,
    /// Stored (already anonymized) event.
    pub event: AppEvent,
}

/// Cap for buffered entries (oldest drop first).
pub const MAX_LOG_ENTRIES: usize = 5000;

/// Mask one serial: first 3 chars plus `***` (short/empty handled).
pub fn mask_serial(serial: &str) -> String {
    let t = serial.trim();
    if t.is_empty() {
        return String::new();
    }
    let count = t.chars().count();
    if count <= 4 {
        return "***".to_string();
    }
    let prefix: String = t.chars().take(3).collect();
    let mut out = prefix;
    out.push_str("***");
    out
}

fn looks_like_serial(token: &str) -> bool {
    if token == gsi_diag::REDACTED {
        return false;
    }
    let len = token.chars().count();
    if !(8..=32).contains(&len) {
        return false;
    }
    let mut has_digit = false;
    let mut has_letter = false;
    let mut has_lower = false;
    let mut has_upper = false;
    for c in token.chars() {
        if !c.is_ascii_alphanumeric() {
            return false;
        }
        if c.is_ascii_digit() {
            has_digit = true;
        } else {
            has_letter = true;
            if c.is_ascii_lowercase() {
                has_lower = true;
            } else {
                has_upper = true;
            }
        }
    }
    if !has_digit || !has_letter {
        return false;
    }
    if !has_lower && has_upper {
        return true;
    }
    if len >= 12 {
        return true;
    }
    false
}

fn mask_serial_tokens(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut token = String::new();
    for c in text.chars() {
        if c.is_ascii_alphanumeric() {
            token.push(c);
        } else {
            if !token.is_empty() {
                if looks_like_serial(&token) {
                    out.push_str(&mask_serial(&token));
                } else {
                    out.push_str(&token);
                }
                token.clear();
            }
            out.push(c);
        }
    }
    if !token.is_empty() {
        if looks_like_serial(&token) {
            out.push_str(&mask_serial(&token));
        } else {
            out.push_str(&token);
        }
    }
    out
}

/// Anonymize free text: delegate to [`gsi_diag::redact`] first, then mask
/// bare serial-like tokens (e.g. `6PQ...` styles).
pub fn anonymize_serials(text: &str) -> String {
    let redacted = gsi_diag::redact(text);
    mask_serial_tokens(&redacted)
}

fn anonymize_event(event: &AppEvent) -> AppEvent {
    match event {
        AppEvent::Info(s) => AppEvent::Info(anonymize_serials(s)),
        AppEvent::Success(s) => AppEvent::Success(anonymize_serials(s)),
        AppEvent::Warning(s) => AppEvent::Warning(anonymize_serials(s)),
        AppEvent::Error(s) => AppEvent::Error(anonymize_serials(s)),
        AppEvent::Debug(s) => AppEvent::Debug(anonymize_serials(s)),
        AppEvent::Progress { done, total } => AppEvent::Progress {
            done: *done,
            total: *total,
        },
        AppEvent::Started(s) => AppEvent::Started(anonymize_serials(s)),
        AppEvent::Finished(s) => AppEvent::Finished(anonymize_serials(s)),
        AppEvent::Cancelled(s) => AppEvent::Cancelled(anonymize_serials(s)),
        AppEvent::Refused(s) => AppEvent::Refused(anonymize_serials(s)),
    }
}

/// In-memory log bus (push/query/filter by level, cap 5000).
#[derive(Debug, Default)]
pub struct LogBus {
    entries: Vec<LogEntry>,
    next_seq: u64,
}

impl LogBus {
    /// Empty bus.
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            next_seq: 0,
        }
    }

    /// Push one event (message anonymized, oldest dropped past cap).
    pub fn push(&mut self, event: AppEvent) {
        let stored = anonymize_event(&event);
        if self.entries.len() >= MAX_LOG_ENTRIES {
            let overflow = self.entries.len() + 1 - MAX_LOG_ENTRIES;
            self.entries.drain(..overflow);
        }
        self.entries.push(LogEntry {
            seq: self.next_seq,
            event: stored,
        });
        self.next_seq += 1;
    }

    /// Entry count.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// True when empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Clear all entries.
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// All entries in order.
    pub fn entries(&self) -> &[LogEntry] {
        &self.entries
    }

    /// Entries with matching level (case-insensitive).
    pub fn filter_level(&self, level: &str) -> Vec<&LogEntry> {
        let want = level.trim().to_lowercase();
        let mut out = Vec::new();
        for e in self.entries.iter() {
            if e.event.level() == want.as_str() {
                out.push(e);
            }
        }
        out
    }

    /// Entries matching a caller predicate.
    pub fn filter(&self, pred: impl Fn(&AppEvent) -> bool) -> Vec<&LogEntry> {
        let mut out = Vec::new();
        for e in self.entries.iter() {
            if pred(&e.event) {
                out.push(e);
            }
        }
        out
    }
}

// ------------------------------------------------------------ safety

/// Caller-collected destructive confirmation.
///
/// The service layer never invents this: GUI/TUI/CLI collect the typed
/// phrase plus a second explicit tick, then pass it in. Refused before
/// any call when it does not match.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SafetyConfirm {
    /// Typed phrase (e.g. `FLASH`, `RESTORE`, `WIPE`, `UPDATE`).
    pub phrase_typed: String,
    /// Second explicit confirmation tick.
    pub double: bool,
}

impl SafetyConfirm {
    /// Build from caller inputs.
    pub fn new(phrase_typed: &str, double: bool) -> Self {
        Self {
            phrase_typed: phrase_typed.to_string(),
            double,
        }
    }

    /// Verify against the exact phrase (plus double tick).
    pub fn verify(&self, expected: &str) -> Result<(), String> {
        if self.phrase_typed.trim() != expected.trim() {
            return Err(format!(
                "refused: missing first confirmation (need {})",
                expected.trim()
            ));
        }
        if !self.double {
            return Err("refused: missing second confirmation".to_string());
        }
        Ok(())
    }
}

// ------------------------------------------------------------ device

/// Device service (delegation only).
pub struct DeviceService;

impl DeviceService {
    /// Parse an `adb devices` transcript.
    pub fn adb_devices(transcript: &str) -> Vec<gsi_device::parse::Device> {
        let lines: Vec<&str> = transcript.lines().collect();
        gsi_device::parse::adb_devices(&lines)
    }

    /// Parse a `fastboot devices` transcript.
    pub fn fastboot_devices(transcript: &str) -> Vec<gsi_device::parse::Device> {
        let lines: Vec<&str> = transcript.lines().collect();
        gsi_device::parse::fastboot_devices(&lines)
    }

    /// Parse a getprop dump into a map.
    pub fn device_info(transcript: &str) -> BTreeMap<String, String> {
        let lines: Vec<&str> = transcript.lines().collect();
        gsi_device::parse::getprop(&lines)
    }

    /// Parse a by-name listing.
    pub fn byname_entries(raw: &str) -> Vec<gsi_device::parse::ByName> {
        gsi_device::parse::byname(raw)
    }

    /// Locate a tool binary (no spawn).
    pub fn locate(name: &str, extra_dirs: &[PathBuf]) -> Option<gsi_tool::Tool> {
        gsi_tool::locate(name, extra_dirs)
    }
}

// ------------------------------------------------------------ image

/// Combined image analysis (container + archive + sparse header).
#[derive(Debug, Clone)]
pub struct ImageAnalysis {
    /// Outer container.
    pub container: gsi_image::Container,
    /// Archive kind by file name.
    pub archive: gsi_archive::ArchiveKind,
    /// Sparse header when present.
    pub sparse: Option<gsi_image::SparseHeader>,
}

/// Image service (delegation only).
pub struct ImageService;

impl ImageService {
    /// Detect container by magic bytes.
    pub fn container_of(path: &Path) -> gsi_image::Container {
        gsi_image::detect_container(path)
    }

    /// Parse sparse header (None when invalid).
    pub fn sparse_header(path: &Path) -> Option<gsi_image::SparseHeader> {
        gsi_image::parse_sparse_header(path)
    }

    /// Classify archive by file name.
    pub fn archive_kind(file_name: &str) -> gsi_archive::ArchiveKind {
        gsi_archive::classify(file_name)
    }

    /// Combined analysis (image magic plus archive name class).
    pub fn analyze(path: &Path) -> ImageAnalysis {
        let container = gsi_image::detect_container(path);
        let sparse = gsi_image::parse_sparse_header(path);
        let archive = match path.file_name().and_then(|n| n.to_str()) {
            Some(n) => gsi_archive::classify(n),
            None => gsi_archive::ArchiveKind::Plain,
        };
        ImageAnalysis {
            container,
            archive,
            sparse,
        }
    }
}

// ------------------------------------------------------------ root

/// Root service (planning only, NO execution).
pub struct RootService;

impl RootService {
    /// Preferred-first root methods table.
    pub fn methods() -> Vec<gsi_gates::RootMethod> {
        gsi_gates::preferred_root_methods()
    }

    /// Root method ids in priority order.
    pub fn method_ids() -> Vec<String> {
        gsi_gates::root_method_ids()
    }

    /// Display name for one method id.
    pub fn method_name(id: &str) -> Result<String, String> {
        gsi_gates::root_method_name(id)
    }

    /// Patch-base plan (which image Magisk patches).
    pub fn patch_plan(
        installed_rom: &str,
        stock_image: &str,
        rom_image: &str,
        entry_gsi: &str,
        base_kind: &str,
    ) -> Result<gsi_gates::PatchBase, String> {
        gsi_gates::patch_base(installed_rom, stock_image, rom_image, entry_gsi, base_kind)
    }

    /// Root-verify plan (steps only, no I/O).
    pub fn verify_plan() -> gsi_workflow::VerifyRootPlan {
        gsi_workflow::plan_verify_root()
    }

    /// Classify verify transcripts (pure, no execution).
    pub fn classify_verify(which_su: &str, su_id: &str) -> gsi_workflow::VerifyVerdict {
        gsi_workflow::classify_verify_root(which_su, su_id)
    }
}

// ------------------------------------------------------------ flash

/// Flash service (plan plus gated run, delegation only).
pub struct FlashService;

impl FlashService {
    /// Safe-flash plan (pure, no I/O).
    pub fn plan_safe_flash(
        image: &str,
        partition: &str,
        backup_available: bool,
        profile_verified: bool,
        fastboot_ready: bool,
        confirm_flash: bool,
        confirm_yes: bool,
    ) -> gsi_workflow::DestructivePlan {
        gsi_workflow::plan_safe_flash(
            image,
            partition,
            backup_available,
            profile_verified,
            fastboot_ready,
            confirm_flash,
            confirm_yes,
        )
    }

    /// System-flash plan (pure, no I/O).
    pub fn plan_system_flash(
        image: &str,
        profile_verified: bool,
        fastboot_ready: bool,
        image_ok: bool,
        registry_blocked: bool,
        confirm_flash: bool,
        confirm_yes: bool,
    ) -> gsi_workflow::DestructivePlan {
        gsi_workflow::plan_system_flash(
            image,
            profile_verified,
            fastboot_ready,
            image_ok,
            registry_blocked,
            confirm_flash,
            confirm_yes,
        )
    }

    /// TWRP-flash plan (pure, no I/O).
    pub fn plan_twrp_flash(
        image: &str,
        partition: &str,
        backup_available: bool,
        profile_verified: bool,
        fastboot_ready: bool,
        confirm_flash: bool,
        confirm_yes: bool,
    ) -> gsi_workflow::DestructivePlan {
        gsi_workflow::plan_twrp_flash(
            image,
            partition,
            backup_available,
            profile_verified,
            fastboot_ready,
            confirm_flash,
            confirm_yes,
        )
    }

    /// Restore plan (pure, no I/O).
    pub fn plan_restore(
        backup_dir: &str,
        partition: &str,
        backup_has_original: bool,
        fastboot_ready: bool,
        confirm_restore: bool,
        confirm_yes: bool,
    ) -> gsi_workflow::DestructivePlan {
        gsi_workflow::plan_restore(
            backup_dir,
            partition,
            backup_has_original,
            fastboot_ready,
            confirm_restore,
            confirm_yes,
        )
    }

    /// Guided-wipe plan (pure, no I/O).
    pub fn plan_guided_wipe(
        fastboot_ready: bool,
        confirm_wipe: bool,
        confirm_yes: bool,
    ) -> gsi_workflow::DestructivePlan {
        gsi_workflow::plan_guided_wipe(fastboot_ready, confirm_wipe, confirm_yes)
    }

    /// Run a destructive plan via caller-provided executor.
    ///
    /// Safety: the caller (GUI/TUI/CLI) collects [`SafetyConfirm`]; it is
    /// verified against the plan words (`FLASH`/`RESTORE`/`WIPE` plus `YES`)
    /// before any executor call, else Refused with zero calls made.
    pub fn run(
        plan: &gsi_workflow::DestructivePlan,
        exec: &mut dyn gsi_exec::Executor,
        confirm: &SafetyConfirm,
    ) -> Result<gsi_exec::RunReport, gsi_exec::ExecError> {
        let want = plan.first_word.trim();
        let got = confirm.phrase_typed.trim();
        let first_ok = if want == "FLASH" {
            got == "FLASH" || got == "FLASHEN"
        } else {
            got == want
        };
        if !first_ok {
            return Err(gsi_exec::ExecError::refused(format!(
                "missing first token '{}'",
                plan.first_word.as_str()
            )));
        }
        if !confirm.double {
            return Err(gsi_exec::ExecError::refused(format!(
                "missing second token '{}'",
                plan.second_word.as_str()
            )));
        }
        gsi_exec::run_plan(
            plan,
            exec,
            confirm.phrase_typed.as_str(),
            plan.second_word.as_str(),
        )
    }
}

// ------------------------------------------------------------ backup

/// Backup service (delegation only).
pub struct BackupService;

impl BackupService {
    /// Backup plan (pure).
    pub fn plan(
        partition: &str,
        dest_dir: &str,
        model: &str,
        firmware: &str,
    ) -> Result<gsi_diag::BackupPlan, String> {
        gsi_diag::plan_backup(partition, dest_dir, model, firmware)
    }

    /// Backup metadata JSON.
    pub fn metadata(plan: &gsi_diag::BackupPlan) -> String {
        gsi_diag::backup_metadata_json(plan)
    }

    /// Export plan for a ROM file (pure).
    pub fn export_plan(
        rom_file_name: &str,
        image_kind: &str,
    ) -> Result<gsi_diag::ExportPlan, String> {
        gsi_diag::plan_export(rom_file_name, image_kind)
    }
}

// ------------------------------------------------------------ firmware

/// Firmware service (delegation only).
pub struct FirmwareService;

impl FirmwareService {
    /// Validate a downloaded firmware file (hash plus size heuristic).
    pub fn validate_file(path: &Path) -> Result<gsi_update::FirmwareCheck, String> {
        gsi_update::verify_downloaded_firmware(path)
    }

    /// Verify a hash sidecar (`.sha256` next to the file).
    pub fn hash_sidecar(path: &Path) -> Result<String, String> {
        gsi_update::verify_hash_sidecar(path)
    }

    /// Check a download URL without fetching (pure).
    pub fn download_url_ok(url: &str) -> bool {
        gsi_update::valid_download_url(url)
    }

    /// Parse a release tag from a GitHub API body (pure, no network).
    pub fn release_tag(body: &str) -> Option<String> {
        gsi_update::parse_tag(body)
    }
}

// ------------------------------------------------------------ diagnostic

/// Diagnostic service (delegation only).
pub struct DiagnosticService;

impl DiagnosticService {
    /// Pure preflight over explicit inputs.
    pub fn preflight(input: &gsi_gates::PreflightInput) -> gsi_gates::PreflightReport {
        gsi_gates::preflight(input)
    }

    /// Aggregate flash-readiness parts.
    pub fn readiness(parts: &gsi_gates::ReadinessParts) -> gsi_gates::FlashReadiness {
        gsi_gates::check_readiness(parts)
    }

    /// Evaluate one step gate.
    pub fn step_gate(input: &gsi_gates::GateInput) -> gsi_gates::StepGate {
        gsi_gates::step_gate(input)
    }

    /// Firmware compatibility verdict (registry rules).
    pub fn firmware_compat(
        model: &str,
        firmware: &str,
        region: &str,
    ) -> gsi_registry::CompatVerdict {
        gsi_registry::firmware_compat(model, firmware, region)
    }

    /// Vendor advice for an EMUI plus target Android pair.
    pub fn vendor_advice(emui: &str, target_android: &str) -> String {
        gsi_registry::vendor_advice(emui, target_android)
    }

    /// Workflow plan for a goal (pure, no I/O).
    pub fn workflow_plan(goal: &str) -> Result<gsi_state::WorkflowPlan, String> {
        gsi_state::new_workflow_plan(goal)
    }

    /// Ordered steps for a goal (unknown goal yields empty).
    pub fn goal_steps(goal: &str) -> Vec<String> {
        gsi_state::goal_steps(goal)
    }

    /// Read-only post-flash validation via caller executor.
    pub fn validate(exec: &mut dyn gsi_exec::Executor) -> gsi_exec::ValidationReport {
        gsi_exec::run_validate(exec)
    }

    /// Read-only root verify via caller executor.
    pub fn verify_root(exec: &mut dyn gsi_exec::Executor) -> gsi_exec::VerifyReport {
        gsi_exec::run_verify_root(exec)
    }
}

// ------------------------------------------------------------ tool

/// Tool service (delegation only).
pub struct ToolService;

impl ToolService {
    /// Locate a tool binary (no spawn).
    pub fn locate(name: &str, extra_dirs: &[PathBuf]) -> Option<gsi_tool::Tool> {
        gsi_tool::locate(name, extra_dirs)
    }

    /// Load tool paths config (None when missing or unparsable).
    pub fn load_config(path: &Path) -> Option<gsi_config::ToolPaths> {
        gsi_config::load_tool_config(path)
    }

    /// Save tool paths config (creates parent dirs).
    pub fn save_config(path: &Path, cfg: &gsi_config::ToolPaths) -> Result<(), String> {
        gsi_config::save_tool_config(path, cfg).map_err(|e| format!("write: {e}"))
    }

    /// Build tool paths (pure).
    pub fn tool_paths(
        adb: &str,
        fastboot: &str,
        scrcpy: &str,
        updated: &str,
    ) -> gsi_config::ToolPaths {
        gsi_config::ToolPaths::new(adb, fastboot, scrcpy, updated)
    }
}

// ------------------------------------------------------------ update

/// Update service (delegation only).
pub struct UpdateService;

impl UpdateService {
    /// Parse a release tag from a GitHub API body (pure, no network).
    pub fn check_tag(body: &str) -> Option<String> {
        gsi_update::parse_tag(body)
    }

    /// Check a download URL without fetching (pure).
    pub fn url_ok(url: &str) -> bool {
        gsi_update::valid_download_url(url)
    }

    /// Verify a staged file against an expected SHA-256.
    pub fn verify(path: &Path, expected: &str) -> Result<gsi_update::SignatureState, String> {
        gsi_update::verify_staged(path, expected)
    }

    /// Install a verified staged file.
    ///
    /// Safety: the caller (GUI/TUI/CLI) collects [`SafetyConfirm`] with
    /// phrase `UPDATE` plus double tick; refused before any file op else.
    pub fn install(
        current: &Path,
        staged: &Path,
        confirm: &SafetyConfirm,
    ) -> Result<PathBuf, String> {
        confirm.verify("UPDATE")?;
        gsi_update::install_staged(current, staged)
    }
}

// ------------------------------------------------------------ settings

/// Settings service (JSON under gsi-config dirs, delegation only).
pub struct SettingsService;

impl SettingsService {
    /// Default settings file (`<config_dir>/settings.json`).
    pub fn settings_file() -> Option<PathBuf> {
        gsi_config::config_dir().map(|d| d.join("settings.json"))
    }

    /// Load settings (missing file yields default; corrupt yields Err).
    pub fn load(path: &Path) -> Result<SettingsState, String> {
        let text = match std::fs::read_to_string(path) {
            Ok(t) => t,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Ok(SettingsState::default());
            }
            Err(e) => return Err(format!("read: {e}")),
        };
        serde_json::from_str(&text).map_err(|e| format!("settings: invalid json: {e}"))
    }

    /// Save settings (creates parent dirs).
    pub fn save(path: &Path, settings: &SettingsState) -> Result<(), String> {
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent).map_err(|e| format!("mkdir: {e}"))?;
            }
        }
        let text = serde_json::to_string_pretty(settings).map_err(|e| format!("json: {e}"))?;
        std::fs::write(path, text).map_err(|e| format!("write: {e}"))?;
        Ok(())
    }

    /// Language code (`en` / `de`).
    pub fn language_code(lang: gsi_i18n::Language) -> &'static str {
        lang.code()
    }

    /// Parse a language code (unknown yields English).
    pub fn parse_language(code: &str) -> gsi_i18n::Language {
        gsi_i18n::Language::from_code(code)
    }

    /// Look up a UI message.
    pub fn message(lang: gsi_i18n::Language, key: &str) -> &'static str {
        gsi_i18n::lookup(lang, key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write as _;

    fn tmp_dir(tag: &str) -> PathBuf {
        static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let n = N.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let mut p = std::env::temp_dir();
        p.push(format!("gsi-app-test-{}-{}-{}", std::process::id(), tag, n));
        let _ = std::fs::create_dir_all(&p);
        p
    }

    fn write_bytes(path: &Path, bytes: &[u8]) {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let mut f = std::fs::File::create(path).unwrap();
        f.write_all(bytes).unwrap();
    }

    #[test]
    fn logbus_cap_keeps_newest() {
        let mut bus = LogBus::new();
        for i in 0..(MAX_LOG_ENTRIES + 5) {
            bus.push(AppEvent::Info(format!("line {i}")));
        }
        assert_eq!(bus.len(), MAX_LOG_ENTRIES);
        assert_eq!(bus.entries().len(), MAX_LOG_ENTRIES);
        let first = bus.entries().first().unwrap();
        assert_eq!(first.seq, 5);
        let last = bus.entries().last().unwrap();
        assert!(matches!(&last.event, AppEvent::Info(s) if s.contains("5004")));
    }

    #[test]
    fn logbus_filter_by_level() {
        let mut bus = LogBus::new();
        bus.push(AppEvent::Info("a".to_string()));
        bus.push(AppEvent::Warning("b".to_string()));
        bus.push(AppEvent::Error("c".to_string()));
        bus.push(AppEvent::Warning("d".to_string()));
        assert_eq!(bus.filter_level("warning").len(), 2);
        assert_eq!(bus.filter_level("WARNING").len(), 2);
        assert_eq!(bus.filter_level("error").len(), 1);
        assert_eq!(bus.filter_level("debug").len(), 0);
        assert_eq!(bus.filter(|e| e.level() == "info").len(), 1);
        assert!(!bus.is_empty());
        bus.clear();
        assert!(bus.is_empty());
        assert_eq!(bus.len(), 0);
    }

    #[test]
    fn logbus_anonymizes_on_push() {
        let mut bus = LogBus::new();
        bus.push(AppEvent::Info(
            "device 6PQ0217B08003446 attached".to_string(),
        ));
        let e = bus.entries().first().unwrap();
        let t = e.event.text();
        assert!(!t.contains("6PQ0217B08003446"));
        assert!(t.contains("6PQ"));
        bus.push(AppEvent::Progress { done: 1, total: 4 });
        assert_eq!(bus.filter_level("progress").len(), 1);
    }

    #[test]
    fn anonymize_bare_serial() {
        let out = anonymize_serials("connected 6PQ0217B08003446 ok");
        assert!(!out.contains("6PQ0217B08003446"));
        assert!(out.contains("6PQ"));
        let same = anonymize_serials("plain words attached here");
        assert_eq!(same, "plain words attached here");
    }

    #[test]
    fn anonymize_leaves_hash_line() {
        let hash = "abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789";
        let line = format!("sha256: {hash}");
        let out = anonymize_serials(&line);
        assert!(out.contains(hash));
    }

    #[test]
    fn mask_serial_samples() {
        assert_eq!(mask_serial("6PQ0217B08003446"), "6PQ***");
        assert_eq!(mask_serial(""), "");
        assert_eq!(mask_serial("ab"), "***");
        assert_eq!(mask_serial("  6PQ0217B08003446  "), "6PQ***");
    }

    #[test]
    fn device_service_parses_transcripts() {
        let adb = "List of devices attached\n6PQ0217B08003446\tdevice\n";
        let devs = DeviceService::adb_devices(adb);
        assert_eq!(devs.len(), 1);
        assert_eq!(devs[0].serial, "6PQ0217B08003446");
        let fb = DeviceService::fastboot_devices("ABC123\tfastboot\n");
        assert_eq!(fb.len(), 1);
        let props = DeviceService::device_info("[ro.product.model]: [VTR-L29]\n");
        assert_eq!(
            props.get("ro.product.model").map(String::as_str),
            Some("VTR-L29")
        );
        let by = DeviceService::byname_entries("recovery_ramdisk -> /dev/block/x\n");
        assert_eq!(by.len(), 1);
        assert_eq!(by[0].name, "recovery_ramdisk");
        assert!(DeviceService::locate("gsi-app-test-no-such-tool-xyz", &[]).is_none());
    }

    #[test]
    fn image_service_analyzes_fixtures() {
        let d = tmp_dir("image");
        let gz = d.join("a.img.gz");
        write_bytes(&gz, &[0x1F, 0x8B, 0x08, 0x00]);
        assert_eq!(ImageService::container_of(&gz), gsi_image::Container::Gzip);
        assert_eq!(
            ImageService::archive_kind("rom.zip"),
            gsi_archive::ArchiveKind::Zip
        );
        let rep = ImageService::analyze(&gz);
        assert_eq!(rep.container, gsi_image::Container::Gzip);
        assert!(rep.sparse.is_none());
        let plain = d.join("plain.img");
        write_bytes(&plain, b"hello");
        assert_eq!(
            ImageService::container_of(&plain),
            gsi_image::Container::Raw
        );
        assert!(ImageService::sparse_header(&plain).is_none());
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn root_service_tables_no_exec() {
        assert_eq!(RootService::method_ids().len(), 4);
        assert_eq!(RootService::methods().len(), 4);
        assert!(RootService::method_name("magisk-recovery").is_ok());
        assert!(RootService::method_name("nope").is_err());
        let ok = RootService::patch_plan("stock", "/tmp/a/stock.img", "", "", "boot");
        assert!(ok.is_ok());
        let refused = RootService::patch_plan("stock", "/tmp/a/stock.img", "", "", "system");
        assert!(refused.is_err());
        let plan = RootService::verify_plan();
        assert!(!plan.steps.is_empty());
        assert_eq!(
            RootService::classify_verify("", "uid=0(root)"),
            gsi_workflow::VerifyVerdict::Rooted
        );
        assert_eq!(
            RootService::classify_verify("", ""),
            gsi_workflow::VerifyVerdict::NotRooted
        );
    }

    #[test]
    fn flash_service_refuses_before_any_call() {
        let plan = FlashService::plan_safe_flash(
            "m.img",
            "recovery_ramdisk",
            true,
            true,
            true,
            true,
            true,
        );
        assert!(plan.ready);
        let bad = SafetyConfirm::new("WRONG", true);
        let mut stub = gsi_exec::StubExecutor::new();
        let r = FlashService::run(&plan, &mut stub, &bad);
        assert!(r.is_err());
        assert_eq!(stub.call_count(), 0);
        let half = SafetyConfirm::new("FLASH", false);
        let mut stub2 = gsi_exec::StubExecutor::new();
        assert!(FlashService::run(&plan, &mut stub2, &half).is_err());
        assert_eq!(stub2.call_count(), 0);
    }

    #[test]
    fn flash_service_runs_with_confirm() {
        let plan = FlashService::plan_safe_flash(
            "m.img",
            "recovery_ramdisk",
            true,
            true,
            true,
            true,
            true,
        );
        let confirm = SafetyConfirm::new("FLASH", true);
        assert!(confirm.verify("FLASH").is_ok());
        assert!(confirm.verify("WIPE").is_err());
        let out = gsi_exec::ExecOut::new(
            "OKAY [ 1]\nFinished. Total time: 0.500s\n".to_string(),
            String::new(),
            Some(0),
        );
        let mut stub = gsi_exec::StubExecutor::with_outputs(vec![out]);
        let r = FlashService::run(&plan, &mut stub, &confirm);
        assert!(r.is_ok());
        assert_eq!(stub.call_count(), 1);
        assert!(r.unwrap().is_ok());
        let wipe = FlashService::plan_guided_wipe(true, true, true);
        assert_eq!(wipe.partition, "userdata");
        let sys = FlashService::plan_system_flash("g.img", true, true, true, false, true, true);
        assert!(sys.ready);
        let twrp = FlashService::plan_twrp_flash(
            "t.img",
            "recovery_ramdisk",
            true,
            true,
            true,
            true,
            true,
        );
        assert!(twrp.ready);
        let rest = FlashService::plan_restore("bd", "recovery_ramdisk", true, true, true, true);
        assert!(rest.ready);
    }

    #[test]
    fn backup_service_plan_metadata() {
        let plan = BackupService::plan("recovery_ramdisk", "/tmp/bd", "VTR-L29", "fw").unwrap();
        assert_eq!(plan.partition, "recovery_ramdisk");
        let meta = BackupService::metadata(&plan);
        assert!(meta.contains("VTR-L29"));
        let exp = BackupService::export_plan("rom.zip", "boot").unwrap();
        assert_eq!(exp.kind, "rom-zip");
        assert!(BackupService::plan("", "/tmp/bd", "m", "f").is_err());
    }

    #[test]
    fn firmware_service_validates_locally() {
        let d = tmp_dir("fw");
        let f = d.join("fw.bin");
        write_bytes(&f, b"firmware-bytes");
        let chk = FirmwareService::validate_file(&f).unwrap();
        assert!(!chk.sha256.is_empty());
        assert!(
            FirmwareService::release_tag("{\"tag_name\": \"v1.2.3\"}").as_deref() == Some("v1.2.3")
        );
        assert!(FirmwareService::release_tag("nope").is_none());
        assert!(FirmwareService::download_url_ok(
            "https://example.com/rom.zip"
        ));
        assert!(!FirmwareService::download_url_ok(
            "ftp://example.com/rom.zip"
        ));
        assert!(FirmwareService::validate_file(&d.join("missing.bin")).is_err());
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn diagnostic_service_gates() {
        let input = gsi_gates::PreflightInput {
            adb_path: "/usr/bin/adb".to_string(),
            fastboot_path: "/usr/bin/fastboot".to_string(),
            shell_ok: true,
            writables: vec![],
            files: vec![],
            hashes: vec![],
        };
        assert!(DiagnosticService::preflight(&input).go);
        let parts = gsi_gates::ReadinessParts {
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
        assert!(DiagnosticService::readiness(&parts).go);
        let gate = gsi_gates::GateInput {
            step: "analyze".to_string(),
            states: gsi_gates::DeviceStates {
                overall: "READY".to_string(),
                adb: "ADB_READY".to_string(),
                fastboot: "FASTBOOT_READY".to_string(),
            },
            profile_verified: true,
            patched_present: true,
            patch_base_present: true,
            patch_base_source: "stock".to_string(),
        };
        assert!(DiagnosticService::step_gate(&gate).pass);
        let v =
            DiagnosticService::firmware_compat("VTR-L29", "VTR-L29 9.1.0.297(C432E5R1P9)", "C432");
        assert_eq!(v.status, "PASS");
        assert!(!DiagnosticService::vendor_advice("9.1", "13").is_empty());
        assert!(DiagnosticService::workflow_plan("root").is_ok());
        assert!(DiagnosticService::workflow_plan("nope").is_err());
        assert_eq!(DiagnosticService::goal_steps("root").len(), 10);
    }

    #[test]
    fn diagnostic_service_validate_stub() {
        let outs = vec![
            gsi_exec::ExecOut::new(
                "List of devices attached\nABC\tdevice\n".to_string(),
                String::new(),
                Some(0),
            ),
            gsi_exec::ExecOut::new("13".to_string(), String::new(), Some(0)),
            gsi_exec::ExecOut::new("lineage".to_string(), String::new(), Some(0)),
            gsi_exec::ExecOut::new("Enforcing".to_string(), String::new(), Some(0)),
            gsi_exec::ExecOut::new("uid=0(root)".to_string(), String::new(), Some(0)),
            gsi_exec::ExecOut::new("/system /vendor".to_string(), String::new(), Some(0)),
            gsi_exec::ExecOut::new("Wi-Fi is enabled".to_string(), String::new(), Some(0)),
            gsi_exec::ExecOut::new("1".to_string(), String::new(), Some(0)),
            gsi_exec::ExecOut::new("level: 80".to_string(), String::new(), Some(0)),
            gsi_exec::ExecOut::new("sensor list".to_string(), String::new(), Some(0)),
        ];
        let mut stub = gsi_exec::StubExecutor::with_outputs(outs);
        let rep = DiagnosticService::validate(&mut stub);
        assert!(rep.is_ok());
        assert_eq!(rep.checks.len(), 10);
        let mut stub2 = gsi_exec::StubExecutor::with_outputs(vec![
            gsi_exec::ExecOut::new("/system/bin/su".to_string(), String::new(), Some(0)),
            gsi_exec::ExecOut::new("uid=0(root)".to_string(), String::new(), Some(0)),
            gsi_exec::ExecOut::new("27.0".to_string(), String::new(), Some(0)),
        ]);
        let vr = DiagnosticService::verify_root(&mut stub2);
        assert_eq!(vr.verdict, gsi_exec::VerifyVerdict::Rooted);
    }

    #[test]
    fn tool_service_locate_config() {
        assert!(ToolService::locate("gsi-app-test-no-such-tool-xyz", &[]).is_none());
        let cfg = ToolService::tool_paths("/a/adb", "/a/fastboot", "", "u");
        assert_eq!(cfg.adb, "/a/adb");
        let d = tmp_dir("toolcfg");
        let f = d.join("sub").join("config.json");
        assert!(ToolService::save_config(&f, &cfg).is_ok());
        assert_eq!(ToolService::load_config(&f), Some(cfg));
        assert_eq!(ToolService::load_config(&d.join("absent.json")), None);
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn update_service_verify_install_gated() {
        let d = tmp_dir("upd");
        let staged = d.join("new.bin");
        write_bytes(&staged, b"new-binary");
        let current = d.join("cur.bin");
        write_bytes(&current, b"old-binary");
        let hash = gsi_update::sha256_file(&staged).unwrap();
        assert!(UpdateService::verify(&staged, &hash).is_ok());
        assert!(UpdateService::verify(&staged, "deadbeef").is_err());
        assert!(UpdateService::check_tag("{\"tag_name\":\"v2\"}").as_deref() == Some("v2"));
        assert!(UpdateService::url_ok("https://example.com/a.zip"));
        let refused = UpdateService::install(&current, &staged, &SafetyConfirm::new("NOPE", true));
        assert!(refused.is_err());
        assert_eq!(std::fs::read(&current).unwrap(), b"old-binary");
        let ok = UpdateService::install(&current, &staged, &SafetyConfirm::new("UPDATE", true));
        assert!(ok.is_ok());
        assert_eq!(std::fs::read(&current).unwrap(), b"new-binary");
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn settings_service_roundtrip() {
        let d = tmp_dir("settings");
        let f = d.join("settings.json");
        assert_eq!(
            SettingsService::load(&d.join("absent.json")).unwrap(),
            SettingsState::default()
        );
        let s = SettingsState {
            language: "de".to_string(),
            paths: vec!["/tmp/a".to_string()],
            safety: true,
        };
        assert!(SettingsService::save(&f, &s).is_ok());
        assert_eq!(SettingsService::load(&f).unwrap(), s);
        assert!(SettingsService::load(&f).unwrap().safety);
        assert!(SettingsService::save(&f, &s).is_ok());
        let bad = d.join("bad.json");
        write_bytes(&bad, b"{oops");
        assert!(SettingsService::load(&bad).is_err());
        assert_eq!(SettingsService::language_code(gsi_i18n::Language::De), "de");
        assert_eq!(
            SettingsService::parse_language("de-DE"),
            gsi_i18n::Language::De
        );
        assert_eq!(
            SettingsService::parse_language("fr"),
            gsi_i18n::Language::En
        );
        assert!(!SettingsService::message(gsi_i18n::Language::En, "device.no_device").is_empty());
        let _ = SettingsService::settings_file();
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn appstate_serde_roundtrip() {
        let mut st = AppState::default();
        st.device.model = "VTR-L29".to_string();
        st.device.serial_masked = mask_serial("6PQ0217B08003446");
        st.settings.language = "en".to_string();
        st.update.current = "v1".to_string();
        st.update.latest = "v2".to_string();
        let s = serde_json::to_string(&st).unwrap();
        let back: AppState = serde_json::from_str(&s).unwrap();
        assert_eq!(back, st);
        assert_eq!(back.device.serial_masked, "6PQ***");
        assert_eq!(AppEvent::Info("x".to_string()).level(), "info");
        assert_eq!(AppEvent::Progress { done: 1, total: 2 }.level(), "progress");
        assert_eq!(AppEvent::Refused("r".to_string()).level(), "refused");
    }
}
