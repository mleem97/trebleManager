//! Device GUI center renders (read-only, never executing).
//!
//! Covers the Device center cluster: `DeviceCenter` (overview card),
//! `DetectCenter` (device states + multi-device select list) and
//! `DiagCenter` (diagnostics table).
//!
//! Reuse, never duplicate:
//! - overview inputs are caller-measured via `gsi-device::parse`
//!   (`adb_devices`, `fastboot_devices`, `getprop`, `byname`, `getvar`);
//!   serials render masked here, never raw.
//! - detect states mirror the `detect_text` shape in the crate root
//!   (managed `gsi-tool::locate` + `run` for `adb`/`fastboot`, parsed with
//!   `gsi-device::parse`); this module only renders the caller-passed
//!   snapshot, it never spawns a process.
//! - diagnostics rows are caller-measured via `gsi-diag` planning;
//!   maturity renders as `VERIFIED` / `DOCUMENTED` / `EXPERIMENTAL` /
//!   `UNSUPPORTED` only, never as `Ready`.
//! - future live wiring (shell-owned): `live` context for saved system and
//!   tool presence; transcripts stay empty without a live device.
//!
//! Slint side lives in `ui/center_device.slint` (`DeviceCenter`,
//! `DetectCenter`, `DiagCenter`) with the same visual pattern as the Detect
//! page (baked title, note, Refresh button, read-only log view) and the
//! component contract `in-out property <string> result; callback refresh() ->
//! string;` plus an `append-log` passthrough. No device, no network, no
//! file access, no `unwrap` / `expect` / `panic` outside tests.

/// Overview card inputs (all explicit, caller-measured).
///
/// Empty strings render as `unknown`; `serial` renders masked via
/// [`mask_serial`]. `adb_state` / `fastboot_state` are the current transport
/// states shown for context (for example `device`, `fastboot`, `unknown`).
#[derive(Debug, Clone, Default)]
pub struct DeviceOverview {
    pub vendor: String,
    pub model: String,
    pub variant: String,
    pub serial: String,
    pub android: String,
    pub build: String,
    pub abi: String,
    pub arch: String,
    pub boot_mode: String,
    pub adb_state: String,
    pub fastboot_state: String,
}

/// Mask a device serial for display.
///
/// Empty renders as `unknown`; 4 chars or fewer render as `****`;
/// otherwise the first 4 and last 4 chars show with `****` between
/// (for example `6PQ0217B08003446` becomes `6PQ0****3446`).
/// Char-safe, never panics, never leaks the full value.
pub fn mask_serial(serial: &str) -> String {
    let t = serial.trim();
    if t.is_empty() {
        return "unknown".to_string();
    }
    let chars: Vec<char> = t.chars().collect();
    if chars.len() <= 4 {
        return "****".to_string();
    }
    if chars.len() <= 8 {
        let head: String = chars.iter().take(2).copied().collect();
        let mut tail_rev: Vec<char> = chars.iter().rev().take(2).copied().collect();
        tail_rev.reverse();
        let tail: String = tail_rev.iter().collect();
        let mut out = String::new();
        out.push_str(&head);
        out.push_str("****");
        out.push_str(&tail);
        return out;
    }
    let head: String = chars.iter().take(4).copied().collect();
    let mut tail_rev: Vec<char> = chars.iter().rev().take(4).copied().collect();
    tail_rev.reverse();
    let tail: String = tail_rev.iter().collect();
    let mut out = String::new();
    out.push_str(&head);
    out.push_str("****");
    out.push_str(&tail);
    out
}

fn line_or_unknown(out: &mut String, label: &str, value: &str) {
    let v = value.trim();
    if v.is_empty() {
        out.push_str(&format!("{label}: unknown\n"));
    } else {
        out.push_str(&format!("{label}: {v}\n"));
    }
}

/// Device overview card render.
///
/// Mirrors `Show-TTStatus` / `status_screen` fields plus the serial-mask
/// rule. All inputs explicit; missing values render as `unknown`, never
/// guessed. Read-only; live `adb` queries stay shell-owned (Phase 8).
pub fn device_center_text(info: &DeviceOverview) -> String {
    let mut out = String::new();
    out.push_str("Device center (read-only, nothing executed)\n");
    line_or_unknown(&mut out, "vendor", &info.vendor);
    line_or_unknown(&mut out, "model", &info.model);
    line_or_unknown(&mut out, "variant", &info.variant);
    out.push_str(&format!("serial: {}\n", mask_serial(&info.serial)));
    line_or_unknown(&mut out, "android", &info.android);
    line_or_unknown(&mut out, "build", &info.build);
    line_or_unknown(&mut out, "abi", &info.abi);
    line_or_unknown(&mut out, "arch", &info.arch);
    line_or_unknown(&mut out, "boot mode", &info.boot_mode);
    line_or_unknown(&mut out, "adb state", &info.adb_state);
    line_or_unknown(&mut out, "fastboot state", &info.fastboot_state);
    out.push_str("note: serials are masked; full values never shown here\n");
    out.push_str(
        "refused: live property reads need a device; this view only renders injected values\n",
    );
    out
}

/// One transport entry for the detect snapshot.
#[derive(Debug, Clone, Default)]
pub struct DetectEntry {
    pub serial: String,
    pub state: String,
}

fn detect_state_label(
    adb: &[DetectEntry],
    fastboot: &[DetectEntry],
    adb_present: bool,
    fastboot_present: bool,
) -> &'static str {
    if adb.is_empty() && fastboot.is_empty() {
        if !adb_present && !fastboot_present {
            return "Unknown";
        }
        return "NoDevice";
    }
    if adb.len() > 1 {
        return "ADB-multiple";
    }
    if fastboot.len() > 1 {
        return "Fastboot-multiple";
    }
    if adb.len() == 1 && fastboot.is_empty() {
        let st = adb[0].state.trim().to_ascii_lowercase();
        if st == "device" {
            return "ADB";
        }
        if st == "unauthorized" {
            return "ADB-unauthorized";
        }
        return "Unknown";
    }
    if fastboot.len() == 1 && adb.is_empty() {
        let st = fastboot[0].state.trim().to_ascii_lowercase();
        if st == "fastboot" || st == "device" {
            return "Fastboot";
        }
        return "Unknown";
    }
    "Unknown"
}

/// Detect center render over an explicit transport snapshot.
///
/// `adb` / `fastboot` are caller-parsed via
/// `gsi-device::parse::adb_devices` / `fastboot_devices`;
/// `adb_present` / `fastboot_present` are caller-measured via
/// `gsi-tool::locate` (tool found or not). `selected_serial` is the raw
/// serial currently selected in the UI (empty when none); matching is by
/// raw value, display is always masked. Empty snapshot renders the honest
/// `NoDevice` (tools present) or `Unknown` (no tools) form.
pub fn detect_center_text(
    adb: &[DetectEntry],
    fastboot: &[DetectEntry],
    adb_present: bool,
    fastboot_present: bool,
    selected_serial: &str,
) -> String {
    let mut out = String::new();
    out.push_str("Detect center (read-only queries, nothing executed)\n");
    let state = detect_state_label(adb, fastboot, adb_present, fastboot_present);
    out.push_str(&format!("state: {state}\n"));
    if adb_present {
        out.push_str("adb tool: present\n");
    } else {
        out.push_str("adb tool: MISSING (native protocol: Phase 8)\n");
    }
    if fastboot_present {
        out.push_str("fastboot tool: present\n");
    } else {
        out.push_str("fastboot tool: MISSING (native protocol: Phase 8)\n");
    }
    let sel = selected_serial.trim();
    if adb.is_empty() && fastboot.is_empty() {
        if state == "Unknown" {
            out.push_str("devices: none listed (no tools to query)\n");
        } else {
            out.push_str("devices: none (or no tools)\n");
        }
        out.push_str("hint: connect a device, authorize the RSA prompt, or reboot to fastboot\n");
        return out;
    }
    out.push_str("devices (masked serials, select one):\n");
    for d in adb.iter() {
        let masked = mask_serial(&d.serial);
        let st = if d.state.trim().is_empty() {
            "unknown"
        } else {
            d.state.trim()
        };
        let mark = if !sel.is_empty() && d.serial.trim() == sel {
            " <-- selected"
        } else {
            ""
        };
        out.push_str(&format!(" [adb] {masked} {st}{mark}\n"));
    }
    for d in fastboot.iter() {
        let masked = mask_serial(&d.serial);
        let st = if d.state.trim().is_empty() {
            "fastboot"
        } else {
            d.state.trim()
        };
        let mark = if !sel.is_empty() && d.serial.trim() == sel {
            " <-- selected"
        } else {
            ""
        };
        out.push_str(&format!(" [fastboot] {masked} {st}{mark}\n"));
    }
    if state == "ADB-unauthorized" {
        out.push_str("action: authorize the on-phone RSA prompt, then Refresh\n");
    } else if state == "ADB-multiple" || state == "Fastboot-multiple" {
        out.push_str("action: pick one serial above; commands run against the selection only\n");
    } else if state == "Unknown" {
        out.push_str(
            "action: mixed or unrecognized transport; check cable, mode, and authorization\n",
        );
    }
    out
}

/// One diagnostics table row (caller-measured via `gsi-diag` planning).
#[derive(Debug, Clone, Default)]
pub struct DiagRow {
    pub area: String,
    pub status: String,
    pub detail: String,
    pub maturity: String,
}

fn maturity_badge(raw: &str) -> &'static str {
    let t = raw.trim().to_ascii_uppercase();
    if t == "VERIFIED" {
        "VERIFIED"
    } else if t == "DOCUMENTED" {
        "DOCUMENTED"
    } else if t == "EXPERIMENTAL" {
        "EXPERIMENTAL"
    } else {
        "UNSUPPORTED"
    }
}

/// Diagnostics table render.
///
/// Each row shows `area | status | maturity | detail`. Maturity renders only
/// as `VERIFIED` / `DOCUMENTED` / `EXPERIMENTAL` / `UNSUPPORTED`, never as
/// `Ready`. Empty input renders the honest empty form. Read-only.
pub fn diag_center_text(rows: &[DiagRow]) -> String {
    let mut out = String::new();
    out.push_str("Diagnostics center (read-only, nothing executed)\n");
    if rows.is_empty() {
        out.push_str("no diagnostics collected yet (run Refresh after wiring live values)\n");
        return out;
    }
    out.push_str("area | status | maturity | detail:\n");
    for r in rows {
        let area = if r.area.trim().is_empty() {
            "(unnamed)"
        } else {
            r.area.trim()
        };
        let status = if r.status.trim().is_empty() {
            "unknown"
        } else {
            r.status.trim()
        };
        let detail = if r.detail.trim().is_empty() {
            "-"
        } else {
            r.detail.trim()
        };
        out.push_str(&format!(
            " - {area} | {status} | {} | {detail}\n",
            maturity_badge(&r.maturity)
        ));
    }
    out.push_str("maturity: VERIFIED = proven on hardware; DOCUMENTED = described, not proven here; EXPERIMENTAL = outcome unknown; UNSUPPORTED = out of scope\n");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn overview() -> DeviceOverview {
        DeviceOverview {
            vendor: "Huawei".to_string(),
            model: "VTR-L29".to_string(),
            variant: "Global market (UFS storage)".to_string(),
            serial: "6PQ0217B08003446".to_string(),
            android: "9".to_string(),
            build: "VTR-L29 9.1.0.297(C432E5R1P9)".to_string(),
            abi: "arm64-v8a".to_string(),
            arch: "arm64".to_string(),
            boot_mode: "android".to_string(),
            adb_state: "device".to_string(),
            fastboot_state: "unknown".to_string(),
        }
    }

    fn entry(serial: &str, state: &str) -> DetectEntry {
        DetectEntry {
            serial: serial.to_string(),
            state: state.to_string(),
        }
    }

    fn row(area: &str, status: &str, detail: &str, maturity: &str) -> DiagRow {
        DiagRow {
            area: area.to_string(),
            status: status.to_string(),
            detail: detail.to_string(),
            maturity: maturity.to_string(),
        }
    }

    #[test]
    fn mask_hides_full_serial() {
        assert_eq!(mask_serial("6PQ0217B08003446"), "6PQ0****3446");
        assert_eq!(mask_serial(""), "unknown");
        assert_eq!(mask_serial("  "), "unknown");
        assert_eq!(mask_serial("AB"), "****");
        let short = mask_serial("ABCDEFGH");
        assert!(short.contains("****"));
        assert!(!short.contains("ABCDEFGH"));
    }

    #[test]
    fn overview_renders_card_with_masked_serial() {
        let t = device_center_text(&overview());
        assert!(t.contains("Device center"));
        assert!(t.contains("vendor: Huawei"));
        assert!(t.contains("model: VTR-L29"));
        assert!(t.contains("serial: 6PQ0****3446"));
        assert!(!t.contains("6PQ0217B08003446"));
        assert!(t.contains("android: 9"));
        assert!(t.contains("abi: arm64-v8a"));
        assert!(t.contains("arch: arm64"));
        assert!(t.contains("boot mode: android"));
        assert!(t.contains("refused: live property reads"));
    }

    #[test]
    fn overview_empty_is_unknown() {
        let t = device_center_text(&DeviceOverview::default());
        assert!(t.contains("vendor: unknown"));
        assert!(t.contains("model: unknown"));
        assert!(t.contains("serial: unknown"));
        assert!(t.contains("android: unknown"));
    }

    #[test]
    fn detect_no_device_with_tools() {
        let t = detect_center_text(&[], &[], true, true, "");
        assert!(t.contains("state: NoDevice"));
        assert!(t.contains("devices: none (or no tools)"));
        assert!(t.contains("adb tool: present"));
    }

    #[test]
    fn detect_unknown_without_tools() {
        let t = detect_center_text(&[], &[], false, false, "");
        assert!(t.contains("state: Unknown"));
        assert!(t.contains("no tools to query"));
        assert!(t.contains("MISSING"));
    }

    #[test]
    fn detect_single_adb_device() {
        let adb = vec![entry("6PQ0217B08003446", "device")];
        let t = detect_center_text(&adb, &[], true, false, "6PQ0217B08003446");
        assert!(t.contains("state: ADB"));
        assert!(t.contains("[adb] 6PQ0****3446 device <-- selected"));
        assert!(!t.contains("6PQ0217B08003446"));
    }

    #[test]
    fn detect_unauthorized_state() {
        let adb = vec![entry("ABC123", "unauthorized")];
        let t = detect_center_text(&adb, &[], true, true, "");
        assert!(t.contains("state: ADB-unauthorized"));
        assert!(t.contains("RSA prompt"));
    }

    #[test]
    fn detect_multiple_adb_lists_all_masked() {
        let adb = vec![entry("AAA111", "device"), entry("BBB222", "device")];
        let t = detect_center_text(&adb, &[], true, true, "BBB222");
        assert!(t.contains("state: ADB-multiple"));
        assert!(t.contains("pick one serial"));
        assert!(t.contains("<-- selected"));
    }

    #[test]
    fn detect_single_fastboot() {
        let fb = vec![entry("6PQ0217B08003446", "fastboot")];
        let t = detect_center_text(&[], &fb, false, true, "");
        assert!(t.contains("state: Fastboot"));
        assert!(t.contains("[fastboot] 6PQ0****3446"));
    }

    #[test]
    fn detect_multiple_fastboot() {
        let fb = vec![entry("A1", "fastboot"), entry("B2", "fastboot")];
        let t = detect_center_text(&[], &fb, true, true, "");
        assert!(t.contains("state: Fastboot-multiple"));
    }

    #[test]
    fn detect_mixed_is_unknown() {
        let adb = vec![entry("A1", "device")];
        let fb = vec![entry("B2", "fastboot")];
        let t = detect_center_text(&adb, &fb, true, true, "");
        assert!(t.contains("state: Unknown"));
        assert!(t.contains("mixed or unrecognized"));
    }

    #[test]
    fn diag_table_renders_badges_never_ready() {
        let rows = vec![
            row("adb", "pass", "device seen", "verified"),
            row("flash", "blocked", "needs fastboot", "experimental"),
            row("docs", "info", "wiki only", "documented"),
            row("x", "n/a", "", "ready"),
        ];
        let t = diag_center_text(&rows);
        assert!(t.contains("Diagnostics center"));
        assert!(t.contains("adb | pass | VERIFIED"));
        assert!(t.contains("flash | blocked | EXPERIMENTAL"));
        assert!(t.contains("DOCUMENTED"));
        assert!(t.contains("UNSUPPORTED"));
        assert!(!t.contains("Ready"));
        assert!(!t.contains("READY"));
    }

    #[test]
    fn diag_empty_is_honest() {
        let t = diag_center_text(&[]);
        assert!(t.contains("no diagnostics collected yet"));
    }
}
