//! Tiny bilingual (en/de) message table for gsi-root flows.
//!
//! Mirrors the PS1 `L "en" "de"` helper: German only when the UI language
//! is German, otherwise English. No external deps, no I/O, no panics.
//!
//! Coverage is intentionally tight (~30 keys): messages for the
//! already-ported flows (analyze / patch refusal / update check / flash
//! verdict / device states) plus Detect (`adb`/`fastboot` device queries)
//! and Tools (managed `adb`/`fastboot`/extractor listing) flows. English
//! strings match the current core/CLI/GUI messages where they exist
//! (prefixes for parameterized lines). The full PS1 string extraction stays
//! a Phase-9 epic.
//!
//! Fallback rule: German lookup with a missing/empty `de` entry returns
//! the English string. Unknown keys return `"missing message"` (never empty,
//! never panics) so a missing entry stays visible in the UI.

/// UI language.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    En,
    De,
}

impl Language {
    /// BCP-47-ish code (`en` / `de`).
    pub fn code(self) -> &'static str {
        match self {
            Self::En => "en",
            Self::De => "de",
        }
    }

    /// Parse a language code. `de` (any case, `de-*` prefix) maps to German,
    /// everything else (including empty) maps to English. Never fails.
    pub fn from_code(code: &str) -> Self {
        let mut lower = String::new();
        for c in code.chars() {
            lower.push(c.to_ascii_lowercase());
            if lower.len() >= 5 {
                break;
            }
        }
        if lower == "de" || lower.starts_with("de-") || lower.starts_with("de_") {
            Self::De
        } else {
            Self::En
        }
    }

    /// Detect the UI language from the environment (`LANG` / `LANGUAGE`).
    /// German only when the value starts with `de` (case insensitive),
    /// otherwise English. Never panics, never reads beyond env vars.
    pub fn detect() -> Self {
        for name in ["LANGUAGE", "LANG"] {
            if let Ok(v) = std::env::var(name) {
                let t = v.trim();
                if !t.is_empty() {
                    // Split `de_DE.UTF-8`-style values at common separators.
                    let mut head = String::new();
                    for c in t.chars() {
                        if c == '_' || c == '-' || c == '.' || c == '@' || c == ':' {
                            break;
                        }
                        head.push(c);
                        if head.len() >= 8 {
                            break;
                        }
                    }
                    if Language::from_code(head.as_str()) == Language::De {
                        return Self::De;
                    }
                    // Keep scanning: LANGUAGE may list several fallbacks.
                    if name == "LANG" {
                        return Self::En;
                    }
                }
            }
        }
        Self::En
    }
}

/// One message row: stable key plus English and German strings.
///
/// An empty `de` entry means "German pending" and falls back to English
/// via [`lookup`].
struct Entry {
    key: &'static str,
    en: &'static str,
    de: &'static str,
}

/// Core message table (~30 keys). English matches current core/CLI/GUI
/// strings where they exist (prefixes for parameterized lines).
const MESSAGES: &[Entry] = &[
    Entry { key: "analyze.no_file", en: "no file given", de: "keine Datei angegeben" },
    Entry { key: "analyze.no_container", en: "no container detected", de: "kein Container erkannt" },
    Entry { key: "analyze.sparse_none", en: "sparse: no", de: "sparse: nein" },
    Entry { key: "analyze.poc_pending", en: "hardware POC pending: normal power-boot with active root not yet proven", de: "Hardware-POC ausstehend: normaler Power-Boot mit aktivem Root noch nicht nachgewiesen" },
    Entry { key: "analyze.no_engine", en: "no root engine", de: "keine Root-Engine" },
    Entry { key: "patch.refused", en: "EXPERIMENTAL \u{2014} refused.", de: "EXPERIMENTELL \u{2014} abgelehnt." },
    Entry { key: "patch.no_poc", en: "No hardware POC exists yet: nothing is written, nothing is faked as done.", de: "Es gibt noch keinen Hardware-POC: nichts wird geschrieben, nichts als fertig vorget\u{00e4}uscht." },
    Entry { key: "patch.poc_plan", en: "See devices/huawei-p10 for the POC plan.", de: "Siehe devices/huawei-p10 f\u{00fc}r den POC-Plan." },
    Entry { key: "update.latest", en: "latest:", de: "neueste:" },
    Entry { key: "update.check_failed", en: "check failed:", de: "Pr\u{00fc}fung fehlgeschlagen:" },
    Entry { key: "flash.cannot_safe", en: "Cannot safely flash image (image not verifiable)", de: "Image kann nicht sicher geflasht werden (Image nicht verifizierbar)" },
    Entry { key: "flash.failed", en: "Flash failed (Fastboot flash failed)", de: "Flash fehlgeschlagen (Fastboot-Flash fehlgeschlagen)" },
    Entry { key: "flash.patch_failed", en: "Patch failed (Magisk patching failed)", de: "Patch fehlgeschlagen (Magisk-Patching fehlgeschlagen)" },
    Entry { key: "flash.boot_verify_failed", en: "Boot verification failed (Android won't boot -> offer restore)", de: "Boot-Verifizierung fehlgeschlagen (Android startet nicht -> Wiederherstellung anbieten)" },
    Entry { key: "device.no_device", en: "No device detected", de: "Kein Ger\u{00e4}t erkannt" },
    Entry { key: "device.adb_not_found", en: "ADB not found", de: "ADB nicht gefunden" },
    Entry { key: "device.fastboot_not_found", en: "Fastboot not found", de: "Fastboot nicht gefunden" },
    Entry { key: "device.unauthorized", en: "USB debugging authorization required (ADB unauthorized)", de: "USB-Debugging-Autorisierung erforderlich (ADB nicht autorisiert)" },
    Entry { key: "device.huawei_denied", en: "Command not allowed (Huawei getvar refused, NOT auto locked)", de: "Befehl nicht erlaubt (Huawei-getvar verweigert, NICHT automatisch gesperrt)" },
    Entry { key: "device.unsupported_layout", en: "Unsupported partition layout (recovery_ramdisk missing)", de: "Nicht unterst\u{00fc}tztes Partitionslayout (recovery_ramdisk fehlt)" },
    Entry { key: "device.unsupported_model", en: "Unsupported model (wrong device)", de: "Nicht unterst\u{00fc}tztes Modell (falsches Ger\u{00e4}t)" },
    Entry { key: "device.firmware_mismatch", en: "Firmware mismatch (wrong firmware)", de: "Firmware-Fehlanpassung (falsche Firmware)" },
    Entry { key: "device.no_devices_tools", en: "no devices (or no tools)", de: "keine Ger\u{00e4}te (oder keine Tools)" },
    Entry { key: "device.adb_missing_managed", en: "adb: not found (managed binding needs adb on PATH)", de: "adb: nicht gefunden (verwaltetes Binding braucht adb im PATH)" },
    Entry { key: "device.fastboot_missing_managed", en: "fastboot: not found (managed binding needs fastboot on PATH)", de: "fastboot: nicht gefunden (verwaltetes Binding braucht fastboot im PATH)" },
    Entry { key: "slot.shared", en: "TWRP and Magisk share the recovery_ramdisk slot (mutual overwrite).", de: "TWRP und Magisk teilen sich den recovery_ramdisk-Slot (gegenseitiges \u{00dc}berschreiben)." },
    Entry { key: "slot.unknown_current", en: "current occupant unknown (fresh/trustworthy only after a tool flash) \u{2014} verify before flashing.", de: "aktueller Bewohner unbekannt (erst nach einem Tool-Flash frisch/vertrauensw\u{00fc}rdig) \u{2014} vor dem Flashen verifizieren." },
    Entry { key: "slot.no_magisk_image", en: "no registered Magisk patched image (patch first)", de: "kein registriertes Magisk-Patch-Image (zuerst patchen)" },
    Entry { key: "slot.no_twrp_image", en: "no known TWRP image (provide exact model build)", de: "kein bekanntes TWRP-Image (exakten Modell-Build angeben)" },
    Entry { key: "slot.stock_restore", en: "restore flow owns stock returns (backup-gated)", de: "Stock-R\u{00fc}ckkehr geh\u{00f6}rt zum Restore-Flow (Backup-gesch\u{00fc}tzt)" },
    Entry { key: "tools.missing_native", en: "MISSING (native protocol: Phase 8)", de: "FEHLT (natives Protokoll: Phase 8)" },
    Entry { key: "detect.adb_failed", en: "adb devices failed", de: "adb-Geräteabfrage fehlgeschlagen" },
    Entry { key: "detect.fastboot_failed", en: "fastboot devices failed", de: "fastboot-Geräteabfrage fehlgeschlagen" },
    Entry { key: "detect.version", en: "version:", de: "Version:" },
    Entry { key: "tools.extractor_present", en: "present", de: "vorhanden" },
    // German pending on purpose: exercises the English fallback in tests/UI.
    Entry { key: "tools.no_extractor", en: "extractor: none found (place HuaweiFirmwareExtractor/payload-dumper-go into data/tools/)", de: "" },
];

/// Look up `key` for `lang`.
///
/// * German with a missing/empty `de` entry falls back to English.
/// * Unknown keys return `"missing message"` (static English marker, never
///   empty, never panics) so a missing entry stays visible in the UI.
pub fn lookup(lang: Language, key: &str) -> &'static str {
    let k = key.trim();
    for e in MESSAGES {
        if e.key == k {
            match lang {
                Language::En => return e.en,
                Language::De => {
                    if e.de.is_empty() {
                        return e.en;
                    }
                    return e.de;
                }
            }
        }
    }
    "missing message"
}

/// English string for `key`, or `"missing message"` for unknown keys.
pub fn english(key: &str) -> &'static str {
    lookup(Language::En, key)
}

/// True when `key` exists in the table.
pub fn is_known(key: &str) -> bool {
    let k = key.trim();
    for e in MESSAGES {
        if e.key == k {
            return true;
        }
    }
    false
}

/// Number of message keys in the table.
pub fn key_count() -> usize {
    MESSAGES.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn language_code_roundtrip() {
        assert_eq!(Language::En.code(), "en");
        assert_eq!(Language::De.code(), "de");
        assert_eq!(Language::from_code("de"), Language::De);
        assert_eq!(Language::from_code("DE"), Language::De);
        assert_eq!(Language::from_code("de-DE"), Language::De);
        assert_eq!(Language::from_code("en"), Language::En);
        assert_eq!(Language::from_code(""), Language::En);
        assert_eq!(Language::from_code("fr"), Language::En);
    }

    #[test]
    fn lookup_matrix_english_matches_core() {
        // Exact English strings where core messages exist.
        assert_eq!(lookup(Language::En, "analyze.no_file"), "no file given");
        assert_eq!(lookup(Language::En, "analyze.no_container"), "no container detected");
        assert_eq!(
            lookup(Language::En, "analyze.poc_pending"),
            "hardware POC pending: normal power-boot with active root not yet proven"
        );
        assert_eq!(lookup(Language::En, "patch.refused"), "EXPERIMENTAL \u{2014} refused.");
        assert_eq!(
            lookup(Language::En, "patch.poc_plan"),
            "See devices/huawei-p10 for the POC plan."
        );
        assert_eq!(lookup(Language::En, "update.latest"), "latest:");
        assert_eq!(lookup(Language::En, "update.check_failed"), "check failed:");
        assert_eq!(
            lookup(Language::En, "slot.shared"),
            "TWRP and Magisk share the recovery_ramdisk slot (mutual overwrite)."
        );
        assert_eq!(
            lookup(Language::En, "device.huawei_denied"),
            "Command not allowed (Huawei getvar refused, NOT auto locked)"
        );
        assert_eq!(lookup(Language::En, "detect.adb_failed"), "adb devices failed");
        assert_eq!(
            lookup(Language::En, "detect.fastboot_failed"),
            "fastboot devices failed"
        );
        assert_eq!(lookup(Language::En, "detect.version"), "version:");
        assert_eq!(lookup(Language::En, "tools.extractor_present"), "present");
    }

    #[test]
    fn lookup_matrix_german_present() {
        assert_eq!(lookup(Language::De, "analyze.no_file"), "keine Datei angegeben");
        assert_eq!(lookup(Language::De, "device.no_device"), "Kein Ger\u{00e4}t erkannt");
        assert_eq!(lookup(Language::De, "patch.refused"), "EXPERIMENTELL \u{2014} abgelehnt.");
        assert_eq!(lookup(Language::De, "update.latest"), "neueste:");
        assert_eq!(
            lookup(Language::De, "slot.shared"),
            "TWRP und Magisk teilen sich den recovery_ramdisk-Slot (gegenseitiges \u{00dc}berschreiben)."
        );
        assert_eq!(
            lookup(Language::De, "detect.adb_failed"),
            "adb-Geräteabfrage fehlgeschlagen"
        );
        assert_eq!(
            lookup(Language::De, "detect.fastboot_failed"),
            "fastboot-Geräteabfrage fehlgeschlagen"
        );
        assert_eq!(lookup(Language::De, "detect.version"), "Version:");
        assert_eq!(lookup(Language::De, "tools.extractor_present"), "vorhanden");
    }

    #[test]
    fn fallback_missing_german_returns_english() {
        // `tools.no_extractor` has an empty `de` entry on purpose.
        let en = lookup(Language::En, "tools.no_extractor");
        assert!(!en.is_empty());
        assert_eq!(lookup(Language::De, "tools.no_extractor"), en);
        assert_eq!(english("tools.no_extractor"), en);
    }

    #[test]
    fn fallback_unknown_key_never_empty() {
        let got = lookup(Language::De, "does.not.exist");
        assert!(!got.is_empty());
        let got_en = lookup(Language::En, "does.not.exist");
        assert!(!got_en.is_empty());
        assert!(!is_known("does.not.exist"));
    }

    #[test]
    fn table_shape_stays_tight() {
        let n = key_count();
        assert!(n >= 28 && n <= 40, "want ~30 keys, got {n}");
        for e in MESSAGES {
            assert!(!e.key.is_empty());
            assert!(!e.en.is_empty(), "empty en for {}", e.key);
            assert!(is_known(e.key));
            assert!(!lookup(Language::En, e.key).is_empty());
            assert!(!lookup(Language::De, e.key).is_empty());
        }
        assert!(is_known("analyze.no_file"));
        assert!(is_known("tools.no_extractor"));
        assert!(is_known("detect.adb_failed"));
        assert!(is_known("detect.fastboot_failed"));
        assert!(is_known("detect.version"));
        assert!(is_known("tools.extractor_present"));
    }

    #[test]
    fn detect_tools_flows_have_keys() {
        // Detect flow (mirrors `detect_text` prefixes): both languages non-empty.
        for key in [
            "detect.adb_failed",
            "detect.fastboot_failed",
            "detect.version",
            "device.no_devices_tools",
            "device.adb_missing_managed",
            "device.fastboot_missing_managed",
        ] {
            assert!(is_known(key), "missing Detect key {key}");
            assert!(!lookup(Language::En, key).is_empty());
            assert!(!lookup(Language::De, key).is_empty());
        }
        // Tools flow (mirrors `tools_text` markers): both languages non-empty.
        for key in [
            "tools.missing_native",
            "tools.extractor_present",
            "tools.no_extractor",
        ] {
            assert!(is_known(key), "missing Tools key {key}");
            assert!(!lookup(Language::En, key).is_empty());
            assert!(!lookup(Language::De, key).is_empty());
        }
        // GUI `detect_text`/`tools_text` prefixes stay covered by the table.
        assert!(lookup(Language::En, "detect.adb_failed").contains("adb devices failed"));
        assert!(lookup(Language::En, "detect.fastboot_failed").contains("fastboot devices failed"));
        assert_eq!(lookup(Language::En, "tools.missing_native"), "MISSING (native protocol: Phase 8)");
    }

    #[test]
    fn detect_never_panics() {
        let _ = Language::detect();
    }
}
