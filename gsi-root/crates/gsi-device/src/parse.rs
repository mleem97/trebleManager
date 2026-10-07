//! Device-output parsers. Mirrors the reference implementations
//! (`ConvertFrom-AdbDevices`, `ConvertFrom-FastbootDevices`,
//! `ConvertFrom-GetpropDump`, `ConvertFrom-ByNameListing`,
//! `ConvertFrom-FastbootGetvar`) including Huawei semantics:
//! `FAILED (remote: Command not allowed)` is a Huawei quirk, never proof
//! of a lock.

use std::collections::BTreeMap;

/// One attached device.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Device {
    pub serial: String,
    pub state: String,
}

/// `adb devices` output → devices (skips the header line).
pub fn adb_devices(lines: &[&str]) -> Vec<Device> {
    let mut out = Vec::new();
    for line in lines {
        let t = line.trim();
        if t.is_empty() || t.starts_with("List of") {
            continue;
        }
        let mut parts = t.split_whitespace();
        if let (Some(serial), Some(state)) = (parts.next(), parts.next()) {
            out.push(Device {
                serial: serial.to_string(),
                state: state.to_string(),
            });
        }
    }
    out
}

/// `fastboot devices` output → devices.
pub fn fastboot_devices(lines: &[&str]) -> Vec<Device> {
    let mut out = Vec::new();
    for line in lines {
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        let mut parts = t.split_whitespace();
        if let (Some(serial), Some(state)) = (parts.next(), parts.next()) {
            out.push(Device {
                serial: serial.to_string(),
                state: state.to_string(),
            });
        }
    }
    out
}

/// getprop dump → map. Supports `[k]: [v]` and `k = v`.
pub fn getprop(lines: &[&str]) -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();
    for line in lines {
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        if t.starts_with('[') {
            // [key]: [value]
            if let Some(mid) = t.find("]: [") {
                let key = t[1..mid].trim().to_string();
                let rest = &t[mid + 4..];
                let val = rest.strip_suffix(']').unwrap_or(rest).to_string();
                if !key.is_empty() {
                    map.insert(key, val);
                }
                continue;
            }
        }
        if let Some(eq) = t.find('=') {
            let key = t[..eq].trim().to_string();
            let val = t[eq + 1..].trim().to_string();
            if !key.is_empty() {
                map.insert(key, val);
            }
        }
    }
    map
}

/// One `/dev/block/by-name` entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ByName {
    pub name: String,
    pub target: String,
}

/// by-name listing → entries. Accepts `name -> target` and
/// `lrwxrwxrwx ... name -> target` lines.
pub fn byname(raw: &str) -> Vec<ByName> {
    let mut out = Vec::new();
    for line in raw.lines() {
        let t = line.trim();
        if t.is_empty() || t.contains("by-name") && !t.contains("->") {
            continue;
        }
        if let Some(pos) = t.find("->") {
            let left = t[..pos].trim();
            let target = t[pos + 2..].trim();
            if target.is_empty() {
                continue;
            }
            let name = left.split_whitespace().last().unwrap_or("").to_string();
            if !name.is_empty() {
                out.push(ByName { name, target: target.to_string() });
            }
        }
    }
    out
}

/// fastboot getvar result: values plus Huawei-specific denial info.
#[derive(Debug, Clone, Default)]
pub struct Getvar {
    pub vars: BTreeMap<String, String>,
    /// A `FAILED (remote: ...)` line was seen (quirk, not a lock proof).
    pub command_denied: bool,
    pub last_failed_reason: String,
}

/// Parse `fastboot getvar` output lines.
pub fn getvar(lines: &[&str]) -> Getvar {
    let mut out = Getvar::default();
    for line in lines {
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        let lower = t.to_lowercase();
        if lower.starts_with("finished") || lower.starts_with("getvar") {
            // `getvar:all` headers and `finished.` footers carry no values —
            // except `FAILED (remote: ...)` lines, handled below.
            if !(t.contains("FAILED") || t.contains("failed")) {
                continue;
            }
        }
        if let Some(pos) = t.find("FAILED") {
            out.command_denied = true;
            if let Some(r1) = t[pos..].find("(remote:") {
                let rest = &t[pos + r1 + "(remote:".len()..];
                if let Some(r2) = rest.find(')') {
                    out.last_failed_reason = rest[..r2].trim().to_string();
                }
            }
            continue;
        }
        // Same broad rule as the reference: any such line marks denial.
        if t.contains("Command not allowed") {
            out.command_denied = true;
        }
        if let Some(colon) = t.find(':') {
            let key = t[..colon].trim().to_string();
            let val = t[colon + 1..].trim().to_string();
            if !key.is_empty() {
                out.vars.insert(key, val);
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    // Same fixtures as the PS1/bash suites (incl. the real serial format).
    const SERIAL: &str = "6PQ0217B08003446";

    #[test]
    fn adb_parses_serial_and_states() {
        let ds = adb_devices(&[
            "List of devices attached",
            "6PQ0217B08003446\tdevice",
            "",
            "ABC123\tunauthorized",
        ]);
        assert_eq!(ds.len(), 2);
        assert_eq!(ds[0].serial, SERIAL);
        assert_eq!(ds[0].state, "device");
        assert_eq!(ds[1].state, "unauthorized");
    }

    #[test]
    fn fastboot_parses() {
        let ds = fastboot_devices(&["6PQ0217B08003446\tfastboot", ""]);
        assert_eq!(ds.len(), 1);
        assert_eq!(ds[0].serial, SERIAL);
    }

    #[test]
    fn huawei_denied_is_quirk_not_lock() {
        let g = getvar(&[
            "getvar:unlocked FAILED (remote: Command not allowed)",
            "finished. total time: 0.016s",
        ]);
        assert!(g.command_denied);
        assert!(g.vars.is_empty());
    }

    #[test]
    fn getvar_key_value() {
        // Same split rule as the reference: first colon wins.
        let g = getvar(&["partition-size:recovery_ramdisk: 0x02000000", "finished."]);
        assert_eq!(
            g.vars.get("partition-size").map(String::as_str),
            Some("recovery_ramdisk: 0x02000000")
        );
        assert!(!g.command_denied);
    }

    #[test]
    fn getprop_both_formats() {
        let m = getprop(&[
            "[ro.product.model]: [TrebleDroid with GApps]",
            "[ro.build.version.release]: [13]",
            "ro.secure = 1",
        ]);
        assert_eq!(
            m.get("ro.product.model").map(String::as_str),
            Some("TrebleDroid with GApps")
        );
        assert_eq!(m.get("ro.secure").map(String::as_str), Some("1"));
    }

    #[test]
    fn byname_both_forms() {
        let v = byname("recovery_ramdisk -> /dev/block/mmcblk0p30\nboot -> /dev/block/mmcblk0p28\nsystem -> /dev/block/mmcblk0p60");
        assert!(v.iter().any(|e| e.name == "recovery_ramdisk"));
        assert!(v.iter().any(|e| e.name == "boot"));
        let v2 = byname("lrwxrwxrwx 1 root root 16 2026-10-07 01:45 recovery_ramdisk -> /dev/block/sdd37");
        assert_eq!(v2.len(), 1);
        assert_eq!(v2[0].target, "/dev/block/sdd37");
    }
}
