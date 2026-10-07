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
    "platform_tools_url": ("gsi-registry::platform_tools_url", 85, "fetch/extract stays caller-side"),
    "install_base_dir": ("gsi-config::install_base_dir", 90, "writability is explicit input"),
    "profile_verified": ("gsi-registry::load_roms (parse), policy stays script", 50, ""),
    "marketing_name": ("inline (trivial)", 100, ""),
    # --- ROM/registry presentation (partial: labels/options ported, selection UI not) ---
    "Get-RomEntryLabel": ("treble_core::roms::rom_entry_label", 100, ""),
    "rom_options": ("gsi-registry::rom_options", 90, "listing UI stays script/GUI"),
    "rom_broken": ("inline filter (trivial)", 80, ""),
    "compat_roms": ("gsi-registry::load_roms", 90, "policy UI stays script"),
    "compat_broken_markers": ("gsi-registry::is_selectable", 90, ""),
    "compat_file": ("gsi-config::resolve_compat_profile", 95, ""),
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
    "Get-TTScriptRoot": ("gsi-config::script_root", 90, "single documented env/CWD fallback"),
    "Get-TTToolRoot": ("gsi-config::tool_root", 100, ""),
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
    "goal_screen": ("CLI render_goal_plan", 90, "text only, no execution"),
    "Select-TargetImage": ("CLI registry resolvers + render_target_config", 90, "flags replace ReadKey"),
    "select_target_image": ("CLI registry resolvers + render_target_config", 90, ""),
    "select_rom": ("CLI cli_rom_options + resolve_rom_choice", 95, ""),
    "Select-RootTarget": ("CLI render/resolve_root_target_choice", 85, "stock sub-choice stays guided note"),
    "select_root_target": ("CLI render/resolve_root_target_choice", 85, ""),
    "Invoke-FailureFlow": ("gsi-gates::failure_outcome/message", 80, "TUI + restore exec refused"),
    "Get-DeviceStates": ("gsi-workflow::device_states", 75, "live queries open"),
    "device_states": ("gsi-workflow::device_states", 75, ""),
    "Select-TargetDevice": ("CLI render/resolve_device_serial", 90, "live scan in dispatch"),
    "select_target": ("CLI render/resolve_device_serial", 90, ""),
    "Test-StepGate": ("gsi-gates::step_gate", 95, "live probing stays caller-side"),
    "step_gate": ("gsi-gates::step_gate", 95, ""),
    "Test-TTFlashReadiness": ("gsi-gates::check_readiness", 90, "live re-query stays caller-side"),
    "check_readiness": ("gsi-gates::check_readiness", 90, ""),
    "Invoke-Preflight": ("gsi-gates::preflight (pure subset)", 85, "live parts refused"),
    "preflight": ("gsi-gates::preflight (pure subset)", 85, ""),
    "Show-PreflightBlocked": ("TUI (Phase 9)", 0, ""),
    "Find-TTTools": ("gsi-tool::locate + CLI ToolConfig merge", 85, ""),
    "find_tools": ("gsi-tool::locate + CLI ToolConfig merge", 85, ""),
    "scan_dir_for_tools": ("gsi-config::scan_dir_for_tools", 90, ""),
    "ensure_tool": ("guided flow open (TUI)", 0, ""),
    "ensure_scrcpy": ("guided flow open (TUI)", 0, ""),
    "install_platform_tools": ("registry URL + CLI install plan", 85, "fetch/extract stays caller-side"),
    "link_into_tools": ("CLI LinkPlan (no mutation)", 80, "explicit [y/N] plan"),
    "add_to_path": ("CLI PathPlan + refuse_env_mutation", 80, "no env mutation in lib"),
    "save_config": ("gsi-config::save/load_tool_config", 95, ""),
    "Find-TTRecoveryImage": ("gsi-config::find_recovery_images", 95, "dirs passed in"),
    "Find-TTRomBaseImage": ("gsi-config::find_rom_base_image", 95, ""),
    "rom_base_image": ("gsi-config::find_rom_base_image", 95, "documented tiebreak"),
    "Find-LocalSystemImage": ("gsi-config::find_system_image", 95, ""),
    "Get-PatchBase": ("gsi-gates::patch_base", 95, "fs search passed in"),
    "patch_base": ("gsi-gates::patch_base", 95, ""),
    "target_partition": ("gsi-gates::target_partition", 100, ""),
    "Get-BootstrapReleaseFile": ("gsi-update::bootstrap_release", 70, "download/verify stays caller-side"),
    "bootstrap_fetch": ("gsi-update::bootstrap_release", 70, ""),
    "Get-RomImageEntries": ("gsi-archive::zip_list", 95, ""),
    "zip_entries": ("gsi-archive::zip_list", 95, ""),
    "Expand-TTRomArchive": ("gsi-archive::extract_auto/extract_zip", 85, "thin path wrapper stays CLI-side"),
    "Invoke-TTUpdateAppAnalysis": ("gsi-archive::probe_update_app", 90, "manual-extractor note kept"),
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
    "Test-RecoveryImageFile": ("gsi-config::test_recovery_image", 95, ""),
    "test_image": ("gsi-config::test_recovery_image", 95, ""),
    "test_system_image": ("gsi-config::test_system_image", 95, ""),
    "Test-BootImageMagic": ("done above", 100, ""),
    "Test-ImageKind": ("done above", 100, ""),
    "boot_magic_ver": ("done above", 100, ""),
    "image_kind": ("done above", 100, ""),
    "Show-FlashVerdict": ("CLI evaluate/render_flash_verdict", 90, "FAILED veto kept"),
    "New-TTBackup": ("gsi-diag::plan_backup", 60, "device dd refused, Phase 8"),
    "do_backup": ("gsi-diag::plan_backup", 60, ""),
    "Invoke-TTSafeFlash": ("gsi-workflow::plan/run_safe_flash", 60, "live exec Phase 8"),
    "safe_flash": ("gsi-workflow::plan/run_safe_flash", 60, ""),
    "system_flash": ("gsi-workflow::plan/run_system_flash", 60, ""),
    "Invoke-SystemFlash": ("gsi-workflow::plan/run_system_flash", 60, "live exec Phase 8"),
    "Invoke-TwrpFlash": ("gsi-workflow::plan/run_twrp_flash", 60, "live exec Phase 8"),
    "twrp_flash": ("gsi-workflow::plan/run_twrp_flash", 60, ""),
    "Invoke-TTRestoreFlow": ("gsi-workflow::plan/run_restore", 60, "live exec Phase 8"),
    "do_restore": ("gsi-workflow::plan/run_restore", 60, ""),
    "Invoke-GuidedWipe": ("gsi-workflow::plan/run_guided_wipe", 60, "live exec Phase 8"),
    "guided_wipe": ("gsi-workflow::plan/run_guided_wipe", 60, ""),
    "Install-PersistFixes": ("gsi-workflow::plan_persist + embedded scripts", 60, "live adb Phase 8"),
    "install_persist_fixes": ("gsi-workflow::plan_persist + embedded scripts", 60, ""),
    "Invoke-TTRootVerification": ("adb sequence open (Phase 8)", 0, ""),
    "verify_root": ("gsi-workflow::plan/classify_verify_root", 50, "live adb Phase 8"),
    "validate_device": ("gsi-workflow::plan_validate_device", 50, "live adb Phase 8"),
    "validate_checked": ("gsi-workflow::validate_checked", 80, ""),
    "Invoke-TTValidate": ("gsi-workflow::plan/run_validate_device", 50, "live adb Phase 8"),
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
    "Update-TTMode": ("gsi-workflow::detect_mode", 75, "live queries open"),
    "detect_mode": ("gsi-workflow::detect_mode", 75, ""),
    "Invoke-TTAndroidAnalysis": ("gsi-workflow::android_analysis", 70, "live adb spawn open"),
    "android_analysis": ("gsi-workflow::android_analysis", 70, ""),
    "Invoke-TTFastbootAnalysis": ("gsi-workflow::fastboot_analysis", 70, "live fastboot spawn open"),
    "fastboot_analysis": ("gsi-workflow::fastboot_analysis", 70, ""),
    "Get-TTFirmwareBaseline": ("gsi-gates::firmware_baseline_from_parts", 85, "live reads stay caller-side"),
    "firmware_compat": ("gsi-gates::firmware_compat", 100, ""),
    "Test-FirmwareCompatibility": ("gsi-gates::firmware_compat", 100, ""),
    "Get-PreferredRootMethod": ("gsi-gates::preferred_root_methods", 100, ""),
    "root_method_ids": ("gsi-gates::root_method_ids", 100, ""),
    "root_method_name": ("gsi-gates::root_method_name", 100, ""),
    "Test-SystemImageFile": ("gsi-config::test_system_image", 90, "arm64 rules advisory like PS"),
    "log": ("tracing planned (println today)", 20, ""),
    "Write-TTLog": ("tracing planned", 20, ""),
    "header": ("—", "n/a", "TUI plumbing without portable semantics; GUI event loop replaces"),
    "Show-TTHeader": ("—", "n/a", "TUI chrome; GUI has its own chrome"),
    "menu": ("—", "n/a", "TUI plumbing; navigation lives in GUI nav + CLI dispatch"),
    "Pause-TT": ("—", "n/a", "TUI plumbing without portable semantics"),
    "pause_tt": ("—", "n/a", "TUI plumbing without portable semantics"),
    "iread": ("—", "n/a", "TUI plumbing; GUI uses native inputs"),
    "L": ("gsi-i18n::lookup (36 keys en/de)", 100, "full PS1 string extraction stays Phase-9 epic"),
    "Screen-Detect": ("gsi-root-gui Detect page", 100, ""),
    "screen_detect": ("gsi-root-gui Detect page", 100, ""),
    "Screen-Tools": ("gsi-root-gui Tools page", 100, ""),
    "screen_tools": ("gsi-root-gui Tools page", 100, ""),
    "Test-TTAdmin": ("gsi-gates::admin_from_proc_status + guidance", 80, "elevation OS-side by design"),
    "Invoke-TTSelfElevate": ("gsi-gates::windows_admin_guidance", 80, "UAC stays OS-side by design"),
    "Invoke-TTFirstRun": ("guided flow open (GUI)", 0, ""),
    "Screen-Flash": ("GUI FlashPage (static preview)", 90, "live flash Phase 8 refused"),
    "screen_flash": ("GUI FlashPage (static preview)", 90, ""),
    "Screen-FlashSystem": ("GUI FlashSystemPage (static preview)", 90, "live flash Phase 8 refused"),
    "screen_flashsystem": ("GUI FlashSystemPage (static preview)", 90, ""),
    "Screen-Wipe": ("GUI WipePage (static preview)", 95, "live erase Phase 8 refused"),
    "screen_wipe": ("GUI WipePage (static preview)", 95, ""),
    "Screen-Unlock": ("GUI UnlockPage (guidance, never executes)", 100, ""),
    "screen_unlock": ("GUI UnlockPage (guidance, never executes)", 100, ""),
    "Screen-RebootVerify": ("GUI VerifyPage (static preview)", 90, "live adb Phase 8 refused"),
    "screen_verify": ("GUI VerifyPage (static preview)", 90, ""),
    "Show-PreflightBlocked": ("GUI PreflightPage (static preview)", 90, "setup help is text only"),
    "Screen-Backup": ("GUI BackupPage (static preview)", 70, "live copy/hash Phase 8"),
    "screen_backup": ("GUI BackupPage (static preview)", 70, ""),
    "Screen-Restore": ("GUI RestorePage (static preview)", 60, "live flash refused"),
    "Screen-Reinstall": ("GUI ReinstallPage (static preview)", 65, "wipe/flash refused"),
    "screen_reinstall": ("GUI ReinstallPage (static preview)", 65, ""),
    "Screen-Resume": ("GUI ResumePage (static preview)", 75, "exec stays guided runner"),
    "screen_resume": ("GUI ResumePage (static preview)", 75, ""),
    "Screen-Bootkeys": ("GUI BootkeysPage (guide)", 85, ""),
    "screen_bootkeys": ("GUI BootkeysPage (guide)", 85, ""),
    "Screen-Logs": ("GUI Logs page (session log live)", 40, "file browser open"),
    "Screen-Extract": ("GUI ExtractPage (static preview)", 50, "extraction stays manual"),
    "screen_extract": ("GUI ExtractPage (static preview)", 50, ""),
    "Screen-DownloadFirmware": ("GUI DownloadPage (static preview)", 30, "no fetch from page"),
    "screen_download": ("GUI DownloadPage (static preview)", 30, ""),
    "Screen-Firmware": ("GUI FirmwarePage (static preview)", 60, "no download from page"),
    "screen_firmware": ("GUI FirmwarePage (static preview)", 60, ""),
    "Screen-ExportRecovery": ("GUI ExportPage (static preview)", 40, "repack open"),
    "screen_export": ("GUI ExportPage (static preview)", 40, ""),
    "Screen-KernelFixes": ("GUI KernelPage (informational)", 80, ""),
    "screen_kernelfixes": ("GUI KernelPage (informational)", 80, ""),
    "Screen-Twrp": ("GUI TwrpPage (static preview)", 40, "flash gated"),
    "screen_twrp": ("GUI TwrpPage (static preview)", 40, ""),
    "Start-TTTui": ("GUI app shell replaces TUI loop", 100, ""),
    "main_menu": ("GUI nav replaces menu dispatch", 100, ""),
    "Screen-Analyze": ("Analyze-device page still needed", 20, "GSI page covers image-side only"),
    "screen_analyze": ("Analyze-device page still needed", 20, ""),
    "Screen-Compatibility": ("GUI CompatPage (static preview)", 75, "live registry load pending"),
    "screen_compat": ("GUI CompatPage (static preview)", 75, ""),
    "Screen-GoalSelect": ("GUI GoalsPage (static preview)", 70, "live plan + run stays scripts"),
    "screen_goals": ("GUI GoalsPage (static preview)", 70, ""),
    "Screen-RootMethods": ("GUI RootMethodsPage (static table)", 80, "live wiring pending"),
    "screen_rootmethods": ("GUI RootMethodsPage (static table)", 80, ""),
    "Screen-Patch": ("GUI PatchPage (rule + refusal)", 65, "live Magisk flow refused"),
    "screen_patch": ("GUI PatchPage (rule + refusal)", 65, ""),
    "Screen-PersistFixes": ("GUI PersistPage (plan render)", 70, "live install refused"),
    "screen_persist": ("GUI PersistPage (plan render)", 70, ""),
    "Select-InstalledRom": ("CLI cli_rom_options + gsi-state persist", 95, "twin of select_rom"),
    "Show-TTHelp": ("GUI HelpPage (static text)", 90, ""),
    "show_help": ("GUI HelpPage (static text)", 90, ""),
    "Show-TTStatus": ("GUI StatusPage (static preview)", 75, "missing renders as unknown"),
    "status_screen": ("GUI StatusPage (static preview)", 75, ""),
    "Start-TTWizard": ("GUI WizardPage (text-only path)", 60, "live run refused"),
    "wizard": ("GUI WizardPage (text-only path)", 60, ""),
    "Show-TTMenu": ("GUI nav is the replacement", 90, "OS actions stay OS-side"),
    "Show-TTCheckMenu": ("GUI nav group Check", 90, ""),
    "Show-TTStepsMenu": ("GUI nav group Steps", 90, ""),
    "Show-TTWorkflowMenu": ("GUI nav group Workflows", 90, ""),
    "Show-TTSettingsMenu": ("GUI nav group Settings", 90, ""),
    "menu_check": ("GUI nav group Check", 90, ""),
    "menu_settings": ("GUI nav group Settings", 90, ""),
    "menu_steps": ("GUI nav group Steps", 90, ""),
    "menu_workflows": ("GUI nav group Workflows", 90, ""),
    "rom_broken": ("filter inside gsi-registry::rom_options", 100, ""),
    "ensure_tool": ("CLI platform_tools_install_plan", 80, "interactive guiding in dispatch"),
    "ensure_scrcpy": ("CLI platform_tools_install_plan", 80, ""),
    "Get-RomOptions": ("gsi-registry::rom_options + GUI CompatPage", 95, ""),
    "rom_options": ("gsi-registry::rom_options + GUI CompatPage", 95, ""),
    "Get-CompatRegistry": ("gsi-registry::load_roms + GUI CompatPage", 95, ""),
    "compat_roms": ("gsi-registry::load_roms + GUI CompatPage", 95, ""),
    "compat_broken_markers": ("gsi-registry::is_selectable + GUI CompatPage", 95, ""),
    "compat_firmware_base": ("gsi-registry::firmware_compat + GUI CompatPage", 95, ""),
    "profile_gsi_advice": ("gsi-registry::target_config + GUI CompatPage", 90, ""),
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
