//! State/persistence layer ported from the scripts.
//!
//! Covers `data/installed-rom.txt` (single trimmed token),
//! `workflow-state.json` (`goal` + `steps` + `slot` + `last_root` +
//! `updated`, other keys preserved), path helpers, the generic goal
//! table, and the `New-WorkflowPlan` plan struct.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Stamp used for `updated` / `timestamp` fields (`yyyy-MM-dd HH:mm:ss`, UTC).
fn now_stamp() -> String {
    let secs: u64 = match std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
        Ok(d) => d.as_secs(),
        Err(_) => 0,
    };
    let days = (secs / 86_400) as i64;
    let tod = (secs % 86_400) as u32;
    let (y, m, d) = civil_from_days(days);
    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
        y,
        m,
        d,
        tod / 3600,
        (tod % 3600) / 60,
        tod % 60
    )
}

/// Days since 1970-01-01 -> (year, month, day), Howard Hinnant algorithm.
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z2 = z + 719_468;
    let era = if z2 >= 0 { z2 } else { z2 - 146_096 } / 146_097;
    let doe = (z2 - era * 146_097) as u64;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

/// Create parent directories for a file path (no-op when none).
fn ensure_parent(path: &Path) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).map_err(|e| format!("mkdir: {e}"))?;
        }
    }
    Ok(())
}

/// Read a JSON object from a file, preserving nothing on error.
fn read_object(path: &Path) -> Result<Option<BTreeMap<String, serde_json::Value>>, String> {
    let text = match std::fs::read_to_string(path) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(format!("read: {e}")),
    };
    let map: BTreeMap<String, serde_json::Value> =
        serde_json::from_str(&text).map_err(|e| format!("workflow-state: invalid json: {e}"))?;
    Ok(Some(map))
}

/// Write a JSON object pretty-printed (creates parent dirs).
fn write_object(path: &Path, map: &BTreeMap<String, serde_json::Value>) -> Result<(), String> {
    ensure_parent(path)?;
    let text = serde_json::to_string_pretty(map).map_err(|e| format!("json: {e}"))?;
    std::fs::write(path, text).map_err(|e| format!("write: {e}"))?;
    Ok(())
}

// ------------------------------------------------------------- path helpers

/// `Get-InstalledRomFile` / `rom_file`: `<tool_root>/data/installed-rom.txt`.
pub fn installed_rom_file(tool_root: &Path) -> PathBuf {
    tool_root.join("data").join("installed-rom.txt")
}

/// `Get-WorkflowStateFile` / `state_file`: `<log_dir>/workflow-state.json`.
pub fn workflow_state_file(log_dir: &Path) -> PathBuf {
    log_dir.join("workflow-state.json")
}

/// Native log dir default: `<config_dir>/logs` via `gsi-config`.
pub fn default_log_dir() -> Option<PathBuf> {
    gsi_config::config_dir().map(|b| gsi_config::subdir(&b, "logs"))
}

/// Native state file default: `<config_dir>/logs/workflow-state.json`.
pub fn default_workflow_state_file() -> Option<PathBuf> {
    default_log_dir().map(|d| d.join("workflow-state.json"))
}

/// Native installed-rom default: `<config_dir>/data/installed-rom.txt`.
pub fn default_installed_rom_file() -> Option<PathBuf> {
    gsi_config::config_dir().map(|b| b.join("data").join("installed-rom.txt"))
}

// ------------------------------------------------------------ installed-rom

/// Validate a ROM id: single trimmed token, no inner whitespace.
pub fn validate_rom_id(id: &str) -> Result<String, String> {
    let t = id.trim().trim_start_matches('\u{FEFF}');
    if t.is_empty() {
        return Err("installed-rom: empty id".to_string());
    }
    if t.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return Err("installed-rom: id must be a single token".to_string());
    }
    Ok(t.to_string())
}

/// `Load-InstalledRom` / `load_rom`: missing/empty file -> `None`.
pub fn read_installed_rom(path: &Path) -> Result<Option<String>, String> {
    let text = match std::fs::read_to_string(path) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(format!("read: {e}")),
    };
    let t = text.trim().trim_start_matches('\u{FEFF}');
    // Bash strips all spaces/newlines; a file with only whitespace is empty.
    let compact: String = t.chars().filter(|c| !c.is_whitespace()).collect();
    if compact.is_empty() {
        return Ok(None);
    }
    // Anything with inner whitespace is corrupt (not a single token).
    if t.chars().any(|c| c.is_whitespace()) {
        return Err("installed-rom: invalid content (must be a single token)".to_string());
    }
    Ok(Some(compact))
}

/// `Save-InstalledRom` / `save_rom`: persist trimmed token, no newline.
pub fn write_installed_rom(path: &Path, id: &str) -> Result<(), String> {
    let clean = validate_rom_id(id)?;
    ensure_parent(path)?;
    std::fs::write(path, clean).map_err(|e| format!("write: {e}"))?;
    Ok(())
}

// -------------------------------------------------------------- root record

/// `last_root` record as written by `Save-RootState` / `save_root_state`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RootRecord {
    /// Verified root state (e.g. `ROOTED`, `ROOTED+persisted-fixes`).
    pub state: String,
    /// Stamp `yyyy-MM-dd HH:mm:ss` (informational only).
    #[serde(default)]
    pub timestamp: String,
    /// Optional boot mode (kept across writes when not overridden).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub boot_mode: Option<String>,
}

/// Read `last_root` from a state file. Missing file/key -> `None`.
pub fn read_root_state(state_file: &Path) -> Result<Option<RootRecord>, String> {
    let map = match read_object(state_file)? {
        Some(m) => m,
        None => return Ok(None),
    };
    let v = match map.get("last_root") {
        Some(v) if !v.is_null() => v,
        _ => return Ok(None),
    };
    let obj = match v.as_object() {
        Some(o) => o,
        None => return Err("last_root: invalid (must be an object)".to_string()),
    };
    let state = match obj.get("state").and_then(|s| s.as_str()) {
        Some(s) if !s.trim().is_empty() => s.to_string(),
        _ => return Err("last_root: invalid (missing state)".to_string()),
    };
    let timestamp = match obj.get("timestamp").and_then(|s| s.as_str()) {
        Some(s) => s.to_string(),
        None => String::new(),
    };
    let boot_mode = obj
        .get("boot_mode")
        .and_then(|s| s.as_str())
        .map(|s| s.to_string());
    Ok(Some(RootRecord {
        state,
        timestamp,
        boot_mode,
    }))
}

/// `Save-RootState` / `save_root_state`: merge `last_root`, keep other keys.
///
/// When `boot_mode` is `None`/empty the previously stored value is kept.
pub fn write_root_state(
    state_file: &Path,
    state: &str,
    boot_mode: Option<&str>,
) -> Result<(), String> {
    let clean = state.trim();
    if clean.is_empty() {
        return Err("root state: empty state".to_string());
    }
    let mut map = match read_object(state_file)? {
        Some(m) => m,
        None => BTreeMap::new(),
    };
    let keep = map
        .get("last_root")
        .and_then(|v| v.as_object())
        .and_then(|o| o.get("boot_mode"))
        .and_then(|s| s.as_str())
        .map(|s| s.to_string());
    let wanted = match boot_mode {
        Some(b) if !b.trim().is_empty() => Some(b.trim().to_string()),
        _ => keep,
    };
    let mut lr = serde_json::Map::new();
    lr.insert(
        "state".to_string(),
        serde_json::Value::String(clean.to_string()),
    );
    lr.insert(
        "timestamp".to_string(),
        serde_json::Value::String(now_stamp()),
    );
    if let Some(b) = wanted {
        if !b.is_empty() {
            lr.insert("boot_mode".to_string(), serde_json::Value::String(b));
        }
    }
    map.insert(
        "last_root".to_string(),
        serde_json::Value::Object(lr),
    );
    map.insert(
        "updated".to_string(),
        serde_json::Value::String(now_stamp()),
    );
    write_object(state_file, &map)
}

// ---------------------------------------------------------- workflow state

/// One step entry (`{id, status, detail}`; `detail` optional in bash files).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StepEntry {
    /// Step id (e.g. `reconnaissance`).
    pub id: String,
    /// Step status (e.g. `pending`, `active`, `done`).
    #[serde(default)]
    pub status: String,
    /// Optional detail (PS1 shape; empty in bash shape).
    #[serde(default)]
    pub detail: String,
}

/// Full `workflow-state.json` view; unknown keys kept in `extra`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkflowFile {
    /// Active goal id (empty = none).
    #[serde(default)]
    pub goal: String,
    /// Last write stamp.
    #[serde(default)]
    pub updated: String,
    /// Per-step records.
    #[serde(default)]
    pub steps: Vec<StepEntry>,
    /// Shared-slot record (passthrough, `gsi-device` shape).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub slot: Option<gsi_device::SlotRecord>,
    /// Last verified root (passthrough).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_root: Option<RootRecord>,
    /// Any other keys (`version`, `started`, `device`, legacy `step`...).
    #[serde(flatten)]
    pub extra: BTreeMap<String, serde_json::Value>,
}

/// `Read-WorkflowState`: missing file -> `None`, corrupt JSON -> `Err`.
pub fn read_workflow_state(path: &Path) -> Result<Option<WorkflowFile>, String> {
    let text = match std::fs::read_to_string(path) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(format!("read: {e}")),
    };
    let wf: WorkflowFile =
        serde_json::from_str(&text).map_err(|e| format!("workflow-state: invalid json: {e}"))?;
    Ok(Some(wf))
}

/// `read_state_goal`: goal field; missing file -> `None`.
pub fn read_state_goal(path: &Path) -> Result<Option<String>, String> {
    match read_workflow_state(path)? {
        Some(wf) => Ok(Some(wf.goal)),
        None => Ok(None),
    }
}

/// `Write-WorkflowState`: write full state, refreshing `updated`.
pub fn write_workflow_state(path: &Path, state: &WorkflowFile) -> Result<(), String> {
    let mut clone = state.clone();
    clone.updated = now_stamp();
    let value =
        serde_json::to_value(&clone).map_err(|e| format!("json: {e}"))?;
    let map: BTreeMap<String, serde_json::Value> =
        serde_json::from_value(value).map_err(|e| format!("json: {e}"))?;
    write_object(path, &map)
}

/// `write_state` (bash): set goal + upsert one step, keep other keys.
pub fn write_step_state(
    path: &Path,
    goal: &str,
    step: &str,
    status: &str,
) -> Result<(), String> {
    let g = goal.trim();
    let s = step.trim();
    if g.is_empty() {
        return Err("workflow-state: empty goal".to_string());
    }
    if s.is_empty() {
        return Err("workflow-state: empty step".to_string());
    }
    let mut wf = match read_workflow_state(path)? {
        Some(w) => w,
        None => WorkflowFile {
            goal: String::new(),
            updated: String::new(),
            steps: Vec::new(),
            slot: None,
            last_root: None,
            extra: BTreeMap::new(),
        },
    };
    wf.goal = g.to_string();
    wf.steps.retain(|e| e.id != s);
    wf.steps.push(StepEntry {
        id: s.to_string(),
        status: status.to_string(),
        detail: String::new(),
    });
    write_workflow_state(path, &wf)
}

// ---------------------------------------------------------- slot passthrough

/// Slot read reusing `gsi-device` (missing/keys -> `unknown`, no side effects).
pub fn read_slot(path: &Path) -> gsi_device::SlotRecord {
    gsi_device::read_slot(path)
}

/// Slot write reusing `gsi-device` (preserves other keys).
pub fn write_slot(
    path: &Path,
    occupant: gsi_device::SlotOccupant,
    detail: &str,
) -> Result<(), String> {
    gsi_device::write_slot(path, occupant, detail)
}

// ------------------------------------------------------------ goal table

/// Normalize a goal id (lowercase + trim, like both script sides).
pub fn normalize_goal(goal: &str) -> String {
    goal.trim().to_lowercase()
}

/// `Get-GoalSteps` / `goal_steps`: goal id -> ordered steps (unknown -> empty).
pub fn goal_steps(goal: &str) -> Vec<String> {
    goal_steps_static(&normalize_goal(goal))
        .iter()
        .map(|s| s.to_string())
        .collect()
}

/// Static goal table (shared source for [`goal_steps`]).
fn goal_steps_static(goal: &str) -> &'static [&'static str] {
    match goal {
        "root" => &[
            "reconnaissance",
            "compatibility",
            "firmware",
            "extract",
            "magisk_patch",
            "backup",
            "flash",
            "reboot",
            "root_verify",
            "validate",
        ],
        "custom_rom" => &[
            "reconnaissance",
            "compatibility",
            "firmware",
            "rom_validation",
            "backup_if_required",
            "flash_system",
            "reboot",
            "validate",
        ],
        "stock_rom" => &[
            "reconnaissance",
            "firmware_selection",
            "firmware_validation",
            "artifact_extraction",
            "backup",
            "flash_plan",
            "safety_gate",
            "flash",
            "reboot",
            "validate",
        ],
        "root_custom_rom" => &[
            "reconnaissance",
            "custom_rom_compatibility",
            "firmware",
            "rom_installation",
            "boot",
            "root_preparation",
            "backup",
            "root_flash",
            "root_verify",
            "validate",
        ],
        "root_stock_rom" => &[
            "reconnaissance",
            "stock_firmware_validation",
            "root_image_preparation",
            "backup",
            "root_flash",
            "reboot",
            "root_verify",
            "validate",
        ],
        "root_custom_rom_recovery" => &[
            "reconnaissance",
            "rom_compatibility",
            "recovery_compatibility",
            "firmware",
            "backup",
            "rom_flash",
            "recovery_flash",
            "root_preparation",
            "root_flash",
            "boot",
            "verify",
            "validate",
        ],
        "root_stock_rom_recovery" => &[
            "reconnaissance",
            "stock_firmware_validation",
            "recovery_validation",
            "backup",
            "recovery_flash",
            "root_preparation",
            "root_flash",
            "boot",
            "verify",
            "validate",
        ],
        "restore_original" => &[
            "reconnaissance",
            "identify_original_artifact",
            "validate_backup",
            "rollback_plan",
            "safety_gate",
            "restore",
            "reboot",
            "validate",
        ],
        "full_reinstall" => &[
            "reconnaissance",
            "compatibility",
            "firmware",
            "rom_validation",
            "backup",
            "wipe",
            "flash_system",
            "reboot",
            "validate",
        ],
        _ => &[],
    }
}

/// All known goal ids.
pub fn goal_ids() -> Vec<String> {
    vec![
        "root",
        "custom_rom",
        "stock_rom",
        "root_custom_rom",
        "root_stock_rom",
        "root_custom_rom_recovery",
        "root_stock_rom_recovery",
        "restore_original",
        "full_reinstall",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect()
}

/// True when the goal id has a step list.
pub fn is_known_goal(goal: &str) -> bool {
    !goal_steps_static(&normalize_goal(goal)).is_empty()
}

/// Gate bucket used by `New-WorkflowPlan` (`flash`/`verify`/`backup`/`analyze`).
pub fn step_gate(step: &str) -> Option<String> {
    let s = step.to_lowercase();
    if s.contains("flash") {
        Some("flash".to_string())
    } else if s.contains("verify") || s.contains("validat") {
        Some("verify".to_string())
    } else if s.contains("backup") {
        Some("backup".to_string())
    } else if s.contains("recon") || s.contains("analy") {
        Some("analyze".to_string())
    } else {
        None
    }
}

// ------------------------------------------------------------------ planner

/// Per-step disposition: plannable (`Ready`) or refused with a reason.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StepDisposition {
    /// Step is listed for the goal (live gates run at execution time).
    Ready,
    /// Step cannot be planned (reason carried).
    Refused(String),
}

/// One planned step.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlannedStep {
    /// Step id.
    pub id: String,
    /// Typed disposition.
    pub disposition: StepDisposition,
}

/// `New-WorkflowPlan`: goal -> ordered typed plan (no execution, no I/O).
pub struct WorkflowPlan {
    /// Normalized goal id.
    pub goal: String,
    /// Ordered steps.
    pub steps: Vec<PlannedStep>,
}

/// Build a plan for a goal; unknown goal -> `Err` (honest, no guessing).
pub fn new_workflow_plan(goal: &str) -> Result<WorkflowPlan, String> {
    let g = normalize_goal(goal);
    let steps = goal_steps_static(&g);
    if steps.is_empty() {
        return Err(format!("unknown goal: {goal}"));
    }
    Ok(WorkflowPlan {
        goal: g,
        steps: steps
            .iter()
            .map(|s| PlannedStep {
                id: s.to_string(),
                disposition: StepDisposition::Ready,
            })
            .collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static N: AtomicUsize = AtomicUsize::new(0);

    fn tmp_dir() -> PathBuf {
        let n = N.fetch_add(1, Ordering::SeqCst);
        let mut p = std::env::temp_dir();
        p.push(format!("gsi-state-test-{}-{}", std::process::id(), n));
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    fn write_text(path: &Path, text: &str) {
        if let Some(p) = path.parent() {
            std::fs::create_dir_all(p).unwrap();
        }
        std::fs::write(path, text).unwrap();
    }

    #[test]
    fn rom_roundtrip() {
        let d = tmp_dir();
        let f = installed_rom_file(&d);
        assert_eq!(f, d.join("data").join("installed-rom.txt"));
        write_installed_rom(&f, "  rom:TrebleDroid  ").unwrap();
        assert_eq!(read_installed_rom(&f).unwrap(), Some("rom:TrebleDroid".to_string()));
        // Raw file content is the bare token (script shape).
        assert_eq!(std::fs::read_to_string(&f).unwrap(), "rom:TrebleDroid");
    }

    #[test]
    fn rom_missing_and_empty_are_none() {
        let d = tmp_dir();
        let missing = d.join("nope").join("installed-rom.txt");
        assert_eq!(read_installed_rom(&missing).unwrap(), None);
        let e = d.join("data").join("installed-rom.txt");
        write_text(&e, "   \r\n  ");
        assert_eq!(read_installed_rom(&e).unwrap(), None);
    }

    #[test]
    fn rom_invalid_is_err() {
        let d = tmp_dir();
        let f = d.join("data").join("installed-rom.txt");
        write_text(&f, "two tokens here");
        assert!(read_installed_rom(&f).is_err());
        assert!(write_installed_rom(&f, "   ").is_err());
        assert!(write_installed_rom(&f, "a b").is_err());
        assert_eq!(validate_rom_id("stock").unwrap(), "stock");
        assert!(validate_rom_id("").is_err());
    }

    #[test]
    fn root_roundtrip_and_boot_mode_kept() {
        let d = tmp_dir();
        let f = workflow_state_file(&d);
        assert_eq!(f, d.join("workflow-state.json"));
        write_root_state(&f, "ROOTED", Some("vol-up")).unwrap();
        let r = read_root_state(&f).unwrap().unwrap();
        assert_eq!(r.state, "ROOTED");
        assert_eq!(r.boot_mode.as_deref(), Some("vol-up"));
        // Empty override keeps the stored boot mode (script behavior).
        write_root_state(&f, "ROOTED+persisted-fixes", None).unwrap();
        let r2 = read_root_state(&f).unwrap().unwrap();
        assert_eq!(r2.state, "ROOTED+persisted-fixes");
        assert_eq!(r2.boot_mode.as_deref(), Some("vol-up"));
    }

    #[test]
    fn root_reads_script_shapes() {
        // PS1 Save-RootState shape.
        let d = tmp_dir();
        let f = d.join("workflow-state.json");
        write_text(
            &f,
            r#"{"version":"1.0","goal":"","steps":[],"last_root":{"state":"ROOTED","timestamp":"2026-10-07 05:00:00","boot_mode":"vol-up"},"updated":"2026-10-07 05:00:00"}"#,
        );
        let r = read_root_state(&f).unwrap().unwrap();
        assert_eq!(r.state, "ROOTED");
        // Bash save_root_state shape (no boot_mode).
        write_text(
            &f,
            r#"{"goal":"","steps":[],"last_root":{"state":"ROOTED","timestamp":"2026-10-07 05:00:00"},"updated":"2026-10-07 05:00:00"}"#,
        );
        let r = read_root_state(&f).unwrap().unwrap();
        assert_eq!(r.boot_mode, None);
    }

    #[test]
    fn root_missing_none_invalid_err() {
        let d = tmp_dir();
        let missing = d.join("absent.json");
        assert_eq!(read_root_state(&missing).unwrap(), None);
        let f = d.join("s.json");
        write_text(&f, "{not json");
        assert!(read_root_state(&f).is_err());
        write_text(&f, r#"{"goal":""}"#);
        assert_eq!(read_root_state(&f).unwrap(), None);
        write_text(&f, r#"{"last_root":{"timestamp":"x"}}"#);
        assert!(read_root_state(&f).is_err());
    }

    #[test]
    fn workflow_goal_and_step_roundtrip() {
        let d = tmp_dir();
        let f = workflow_state_file(&d);
        write_step_state(&f, "root", "reconnaissance", "active").unwrap();
        write_step_state(&f, "root", "reconnaissance", "done").unwrap();
        write_step_state(&f, "root", "compatibility", "done").unwrap();
        assert_eq!(read_state_goal(&f).unwrap(), Some("root".to_string()));
        let wf = read_workflow_state(&f).unwrap().unwrap();
        assert_eq!(wf.goal, "root");
        assert_eq!(wf.steps.len(), 2);
        assert!(!wf.updated.is_empty());
    }

    #[test]
    fn workflow_reads_script_shapes() {
        // Bash write_state shape.
        let d = tmp_dir();
        let f = d.join("w.json");
        write_text(
            &f,
            r#"{"goal":"root","steps":[{"id":"reconnaissance","status":"done"}],"updated":"2026-10-07 05:00:00"}"#,
        );
        let wf = read_workflow_state(&f).unwrap().unwrap();
        assert_eq!(wf.goal, "root");
        assert_eq!(wf.steps[0].id, "reconnaissance");
        // PS1 full shape with slot + last_root + extras preserved.
        write_text(
            &f,
            r#"{"version":"1.0","goal":"root","started":"2026-10-07 05:00:00","device":"","steps":[{"id":"flash","status":"pending","detail":""}],"slot":{"occupant":"twrp","detail":"C:/twrp.img","timestamp":"2026-10-07 05:00:00"},"last_root":{"state":"ROOTED","timestamp":"2026-10-07 05:00:00"},"updated":"2026-10-07 05:00:00"}"#,
        );
        let wf = read_workflow_state(&f).unwrap().unwrap();
        assert_eq!(wf.slot.unwrap().occupant, "twrp");
        assert_eq!(wf.last_root.unwrap().state, "ROOTED");
        assert!(wf.extra.contains_key("version"));
        // Slot write preserves goal/steps.
        write_slot(&f, gsi_device::SlotOccupant::Magisk, "").unwrap();
        let back = std::fs::read_to_string(&f).unwrap();
        assert!(back.contains("\"goal\""));
        assert_eq!(read_slot(&f).occupant, "magisk");
    }

    #[test]
    fn workflow_missing_none_invalid_err() {
        let d = tmp_dir();
        let missing = d.join("absent.json");
        assert_eq!(read_workflow_state(&missing).unwrap(), None);
        assert_eq!(read_state_goal(&missing).unwrap(), None);
        let f = d.join("bad.json");
        write_text(&f, "{oops");
        assert!(read_workflow_state(&f).is_err());
        assert!(read_state_goal(&f).is_err());
    }

    #[test]
    fn goal_table_matches_scripts() {
        assert_eq!(goal_ids().len(), 9);
        assert_eq!(goal_steps("root").len(), 10);
        assert_eq!(goal_steps("ROOT").len(), 10);
        assert_eq!(goal_steps("  custom_rom ").len(), 8);
        assert!(goal_steps("nope").is_empty());
        assert!(is_known_goal("full_reinstall"));
        assert!(!is_known_goal("nope"));
        // Spot-check exact script order for two goals.
        assert_eq!(
            goal_steps("stock_rom"),
            vec![
                "reconnaissance",
                "firmware_selection",
                "firmware_validation",
                "artifact_extraction",
                "backup",
                "flash_plan",
                "safety_gate",
                "flash",
                "reboot",
                "validate"
            ]
            .iter()
            .map(|s| s.to_string())
            .collect::<Vec<_>>()
        );
        assert_eq!(goal_steps("restore_original").len(), 8);
    }

    #[test]
    fn plan_typed_and_honest() {
        let p = new_workflow_plan("root").unwrap();
        assert_eq!(p.goal, "root");
        assert_eq!(p.steps.len(), goal_steps("root").len());
        assert!(p.steps.iter().all(|s| s.disposition == StepDisposition::Ready));
        assert!(new_workflow_plan("unknown-goal").is_err());
        // Gate buckets mirror New-WorkflowPlan mapping.
        assert_eq!(step_gate("root_flash"), Some("flash".to_string()));
        assert_eq!(step_gate("root_verify"), Some("verify".to_string()));
        assert_eq!(step_gate("backup_if_required"), Some("backup".to_string()));
        assert_eq!(step_gate("reconnaissance"), Some("analyze".to_string()));
        assert_eq!(step_gate("wipe"), None);
    }

    #[test]
    fn full_state_roundtrip() {
        let d = tmp_dir();
        let f = workflow_state_file(&d);
        let wf = WorkflowFile {
            goal: "root".to_string(),
            updated: String::new(),
            steps: vec![StepEntry {
                id: "reconnaissance".to_string(),
                status: "done".to_string(),
                detail: String::new(),
            }],
            slot: Some(gsi_device::SlotRecord {
                occupant: "magisk".to_string(),
                detail: String::new(),
                timestamp: String::new(),
            }),
            last_root: Some(RootRecord {
                state: "ROOTED".to_string(),
                timestamp: String::new(),
                boot_mode: None,
            }),
            extra: BTreeMap::new(),
        };
        write_workflow_state(&f, &wf).unwrap();
        let back = read_workflow_state(&f).unwrap().unwrap();
        assert_eq!(back.goal, "root");
        assert_eq!(back.slot.unwrap().occupant, "magisk");
    }
}
