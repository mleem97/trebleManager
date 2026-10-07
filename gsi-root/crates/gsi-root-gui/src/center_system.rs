//! System GUI center renders (read-only, never executing).
//!
//! Covers the System center cluster: `ToolsCenter` (managed external
//! tools), `UpdatesCenter` (current/latest/status plus Check contract),
//! `LogsCenter` (level filter + search + export/clear notes + anonymize
//! toggle display), `SettingsCenter` (language/paths/tools/safety groups
//! plus values) and `HelpCenter` (quickstart/troubleshooting/about with a
//! version string param).
//!
//! Reuse, never duplicate:
//! - tool rows render caller-measured `gsi-tool::locate` +
//!   `gsi-tool::probe_version` values (same sources as the `tools` page);
//!   lookup and version probing stay shell-owned.
//! - update checks stay behind `gsi-update::latest_tag` (network) with
//!   `gsi-update::install_staged` for atomic install + rollback; the Check
//!   button contract below only describes that shell call.
//! - log file rows render caller-measured `gsi-diag::list_log_files`
//!   values (names + sizes, never content); redaction stays behind
//!   `gsi-diag::redact`.
//! - settings paths resolve via `gsi-config` (`config_dir`, `cache_dir`,
//!   data/log dirs); this module only renders the passed-in values.
//! - the about body delegates to `crate::pages_flows::help_text`.
//!
//! Slint side lives in `ui/center_system.slint` (`ToolsCenter`,
//! `UpdatesCenter`, `LogsCenter`, `SettingsCenter`, `HelpCenter`) with the
//! same visual pattern as the Detect page (baked title, note, Refresh
//! button, read-only log view) and the component contract `in-out
//! property <string> result; callback refresh() -> string;` plus an
//! `append-log` passthrough. `UpdatesCenter` additionally carries the
//! `check-update(string) -> string` contract; `LogsCenter` additionally
//! carries filter display properties the shell reads on refresh. No
//! device, no network, no file access, no subprocess, no `unwrap` /
//! `expect` / `panic` outside tests.

/// Managed-tool table.
///
/// `entries` are `(name, path, version, source, status)` tuples. Empty
/// fields render honestly (`unknown` path, `MISSING` status note); an
/// empty list renders the honest empty form. Locate
/// (`gsi-tool::locate`) and version probing (`gsi-tool::probe_version`)
/// stay shell-owned; this renders the measured values plus the
/// locate/refresh notes.
pub fn tools_center_text(entries: &[(String, String, String, String, String)]) -> String {
    let mut out = String::new();
    out.push_str("Tools center (read-only, nothing executed)\n");
    if entries.is_empty() {
        out.push_str("no tool entries recorded\n");
    } else {
        out.push_str("tools:\n");
        for (name, path, version, source, status) in entries {
            let n = name.trim();
            if n.is_empty() {
                continue;
            }
            out.push_str(&format!(" - {n}\n"));
            if path.trim().is_empty() {
                out.push_str("   path: unknown\n");
            } else {
                out.push_str(&format!("   path: {}\n", path.trim()));
            }
            if version.trim().is_empty() {
                out.push_str("   version: unknown\n");
            } else {
                out.push_str(&format!("   version: {}\n", version.trim()));
            }
            if !source.trim().is_empty() {
                out.push_str(&format!("   source: {}\n", source.trim()));
            }
            if status.trim().is_empty() {
                out.push_str("   status: MISSING\n");
            } else {
                out.push_str(&format!("   status: {}\n", status.trim()));
            }
        }
    }
    out.push_str("locate note: PATH plus tools/platform-tools, data/tools, platform-tools, tools (gsi-tool::locate, shell-owned)\n");
    out.push_str("refresh note: version probe when present (gsi-tool::probe_version); missing tools render MISSING, never faked\n");
    out
}

/// Settings groups (language/paths/tools/safety) plus values.
///
/// All values are caller-provided (resolved via `gsi-config` in the
/// shell); empty values render as `unknown`. Display only; nothing is
/// written here.
#[allow(clippy::too_many_arguments)] // explicit inputs by design: no hidden state, all caller-measured and tested
pub fn settings_center_text(
    language: &str,
    config_dir: &str,
    cache_dir: &str,
    data_dir: &str,
    log_dir: &str,
    backups_dir: &str,
    tools: &[(String, String)],
    safety: &str,
) -> String {
    let mut out = String::new();
    out.push_str("Settings center (read-only display, nothing written)\n");
    out.push_str("language:\n");
    if language.trim().is_empty() {
        out.push_str(" value: unknown\n");
    } else {
        out.push_str(&format!(" value: {}\n", language.trim()));
    }
    out.push_str("paths:\n");
    let paths = [
        ("config", config_dir),
        ("cache", cache_dir),
        ("data", data_dir),
        ("logs", log_dir),
        ("backups", backups_dir),
    ];
    for (label, value) in paths {
        if value.trim().is_empty() {
            out.push_str(&format!(" - {label}: unknown\n"));
        } else {
            out.push_str(&format!(" - {label}: {}\n", value.trim()));
        }
    }
    out.push_str("tools:\n");
    if tools.is_empty() {
        out.push_str(" (no tool overrides)\n");
    } else {
        for (name, path) in tools {
            let n = name.trim();
            if n.is_empty() {
                continue;
            }
            if path.trim().is_empty() {
                out.push_str(&format!(" - {n}: unknown\n"));
            } else {
                out.push_str(&format!(" - {n}: {}\n", path.trim()));
            }
        }
    }
    out.push_str("safety:\n");
    if safety.trim().is_empty() {
        out.push_str(" (no safety notes)\n");
    } else {
        out.push_str(&format!(" {}\n", safety.trim()));
    }
    out.push_str("note: config, cache, downloads, artifacts, logs and backups resolve to platform-correct user directories (override via environment)\n");
    out
}

/// Quickstart, troubleshooting, and about (with version string param).
///
/// The about body delegates to `crate::pages_flows::help_text(version)`.
/// Read-only; the GUI never runs these commands itself.
pub fn help_center_text(version: &str) -> String {
    let mut out = String::new();
    out.push_str("Help center (read-only)\n");
    out.push_str("quickstart:\n");
    out.push_str(" 1. Detect: confirm adb/fastboot plus the attached device\n");
    out.push_str(" 2. Analyze Device: read the OS class and recovery_ramdisk state\n");
    out.push_str(" 3. Backup: back up the base before any flash\n");
    out.push_str(
        " 4. Flash: plan preview only; live flash needs a device plus double confirmation\n",
    );
    out.push_str("troubleshooting:\n");
    out.push_str(" - 'Command not allowed' = Huawei refused (retry, cable, TROUBLESHOOTING)\n");
    out.push_str(" - 'too large' = image bigger than partition\n");
    out.push_str(" - no device: check cable, drivers, and that adb/fastboot are on PATH\n");
    out.push_str("about:\n");
    out.push_str(&crate::pages_flows::help_text(version));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tools_table_rows_and_notes() {
        let rows = vec![
            (
                "adb".to_string(),
                "/usr/bin/adb".to_string(),
                "1.0.41".to_string(),
                "PATH".to_string(),
                "ready".to_string(),
            ),
            (
                "fastboot".to_string(),
                "".to_string(),
                "".to_string(),
                "".to_string(),
                "".to_string(),
            ),
        ];
        let t = tools_center_text(&rows);
        assert!(t.contains("Tools center"));
        assert!(t.contains(" - adb"));
        assert!(t.contains("path: /usr/bin/adb"));
        assert!(t.contains("version: 1.0.41"));
        assert!(t.contains("source: PATH"));
        assert!(t.contains("status: ready"));
        assert!(t.contains("path: unknown"));
        assert!(t.contains("status: MISSING"));
        assert!(t.contains("gsi-tool::locate"));
        assert!(t.contains("never faked"));
    }

    #[test]
    fn tools_empty_is_honest() {
        let t = tools_center_text(&[]);
        assert!(t.contains("no tool entries recorded"));
        assert!(t.contains("gsi-tool::probe_version"));
    }

    #[test]
    fn settings_groups_and_values() {
        let tools = vec![("adb".to_string(), "/usr/bin/adb".to_string())];
        let t = settings_center_text(
            "en",
            "/home/user/.config/treble",
            "/home/user/.cache/treble",
            "/tool/data",
            "/tool/logs",
            "/tool/data/backups",
            &tools,
            "double confirmation before any destructive step",
        );
        assert!(t.contains("Settings center"));
        assert!(t.contains("value: en"));
        assert!(t.contains(" - config: /home/user/.config/treble"));
        assert!(t.contains(" - backups: /tool/data/backups"));
        assert!(t.contains(" - adb: /usr/bin/adb"));
        assert!(t.contains("double confirmation"));
        assert!(t.contains("platform-correct user directories"));
    }

    #[test]
    fn settings_empty_is_unknown() {
        let t = settings_center_text("", "", "", "", "", "", &[], "");
        assert!(t.contains("value: unknown"));
        assert!(t.contains(" - cache: unknown"));
        assert!(t.contains("(no tool overrides)"));
        assert!(t.contains("(no safety notes)"));
    }

    #[test]
    fn help_has_quickstart_troubleshooting_about() {
        let t = help_center_text("0.5.0");
        assert!(t.contains("Help center"));
        assert!(t.contains("quickstart:"));
        assert!(t.contains("back up the base before any flash"));
        assert!(t.contains("troubleshooting:"));
        assert!(t.contains("Command not allowed"));
        assert!(t.contains("about:"));
        assert!(t.contains("Huawei P10 Root Manager v0.5.0"));
    }

    #[test]
    fn help_empty_version_omits_number() {
        let t = help_center_text("");
        assert!(t.contains("Huawei P10 Root Manager"));
        assert!(!t.contains(" v\n"));
    }
}
