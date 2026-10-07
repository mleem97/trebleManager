#!/usr/bin/env python3
"""Generate PORTING.md: every script function vs Rust coverage (0-100%).

Usage: python3 tools/gen-porting.py > PORTING.md
Sources: scripts/Treble-Toolkit.ps1, scripts/treble-toolkit.sh,
         core/treble_core + gsi-root/crates/*/src.
Statuses are curated below; unknown functions default by area.
Legend: n/a = stays script by design (bootstrap launchers, Spec §4).
"""
import re
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
PS1 = ROOT / "scripts" / "Treble-Toolkit.ps1"
SH = ROOT / "scripts" / "treble-toolkit.sh"

# name -> (rust counterpart, status 0-100 or "n/a", note)
KNOWN = {
    # --- exact pure ports (100%) ---
    "ConvertFrom-AdbDevices": ("gsi-device::parse::adb_devices", 100, ""),
    "ConvertFrom-FastbootDevices": ("gsi-device::parse::fastboot_devices", 100, ""),
    "ConvertFrom-GetpropDump": ("gsi-device::parse::getprop", 100, ""),
    "ConvertFrom-ByNameListing": ("gsi-device::parse::byname", 100, ""),
    "ConvertFrom-FastbootGetvar": ("gsi-device::parse::getvar", 100, ""),
    "Get-FlashVerdict": ("treble_core::fastboot::flash_verdict", 100, ""),
    "Test-FirmwareUrl": ("treble_core::firmware::check_url", 100, "/download-strip parity"),
    "Test-BootImageMagic": ("treble_core::images::boot_magic_ver", 100, ""),
    "Test-ImageKind": ("treble_core::images::image_kind", 100, ""),
    "Get-RomLabel": ("treble_core::roms::rom_label", 100, ""),
    "Get-RomSuggested": ("treble_core::roms::rom_suggest", 100, ""),
    "Resolve-RunMode": ("inline (trivial)", 100, "2-line logic"),
    "Unquote-Path": ("inline (trivial)", 100, ""),
    "valid_url": ("treble_core::firmware::check_url", 100, ""),
    "boot_magic_ver": ("treble_core::images::boot_magic_ver", 100, "NUL-safe in both"),
    "image_kind": ("treble_core::images::image_kind", 100, ""),
    "flash_verdict": ("treble_core::fastboot::flash_verdict", 100, ""),
    "rom_suggest": ("treble_core::roms::rom_suggest", 100, ""),
    "rom_label": ("treble_core::roms::rom_label", 100, ""),
    "resolve_mode": ("inline (trivial)", 100, ""),
    "platform_tools_url": ("registry tools block (data, no code yet)", 20, "URLs only in scripts"),
    "install_base_dir": ("gsi-config (platform dirs)", 80, "logic ported, tool-linking not"),
    "profile_verified": ("device profile (registry read open)", 10, ""),
    "marketing_name": ("inline (trivial)", 100, ""),
    # --- ROM/registry presentation (partial: labels/options ported, selection UI not) ---
    "Get-RomEntryLabel": ("treble_core::roms::rom_entry_label", 100, ""),
    "rom_options": ("treble_core (label builder)", 60, "listing UI stays script/GUI"),
    "rom_broken": ("inline filter (trivial)", 80, ""),
    "compat_roms": ("registry read open", 10, ""),
    "compat_broken_markers": ("registry read open", 10, ""),
    "compat_file": ("gsi-config paths", 50, "path only, no query layer"),
    "rom_entry_gsi": ("inline lookup", 40, "no gsi-registry crate yet"),
    "Get-RomEntry": ("inline lookup", 40, ""),
    "Get-RomOptions": ("treble_core (label builder)", 60, ""),
    "Get-CompatRegistry": ("registry read open", 10, "no gsi-registry crate yet"),
    "Test-RomAgainstRegistry": ("registry read open", 10, ""),
    "Get-VendorAdvice": ("static strings open", 0, "not ported"),
    "Get-ResolverSystems": ("resolver_entries logic open", 0, ""),
    "Get-ResolverVariants": ("resolver_entries logic open", 0, ""),
    "Get-TargetAndroidVersions": ("target_androids logic open", 0, ""),
    "Get-TargetConfig": ("target config struct open", 0, ""),
    "target_androids": ("logic open", 0, ""),
    "resolver_entries": ("logic open", 0, ""),
    "compat_firmware_base": ("registry read open", 10, ""),
    "profile_gsi_advice": ("registry read open", 10, ""),
    "profile_variant": ("registry read open", 10, ""),
    "Get-RomDownloads": ("rom_downloads logic open", 0, ""),
    "rom_downloads": ("logic open", 0, ""),
    "Get-MagiskStable": ("manual fetch (no crate fn)", 0, ""),
    "magisk_stable": ("manual fetch (no crate fn)", 0, ""),
    "Get-TTMagiskInfo": ("hash/size only", 20, ""),
    "Find-TTMagiskApk": ("glob (trivial)", 50, ""),
    "magisk_apk": ("glob (trivial)", 50, ""),
    "Get-FileHashInfo": ("gsi-workflow::sha256_file", 90, "sha512 missing"),
    "file_hash": ("gsi-workflow::sha256_file", 90, ""),
    "Get-TTScriptRoot": ("gsi-config (platform dirs)", 70, "same rule, different API"),
    "Get-TTToolRoot": ("gsi-config paths", 50, ""),
    "Get-InstalledRomFile": ("gsi-config paths", 50, ""),
    "rom_file": ("gsi-config paths", 50, ""),
    "Save-InstalledRom": ("file write (trivial)", 70, "no dedicated fn"),
    "Load-InstalledRom": ("file read (trivial)", 70, "no dedicated fn"),
    "save_rom": ("file write (trivial)", 70, ""),
    "load_rom": ("file read (trivial)", 70, ""),
    "Save-RootState": ("state file open", 20, "slot pattern exists in gsi-device"),
    "save_root_state": ("state file open", 20, ""),
    "Save-SlotState": ("gsi-device read/write slot", 100, "same JSON shape"),
    "save_slot": ("gsi-device read/write slot", 100, ""),
    "Get-SlotState": ("gsi-device read_slot", 100, ""),
    "read_slot": ("gsi-device read_slot", 100, ""),
    "Read-WorkflowState": ("gsi-device (slot part)", 40, "full state open"),
    "Write-WorkflowState": ("gsi-device (slot part)", 40, ""),
    "Get-WorkflowStateFile": ("gsi-config paths", 50, ""),
    "read_state_goal": ("state file open", 20, ""),
    "write_state": ("state file open", 20, ""),
    "state_file": ("gsi-config paths", 50, ""),
    "Get-GoalSteps": ("gsi-workflow p10_lineage20 (one goal)", 30, "generic goals open"),
    "goal_steps": ("gsi-workflow (one goal)", 30, ""),
    "New-WorkflowPlan": ("gsi-workflow plan types", 40, "planner logic open"),
    "Start-GoalWorkflow": ("gsi-workflow runner (partial)", 30, "executes safe steps, refuses rest"),
    "run_goal": ("gsi-workflow runner (partial)", 30, ""),
    "goal_screen": ("CLI dispatch (trivial)", 60, ""),
    "Invoke-FailureFlow": ("StepOutcome::Failed exists", 20, "no flow runner"),
    "Get-DeviceStates": ("gsi-device parse + detect open", 30, "parsers done, state machine open"),
    "device_states": ("gsi-device parse + detect open", 30, ""),
    "Select-TargetDevice": ("CLI select open", 0, ""),
    "select_target": ("CLI select open", 0, ""),
    "Test-StepGate": ("gate types open", 10, ""),
    "step_gate": ("gate types open", 10, ""),
    "Test-TTFlashReadiness": ("gate types open", 10, ""),
    "Invoke-Preflight": ("checks open", 10, "tool part via gsi-tool"),
    "preflight": ("checks open", 10, ""),
    "Show-PreflightBlocked": ("TUI (Phase 9)", 0, ""),
    "Find-TTTools": ("gsi-tool::locate", 80, "config-file merge open"),
    "find_tools": ("gsi-tool::locate", 80, ""),
    "scan_dir_for_tools": ("dir scan (trivial)", 60, ""),
    "ensure_tool": ("guided flow open (TUI)", 0, ""),
    "ensure_scrcpy": ("guided flow open (TUI)", 0, ""),
    "install_platform_tools": ("URL pattern only", 10, ""),
    "link_into_tools": ("symlink (trivial)", 50, ""),
    "add_to_path": ("gsi-root install (self only)", 30, "generic PATH mgmt open"),
    "save_config": ("gsi-config dirs", 40, "tool-path JSON open"),
    "Find-TTRecoveryImage": ("glob (trivial)", 60, ""),
    "Find-TTRomBaseImage": ("glob newest (trivial)", 60, ""),
    "rom_base_image": ("glob newest (trivial)", 60, ""),
    "Find-LocalSystemImage": ("glob (trivial)", 60, ""),
    "Get-PatchBase": ("rule open (needs registry layer)", 20, ""),
    "patch_base": ("rule open", 20, ""),
    "Get-RomImageEntries": ("zip list open (needs archive crate)", 0, ""),
    "zip_entries": ("zip list open (needs archive crate)", 0, ""),
    "Expand-TTRomArchive": ("archive crate open", 0, "tar.exe/python today"),
    "Expand-TTGzipImage": ("archive crate open (flate2 planned)", 0, ""),
    "Expand-TTXzImage": ("archive crate open (lzma planned)", 0, ""),
    "Export-RecoveryFromRom": ("archive+ext4 needed", 5, "refusal logic portable anytime"),
    "export_recovery": ("archive+ext4 needed", 5, ""),
    "Invoke-FirmwareDownload": ("gsi-update::download_to", 70, "no progress events yet"),
    "download_firmware": ("gsi-update::download_to", 70, ""),
    "download_file": ("gsi-update::download_to", 70, ""),
    "Test-DownloadedFirmware": ("hash sidecar (trivial)", 70, "size heuristic open"),
    "verify_download": ("hash sidecar (trivial)", 70, ""),
    "Invoke-RomDownload": ("gsi-update + rom list (glue open)", 30, ""),
    "download_rom": ("gsi-update + rom list (glue open)", 30, ""),
    "rom_downloads": ("registry query open", 10, ""),
    "Invoke-MagiskDownload": ("gsi-update + stable.json (glue open)", 30, ""),
    "download_magisk": ("gsi-update + stable.json (glue open)", 30, ""),
    "Prepare-TTMagiskPatch": ("staging copy (trivial)", 60, "instructions text open"),
    "prepare_patch": ("staging copy (trivial)", 60, ""),
    "Test-RecoveryImageFile": ("images::boot_magic_ver + size", 60, "size policy open"),
    "test_image": ("images + size", 60, ""),
    "test_system_image": ("images + size", 60, ""),
    "Test-BootImageMagic": ("done above", 100, ""),
    "Test-ImageKind": ("done above", 100, ""),
    "boot_magic_ver": ("done above", 100, ""),
    "image_kind": ("done above", 100, ""),
    "Show-FlashVerdict": ("CLI text (trivial)", 80, "ttcore prints verdicts"),
    "New-TTBackup": ("backup packaging open", 10, "needs device dump path"),
    "do_backup": ("backup packaging open", 10, ""),
    "Invoke-TTSafeFlash": ("plan exists, exec needs fastboot", 20, ""),
    "safe_flash": ("plan exists, exec needs fastboot", 20, ""),
    "system_flash": ("plan exists, exec needs fastboot", 20, ""),
    "Invoke-SystemFlash": ("plan exists, exec needs fastboot", 20, ""),
    "Invoke-TwrpFlash": ("plan exists, exec needs fastboot", 20, ""),
    "twrp_flash": ("plan exists, exec needs fastboot", 20, ""),
    "Invoke-TTRestoreFlow": ("plan exists, exec needs fastboot", 20, ""),
    "do_restore": ("plan exists, exec needs fastboot", 20, ""),
    "Invoke-GuidedWipe": ("plan exists, exec needs fastboot", 20, ""),
    "guided_wipe": ("plan exists, exec needs fastboot", 20, ""),
    "Install-PersistFixes": ("plan exists, exec needs adb", 20, ""),
    "install_persist_fixes": ("plan exists, exec needs adb", 20, ""),
    "Invoke-TTRootVerification": ("adb sequence open (Phase 8)", 0, ""),
    "verify_root": ("adb sequence open (Phase 8)", 0, ""),
    "validate_device": ("adb sequence open (Phase 8)", 0, ""),
    "validate_checked": ("adb sequence open (Phase 8)", 0, ""),
    "Invoke-TTValidate": ("adb sequence open (Phase 8)", 0, ""),
    "Write-ValidationReport": ("manifest struct exists", 30, ""),
    "New-TTDiagnostic": ("zip packaging open (needs archive crate)", 10, ""),
    "do_diagnostic": ("zip packaging open", 10, ""),
    "Invoke-DeveloperDump": ("adb dumps open (Phase 8)", 0, ""),
    "developer_dump": ("adb dumps open (Phase 8)", 0, ""),
    "Invoke-TTAdb": ("gsi-tool::run + adb (binding)", 50, "protocol native: Phase 8"),
    "Invoke-TTFastboot": ("gsi-tool::run + fastboot (binding)", 50, ""),
    "Invoke-FastbootLogged": ("gsi-tool::run (binding)", 50, ""),
    "adb_run": ("gsi-tool::run (binding)", 50, ""),
    "fb_run": ("gsi-tool::run (binding)", 50, ""),
    "fb_flash": ("gsi-tool::run (binding)", 50, ""),
    "adb_prop": ("gsi-device::parse::getprop + binding", 60, "glue open"),
    "Get-TTProp": ("gsi-device::parse::getprop + binding", 60, ""),
    "Update-TTMode": ("state machine open (Phase 8)", 10, "parsers done"),
    "detect_mode": ("state machine open (Phase 8)", 10, ""),
    "Invoke-TTAndroidAnalysis": ("sequence open (Phase 8)", 10, "parsers done"),
    "android_analysis": ("sequence open (Phase 8)", 10, ""),
    "Invoke-TTFastbootAnalysis": ("sequence open (Phase 8)", 10, ""),
    "fastboot_analysis": ("sequence open (Phase 8)", 10, ""),
    "Get-TTFirmwareBaseline": ("string logic open", 20, ""),
    "firmware_compat": ("compat rule open (needs registry layer)", 20, ""),
    "Test-FirmwareCompatibility": ("compat rule open", 20, ""),
    "Get-PreferredRootMethod": ("static table open", 10, ""),
    "root_method_ids": ("static table open", 10, ""),
    "root_method_name": ("static table open", 10, ""),
    "Get-VendorAdvice": ("static strings open", 0, ""),
    "vendor_advice": ("static strings open", 0, ""),
    "Test-SystemImageFile": ("images + size", 60, ""),
    "log": ("tracing planned (println today)", 20, ""),
    "Write-TTLog": ("tracing planned", 20, ""),
    "header": ("GUI/TUI (Phase 9)", 0, ""),
    "Show-TTHeader": ("GUI (Phase 9)", 0, ""),
    "Show-TTMenu": ("GUI (Phase 9)", 0, ""),
    "menu": ("GUI (Phase 9)", 0, ""),
    "Pause-TT": ("GUI (Phase 9)", 0, ""),
    "pause_tt": ("GUI (Phase 9)", 0, ""),
    "iread": ("GUI (Phase 9)", 0, ""),
    "Test-TTAdmin": ("platform check (trivial)", 70, "not ported yet"),
    "Invoke-TTSelfElevate": ("platform guidance open", 10, "UAC stays OS-side"),
    "Invoke-TTFirstRun": ("guided flow open (GUI)", 0, ""),
    "L": ("i18n system open (GUI)", 0, ""),
    "os_classify": ("string rules open", 20, ""),
    "Get-OSClassification": ("string rules open", 20, ""),
    "step_gate": ("gate types open", 10, ""),
}

# area fallback by name pattern: (area, default status, note)
AREAS = [
    # ORDER matters: first match wins. Prefix-style patterns first.
        (["Screen-", "screen_", "Show-", "Menu", "menu_", "pause_",
      "Pause-", "iread", "Header", "header", "Start-", "Help", "help",
      "Status", "status", "Settings", "Logs", "Bootkeys", "Unlock", "Kernel",
      "Compat", "RootMethods", "FlashSystem", "Reinstall", "Goals",
      "wizard", "Wizard", "ensure_", "select_rom", "Select-InstalledRom",
      "select_root_target", "Select-RootTarget", "select_target_image",
      "Select-TargetImage", "TargetDevice"],
     "TUI/interaktiv", 0,
     "GUI Phase 9 (Slint)"),
    (["bootstrap", "Bootstrap", "Get-BootstrapReleaseFile"], "Bootstrap-Logik", 30,
     "Mechanik in gsi-update vorhanden, Release-File-Logik offen"),
    (["ConvertFrom-", "AdbDevices", "FastbootDevices", "GetpropDump",
      "ByNameListing", "FastbootGetvar", "Devices", "Getvar", "Getprop",
      "ByNameListing", "ByName",
      "Invoke-TTAdb", "Invoke-TTFastboot", "Invoke-FastbootLogged",
      "adb_run", "adb_prop", "fb_run", "fb_flash", "verify_root",
      "Validate", "validate", "Readiness", "readiness",
      "validate_device", "validate_checked", "developer_dump", "DeveloperDump",
      "install_persist", "Install-Persist", "safe_flash", "SafeFlash",
      "twrp_flash", "TwrpFlash", "do_restore", "RestoreFlow",
      "guided_wipe", "GuidedWipe", "system_flash", "SystemFlash",
      "diagnostic", "Diagnostic", "FlashReadiness",
      "Preflight", "preflight", "Blocked", "DeviceStates", "device_states",
      "detect_mode", "Update-TTMode", "AndroidAnalysis", "android_analysis",
      "FastbootAnalysis", "fastboot_analysis", "TTFirmwareBaseline",
      "FirmwareBaseline", "TTTools", "find_tools", "Tools", "tools",
      "Scrcpy", "scrcpy", "Platform", "platform", "Path", "path",
      "Backup", "backup", "Persist", "persist", "Wipe", "wipe", "Restore",
      "restore", "Patch", "patch", "Verify", "verify", "Reboot", "reboot",
      "Flash", "flash", "Extract", "extract", "Firmware", "firmware",
      "Analyze", "analyze", "Detect", "detect", "Prop", "prop"],
     "Device-Ausfuehrung", 10,
     "Binding+Parser da, Protokoll Phase 8"),
    (["download", "Download", "magisk_stable", "MagiskStable", "MagiskDownload",
      "MagiskApk", "magisk_apk", "MagiskInfo", "stable"], "Download", 60,
     "gsi-update kann laden+pruefen, Auswahl-UI offen"),
    (["Expand-TT", "zip_entries", "RomArchive", "Gzip", "XzImage",
      "Export", "export", "Extract", "extract", "archive", "Archive",
      "UpdateAppAnalysis", "UpdateAPP"], "Archiv/Export", 20,
     "Refusal-Logik portierbar, Dekomprimierung braucht Archive-Crate"),
    (["Write-TTLog", "ValidationReport", "read_state_goal", "write_state",
      "install_base_dir", "BaseDir", "RecoveryImage", "ImageEntries",
      "BaseImage", "RomFile", "Advice", "advice", "Verified", "verified",
      "RootVerification", "RootMethods", "goal_screen", "resolver_entries",
      "rom_base_image", "rom_file", "rom_label", "rom_suggest", "run_goal",
      "select_target", "target_androids", "target_partition",
      "rom_options", "rom_broken", "rom_entry", "RomEntry", "RomOptions",
      "RomDownloads", "rom_downloads", "firmware_compat", "FirmwareCompatibility",
      "VendorAdvice", "vendor_advice", "TargetAndroid", "ResolverSystems",
      "ResolverVariants", "TargetConfig", "target_config", "InstalledRom",
      "installed-rom", "save_rom", "load_rom", "Save-Installed", "Load-Installed",
      "PatchBase", "patch_base", "RomBaseImage", "BaseImage", "RomLabel",
      "RomSuggested", "SystemImage", "LocalSystemImage", "RootTarget",
      "TargetImage", "GoalSteps", "goal_steps", "WorkflowPlan", "WorkflowState",
      "workflow_state", "state_file", "RootState", "root_state", "SlotState",
      "slot", "Slot", "save_slot", "read_slot", "save_config", "Save-",
      "StepGate", "step_gate", "Config", "config", "Setting", "Resume",
      "resume", "FailureFlow", "FirstRun", "SelfElevate", "Admin", "Mode",
      "mode", "OSClassification", "os_classify", "PreferredRootMethod",
      "root_method", "ScriptRoot", "ToolRoot", "RomFile", "GsiAdvice",
      "Variant", "variant", "Marketing", "marketing", "Bootkeys"], "Logik/State/Planung", 30,
     "Einzelfall pruefen"),
    (["Test-", "test_", "boot_magic", "image_kind", "flash_verdict",
      "valid_url", "sha", "Sha", "SHA", "hash", "Hash", "magic", "Magic",
      "Resolve-", "Unquote", "FileHashInfo", "file_hash",
      "DownloadedFirmware", "RecoveryImageFile", "RomAgainstRegistry"],
     "Pure Tests/Pruefer", 90,
     "Einzelfall pruefen"),
]
GENERATED_NOTE = """<!-- GENERATED by tools/gen-porting.py — do not hand-edit rows; adjust KNOWN/AREAS there. -->
"""


def area_of(name):
    if name == "L":
        return ("TUI/interaktiv", 0, "GUI Phase 9 (Slint)")
    if name == "log":
        return ("Logik/State/Planung", 20, "tracing planned")
    low = name.lower()
    for patterns, area, default, note in AREAS:
        for p in patterns:
            if p.lower() in low:
                return area, default, note
    return "Sonstiges", 0, ""


def main():
    ps1 = sorted(
        {
            re.sub(r"\s*\{.*", "", l.replace("function ", "").strip())
            for l in (PS1.read_text(encoding="utf-8", errors="replace").splitlines())
            if l.startswith("function ")
        }
    )
    # fix multiline Get-InstalledRomFile artifact
    ps1 = ["Get-InstalledRomFile" if x.startswith("Get-InstalledRomFile") else x for x in ps1]
    sh = sorted(
        {
            m.group(1)
            for line in (SH.read_text(encoding="utf-8", errors="replace").splitlines())
            if (m := re.match(r"^([a-z_][a-z_0-9]*)\(\) \{", line))
        }
    )
    rows = []
    for name in ps1:
        rows.append(("PowerShell", name))
    for name in sh:
        rows.append(("Bash", name))
    # rust inventory
    rust_fns = set()
    for src in list((ROOT / "core" / "treble_core" / "src").glob("*.rs")) + list(
        (ROOT / "gsi-root").rglob("src/*.rs")
    ):
        for m in re.finditer(r"pub fn ([a-z_0-9]+)", src.read_text(encoding="utf-8", errors="replace")):
            rust_fns.add(m.group(1))

    by_area = {}
    total = 0
    tsum = 0
    for shell, name in rows:
        area, default, anote = area_of(name)
        if name in KNOWN:
            rust, status, note = KNOWN[name]
        else:
            rust, status, note = ("—", default, anote)
        by_area.setdefault(area, []).append((shell, name, rust, status, note))
        if isinstance(status, int):
            total += 1
            tsum += status

    out = []
    out.append("# Portierung nach Rust — Funktionsinventar\n")
    out.append(GENERATED_NOTE)
    out.append(
        "Jede Script-Funktion mit Implementierungsort (PS1/Bash), Rust-Gegenstück "
        "und Portierungsstatus (0–100 %). Bootstrap-Launcher (.bat/.sh/ps1-Dateien, "
        "kein Funktionsinventar) bleiben per Spec §4 dauerhaft Script. "
        "Stand: v" + (ROOT / "VERSION").read_text(encoding="utf-8").strip() + ".\n"
    )
    out.append(
        "\n## Pre-Rust-Ära (v2.1.0 – v2.17.x) / pre-Rust era\n"
        "\nReleases bis einschließlich v2.17.x sind script-only (PowerShell 5.1+, "
        "bash, BAT-Launcher) ohne Rust und ohne Cargo-Pflicht. Ihr dokumentiertes "
        "Verhalten (Wizard, Safety-Gates, Registries, CLI, Diagnostik) ist "
        "eingefroren und bleibt gültig — 0 % in den Tabellen unten ist dort die "
        "Script-Referenz, kein Mangel. Releases up to and including v2.17.x are "
        "script-only with no Rust dependency; their documented behavior stays "
        "valid as-is.\n"
    )
    navg = (tsum / total) if total else 0
    out.append(f"## Gesamt: {total} Script-Funktionen, {len(rust_fns)} Rust-`pub fn`, Schnitt {navg:.0f} %\n")
    out.append("\n> Interaktive TUI-Anteile (0 %) wandern in die Slint-GUI (Phase 9), nicht 1:1.\n")
    for area in sorted(by_area):
        items = by_area[area]
        vals = [s for (_, _, _, s, _) in items if isinstance(s, int)]
        avg = sum(vals) / len(vals) if vals else 0
        out.append(f"\n## {area} — {len(items)} Funktionen, Schnitt {avg:.0f} %\n")
        out.append("\n| Funktion | Shell | Rust | Status | Bemerkung |")
        out.append("\n|---|---|---|---|---|")
        for shell, name, rust, status, note in sorted(items, key=lambda r: r[1].lower()):
            st = f"{status} %" if isinstance(status, int) else status
            out.append(f"\n| `{name}` | {shell} | `{rust}` | {st} | {note} |")
        out.append("\n")
    out.append("\n## Rust-Seite (portierbar, ohne Duplikate)\n")
    out.append("\n`pub fn` insgesamt: %d (treble_core + gsi-root-Crates + gsi-device/parse, gsi-fs, gsi-tool, gsi-update, gsi-workflow, gsi-config).\n" % len(rust_fns))
    (ROOT / "PORTING.md").write_text("".join(out), encoding="utf-8")
    print(f"wrote PORTING.md: {total} functions, avg {navg:.0f} %, {len(rust_fns)} rust fns")

    # ---- TASKS.md: open tasks derived from the same statuses ----
    order = ["Archiv/Export", "Logik/State/Planung", "Device-Ausfuehrung",
             "Download", "Pure Tests/Pruefer", "Bootstrap-Logik",
             "TUI/interaktiv", "Sonstiges"]
    t = []
    t.append("# Offene Portierungs-Tasks (aus PORTING.md generiert)\n")
    t.append(GENERATED_NOTE.replace("PORTING.md", "TASKS.md"))
    t.append(
        "Jeder offene Task = eine Script-Funktion mit Status < 100 %.\n"
        "Reihenfolge = Abhängigkeiten zuerst (Archive/Registry, dann Flows,\n"
        "dann GUI). TUI-Zeilen wandern in Slint (Phase 9), nicht 1:1.\n"
    )
    n_open = 0
    areas_ordered = [a for a in order if a in by_area] + sorted(
        [a for a in by_area if a not in order])
    for area in areas_ordered:
        items = [(s, n, r, st, no) for (s, n, r, st, no) in by_area[area]
                 if isinstance(st, int) and st < 100]
        if not items:
            continue
        # nearly-done first within each area
        items.sort(key=lambda r: -r[3])
        t.append(f"\n## {area} — {len(items)} offen\n")
        for shell, name, rust, st, note in items:
            extra = f" — {note}" if note else ""
            t.append(f"\n- [ ] `{name}` ({shell}, {st} %){extra}")
        t.append("\n")
        n_open += len(items)
    t.append(f"\nOffen gesamt: {n_open} von {total} Funktionen.\n")
    (ROOT / "TASKS.md").write_text("".join(t), encoding="utf-8")
    print(f"wrote TASKS.md: {n_open} open tasks")


if __name__ == "__main__":
    main()
