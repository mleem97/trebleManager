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
    "profile_verified": ("gsi-registry::load_roms (parse), policy stays script", 50, ""),
    "marketing_name": ("inline (trivial)", 100, ""),
    # --- ROM/registry presentation (partial: labels/options ported, selection UI not) ---
    "Get-RomEntryLabel": ("treble_core::roms::rom_entry_label", 100, ""),
    "rom_options": ("gsi-registry::rom_options", 90, "listing UI stays script/GUI"),
    "rom_broken": ("inline filter (trivial)", 80, ""),
    "compat_roms": ("gsi-registry::load_roms", 90, "policy UI stays script"),
    "compat_broken_markers": ("gsi-registry::is_selectable", 90, ""),
    "compat_file": ("gsi-config paths + gsi-registry::repo_profile_path", 60, "profile discovery stays script"),
    "rom_entry_gsi": ("gsi-registry::entry_gsi", 100, ""),
    "Get-RomEntry": ("gsi-registry::entry_by_label", 100, ""),
    "Get-RomOptions": ("gsi-registry::rom_options", 90, ""),
    "Get-CompatRegistry": ("gsi-registry::load_roms", 90, "watch/policy UI stays script"),
    "Test-RomAgainstRegistry": ("gsi-registry::firmware_compat + vendor_advice", 80, "gate wiring stays script"),
    "Get-VendorAdvice": ("gsi-registry::vendor_advice", 100, ""),
    "vendor_advice": ("gsi-registry::vendor_advice", 100, ""),
    "Get-ResolverSystems": ("gsi-registry::systems_for", 100, ""),
    "Get-ResolverVariants": ("gsi-registry::variants_for", 100, ""),
    "Get-TargetAndroidVersions": ("gsi-registry::target_androids", 100, ""),
    "Get-TargetConfig": ("gsi-registry::target_config", 100, ""),
    "target_androids": ("gsi-registry::target_androids", 100, ""),
    "resolver_entries": ("gsi-registry queries (pieces ported)", 70, "chain glue stays script"),
    "compat_firmware_base": ("gsi-registry::firmware_compat", 90, ""),
    "profile_gsi_advice": ("gsi-registry::target_config (gsi branch)", 80, "advice text stays script"),
    "profile_variant": ("gsi-registry::variants_for (needs android first)", 40, ""),
    "Get-RomDownloads": ("gsi-registry::rom_downloads", 100, ""),
    "rom_downloads": ("gsi-registry::rom_downloads", 100, ""),
    "Get-MagiskStable": ("gsi-update::fetch/parse_magisk_stable", 90, "registry load stays caller-side"),
    "magisk_stable": ("gsi-update::fetch/parse_magisk_stable", 90, ""),
    "Get-TTMagiskInfo": ("gsi-update::magisk_info", 90, "sha512 omitted, sha2-only"),
    "Find-TTMagiskApk": ("gsi-update::find_magisk_apk", 100, ""),
    "magisk_apk": ("gsi-update::find_magisk_apk", 100, ""),
    "Get-FileHashInfo": ("gsi-workflow::sha256/512_file", 100, ""),
    "file_hash": ("gsi-workflow::sha256/512_file", 100, ""),
    "Get-TTScriptRoot": ("gsi-config (platform dirs)", 70, "same rule, different API"),
    "Get-TTToolRoot": ("gsi-config paths", 50, ""),
    "Get-InstalledRomFile": ("gsi-state::installed_rom_file", 100, ""),
    "rom_file": ("gsi-state::installed_rom_file", 100, ""),
    "Save-InstalledRom": ("gsi-state::write_installed_rom", 100, ""),
    "Load-InstalledRom": ("gsi-state::read_installed_rom", 95, "multi-token=Err, honest"),
    "save_rom": ("gsi-state::write_installed_rom", 100, ""),
    "load_rom": ("gsi-state::read_installed_rom", 90, "bash mangling replaced by Err"),
    "Save-RootState": ("gsi-state::write/read_root_state", 100, ""),
    "save_root_state": ("gsi-state::write_root_state", 100, ""),
    "Save-SlotState": ("gsi-device read/write slot", 100, "same JSON shape"),
    "save_slot": ("gsi-device read/write slot", 100, ""),
    "Get-SlotState": ("gsi-device read_slot", 100, ""),
    "read_slot": ("gsi-device read_slot", 100, ""),
    "Read-WorkflowState": ("gsi-state::read_workflow_state", 95, "corrupt=Err, honest"),
    "Write-WorkflowState": ("gsi-state::write_workflow_state", 95, ""),
    "Get-WorkflowStateFile": ("gsi-state::workflow_state_file", 100, ""),
    "read_state_goal": ("gsi-state::read_state_goal", 95, ""),
    "write_state": ("gsi-state::write_step_state", 100, ""),
    "state_file": ("gsi-state::workflow_state_file", 100, ""),
    "Get-GoalSteps": ("gsi-state::goal_steps (all 9 goals)", 100, ""),
    "goal_steps": ("gsi-state::goal_steps", 100, ""),
    "New-WorkflowPlan": ("gsi-state::new_workflow_plan", 85, "live gate blocking stays exec layer"),
    "Start-GoalWorkflow": ("gsi-state plan + gsi-gates refusals", 40, "exec open"),
    "run_goal": ("gsi-state plan + gsi-gates refusals", 40, ""),
    "goal_screen": ("CLI dispatch (trivial)", 60, ""),
    "Invoke-FailureFlow": ("gsi-gates::failure_outcome/message", 80, "TUI + restore exec refused"),
    "Get-DeviceStates": ("gsi-device parse + detect open", 30, "parsers done, state machine open"),
    "device_states": ("gsi-device parse + detect open", 30, ""),
    "Select-TargetDevice": ("CLI select open", 0, ""),
    "select_target": ("CLI select open", 0, ""),
    "Test-StepGate": ("gsi-gates::step_gate", 95, "live probing stays caller-side"),
    "step_gate": ("gsi-gates::step_gate", 95, ""),
    "Test-TTFlashReadiness": ("gsi-gates::check_readiness", 90, "live re-query stays caller-side"),
    "check_readiness": ("gsi-gates::check_readiness", 90, ""),
    "Invoke-Preflight": ("gsi-gates::preflight (pure subset)", 85, "live parts refused"),
    "preflight": ("gsi-gates::preflight (pure subset)", 85, ""),
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
    "Get-PatchBase": ("gsi-gates::patch_base", 95, "fs search passed in"),
    "patch_base": ("gsi-gates::patch_base", 95, ""),
    "target_partition": ("gsi-gates::target_partition", 100, ""),
    "Get-BootstrapReleaseFile": ("gsi-update::bootstrap_release", 70, "download/verify stays caller-side"),
    "bootstrap_fetch": ("gsi-update::bootstrap_release", 70, ""),
    "Get-RomImageEntries": ("gsi-archive::tar_list (zip still open)", 20, "zip list open"),
    "zip_entries": ("gsi-archive::tar_list (zip still open)", 20, ""),
    "Expand-TTRomArchive": ("gsi-archive::extract_tar/classify (zip open)", 70, "zip extract open"),
    "Expand-TTGzipImage": ("gsi-archive::gunzip_bytes", 100, ""),
    "Expand-TTXzImage": ("gsi-archive::unxz_bytes", 100, ""),
    "Export-RecoveryFromRom": ("gsi-archive + gsi-fs read ported, repack open", 40, "write/repack + device glue open"),
    "export_recovery": ("gsi-archive + gsi-fs read ported, repack open", 40, ""),
    "Invoke-FirmwareDownload": ("gsi-update::download_to", 70, "no progress events yet"),
    "download_firmware": ("gsi-update::download_to", 70, ""),
    "download_file": ("gsi-update::download_file", 95, "no progress bar, library"),
    "Test-DownloadedFirmware": ("gsi-update::verify_downloaded_firmware", 85, "UPDATE.APP inner check deferred"),
    "verify_download": ("gsi-update::verify_hash_sidecar + heuristic", 90, ""),
    "Invoke-RomDownload": ("gsi-update + gsi-registry::target_config/rom_downloads (glue open)", 40, ""),
    "download_rom": ("gsi-update + gsi-registry::target_config/rom_downloads (glue open)", 40, ""),
    "rom_downloads": ("gsi-registry::rom_downloads", 100, ""),
    "Invoke-MagiskDownload": ("gsi-update::magisk dest + download_file glue", 75, "prompt/cache UI stays CLI-side"),
    "download_magisk": ("gsi-update::magisk dest + download_file glue", 75, ""),
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
    "New-TTBackup": ("gsi-diag::plan_backup", 60, "device dd refused, Phase 8"),
    "do_backup": ("gsi-diag::plan_backup", 60, ""),
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
    "Write-ValidationReport": ("gsi-diag report builder", 90, "timestamping stays caller-side"),
    "New-TTDiagnostic": ("gsi-diag bundle builder", 85, "live adb capture stays caller-side"),
    "do_diagnostic": ("gsi-diag bundle builder", 85, ""),
    "Invoke-DeveloperDump": ("gsi-diag::developer_dump (pure part)", 55, "live adb refused"),
    "developer_dump": ("gsi-diag::developer_dump (pure part)", 55, ""),
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
    "Get-TTFirmwareBaseline": ("gsi-gates::firmware_baseline_from_parts", 85, "live reads stay caller-side"),
    "firmware_compat": ("gsi-gates::firmware_compat", 100, ""),
    "Test-FirmwareCompatibility": ("gsi-gates::firmware_compat", 100, ""),
    "Get-PreferredRootMethod": ("gsi-gates::preferred_root_methods", 100, ""),
    "root_method_ids": ("gsi-gates::root_method_ids", 100, ""),
    "root_method_name": ("gsi-gates::root_method_name", 100, ""),
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
    "Test-TTAdmin": ("gsi-gates::admin_from_proc_status + guidance", 80, "elevation OS-side by design"),
    "Invoke-TTSelfElevate": ("gsi-gates::windows_admin_guidance", 80, "UAC stays OS-side by design"),
    "Invoke-TTFirstRun": ("guided flow open (GUI)", 0, ""),
    "L": ("i18n system open (GUI)", 0, ""),
    "os_classify": ("gsi-gates::os_classify", 95, "live prop collection stays caller-side"),
    "Get-OSClassification": ("gsi-gates::os_classify", 95, ""),
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
    out.append("# Portierung nach Rust — Funktionsinventar / Rust port inventory\n")
    out.append(GENERATED_NOTE)
    out.append(
        "Jede Script-Funktion mit Implementierungsort (PS1/Bash), Rust-Gegenstück "
        "und Portierungsstatus (0–100 %). Every script function with its location "
        "(PS1/Bash), Rust counterpart and port status (0–100 %). Bootstrap-Launcher (.bat/.sh/ps1-Dateien, "
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
        "\n"
        "## 100-%-Definition / 100 % definition\n"
        "\n"
        "100 % = jede Script-Funktion ist portiert (100 %, Dry-Run-getestet) "
        "oder als n/a markiert (bleibt per Spec §4 dauerhaft Script: Launcher, "
        "OS-seitige Elevation). TUI-Screens zählen über ihre Slint-GUI-Seite "
        "(Phase 9). Hardware-Ausführung (Flash/ADB-Sequenzen, Phase 8) zählt "
        "über Plan-Typen mit ehrlicher Refusal-Logik + DRY RUN TESTS; "
        "Live-Nachweis separat in VERIFICATION-*.md. POST-100 (Qualcomm) "
        "zählt NICHT zu den 100 %.\n"
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
    t.append(
        "\n## Nach 100 % blockiert / blocked until 100 % (POST-100)\n"
        "\n"
        "Qualcomm-Erweiterung (neue Geräteserie, Referenz: BQ Aquaris X Pro EU, "
        "Snapdragon 626/MSM8953) — strikt blockiert bis der Rewrite bei 100 % "
        "steht. Qualcomm extension (new device family) — strictly blocked "
        "until the rewrite reaches 100 %.\n"
        "\n- [ ] EDL/Deep-Flash-Protokoll (MSM8953) als `gsi-edl`-Crate "
        "(Firehose-Loader-Handling, signierte Loader, kein Bypass ohne "
        "Device-Consent) — BLOCKIERT bis 100 %\n"
        "- [ ] FRP-Reset-Flow (mit + ohne Deep Flash, nur wo der SoC es ohne "
        "erlaubt, dokumentiert pro Variante) — BLOCKIERT bis 100 %\n"
        "- [ ] Bootloader-Unlock-Flow (BQ Aquaris X Pro als Referenzgerät, "
        "Remote-Tests) — BLOCKIERT bis 100 %\n"
        "- [ ] Geräteprofil "
        "`data/compatibility/qualcomm/bq-aquaris-x-pro.yaml/json` + "
        "Registry-Anbindung — BLOCKIERT bis 100 %\n"
        "- [ ] Safety-Gates (Eigentümernachweis-Flow, Logging, Doppel-Confirm, "
        "kein stiller Bypass) — BLOCKIERT bis 100 %\n"
    )
    (ROOT / "TASKS.md").write_text("".join(t), encoding="utf-8")
    print(f"wrote TASKS.md: {n_open} open tasks")


if __name__ == "__main__":
    main()
