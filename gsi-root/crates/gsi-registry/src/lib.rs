//! Compatibility registry access (`data/compatibility/huawei/p10/*.json`).
//!
//! Mirrors the script logic (`Get-RomOptions`, `target_androids`,
//! `Test-FirmwareCompatibility`, `Get-VendorAdvice`, target configs):
//! model family strict, submodel/region/EMUI as WARN, example firmware
//! advisory only. Data files stay the source of truth.

use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Working statuses selectable in the UI.
pub fn is_selectable(status: &str) -> bool {
    matches!(
        status,
        "working" | "working-slim" | "working-with-fixes" | "variant-dependent"
    )
}

/// One ROM registry entry (subset of fields; unknown fields ignored).
#[derive(Debug, Clone, Default, Deserialize)]
pub struct RomEntry {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub version: serde_json::Value,
    #[serde(default)]
    pub android: serde_json::Value,
    #[serde(default)]
    pub build: serde_json::Value,
    #[serde(default)]
    pub variant: String,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub gsi: String,
    #[serde(default)]
    pub file: String,
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub note: String,
    #[serde(default)]
    pub reason: String,
    #[serde(default)]
    pub markers: Vec<String>,
    #[serde(default)]
    pub root_artifact: RootArtifact,
}

/// Required root artifact for a ROM (`ROM → boot config → root artifact`).
#[derive(Debug, Clone, Deserialize)]
pub struct RootArtifact {
    #[serde(rename = "type", default = "default_artifact_type")]
    pub artifact_type: String,
    #[serde(default = "default_artifact_source")]
    pub source: String,
}

fn default_artifact_type() -> String {
    "recovery_ramdisk".to_string()
}
fn default_artifact_source() -> String {
    "stock_firmware".to_string()
}

impl Default for RootArtifact {
    fn default() -> Self {
        Self {
            artifact_type: default_artifact_type(),
            source: default_artifact_source(),
        }
    }
}

fn str_val(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Number(n) => n.to_string(),
        _ => String::new(),
    }
}

/// Display label: name + version/android/build + variant + build.
/// Same construction as the scripts (labels must match for IDs).
pub fn entry_label(e: &RomEntry) -> String {
    if e.name.trim().is_empty() {
        return String::new();
    }
    let mut label = e.name.trim().to_string();
    let mut ver = str_val(&e.version);
    if ver.is_empty() {
        ver = str_val(&e.android);
    }
    if ver.is_empty() {
        ver = str_val(&e.build);
    }
    if !ver.is_empty() {
        label.push(' ');
        label.push_str(&ver);
    }
    if !e.variant.is_empty() && !label.contains(&e.variant) {
        label.push(' ');
        label.push_str(&e.variant);
    }
    let bld = str_val(&e.build);
    if !bld.is_empty() && !label.contains(&bld) {
        label.push_str(&format!(" ({bld})"));
    }
    label
}

/// Selectable ROM options: (id, label). Deduped, stock first is added by UI.
pub fn rom_options(entries: &[RomEntry]) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for e in entries {
        if !is_selectable(&e.status) {
            continue;
        }
        let label = entry_label(e);
        if label.is_empty() || !seen.insert(label.clone()) {
            continue;
        }
        out.push((format!("rom:{label}"), label));
    }
    out
}

/// Researched-broken entries: (name, reason). Info only, never selectable.
pub fn rom_broken(entries: &[RomEntry]) -> Vec<(String, String)> {
    entries
        .iter()
        .filter(|e| e.status == "broken")
        .map(|e| {
            let reason = if e.reason.is_empty() {
                e.note.clone()
            } else {
                e.reason.clone()
            };
            (e.name.clone(), reason)
        })
        .collect()
}

/// Distinct Android versions with working-image counts, ascending.
pub fn target_androids(entries: &[RomEntry]) -> Vec<(i64, usize)> {
    let mut counts: BTreeMap<i64, usize> = BTreeMap::new();
    for e in entries {
        if !is_selectable(&e.status) {
            continue;
        }
        // int() helper not needed: reuse android_int via temp entry is awkward;
        // compute inline like android_int but on borrowed entry:
        let a = android_of(e);
        if a > 0 {
            *counts.entry(a).or_insert(0) += 1;
        }
    }
    counts.into_iter().collect()
}

fn android_of(e: &RomEntry) -> i64 {
    for cand in [&e.android] {
        let s = str_val(cand);
        if let Ok(n) = s.parse::<i64>() {
            if n > 0 {
                return n;
            }
        }
    }
    let ver = str_val(&e.version);
    let major: String = ver.chars().take_while(|c| c.is_ascii_digit()).collect();
    match major.parse::<i64>() {
        Ok(18) => 11,
        Ok(19) => 12,
        Ok(20) => 13,
        Ok(n) if n > 0 => n,
        _ => 0,
    }
}

/// Distinct system labels for one Android version.
pub fn systems_for(entries: &[RomEntry], android: i64) -> Vec<String> {
    let mut out = Vec::new();
    for e in entries {
        if !is_selectable(&e.status) || android_of(e) != android {
            continue;
        }
        let l = entry_label(e);
        if !l.is_empty() && !out.contains(&l) {
            out.push(l);
        }
    }
    out
}

/// Entries behind one system label (differing variant/build).
pub fn variants_for<'a>(entries: &'a [RomEntry], label: &str) -> Vec<&'a RomEntry> {
    entries
        .iter()
        .filter(|e| is_selectable(&e.status) && entry_label(e) == label)
        .collect()
}

/// Firmware compatibility verdict (mirrors Test-FirmwareCompatibility).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompatVerdict {
    pub status: String, // PASS | WARN | FAIL
    pub reasons: Vec<String>,
    pub fw_cust: String,
}

/// Check a firmware string against model + region.
pub fn firmware_compat(model: &str, firmware: &str, region: &str) -> CompatVerdict {
    let mut reasons = Vec::new();
    let mut status = "PASS".to_string();
    if firmware.trim().is_empty() {
        return CompatVerdict {
            status: "FAIL".to_string(),
            reasons: vec!["No firmware given (baseline missing).".to_string()],
            fw_cust: String::new(),
        };
    }
    let fw = firmware.to_uppercase();
    let mo = model.to_uppercase();
    // Model family strict.
    if mo.starts_with("VTR") && !fw.contains("VTR") {
        status = "FAIL".to_string();
        reasons.push(format!("Firmware contains no VTR (P10), device is {model}."));
    }
    if mo.starts_with("VKY") && !fw.contains("VKY") {
        status = "FAIL".to_string();
        reasons.push(format!("Firmware contains no VKY (P10 Plus), device is {model}."));
    }
    // Submodel WARN.
    if mo == "VTR-L29" && fw.contains("VTR-L09") && !fw.contains("VTR-L29") {
        if status == "PASS" {
            status = "WARN".to_string();
        }
        reasons.push("Submodel mismatch: device L29, firmware L09 -> double-check CUST/region.".to_string());
    }
    if mo == "VTR-L09" && fw.contains("VTR-L29") && !fw.contains("VTR-L09") {
        if status == "PASS" {
            status = "WARN".to_string();
        }
        reasons.push("Submodel mismatch: device L09, firmware L29 -> double-check CUST/region.".to_string());
    }
    // Region Cxxx.
    let mut fw_cust = String::new();
    if let Some(pos) = fw.find("(C") {
        let tail = &fw[pos + 1..];
        let digits: String = tail[1..].chars().take_while(|c| c.is_ascii_digit()).collect();
        if !digits.is_empty() {
            fw_cust = format!("C{digits}");
        }
    }
    if !region.is_empty() && !fw_cust.is_empty() && region.to_uppercase() != fw_cust.to_uppercase() {
        if status == "PASS" {
            status = "WARN".to_string();
        }
        reasons.push(format!(
            "Region: device {region} vs firmware {fw_cust} -> flash only with matching CUST."
        ));
    }
    // EMUI generation notes.
    if fw.contains("9.1.0") {
        reasons.push("EMUI 9.1 base ok (recovery_ramdisk method).".to_string());
    } else if fw.contains("9.0") {
        if status == "PASS" {
            status = "WARN".to_string();
        }
        reasons.push("EMUI 9.0 instead of 9.1 -> method possible, but prefer full 9.1 firmware.".to_string());
    } else if fw.contains("8.") {
        if status == "PASS" {
            status = "WARN".to_string();
        }
        reasons.push("EMUI 8 base -> different boot chain possible, see wiki/kernel notes.".to_string());
    } else {
        if status == "PASS" {
            status = "WARN".to_string();
        }
        reasons.push("EMUI version not recognizable from firmware string -> verify manually.".to_string());
    }
    if reasons.is_empty() {
        reasons.push("Base check passed.".to_string());
    }
    CompatVerdict {
        status,
        reasons,
        fw_cust,
    }
}

/// Vendor advice for an EMUI base + target Android.
pub fn vendor_advice(emui: &str, target_android: &str) -> String {
    if emui.trim().is_empty() {
        return "Vendor base unknown (GSI hides it) - assume nothing about Q/R/S boot.".to_string();
    }
    if emui.contains("8.") {
        return format!("Oreo vendor: Q/8.1 problematic, P good. Target Android {target_android} on Oreo vendor = RISK (see registry vendor_boot).");
    }
    if emui.contains("9.") {
        return format!("Pie vendor: Q/R/S boot. Target Android {target_android} expected to boot.");
    }
    format!("Vendor base '{emui}' unclassified - verify manually.")
}

/// Resolved target configuration (device → android → system → variant).
#[derive(Debug, Clone)]
pub struct TargetConfig {
    pub android: i64,
    pub system: String,
    pub variant: String,
    pub build: String,
    pub gsi: String,
    pub system_file: String,
    pub system_url: String,
    pub root_type: String,
    pub root_source: String,
    pub firmware_base: String,
}

/// Resolve a full target config from a registry entry + firmware base.
pub fn target_config(entry: &RomEntry, firmware_base: &str) -> TargetConfig {
    TargetConfig {
        android: android_of(entry),
        system: entry_label(entry),
        variant: entry.variant.clone(),
        build: str_val(&entry.build),
        gsi: entry.gsi.clone(),
        system_file: entry.file.clone(),
        system_url: entry.url.clone(),
        root_type: entry.root_artifact.artifact_type.clone(),
        root_source: entry.root_artifact.source.clone(),
        firmware_base: firmware_base.to_string(),
    }
}

/// Load ROM entries from a profile JSON file.
pub fn load_roms(profile_json: &Path) -> Result<Vec<RomEntry>, String> {
    let text = std::fs::read_to_string(profile_json).map_err(|e| format!("read: {e}"))?;
    let v: serde_json::Value =
        serde_json::from_str(&text).map_err(|e| format!("json: {e}"))?;
    let arr = v
        .get("roms")
        .and_then(|r| r.as_array())
        .ok_or_else(|| "no roms array".to_string())?;
    let mut out = Vec::new();
    for r in arr {
        let e: RomEntry =
            serde_json::from_value(r.clone()).map_err(|e| format!("entry: {e}"))?;
        out.push(e);
    }
    Ok(out)
}

/// Firmware required_base from a profile JSON file.
pub fn firmware_base(profile_json: &Path) -> String {
    let text = match std::fs::read_to_string(profile_json) {
        Ok(t) => t,
        Err(_) => return String::new(),
    };
    let v: serde_json::Value = match serde_json::from_str(&text) {
        Ok(v) => v,
        Err(_) => return String::new(),
    };
    v.get("firmware")
        .and_then(|f| f.get("required_base"))
        .and_then(|b| b.as_str())
        .unwrap_or("")
        .to_string()
}

/// Repo profile path for tests and tools (repo layout dependent).
pub fn repo_profile_path(profile: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../data/compatibility/huawei/p10")
        .join(format!("{profile}.json"))
}

// ------------------------------------------------------------ tools block

/// Platform-tools reference from the registry `tools` block.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PlatformToolsRef {
    /// Registry source tag (e.g. `direct-official`).
    pub source: String,
    /// URL pattern with `{os}` placeholder (e.g. `...-latest-{os}.zip`).
    pub url_pattern: String,
    /// Provided binaries (e.g. `adb`, `fastboot`).
    pub provides: Vec<String>,
    /// Human note.
    pub note: String,
}

/// Firmware extractor reference from the registry `tools` block.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ExtractorRef {
    /// Display name (e.g. `HuaweiFirmwareExtractor`).
    pub name: String,
    /// File name to fetch (e.g. `huawei_firmware_extractor.py`).
    pub file: String,
    /// Direct download URL.
    pub url: String,
    /// Source tag (e.g. `direct`).
    pub source: String,
    /// Runtime need (e.g. `python3.9+`).
    pub needs: String,
    /// Usage hint (`use` field in JSON).
    pub use_hint: String,
    /// Human note.
    pub note: String,
}

/// Registry `tools` block (platform-tools + extractor references).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ToolsBlock {
    /// Platform-tools entry when present.
    pub platform_tools: Option<PlatformToolsRef>,
    /// Extractor entries (may be empty).
    pub extractors: Vec<ExtractorRef>,
}

fn str_field(v: &serde_json::Value, key: &str) -> String {
    v.get(key)
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .to_string()
}

fn str_list(v: &serde_json::Value, key: &str) -> Vec<String> {
    let mut out = Vec::new();
    if let Some(arr) = v.get(key).and_then(|x| x.as_array()) {
        for item in arr {
            if let Some(s) = item.as_str() {
                out.push(s.to_string());
            }
        }
    }
    out
}

/// Parse the `tools` block from a profile JSON value (pure, no I/O).
///
/// Missing block yields an empty [`ToolsBlock`]; unknown fields ignored.
pub fn parse_tools_block(value: &serde_json::Value) -> ToolsBlock {
    let tools = match value.get("tools") {
        Some(t) => t,
        None => return ToolsBlock::default(),
    };
    let platform_tools = tools
        .get("platform_tools")
        .or_else(|| tools.get("platform-tools"))
        .map(|p| {
            let pattern = {
                let a = str_field(p, "url_pattern");
                if !a.is_empty() {
                    a
                } else {
                    let b = str_field(p, "url");
                    if !b.is_empty() {
                        b
                    } else {
                        str_field(p, "pattern")
                    }
                }
            };
            PlatformToolsRef {
                source: str_field(p, "source"),
                url_pattern: pattern,
                provides: str_list(p, "provides"),
                note: str_field(p, "note"),
            }
        })
        .filter(|p| {
            !p.url_pattern.is_empty() || !p.source.is_empty() || !p.provides.is_empty()
        });
    let mut extractors = Vec::new();
    if let Some(arr) = tools.get("extractors").and_then(|x| x.as_array()) {
        for e in arr {
            let use_hint = {
                let a = str_field(e, "use");
                if !a.is_empty() {
                    a
                } else {
                    str_field(e, "use_hint")
                }
            };
            let r = ExtractorRef {
                name: str_field(e, "name"),
                file: str_field(e, "file"),
                url: str_field(e, "url"),
                source: str_field(e, "source"),
                needs: str_field(e, "needs"),
                use_hint,
                note: str_field(e, "note"),
            };
            if !r.name.is_empty() || !r.file.is_empty() || !r.url.is_empty() {
                extractors.push(r);
            }
        }
    }
    ToolsBlock {
        platform_tools,
        extractors,
    }
}

/// Load the `tools` block from a profile JSON file.
pub fn tools_block(profile_json: &Path) -> Result<ToolsBlock, String> {
    let text = std::fs::read_to_string(profile_json).map_err(|e| format!("read: {e}"))?;
    let v: serde_json::Value =
        serde_json::from_str(&text).map_err(|e| format!("json: {e}"))?;
    Ok(parse_tools_block(&v))
}

/// Normalize an OS hint to a registry token (`linux`/`darwin`/`windows`).
fn normalize_os_token(os: &str) -> String {
    let low = os.trim().to_lowercase();
    if low.contains("linux") {
        "linux".to_string()
    } else if low.contains("darwin") || low.contains("macos") || low == "mac" || low.contains("mac_") {
        "darwin".to_string()
    } else {
        "windows".to_string()
    }
}

/// Select the platform-tools URL for an OS from a [`ToolsBlock`].
///
/// Substitutes `{os}` in the registry `url_pattern` with the normalized
/// token. Only `https://` URLs are returned.
pub fn platform_tools_url(tools: &ToolsBlock, os: &str) -> Result<String, String> {
    let pattern = match &tools.platform_tools {
        Some(p) if !p.url_pattern.trim().is_empty() => p.url_pattern.trim().to_string(),
        _ => return Err("tools: no platform-tools url_pattern in registry".to_string()),
    };
    let token = normalize_os_token(os);
    let url = pattern.replace("{os}", &token);
    if url.starts_with("https://") {
        Ok(url)
    } else {
        Err("tools: platform-tools URL rejected (https only)".to_string())
    }
}

/// Extractor URL by name or file (case-insensitive, pure lookup).
pub fn extractor_url(tools: &ToolsBlock, name: &str) -> Result<String, String> {
    let want = name.trim().to_lowercase();
    if want.is_empty() {
        return firmware_extractor_url(tools);
    }
    for e in &tools.extractors {
        if e.name.to_lowercase() == want || e.file.to_lowercase() == want {
            if e.url.starts_with("https://") {
                return Ok(e.url.clone());
            }
            return Err("tools: extractor URL rejected (https only)".to_string());
        }
    }
    let mut avail = Vec::new();
    for e in &tools.extractors {
        if !e.name.is_empty() {
            avail.push(e.name.clone());
        }
    }
    Err(format!(
        "tools: unknown extractor '{name}' (available: {})",
        avail.join(", ")
    ))
}

/// Primary firmware-extractor URL (first Huawei/firmware match, else first).
pub fn firmware_extractor_url(tools: &ToolsBlock) -> Result<String, String> {
    if tools.extractors.is_empty() {
        return Err("tools: no extractors in registry".to_string());
    }
    let mut first: Option<&ExtractorRef> = None;
    for e in &tools.extractors {
        if first.is_none() {
            first = Some(e);
        }
        let blob = format!("{} {}", e.name, e.file).to_lowercase();
        if blob.contains("huawei") || blob.contains("firmware") || blob.contains("extract") {
            if e.url.starts_with("https://") {
                return Ok(e.url.clone());
            }
            return Err("tools: extractor URL rejected (https only)".to_string());
        }
    }
    match first {
        Some(e) if e.url.starts_with("https://") => Ok(e.url.clone()),
        Some(_) => Err("tools: extractor URL rejected (https only)".to_string()),
        None => Err("tools: no extractors in registry".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Vec<RomEntry> {
        // Mirrors the real registry shape (names/fields, not full content).
        let v: serde_json::Value = serde_json::json!([
            {"name": "LineageOS", "version": 20, "android": 13, "status": "working",
             "gsi": "arm64_bgN", "file": "l20.img.gz", "url": "https://x/l20.img.gz",
             "root_artifact": {"type": "recovery_ramdisk", "source": "stock_firmware"}},
            {"name": "LineageOS", "version": 20, "android": 13, "variant": "vndklite",
             "build": 20251021, "status": "working", "gsi": "arm64_bgN",
             "root_artifact": {"type": "recovery_ramdisk", "source": "stock_firmware"}},
            {"name": "LineageOS 20 Light", "version": 20, "status": "broken",
             "reason": "non-booting", "markers": ["light"]}
        ]);
        v.as_array()
            .unwrap()
            .iter()
            .map(|r| serde_json::from_value(r.clone()).unwrap())
            .collect()
    }

    #[test]
    fn labels_stay_distinct() {
        let e = &sample();
        assert_eq!(entry_label(&e[0]), "LineageOS 20");
        assert_eq!(entry_label(&e[1]), "LineageOS 20 vndklite (20251021)");
    }

    #[test]
    fn options_exclude_broken() {
        let opts = rom_options(&sample());
        assert_eq!(opts.len(), 2);
        assert!(opts.iter().all(|(id, _)| !id.contains("Light")));
    }

    #[test]
    fn androids_with_counts() {
        let a = target_androids(&sample());
        assert_eq!(a, vec![(13, 2)]);
    }

    #[test]
    fn compat_matrix() {
        let ok = firmware_compat("VTR-L29", "VTR-L29 9.1.0.297(C432E5R1P9)", "C432");
        assert_eq!(ok.status, "PASS");
        let wrong = firmware_compat("VTR-L29", "VKY-L29 9.1.0.297(C432E5R1P9)", "C432");
        assert_eq!(wrong.status, "FAIL");
        let sub = firmware_compat("VTR-L29", "VTR-L09 9.1.0.297(C432E5R1P9)", "C432");
        assert_eq!(sub.status, "WARN");
        let empty = firmware_compat("VTR-L29", "", "");
        assert_eq!(empty.status, "FAIL");
    }

    #[test]
    fn vendor_notes() {
        assert!(vendor_advice("", "13").contains("unknown"));
        assert!(vendor_advice("9.1", "13").contains("expected to boot"));
        assert!(vendor_advice("8.0", "13").contains("RISK"));
    }

    #[test]
    fn target_config_carries_root_artifact() {
        let e = &sample()[0];
        let c = target_config(e, "EMUI 9.1");
        assert_eq!(c.android, 13);
        assert_eq!(c.root_type, "recovery_ramdisk");
        assert_eq!(c.root_source, "stock_firmware");
        assert_eq!(c.system_url, "https://x/l20.img.gz");
    }

    #[test]
    fn live_registry_parses() {
        // Real profile file (repo layout); fails loudly if the shape drifts.
        for prof in ["VTR-L09", "VTR-L29"] {
            let entries = load_roms(&repo_profile_path(prof)).expect("profile must load");
            assert!(!entries.is_empty());
            let unofficial: Vec<_> = entries
                .iter()
                .filter(|e| e.variant == "UNOFFICIAL")
                .collect();
            assert!(!unofficial.is_empty(), "UNOFFICIAL entry expected");
            assert_eq!(android_of(&unofficial[0]), 13);
        }
    }

    fn tools_sample() -> serde_json::Value {
        serde_json::json!({
            "tools": {
                "platform_tools": {
                    "source": "direct-official",
                    "url_pattern": "https://dl.google.com/android/repository/platform-tools-latest-{os}.zip",
                    "provides": ["adb", "fastboot"],
                    "note": "Google official CDN, per-OS zip"
                },
                "extractors": [
                    {
                        "name": "HuaweiFirmwareExtractor",
                        "file": "huawei_firmware_extractor.py",
                        "url": "https://raw.githubusercontent.com/Natsume324/HuaweiFirmwareExtractor/main/huawei_firmware_extractor.py",
                        "source": "direct",
                        "needs": "python3.9+",
                        "use": "python huawei_firmware_extractor.py UPDATE.APP -p RECOVERY_RAMDIS -o <dir>",
                        "note": "dependency-free"
                    }
                ]
            }
        })
    }

    #[test]
    fn tools_block_parses_fixture() {
        let t = parse_tools_block(&tools_sample());
        let p = t.platform_tools.expect("platform tools expected");
        assert_eq!(p.source, "direct-official");
        assert!(p.provides.contains(&"adb".to_string()));
        assert_eq!(t.extractors.len(), 1);
        assert_eq!(t.extractors[0].file, "huawei_firmware_extractor.py");
    }

    #[test]
    fn tools_block_missing_is_empty() {
        let t = parse_tools_block(&serde_json::json!({"roms": []}));
        assert!(t.platform_tools.is_none());
        assert!(t.extractors.is_empty());
        assert!(platform_tools_url(&t, "linux").is_err());
        assert!(firmware_extractor_url(&t).is_err());
    }

    #[test]
    fn platform_tools_url_per_os() {
        let t = parse_tools_block(&tools_sample());
        assert_eq!(
            platform_tools_url(&t, "Linux").unwrap(),
            "https://dl.google.com/android/repository/platform-tools-latest-linux.zip"
        );
        assert_eq!(
            platform_tools_url(&t, "Darwin").unwrap(),
            "https://dl.google.com/android/repository/platform-tools-latest-darwin.zip"
        );
        assert_eq!(
            platform_tools_url(&t, "windows-msvc").unwrap(),
            "https://dl.google.com/android/repository/platform-tools-latest-windows.zip"
        );
        assert_eq!(
            platform_tools_url(&t, "").unwrap(),
            "https://dl.google.com/android/repository/platform-tools-latest-windows.zip"
        );
    }

    #[test]
    fn extractor_urls() {
        let t = parse_tools_block(&tools_sample());
        let u = firmware_extractor_url(&t).unwrap();
        assert!(u.contains("huawei_firmware_extractor.py"));
        assert_eq!(extractor_url(&t, "HuaweiFirmwareExtractor").unwrap(), u);
        assert_eq!(
            extractor_url(&t, "huawei_firmware_extractor.py").unwrap(),
            u
        );
        assert!(extractor_url(&t, "nope").is_err());
    }

    #[test]
    fn live_tools_block_parses() {
        let t = tools_block(&repo_profile_path("VTR-L29")).expect("tools must load");
        let url = platform_tools_url(&t, "linux").expect("url expected");
        assert!(url.starts_with("https://dl.google.com/"));
        let ext = firmware_extractor_url(&t).expect("extractor expected");
        assert!(ext.starts_with("https://"));
    }
}
