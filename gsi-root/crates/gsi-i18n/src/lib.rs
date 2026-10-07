//! Tiny bilingual (en/de) message table for gsi-root flows.
//!
//! Mirrors the PS1 `L "en" "de"` helper: German only when the UI language
//! is German, otherwise English. No external deps, no I/O, no panics.
//!
//! Coverage: core flow keys (analyze / patch / update / flash / device /
//! slot / detect / tools) plus `ui` chrome keys (nav / section / buttons /
//! status / dashboard / wizard / hints) mirroring `gsi-root-gui/ui/app.slint`.
//! English strings match current core/CLI/GUI messages where they exist
//! (prefixes for parameterized lines). German uses du-form per PS1 `L()` tone.
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

/// Message table (core flows + `ui` chrome, batch 2 center/wizard/dashboard/dialog/group).
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
    // UI chrome: 34 page labels mirroring gsi-root-gui/ui/app.slint nav.
    Entry { key: "ui.nav.dashboard", en: "Dashboard", de: "Dashboard" },
    Entry { key: "ui.nav.gsi", en: "GSI", de: "GSI" },
    Entry { key: "ui.nav.detect", en: "Detect", de: "Erkennung" },
    Entry { key: "ui.nav.analyze_device", en: "Analyze Device", de: "Gerät analysieren" },
    Entry { key: "ui.nav.tools", en: "Tools", de: "Werkzeuge" },
    Entry { key: "ui.nav.updates", en: "Updates", de: "Aktualisierungen" },
    Entry { key: "ui.nav.logs", en: "Logs", de: "Protokolle" },
    Entry { key: "ui.nav.settings", en: "Settings", de: "Einstellungen" },
    Entry { key: "ui.nav.flash", en: "Flash", de: "Flashen" },
    Entry { key: "ui.nav.flash_system", en: "Flash System", de: "System flashen" },
    Entry { key: "ui.nav.wipe", en: "Wipe", de: "Löschen" },
    Entry { key: "ui.nav.unlock", en: "Unlock", de: "Entsperren" },
    Entry { key: "ui.nav.verify", en: "Verify", de: "Prüfen" },
    Entry { key: "ui.nav.preflight", en: "Preflight", de: "Vorprüfung" },
    Entry { key: "ui.nav.backup", en: "Backup", de: "Sicherung" },
    Entry { key: "ui.nav.restore", en: "Restore", de: "Wiederherstellen" },
    Entry { key: "ui.nav.reinstall", en: "Reinstall", de: "Neu installieren" },
    Entry { key: "ui.nav.resume", en: "Resume", de: "Fortsetzen" },
    Entry { key: "ui.nav.bootkeys", en: "Bootkeys", de: "Boot-Tasten" },
    Entry { key: "ui.nav.extract", en: "Extract", de: "Extrahieren" },
    Entry { key: "ui.nav.download", en: "Download", de: "Herunterladen" },
    Entry { key: "ui.nav.firmware", en: "Firmware", de: "Firmware" },
    Entry { key: "ui.nav.export", en: "Export", de: "Export" },
    Entry { key: "ui.nav.kernel", en: "Kernel", de: "Kernel" },
    Entry { key: "ui.nav.twrp", en: "TWRP", de: "TWRP" },
    Entry { key: "ui.nav.compat", en: "Compatibility", de: "Kompatibilität" },
    Entry { key: "ui.nav.goals", en: "Goals", de: "Ziele" },
    Entry { key: "ui.nav.rootmethods", en: "Root Methods", de: "Root-Methoden" },
    Entry { key: "ui.nav.patch", en: "Patch", de: "Patch" },
    Entry { key: "ui.nav.persist", en: "Persist", de: "Persistenz" },
    Entry { key: "ui.nav.romselect", en: "System", de: "System" },
    Entry { key: "ui.nav.help", en: "Help", de: "Hilfe" },
    Entry { key: "ui.nav.status", en: "Status", de: "Status" },
    Entry { key: "ui.nav.wizard", en: "Wizard", de: "Assistent" },
    // UI chrome: 5 sidebar section headers.
    Entry { key: "ui.section.overview", en: "Overview", de: "Übersicht" },
    Entry { key: "ui.section.check", en: "Check", de: "Prüfung" },
    Entry { key: "ui.section.install", en: "Install", de: "Installation" },
    Entry { key: "ui.section.device", en: "Device", de: "Gerät" },
    Entry { key: "ui.section.more", en: "More", de: "Mehr" },
    // UI chrome: buttons.
    Entry { key: "ui.btn.refresh", en: "Refresh", de: "Aktualisieren" },
    Entry { key: "ui.btn.next", en: "Next", de: "Weiter" },
    Entry { key: "ui.btn.back", en: "Back", de: "Zurück" },
    Entry { key: "ui.btn.start", en: "Start", de: "Start" },
    Entry { key: "ui.btn.start_wizard", en: "Start Wizard", de: "Assistent starten" },
    Entry { key: "ui.btn.check_now", en: "Check now", de: "Jetzt prüfen" },
    Entry { key: "ui.btn.close", en: "Close", de: "Schließen" },
    Entry { key: "ui.btn.analyze", en: "Analyze", de: "Analysieren" },
    Entry { key: "ui.btn.patch", en: "Patch", de: "Patchen" },
    // UI chrome: status words.
    Entry { key: "ui.status.ok", en: "OK", de: "OK" },
    Entry { key: "ui.status.failed", en: "Failed", de: "Fehlgeschlagen" },
    Entry { key: "ui.status.ready", en: "Ready", de: "Bereit" },
    Entry { key: "ui.status.blocked", en: "Blocked", de: "Blockiert" },
    Entry { key: "ui.status.unknown", en: "Unknown", de: "Unbekannt" },
    Entry { key: "ui.status.required", en: "Required", de: "Erforderlich" },
    Entry { key: "ui.status.optional", en: "Optional", de: "Optional" },
    Entry { key: "ui.status.warning", en: "Warning", de: "Warnung" },
    Entry { key: "ui.status.error", en: "Error", de: "Fehler" },
    Entry { key: "ui.status.success", en: "Success", de: "Erfolg" },
    // UI chrome: dashboard headings.
    Entry { key: "ui.dashboard.title", en: "Dashboard", de: "Dashboard" },
    Entry { key: "ui.dashboard.target", en: "Huawei P10 (VTR-L09 / VTR-L29) \u{2014} first hardware target.", de: "Huawei P10 (VTR-L09 / VTR-L29) \u{2014} erstes Hardware-Ziel." },
    Entry { key: "ui.dashboard.reference", en: "Reference GSI: lineage-20.0-20251021-UNOFFICIAL-arm64_bgN-signed", de: "Referenz-GSI: lineage-20.0-20251021-UNOFFICIAL-arm64_bgN-signed" },
    Entry { key: "ui.dashboard.note", en: "Analyze works locally (no writes). Patch/verify refuse honestly until proven on hardware.", de: "Analyse läuft lokal (ohne Schreiben). Patch/Prüfung werden ehrlich verweigert, bis es auf Hardware bewiesen ist." },
    // UI chrome: wizard steps.
    Entry { key: "ui.wizard.title", en: "Wizard", de: "Assistent" },
    Entry { key: "ui.wizard.system", en: "System", de: "System" },
    Entry { key: "ui.wizard.goal", en: "Goal", de: "Ziel" },
    Entry { key: "ui.wizard.backup", en: "Backup", de: "Sicherung" },
    Entry { key: "ui.wizard.patch", en: "Patch", de: "Patchen" },
    Entry { key: "ui.wizard.flash", en: "Flash", de: "Flashen" },
    Entry { key: "ui.wizard.verify", en: "Verify", de: "Prüfen" },
    // UI chrome: generic hints (du-form where imperative).
    Entry { key: "ui.hint.next_step", en: "Next step", de: "Nächster Schritt" },
    Entry { key: "ui.hint.see_logs", en: "See logs", de: "Schau in die Logs" },
    Entry { key: "ui.hint.no_device", en: "No device found", de: "Kein Gerät gefunden" },
    Entry { key: "ui.hint.tools_missing", en: "Tools missing", de: "Tools fehlen" },
    // Batch 2: sidebar groups (mirror app.slint Overview/Check/Install/Device/More).
    Entry { key: "ui.group.overview", en: "Overview", de: "Übersicht" },
    Entry { key: "ui.group.check", en: "Check", de: "Prüfung" },
    Entry { key: "ui.group.install", en: "Install", de: "Installation" },
    Entry { key: "ui.group.device", en: "Device", de: "Gerät" },
    Entry { key: "ui.group.more", en: "More", de: "Mehr" },
    // Batch 2: dialogs (mirror components.slint SafetyDialog/stepper/empty-state).
    Entry { key: "ui.dialog.cancel", en: "Cancel", de: "Abbrechen" },
    Entry { key: "ui.dialog.continue", en: "Continue", de: "Fortsetzen" },
    Entry { key: "ui.dialog.type_confirm", en: "Type the phrase to confirm:", de: "Gib die Phrase zur Bestätigung ein:" },
    Entry { key: "ui.dialog.matched", en: "Phrase matches. Continue is enabled.", de: "Phrase stimmt. Weiter ist aktiviert." },
    Entry { key: "ui.dialog.not_matched", en: "Phrase does not match yet.", de: "Phrase stimmt noch nicht." },
    Entry { key: "ui.dialog.safety_title", en: "Safety check", de: "Sicherheitsprüfung" },
    Entry { key: "ui.dialog.safety_message", en: "Backup plus exact phrase required. Continue only on exact match.", de: "Sicherung plus exakte Phrase erforderlich. Weiter nur bei exakter Übereinstimmung." },
    Entry { key: "ui.dialog.step_done", en: "[done]", de: "[fertig]" },
    Entry { key: "ui.dialog.step_current", en: "[current]", de: "[aktuell]" },
    Entry { key: "ui.dialog.step_pending", en: "[pending]", de: "[ausstehend]" },
    Entry { key: "ui.dialog.empty_title", en: "No data yet", de: "Noch keine Daten" },
    Entry { key: "ui.dialog.empty_action", en: "Refresh now", de: "Jetzt aktualisieren" },
    // Batch 2: dashboard (mirror lib.rs dashboard/recommend plus app.slint cards).
    Entry { key: "ui.dashboard.device_title", en: "Device", de: "Gerät" },
    Entry { key: "ui.dashboard.device_subtitle", en: "Live values from status, detect and analysis refresh. Unknown means not measured, never guessed.", de: "Live-Werte aus Status-, Erkennungs- und Analyse-Aktualisierung. Unbekannt heißt nicht gemessen, nie geraten." },
    Entry { key: "ui.dashboard.recommended_title", en: "Recommended action", de: "Empfohlene Aktion" },
    Entry { key: "ui.dashboard.recommended_subtitle", en: "Honest next step based on live state.", de: "Ehrlicher nächster Schritt auf Basis des Live-Zustands." },
    Entry { key: "ui.dashboard.quick_title", en: "Quick actions", de: "Schnellaktionen" },
    Entry { key: "ui.dashboard.quick_subtitle", en: "Jump to the most used pages.", de: "Springe zu den meistgenutzten Seiten." },
    Entry { key: "ui.dashboard.refresh_device", en: "Refresh device", de: "Gerät aktualisieren" },
    Entry { key: "ui.dashboard.go_detect", en: "Go to Detect", de: "Zu Erkennung" },
    Entry { key: "ui.dashboard.no_data_title", en: "No device data yet", de: "Noch keine Gerätedaten" },
    Entry { key: "ui.dashboard.no_data_message", en: "No live values loaded. Start with Detect to check ADB and fastboot.", de: "Keine Live-Werte geladen. Beginne mit Erkennung, um ADB und Fastboot zu prüfen." },
    Entry { key: "ui.dashboard.open_wizard", en: "Open Wizard", de: "Assistent öffnen" },
    Entry { key: "ui.dashboard.open_preflight", en: "Open Preflight", de: "Vorprüfung öffnen" },
    Entry { key: "ui.dashboard.open_backup", en: "Open Backup", de: "Sicherung öffnen" },
    Entry { key: "ui.dashboard.default_recommend", en: "Run Detect, then Preflight, then follow the Wizard. Backup before any flash.", de: "Führe Erkennung, dann Vorprüfung aus, dann folge dem Assistenten. Sichere vor jedem Flash." },
    Entry { key: "ui.dashboard.adb_unknown", en: "ADB: Unknown - no live query yet", de: "ADB: Unbekannt - noch keine Live-Abfrage" },
    Entry { key: "ui.dashboard.root_unknown", en: "Root: Unknown - needs device verify (uid=0 only)", de: "Root: Unbekannt - braucht Geräteprüfung (nur uid=0)" },
    Entry { key: "ui.dashboard.selinux_unknown", en: "SELinux: Unknown - needs device", de: "SELinux: Unbekannt - braucht Gerät" },
    Entry { key: "ui.dashboard.recommend_detect", en: "Recommended: run Detect, then Preflight, then Back up before any flash.", de: "Empfohlen: Führe Erkennung, dann Vorprüfung aus, dann sichere vor jedem Flash." },
    Entry { key: "ui.dashboard.recommend_pick_goal", en: "Recommended: backup present. Pick a goal in the Wizard.", de: "Empfohlen: Sicherung vorhanden. Wähle ein Ziel im Assistenten." },
    Entry { key: "ui.dashboard.recommend_follow", en: "Recommended: backup present and goal saved. Follow the Wizard plan.", de: "Empfohlen: Sicherung vorhanden und Ziel gespeichert. Folge dem Assistenten-Plan." },
    // Batch 2: wizard steps (mirror app.slint Stepper + lib.rs wizard/execute gates).
    Entry { key: "ui.wizard.detect", en: "Detect", de: "Erkennung" },
    Entry { key: "ui.wizard.method", en: "Method", de: "Methode" },
    Entry { key: "ui.wizard.preflight", en: "Preflight", de: "Vorprüfung" },
    Entry { key: "ui.wizard.plan", en: "Plan", de: "Plan" },
    Entry { key: "ui.wizard.confirm", en: "Confirm", de: "Bestätigen" },
    Entry { key: "ui.wizard.execute", en: "Execute", de: "Ausführen" },
    Entry { key: "ui.wizard.subtitle", en: "Guided path: goal, detect, method, preflight, plan, typed confirm, execute, verify. Plan-only where EXPERIMENTAL.", de: "Geführter Pfad: Ziel, Erkennung, Methode, Vorprüfung, Plan, getippte Bestätigung, Ausführung, Prüfung. Nur Plan, wo EXPERIMENTELL." },
    Entry { key: "ui.wizard.destructive_note", en: "Every destructive step needs a backup plus the exact typed phrase. Without both, execution stays refused and plan-only.", de: "Jeder destruktive Schritt braucht eine Sicherung plus die exakte getippte Phrase. Ohne beides bleibt die Ausführung verweigert und nur Plan." },
    Entry { key: "ui.wizard.step1_title", en: "Step 1: Choose a goal", de: "Schritt 1: Wähle ein Ziel" },
    Entry { key: "ui.wizard.step2_title", en: "Step 2: Detect", de: "Schritt 2: Erkennung" },
    Entry { key: "ui.wizard.step3_title", en: "Step 3: Method", de: "Schritt 3: Methode" },
    Entry { key: "ui.wizard.step4_title", en: "Step 4: Preflight", de: "Schritt 4: Vorprüfung" },
    Entry { key: "ui.wizard.step5_title", en: "Step 5: Plan", de: "Schritt 5: Plan" },
    Entry { key: "ui.wizard.step6_title", en: "Step 6: Typed confirmation", de: "Schritt 6: Getippte Bestätigung" },
    Entry { key: "ui.wizard.step7_title", en: "Step 7: Execute (plan-only)", de: "Schritt 7: Ausführen (nur Plan)" },
    Entry { key: "ui.wizard.step8_title", en: "Step 8: Verify", de: "Schritt 8: Prüfen" },
    Entry { key: "ui.wizard.step1_sub", en: "Goal select renders the existing goals plan text.", de: "Zielauswahl zeigt den vorhandenen Ziele-Plantext." },
    Entry { key: "ui.wizard.step2_sub", en: "Detect renders the existing Detect page text.", de: "Erkennung zeigt den vorhandenen Erkennungs-Seitentext." },
    Entry { key: "ui.wizard.step3_sub", en: "Method renders the existing Root Methods page text.", de: "Methode zeigt den vorhandenen Root-Methoden-Seitentext." },
    Entry { key: "ui.wizard.step4_sub", en: "Preflight renders the existing Preflight page text.", de: "Vorprüfung zeigt den vorhandenen Vorprüfungs-Seitentext." },
    Entry { key: "ui.wizard.step5_sub", en: "Plan renders the existing Wizard and Goals page texts.", de: "Plan zeigt die vorhandenen Assistenten- und Ziele-Seitentexte." },
    Entry { key: "ui.wizard.safety_message", en: "Backup gate plus exact phrase required. Continue enables only on exact match. Plan-only where EXPERIMENTAL.", de: "Backup-Schranke plus exakte Phrase erforderlich. Weiter nur bei exakter Übereinstimmung. Nur Plan, wo EXPERIMENTELL." },
    Entry { key: "ui.wizard.backup_title", en: "Backup gate", de: "Backup-Schranke" },
    Entry { key: "ui.wizard.backup_sub", en: "Execute needs a backup with original.img. Checked via the existing restore plan.", de: "Ausführen braucht eine Sicherung mit original.img. Geprüft über den vorhandenen Restore-Plan." },
    Entry { key: "ui.wizard.backup_unchecked_title", en: "Backup not checked", de: "Sicherung nicht geprüft" },
    Entry { key: "ui.wizard.backup_unchecked_msg", en: "Press Check backup. Without a backup with original.img, Execute stays blocked.", de: "Drücke Sicherung prüfen. Ohne Sicherung mit original.img bleibt Ausführen blockiert." },
    Entry { key: "ui.wizard.execute_note", en: "Honest execution state: flashing is plan-only until hardware POC (Phase 8). Nothing is written from here.", de: "Ehrlicher Ausführungszustand: Flashen ist nur Plan bis zum Hardware-POC (Phase 8). Von hier wird nichts geschrieben." },
    Entry { key: "ui.wizard.backup_missing", en: "missing: no backup with original.img found - back up first", de: "fehlt: keine Sicherung mit original.img gefunden - sichere zuerst" },
    Entry { key: "ui.wizard.gate_note", en: "gate: typed phrase plus backup both required before Execute", de: "Schranke: getippte Phrase plus Sicherung, beides vor Ausführen erforderlich" },
    Entry { key: "ui.wizard.plan_only_note", en: "note: plan-only, nothing executed (EXPERIMENTAL, Phase 8)", de: "Hinweis: nur Plan, nichts ausgeführt (EXPERIMENTELL, Phase 8)" },
    Entry { key: "ui.wizard.refused_mismatch", en: "refused: typed phrase does not match - Execute stays blocked", de: "verweigert: getippte Phrase stimmt nicht - Ausführen bleibt blockiert" },
    Entry { key: "ui.wizard.refused_no_phrase", en: "refused: no expected phrase set", de: "verweigert: keine erwartete Phrase gesetzt" },
    Entry { key: "ui.wizard.confirmed", en: "confirmed: phrase matches and backup present", de: "bestätigt: Phrase stimmt und Sicherung vorhanden" },
    // Batch 2: centers (mirror center_*.rs titles/hints/refusals + center_*.slint headers).
    Entry { key: "ui.center.device_title", en: "Device center (read-only, nothing executed)", de: "Geräte-Center (nur Lesen, nichts ausgeführt)" },
    Entry { key: "ui.center.device_note_masked", en: "note: serials are masked; full values never shown here", de: "Hinweis: Seriennummern sind maskiert; volle Werte werden hier nie gezeigt" },
    Entry { key: "ui.center.device_refused", en: "refused: live property reads need a device; this view only renders injected values", de: "verweigert: Live-Eigenschaftsabfragen brauchen ein Gerät; diese Ansicht zeigt nur injizierte Werte" },
    Entry { key: "ui.center.detect_title", en: "Detect center (read-only queries, nothing executed)", de: "Erkennungs-Center (Nur-Lese-Abfragen, nichts ausgeführt)" },
    Entry { key: "ui.center.detect_hint", en: "hint: connect a device, authorize the RSA prompt, or reboot to fastboot", de: "Hinweis: Schließe ein Gerät an, bestätige die RSA-Abfrage oder starte in Fastboot neu" },
    Entry { key: "ui.center.diag_title", en: "Diagnostics center (read-only, nothing executed)", de: "Diagnose-Center (nur Lesen, nichts ausgeführt)" },
    Entry { key: "ui.center.images_title", en: "Images center (read-only, nothing executed)", de: "Bilder-Center (nur Lesen, nichts ausgeführt)" },
    Entry { key: "ui.center.images_refused", en: "refused: no download or extraction started here", de: "verweigert: kein Download oder Entpacken startet hier" },
    Entry { key: "ui.center.analyzer_title", en: "Analyzer center (read-only, nothing executed)", de: "Analyse-Center (nur Lesen, nichts ausgeführt)" },
    Entry { key: "ui.center.analyzer_drop_note", en: "note: drag & drop is not supported in Slint; paste the path into the LineEdit", de: "Hinweis: Drag & Drop wird in Slint nicht unterstützt; füge den Pfad in das LineEdit ein" },
    Entry { key: "ui.center.compat_title", en: "Compat center (read-only, nothing executed)", de: "Kompatibilitäts-Center (nur Lesen, nichts ausgeführt)" },
    Entry { key: "ui.center.compat_refused", en: "refused: flashing or downloading from a verdict alone is Phase 8 shell work", de: "verweigert: Flashen oder Laden aus einem Urteil allein ist Phase-8-Shell-Arbeit" },
    Entry { key: "ui.center.root_title", en: "Root status (read-only, nothing executed)", de: "Root-Status (nur Lesen, nichts ausgeführt)" },
    Entry { key: "ui.center.root_experimental", en: "EXPERIMENTAL - no hardware POC. Nothing here means Ready.", de: "EXPERIMENTELL - kein Hardware-POC. Nichts hier heißt Bereit." },
    Entry { key: "ui.center.root_bootkeys", en: "Rooted boot needs the key trick every time: Vol-Up + Power until Huawei logo, then release.", de: "Jeder Root-Boot braucht jedes Mal den Tastentrick: Lauter + Power bis zum Huawei-Logo, dann loslassen." },
    Entry { key: "ui.center.methods_title", en: "Root methods (read-only cards, nothing executed)", de: "Root-Methoden (Nur-Lese-Karten, nichts ausgeführt)" },
    Entry { key: "ui.center.magisk_title", en: "Magisk (read-only plan, nothing executed)", de: "Magisk (Nur-Lese-Plan, nichts ausgeführt)" },
    Entry { key: "ui.center.patch_title", en: "Patch center (read-only, nothing executed)", de: "Patch-Center (nur Lesen, nichts ausgeführt)" },
    Entry { key: "ui.center.verify_title", en: "Verify root center (read-only, nothing executed)", de: "Root-Prüf-Center (nur Lesen, nichts ausgeführt)" },
    Entry { key: "ui.center.flash_title", en: "Flash flow (read-only, nothing executed)", de: "Flash-Ablauf (nur Lesen, nichts ausgeführt)" },
    Entry { key: "ui.center.flash_steps", en: "10 steps: device / image / analyze / target / compat / backup / safety / confirm / execute / verify.", de: "10 Schritte: Gerät / Bild / Analyse / Ziel / Kompatibilität / Sicherung / Sicherheit / Bestätigung / Ausführung / Prüfung." },
    Entry { key: "ui.center.flash_refused", en: "refused: live fastboot flash is Phase 8 work; plan only, nothing written", de: "verweigert: Live-Fastboot-Flash ist Phase-8-Arbeit; nur Plan, nichts geschrieben" },
    Entry { key: "ui.center.progress_title", en: "Flash progress (read-only monitor, nothing executed)", de: "Flash-Fortschritt (Nur-Lese-Monitor, nichts ausgeführt)" },
    Entry { key: "ui.center.progress_cancel", en: "Cancel note: cancel is UNSAFE once flash starts (partial write possible); power loss or cable pull can brick the slot. Restore path only.", de: "Abbruch-Hinweis: Abbrechen ist UNSICHER, sobald der Flash startet (Teilschreibung möglich); Stromverlust oder Kabelabzug kann den Slot bricken. Nur Restore-Pfad." },
    Entry { key: "ui.center.system_flash_title", en: "System flash center (read-only, nothing executed)", de: "System-Flash-Center (nur Lesen, nichts ausgeführt)" },
    Entry { key: "ui.center.wipe_title", en: "Wipe center (read-only, nothing executed)", de: "Wipe-Center (nur Lesen, nichts ausgeführt)" },
    Entry { key: "ui.center.backup_title", en: "Backup center (read-only, nothing executed)", de: "Backup-Center (nur Lesen, nichts ausgeführt)" },
    Entry { key: "ui.center.backup_empty", en: "No backups yet", de: "Noch keine Sicherungen" },
    Entry { key: "ui.center.backup_cta", en: "Create CTA: open Create Backup to plan the first backup (back up the base before any flash)", de: "Erstellen-Hinweis: Öffne Backup erstellen, um die erste Sicherung zu planen (sichere die Basis vor jedem Flash)" },
    Entry { key: "ui.center.create_title", en: "Create backup (read-only plan, nothing executed)", de: "Backup erstellen (Nur-Lese-Plan, nichts ausgeführt)" },
    Entry { key: "ui.center.restore_title", en: "Restore (verify-first flow, read-only, nothing executed)", de: "Wiederherstellen (Verify-zuerst-Ablauf, nur Lesen, nichts ausgeführt)" },
    Entry { key: "ui.center.resume_title", en: "Resume center (read-only, nothing executed)", de: "Fortsetzen-Center (nur Lesen, nichts ausgeführt)" },
    Entry { key: "ui.center.firmware_title", en: "Firmware center (read-only, nothing executed)", de: "Firmware-Center (nur Lesen, nichts ausgeführt)" },
    Entry { key: "ui.center.download_title", en: "Download center (options preview; execution note below)", de: "Download-Center (Optionenvorschau; Ausführungshinweis unten)" },
    Entry { key: "ui.center.download_refused", en: "refused: no download started here", de: "verweigert: kein Download startet hier" },
    Entry { key: "ui.center.extract_title", en: "Extract center (read-only, nothing executed)", de: "Extraktions-Center (nur Lesen, nichts ausgeführt)" },
    Entry { key: "ui.center.export_title", en: "Export center (read-only, nothing executed)", de: "Export-Center (nur Lesen, nichts ausgeführt)" },
    Entry { key: "ui.center.hash_note", en: "hash note: SHA-256 saved as <file>.sha256 sidecar (shell-side)", de: "Hash-Hinweis: SHA-256 als <Datei>.sha256-Sidecar gespeichert (shell-seitig)" },
    Entry { key: "ui.center.tools_title", en: "Tools center (read-only, nothing executed)", de: "Werkzeug-Center (nur Lesen, nichts ausgeführt)" },
    Entry { key: "ui.center.updates_title", en: "Updates center (read-only preview; Check runs shell-side)", de: "Aktualisierungs-Center (Nur-Lese-Vorschau; Prüfung läuft shell-seitig)" },
    Entry { key: "ui.center.logs_title", en: "Logs center (read-only, nothing cleared or exported here)", de: "Log-Center (nur Lesen, nichts gelöscht oder exportiert)" },
    Entry { key: "ui.center.settings_title", en: "Settings center (read-only display, nothing written)", de: "Einstellungs-Center (Nur-Lese-Anzeige, nichts geschrieben)" },
    Entry { key: "ui.center.help_title", en: "Help center (read-only)", de: "Hilfe-Center (nur Lesen)" },
    Entry { key: "ui.center.help_quickstart", en: "quickstart: Detect, Analyze Device, Backup, Flash plan preview only", de: "Schnellstart: Erkennung, Geräteanalyse, Sicherung, nur Flash-Planvorschau" },
    Entry { key: "ui.center.help_troubleshooting", en: "troubleshooting: cable, drivers, PATH, Command not allowed, too large", de: "Fehlerbehebung: Kabel, Treiber, PATH, Befehl nicht erlaubt, zu groß" },
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
        assert_eq!(
            lookup(Language::En, "analyze.no_container"),
            "no container detected"
        );
        assert_eq!(
            lookup(Language::En, "analyze.poc_pending"),
            "hardware POC pending: normal power-boot with active root not yet proven"
        );
        assert_eq!(
            lookup(Language::En, "patch.refused"),
            "EXPERIMENTAL \u{2014} refused."
        );
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
        assert_eq!(
            lookup(Language::En, "detect.adb_failed"),
            "adb devices failed"
        );
        assert_eq!(
            lookup(Language::En, "detect.fastboot_failed"),
            "fastboot devices failed"
        );
        assert_eq!(lookup(Language::En, "detect.version"), "version:");
        assert_eq!(lookup(Language::En, "tools.extractor_present"), "present");
    }

    #[test]
    fn lookup_matrix_german_present() {
        assert_eq!(
            lookup(Language::De, "analyze.no_file"),
            "keine Datei angegeben"
        );
        assert_eq!(
            lookup(Language::De, "device.no_device"),
            "Kein Ger\u{00e4}t erkannt"
        );
        assert_eq!(
            lookup(Language::De, "patch.refused"),
            "EXPERIMENTELL \u{2014} abgelehnt."
        );
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
        assert!((200..=250).contains(&n), "want 200-250 keys, got {n}");
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
        assert_eq!(
            lookup(Language::En, "tools.missing_native"),
            "MISSING (native protocol: Phase 8)"
        );
    }

    #[test]
    fn ui_chrome_matrix_english() {
        // Spot-check exact English chrome strings (mirrors app.slint labels).
        assert_eq!(lookup(Language::En, "ui.nav.dashboard"), "Dashboard");
        assert_eq!(
            lookup(Language::En, "ui.nav.analyze_device"),
            "Analyze Device"
        );
        assert_eq!(lookup(Language::En, "ui.nav.flash_system"), "Flash System");
        assert_eq!(lookup(Language::En, "ui.section.overview"), "Overview");
        assert_eq!(lookup(Language::En, "ui.btn.refresh"), "Refresh");
        assert_eq!(lookup(Language::En, "ui.btn.next"), "Next");
        assert_eq!(lookup(Language::En, "ui.btn.back"), "Back");
        assert_eq!(lookup(Language::En, "ui.btn.start_wizard"), "Start Wizard");
        assert_eq!(lookup(Language::En, "ui.btn.check_now"), "Check now");
        assert_eq!(lookup(Language::En, "ui.btn.close"), "Close");
        assert_eq!(lookup(Language::En, "ui.status.ready"), "Ready");
        assert_eq!(lookup(Language::En, "ui.status.blocked"), "Blocked");
        assert_eq!(lookup(Language::En, "ui.status.unknown"), "Unknown");
        assert_eq!(lookup(Language::En, "ui.dashboard.title"), "Dashboard");
        assert_eq!(lookup(Language::En, "ui.wizard.system"), "System");
        assert_eq!(lookup(Language::En, "ui.wizard.goal"), "Goal");
        assert_eq!(lookup(Language::En, "ui.wizard.verify"), "Verify");
        assert_eq!(lookup(Language::En, "ui.hint.next_step"), "Next step");
        assert_eq!(lookup(Language::En, "ui.hint.see_logs"), "See logs");
        assert_eq!(lookup(Language::En, "ui.hint.no_device"), "No device found");
        assert_eq!(
            lookup(Language::En, "ui.hint.tools_missing"),
            "Tools missing"
        );
    }

    #[test]
    fn ui_chrome_matrix_german() {
        // Spot-check exact German chrome strings (du-form where imperative).
        assert_eq!(lookup(Language::De, "ui.nav.dashboard"), "Dashboard");
        assert_eq!(
            lookup(Language::De, "ui.nav.analyze_device"),
            "Gerät analysieren"
        );
        assert_eq!(
            lookup(Language::De, "ui.nav.flash_system"),
            "System flashen"
        );
        assert_eq!(lookup(Language::De, "ui.section.overview"), "Übersicht");
        assert_eq!(lookup(Language::De, "ui.btn.refresh"), "Aktualisieren");
        assert_eq!(lookup(Language::De, "ui.btn.next"), "Weiter");
        assert_eq!(lookup(Language::De, "ui.btn.back"), "Zurück");
        assert_eq!(
            lookup(Language::De, "ui.btn.start_wizard"),
            "Assistent starten"
        );
        assert_eq!(lookup(Language::De, "ui.btn.check_now"), "Jetzt prüfen");
        assert_eq!(lookup(Language::De, "ui.btn.close"), "Schließen");
        assert_eq!(lookup(Language::De, "ui.status.ready"), "Bereit");
        assert_eq!(lookup(Language::De, "ui.status.blocked"), "Blockiert");
        assert_eq!(lookup(Language::De, "ui.status.unknown"), "Unbekannt");
        assert_eq!(lookup(Language::De, "ui.wizard.goal"), "Ziel");
        assert_eq!(lookup(Language::De, "ui.wizard.verify"), "Prüfen");
        assert_eq!(
            lookup(Language::De, "ui.hint.next_step"),
            "Nächster Schritt"
        );
        assert_eq!(
            lookup(Language::De, "ui.hint.see_logs"),
            "Schau in die Logs"
        );
        assert_eq!(
            lookup(Language::De, "ui.hint.no_device"),
            "Kein Gerät gefunden"
        );
        assert_eq!(
            lookup(Language::De, "ui.hint.tools_missing"),
            "Tools fehlen"
        );
    }

    #[test]
    fn ui_chrome_all_keys_present() {
        // Every new `ui` key: known, en + de present, lookup never empty.
        for key in [
            "ui.nav.dashboard",
            "ui.nav.gsi",
            "ui.nav.detect",
            "ui.nav.analyze_device",
            "ui.nav.tools",
            "ui.nav.updates",
            "ui.nav.logs",
            "ui.nav.settings",
            "ui.nav.flash",
            "ui.nav.flash_system",
            "ui.nav.wipe",
            "ui.nav.unlock",
            "ui.nav.verify",
            "ui.nav.preflight",
            "ui.nav.backup",
            "ui.nav.restore",
            "ui.nav.reinstall",
            "ui.nav.resume",
            "ui.nav.bootkeys",
            "ui.nav.extract",
            "ui.nav.download",
            "ui.nav.firmware",
            "ui.nav.export",
            "ui.nav.kernel",
            "ui.nav.twrp",
            "ui.nav.compat",
            "ui.nav.goals",
            "ui.nav.rootmethods",
            "ui.nav.patch",
            "ui.nav.persist",
            "ui.nav.romselect",
            "ui.nav.help",
            "ui.nav.status",
            "ui.nav.wizard",
            "ui.section.overview",
            "ui.section.check",
            "ui.section.install",
            "ui.section.device",
            "ui.section.more",
            "ui.btn.refresh",
            "ui.btn.next",
            "ui.btn.back",
            "ui.btn.start",
            "ui.btn.start_wizard",
            "ui.btn.check_now",
            "ui.btn.close",
            "ui.btn.analyze",
            "ui.btn.patch",
            "ui.status.ok",
            "ui.status.failed",
            "ui.status.ready",
            "ui.status.blocked",
            "ui.status.unknown",
            "ui.status.required",
            "ui.status.optional",
            "ui.status.warning",
            "ui.status.error",
            "ui.status.success",
            "ui.dashboard.title",
            "ui.dashboard.target",
            "ui.dashboard.reference",
            "ui.dashboard.note",
            "ui.wizard.title",
            "ui.wizard.system",
            "ui.wizard.goal",
            "ui.wizard.backup",
            "ui.wizard.patch",
            "ui.wizard.flash",
            "ui.wizard.verify",
            "ui.hint.next_step",
            "ui.hint.see_logs",
            "ui.hint.no_device",
            "ui.hint.tools_missing",
        ] {
            assert!(is_known(key), "missing ui key {key}");
            assert!(!lookup(Language::En, key).is_empty(), "empty en for {key}");
            assert!(!lookup(Language::De, key).is_empty(), "empty de for {key}");
            assert!(!english(key).is_empty(), "empty english() for {key}");
        }
    }

    #[test]
    fn batch2_matrix_english() {
        assert_eq!(lookup(Language::En, "ui.group.overview"), "Overview");
        assert_eq!(lookup(Language::En, "ui.group.check"), "Check");
        assert_eq!(lookup(Language::En, "ui.dialog.cancel"), "Cancel");
        assert_eq!(lookup(Language::En, "ui.dialog.continue"), "Continue");
        assert_eq!(
            lookup(Language::En, "ui.dialog.type_confirm"),
            "Type the phrase to confirm:"
        );
        assert_eq!(lookup(Language::En, "ui.dashboard.device_title"), "Device");
        assert_eq!(
            lookup(Language::En, "ui.dashboard.recommended_title"),
            "Recommended action"
        );
        assert_eq!(
            lookup(Language::En, "ui.dashboard.no_data_title"),
            "No device data yet"
        );
        assert_eq!(lookup(Language::En, "ui.wizard.detect"), "Detect");
        assert_eq!(
            lookup(Language::En, "ui.wizard.step1_title"),
            "Step 1: Choose a goal"
        );
        assert_eq!(
            lookup(Language::En, "ui.wizard.step6_title"),
            "Step 6: Typed confirmation"
        );
        assert_eq!(
            lookup(Language::En, "ui.wizard.backup_missing"),
            "missing: no backup with original.img found - back up first"
        );
        assert_eq!(
            lookup(Language::En, "ui.center.device_title"),
            "Device center (read-only, nothing executed)"
        );
        assert_eq!(
            lookup(Language::En, "ui.center.flash_steps"),
            "10 steps: device / image / analyze / target / compat / backup / safety / confirm / execute / verify."
        );
        assert_eq!(
            lookup(Language::En, "ui.center.backup_empty"),
            "No backups yet"
        );
        assert_eq!(
            lookup(Language::En, "ui.center.help_title"),
            "Help center (read-only)"
        );
    }

    #[test]
    fn batch2_matrix_german() {
        assert_eq!(lookup(Language::De, "ui.group.overview"), "Übersicht");
        assert_eq!(lookup(Language::De, "ui.group.device"), "Gerät");
        assert_eq!(lookup(Language::De, "ui.dialog.cancel"), "Abbrechen");
        assert_eq!(lookup(Language::De, "ui.dialog.continue"), "Fortsetzen");
        assert_eq!(
            lookup(Language::De, "ui.dialog.type_confirm"),
            "Gib die Phrase zur Bestätigung ein:"
        );
        assert_eq!(
            lookup(Language::De, "ui.dashboard.no_data_message"),
            "Keine Live-Werte geladen. Beginne mit Erkennung, um ADB und Fastboot zu prüfen."
        );
        assert_eq!(
            lookup(Language::De, "ui.dashboard.recommend_pick_goal"),
            "Empfohlen: Sicherung vorhanden. Wähle ein Ziel im Assistenten."
        );
        assert_eq!(
            lookup(Language::De, "ui.wizard.step1_title"),
            "Schritt 1: Wähle ein Ziel"
        );
        assert_eq!(
            lookup(Language::De, "ui.wizard.backup_unchecked_msg"),
            "Drücke Sicherung prüfen. Ohne Sicherung mit original.img bleibt Ausführen blockiert."
        );
        assert_eq!(
            lookup(Language::De, "ui.center.detect_hint"),
            "Hinweis: Schließe ein Gerät an, bestätige die RSA-Abfrage oder starte in Fastboot neu"
        );
        assert_eq!(
            lookup(Language::De, "ui.center.analyzer_drop_note"),
            "Hinweis: Drag & Drop wird in Slint nicht unterstützt; füge den Pfad in das LineEdit ein"
        );
        assert_eq!(
            lookup(Language::De, "ui.center.root_bootkeys"),
            "Jeder Root-Boot braucht jedes Mal den Tastentrick: Lauter + Power bis zum Huawei-Logo, dann loslassen."
        );
        assert_eq!(
            lookup(Language::De, "ui.center.backup_empty"),
            "Noch keine Sicherungen"
        );
        assert_eq!(
            lookup(Language::De, "ui.center.help_title"),
            "Hilfe-Center (nur Lesen)"
        );
    }

    #[test]
    fn batch2_all_keys_present() {
        for key in [
            "ui.group.overview",
            "ui.group.check",
            "ui.group.install",
            "ui.group.device",
            "ui.group.more",
            "ui.dialog.cancel",
            "ui.dialog.continue",
            "ui.dialog.type_confirm",
            "ui.dialog.matched",
            "ui.dialog.not_matched",
            "ui.dialog.safety_title",
            "ui.dialog.safety_message",
            "ui.dialog.step_done",
            "ui.dialog.step_current",
            "ui.dialog.step_pending",
            "ui.dialog.empty_title",
            "ui.dialog.empty_action",
            "ui.dashboard.device_title",
            "ui.dashboard.device_subtitle",
            "ui.dashboard.recommended_title",
            "ui.dashboard.recommended_subtitle",
            "ui.dashboard.quick_title",
            "ui.dashboard.quick_subtitle",
            "ui.dashboard.refresh_device",
            "ui.dashboard.go_detect",
            "ui.dashboard.no_data_title",
            "ui.dashboard.no_data_message",
            "ui.dashboard.open_wizard",
            "ui.dashboard.open_preflight",
            "ui.dashboard.open_backup",
            "ui.dashboard.default_recommend",
            "ui.dashboard.adb_unknown",
            "ui.dashboard.root_unknown",
            "ui.dashboard.selinux_unknown",
            "ui.dashboard.recommend_detect",
            "ui.dashboard.recommend_pick_goal",
            "ui.dashboard.recommend_follow",
            "ui.wizard.detect",
            "ui.wizard.method",
            "ui.wizard.preflight",
            "ui.wizard.plan",
            "ui.wizard.confirm",
            "ui.wizard.execute",
            "ui.wizard.subtitle",
            "ui.wizard.destructive_note",
            "ui.wizard.step1_title",
            "ui.wizard.step2_title",
            "ui.wizard.step3_title",
            "ui.wizard.step4_title",
            "ui.wizard.step5_title",
            "ui.wizard.step6_title",
            "ui.wizard.step7_title",
            "ui.wizard.step8_title",
            "ui.wizard.step1_sub",
            "ui.wizard.step2_sub",
            "ui.wizard.step3_sub",
            "ui.wizard.step4_sub",
            "ui.wizard.step5_sub",
            "ui.wizard.safety_message",
            "ui.wizard.backup_title",
            "ui.wizard.backup_sub",
            "ui.wizard.backup_unchecked_title",
            "ui.wizard.backup_unchecked_msg",
            "ui.wizard.execute_note",
            "ui.wizard.backup_missing",
            "ui.wizard.gate_note",
            "ui.wizard.plan_only_note",
            "ui.wizard.refused_mismatch",
            "ui.wizard.refused_no_phrase",
            "ui.wizard.confirmed",
            "ui.center.device_title",
            "ui.center.device_note_masked",
            "ui.center.device_refused",
            "ui.center.detect_title",
            "ui.center.detect_hint",
            "ui.center.diag_title",
            "ui.center.images_title",
            "ui.center.images_refused",
            "ui.center.analyzer_title",
            "ui.center.analyzer_drop_note",
            "ui.center.compat_title",
            "ui.center.compat_refused",
            "ui.center.root_title",
            "ui.center.root_experimental",
            "ui.center.root_bootkeys",
            "ui.center.methods_title",
            "ui.center.magisk_title",
            "ui.center.patch_title",
            "ui.center.verify_title",
            "ui.center.flash_title",
            "ui.center.flash_steps",
            "ui.center.flash_refused",
            "ui.center.progress_title",
            "ui.center.progress_cancel",
            "ui.center.system_flash_title",
            "ui.center.wipe_title",
            "ui.center.backup_title",
            "ui.center.backup_empty",
            "ui.center.backup_cta",
            "ui.center.create_title",
            "ui.center.restore_title",
            "ui.center.resume_title",
            "ui.center.firmware_title",
            "ui.center.download_title",
            "ui.center.download_refused",
            "ui.center.extract_title",
            "ui.center.export_title",
            "ui.center.hash_note",
            "ui.center.tools_title",
            "ui.center.updates_title",
            "ui.center.logs_title",
            "ui.center.settings_title",
            "ui.center.help_title",
            "ui.center.help_quickstart",
            "ui.center.help_troubleshooting",
        ] {
            assert!(is_known(key), "missing batch2 key {key}");
            assert!(!lookup(Language::En, key).is_empty(), "empty en for {key}");
            assert!(!lookup(Language::De, key).is_empty(), "empty de for {key}");
            assert!(!english(key).is_empty(), "empty english() for {key}");
        }
    }

    #[test]
    fn detect_never_panics() {
        let _ = Language::detect();
    }
}
