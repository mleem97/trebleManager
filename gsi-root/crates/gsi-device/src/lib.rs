//! Device layer: `recovery_ramdisk` slot tracking + switch planning.
//!
//! TWRP and Magisk share one slot (mutual overwrite — hardware fact).
//! This crate owns the state (same `workflow-state.json` shape the scripts
//! write: `{"slot":{"occupant":..,"detail":..,"timestamp":..}}`, other keys
//! preserved) and the switch PLAN. Flashing itself needs the native
//! fastboot layer (Phase 8) and is refused here.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

pub mod parse;

/// Who last occupied the shared recovery_ramdisk slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlotOccupant {
    Magisk,
    Twrp,
    Stock,
    Unknown,
}

impl SlotOccupant {
    /// Parse stored value (anything else → Unknown, never guessed).
    pub fn parse(s: &str) -> Self {
        match s.trim().to_lowercase().as_str() {
            "magisk" => Self::Magisk,
            "twrp" => Self::Twrp,
            "stock" => Self::Stock,
            _ => Self::Unknown,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Magisk => "magisk",
            Self::Twrp => "twrp",
            Self::Stock => "stock",
            Self::Unknown => "unknown",
        }
    }
}

/// Slot record as stored (timestamp informational only).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SlotRecord {
    pub occupant: String,
    #[serde(default)]
    pub detail: String,
    #[serde(default)]
    pub timestamp: String,
}

/// Read the slot record from a state file. Missing file/keys → Unknown
/// (never guessed, never created as a side effect).
pub fn read_slot(state_file: &Path) -> SlotRecord {
    let empty = || SlotRecord {
        occupant: "unknown".to_string(),
        detail: String::new(),
        timestamp: String::new(),
    };
    let text = match std::fs::read_to_string(state_file) {
        Ok(t) => t,
        Err(_) => return empty(),
    };
    let v: serde_json::Value = match serde_json::from_str(&text) {
        Ok(v) => v,
        Err(_) => return empty(),
    };
    let s = &v["slot"];
    SlotRecord {
        occupant: s["occupant"].as_str().unwrap_or("unknown").to_string(),
        detail: s["detail"].as_str().unwrap_or("").to_string(),
        timestamp: s["timestamp"].as_str().unwrap_or("").to_string(),
    }
}

/// Write the slot record, preserving every other key in the state file.
pub fn write_slot(state_file: &Path, occupant: SlotOccupant, detail: &str) -> Result<(), String> {
    let mut map: BTreeMap<String, serde_json::Value> = match std::fs::read_to_string(state_file) {
        Ok(t) => serde_json::from_str(&t).unwrap_or_default(),
        Err(_) => BTreeMap::new(),
    };
    map.insert(
        "slot".to_string(),
        serde_json::json!({
            "occupant": occupant.as_str(),
            "detail": detail,
            "timestamp": "",
        }),
    );
    if let Some(parent) = state_file.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("mkdir: {e}"))?;
    }
    let text = serde_json::to_string_pretty(&map).map_err(|e| format!("json: {e}"))?;
    std::fs::write(state_file, text).map_err(|e| format!("write: {e}"))?;
    Ok(())
}

/// What the switch planner recommends.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SwitchAction {
    /// Already there — nothing to do.
    AlreadyThere,
    /// Flash `image` to take the slot (always overwrites occupant).
    Flash { image: String },
    /// Cannot plan: state which input is missing.
    NeedInput(String),
}

/// Plan a one-tap slot switch.
///
/// * `current` — recorded occupant (unknown = caution, still plannable).
/// * `magisk_image` — registered patched image, if any.
/// * `twrp_image` — known TWRP image, if any.
///
/// The shared-slot overwrite warning is part of every Flash plan.
#[derive(Debug, Clone)]
pub struct SwitchPlan {
    pub action: SwitchAction,
    pub warnings: Vec<String>,
    pub target: SlotOccupant,
}

pub fn plan_switch(
    current: SlotOccupant,
    target: SlotOccupant,
    magisk_image: Option<&str>,
    twrp_image: Option<&str>,
) -> SwitchPlan {
    let mut warnings =
        vec!["TWRP and Magisk share the recovery_ramdisk slot (mutual overwrite).".to_string()];
    if current == SlotOccupant::Unknown {
        warnings.push(
            "current occupant unknown (fresh/trustworthy only after a tool flash) — verify before flashing.".to_string(),
        );
    }
    if current == target {
        return SwitchPlan {
            action: SwitchAction::AlreadyThere,
            warnings: vec![],
            target,
        };
    }
    match target {
        SlotOccupant::Magisk => match magisk_image {
            Some(p) if !p.is_empty() => SwitchPlan {
                action: SwitchAction::Flash {
                    image: p.to_string(),
                },
                warnings,
                target,
            },
            _ => SwitchPlan {
                action: SwitchAction::NeedInput(
                    "no registered Magisk patched image (patch first)".to_string(),
                ),
                warnings: vec![],
                target,
            },
        },
        SlotOccupant::Twrp => match twrp_image {
            Some(p) if !p.is_empty() => SwitchPlan {
                action: SwitchAction::Flash {
                    image: p.to_string(),
                },
                warnings,
                target,
            },
            _ => SwitchPlan {
                action: SwitchAction::NeedInput(
                    "no known TWRP image (provide exact model build)".to_string(),
                ),
                warnings: vec![],
                target,
            },
        },
        SlotOccupant::Stock => SwitchPlan {
            action: SwitchAction::NeedInput(
                "restore flow owns stock returns (backup-gated)".to_string(),
            ),
            warnings: vec![],
            target,
        },
        SlotOccupant::Unknown => SwitchPlan {
            action: SwitchAction::NeedInput("no target selected".to_string()),
            warnings: vec![],
            target,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn tmp_state(content: &str) -> std::path::PathBuf {
        static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let n = N.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let mut p = std::env::temp_dir();
        p.push(format!("gsi-device-test-{n}.json"));
        let mut f = std::fs::File::create(&p).unwrap();
        f.write_all(content.as_bytes()).unwrap();
        p
    }

    #[test]
    fn occupant_parsing_never_guesses() {
        assert_eq!(SlotOccupant::parse("magisk"), SlotOccupant::Magisk);
        assert_eq!(SlotOccupant::parse("TWRP"), SlotOccupant::Twrp);
        assert_eq!(SlotOccupant::parse("stock"), SlotOccupant::Stock);
        assert_eq!(SlotOccupant::parse(""), SlotOccupant::Unknown);
        assert_eq!(SlotOccupant::parse("cwm"), SlotOccupant::Unknown);
    }

    #[test]
    fn reads_script_written_state() {
        // Exact shape the PS1/bash scripts write (incl. unrelated keys).
        let p = tmp_state(
            r#"{"goal":"root","last_root":{"state":"ROOTED"},"slot":{"occupant":"twrp","detail":"C:/twrp.img","timestamp":"2026-10-07 05:00:00"}}"#,
        );
        let r = read_slot(&p);
        assert_eq!(SlotOccupant::parse(&r.occupant), SlotOccupant::Twrp);
        assert_eq!(r.detail, "C:/twrp.img");
        std::fs::remove_file(p).ok();
    }

    #[test]
    fn missing_file_is_unknown() {
        let mut p = std::env::temp_dir();
        p.push("gsi-device-test-absent.json");
        std::fs::remove_file(&p).ok();
        assert_eq!(read_slot(&p).occupant, "unknown");
    }

    #[test]
    fn write_preserves_other_keys() {
        let p = tmp_state(r#"{"goal":"root","steps":[]}"#);
        write_slot(&p, SlotOccupant::Magisk, "").unwrap();
        let back = std::fs::read_to_string(&p).unwrap();
        assert!(back.contains("\"goal\""));
        let r = read_slot(&p);
        assert_eq!(r.occupant, "magisk");
        std::fs::remove_file(p).ok();
    }

    #[test]
    fn switch_matrix() {
        // same -> noop
        let pl = plan_switch(
            SlotOccupant::Magisk,
            SlotOccupant::Magisk,
            Some("m"),
            Some("t"),
        );
        assert_eq!(pl.action, SwitchAction::AlreadyThere);
        // twrp -> magisk with image: flash + shared-slot warning
        let pl = plan_switch(
            SlotOccupant::Twrp,
            SlotOccupant::Magisk,
            Some("m.img"),
            None,
        );
        assert_eq!(
            pl.action,
            SwitchAction::Flash {
                image: "m.img".to_string()
            }
        );
        assert!(pl.warnings.iter().any(|w| w.contains("share")));
        // magisk -> twrp without image: honest NeedInput
        let pl = plan_switch(SlotOccupant::Magisk, SlotOccupant::Twrp, Some("m"), None);
        assert!(matches!(pl.action, SwitchAction::NeedInput(_)));
        // unknown current still plans, with caution
        let pl = plan_switch(SlotOccupant::Unknown, SlotOccupant::Magisk, Some("m"), None);
        assert!(matches!(pl.action, SwitchAction::Flash { .. }));
        assert!(pl.warnings.iter().any(|w| w.contains("unknown")));
    }
}
