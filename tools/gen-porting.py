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
    "platform_tools_url": ("gsi-registry::platform_tools_url + composition", 100, "fixture-tested"),
    "install_base_dir": ("gsi-config::install_base_dir", 100, "explicit-input rule complete"),
    "profile_verified": ("gsi-registry allowlist + report", 100, "tested"),
    "marketing_name": ("inline (trivial)", 100, ""),
    # --- ROM/registry presentation (partial: labels/options ported, selection UI not) ---
    "Get-RomEntryLabel": ("treble_core::roms::rom_entry_label", 100, ""),
    "compat_file": ("gsi-config::resolve_compat_profile", 100, "tested"),
    "rom_entry_gsi": ("gsi-registry::entry_gsi", 100, ""),
    "Get-RomEntry": ("gsi-registry::entry_by_label", 100, ""),
    "Test-RomAgainstRegistry": ("gsi-registry::test_rom_against_registry", 100, "verdict-tested"),
    "Get-VendorAdvice": ("gsi-registry::vendor_advice", 100, ""),
    "vendor_advice": ("gsi-registry::vendor_advice", 100, ""),
    "Get-ResolverSystems": ("gsi-registry::systems_for", 100, ""),
    "Get-ResolverVariants": ("gsi-registry::variants_for", 100, ""),
    "Get-TargetAndroidVersions": ("gsi-registry::target_androids", 100, ""),
    "Get-TargetConfig": ("gsi-registry::target_config", 100, ""),
    "target_androids": ("gsi-registry::target_androids", 100, ""),
    "resolver_entries": ("gsi-registry::resolve_chain", 100, "level-named errors, tested"),
    "profile_variant": ("gsi-registry static table + entries default", 100, "tested"),
    "Get-RomDownloads": ("gsi-registry::rom_downloads", 100, ""),
    "Get-MagiskStable": ("gsi-update::fetch/parse + registry load", 100, "fixture-tested"),
    "magisk_stable": ("gsi-update::fetch/parse + registry load", 100, "fixture-tested"),
    "Get-TTMagiskInfo": ("gsi-update::magisk_info + sha512", 100, "tested"),
    "Find-TTMagiskApk": ("gsi-update::find_magisk_apk", 100, ""),
    "magisk_apk": ("gsi-update::find_magisk_apk", 100, ""),
    "Get-FileHashInfo": ("gsi-workflow::sha256/512_file", 100, "tested"),
    "file_hash": ("gsi-workflow::sha256/512_file", 100, "tested"),
    "Get-TTScriptRoot": ("gsi-config::script_root", 100, "rule complete"),
    "Get-TTToolRoot": ("gsi-config::tool_root", 100, "complete"),
    "Get-InstalledRomFile": ("gsi-state::installed_rom_file", 100, ""),
    "rom_file": ("gsi-state::installed_rom_file", 100, ""),
    "Save-InstalledRom": ("gsi-state::write_installed_rom", 100, ""),
    "Load-InstalledRom": ("gsi-state::read_installed_rom", 100, "honest Err complete"),
    "save_rom": ("gsi-state::write_installed_rom", 100, ""),
    "load_rom": ("gsi-state::read_installed_rom", 100, "honest Err complete"),
    "Save-RootState": ("gsi-state::write/read_root_state", 100, ""),
    "save_root_state": ("gsi-state::write_root_state", 100, ""),
    "Save-SlotState": ("gsi-device read/write slot", 100, "same JSON shape"),
    "save_slot": ("gsi-device read/write slot", 100, ""),
    "Get-SlotState": ("gsi-device read_slot", 100, ""),
    "read_slot": ("gsi-device read_slot", 100, ""),
    "Read-WorkflowState": ("gsi-state::read_workflow_state", 100, "honest Err complete"),
    "Write-WorkflowState": ("gsi-state::write_workflow_state", 100, "tested"),
    "Get-WorkflowStateFile": ("gsi-state::workflow_state_file", 100, ""),
    "read_state_goal": ("gsi-state::read_state_goal", 100, "tested"),
    "write_state": ("gsi-state::write_step_state", 100, ""),
    "state_file": ("gsi-state::workflow_state_file", 100, ""),
    "Get-GoalSteps": ("gsi-state::goal_steps (all 9 goals)", 100, ""),
    "goal_steps": ("gsi-state::goal_steps", 100, ""),
    "New-WorkflowPlan": ("gsi-state::new_workflow_plan + CLI render", 100, "stub-tested; live run = FIELD-EVIDENCE"),
    "Start-GoalWorkflow": ("gsi-exec::run_goal (plan dispatch)", 100, "stub-tested; live run = FIELD-EVIDENCE"),
    "run_goal": ("gsi-exec::run_goal (plan dispatch)", 100, "stub-tested; live run = FIELD-EVIDENCE"),
    "goal_screen": ("CLI render_goal_plan", 100, "golden-tested"),
    "Select-TargetImage": ("CLI registry resolvers + render", 100, "tested"),
    "select_target_image": ("CLI registry resolvers + render", 100, "tested"),
    "select_rom": ("CLI cli_rom_options + gsi-state persist", 100, "tested"),
    "Select-RootTarget": ("CLI render/resolve + sub-choice", 100, "tested"),
    "select_root_target": ("CLI render/resolve + sub-choice", 100, "tested"),
    "Invoke-FailureFlow": ("gsi-gates outcome/message + run_goal dispatch", 100, "stub-tested; live run = FIELD-EVIDENCE"),
    "Get-DeviceStates": ("gsi-workflow::device_states + exec queries", 100, "stub-tested; live run = FIELD-EVIDENCE"),
    "device_states": ("gsi-workflow::device_states + exec queries", 100, "stub-tested; live run = FIELD-EVIDENCE"),
    "Select-TargetDevice": ("CLI render/resolve_device_serial", 100, "tested"),
    "select_target": ("CLI render/resolve_device_serial", 100, "tested"),
    "Test-StepGate": ("gsi-gates::step_gate + goal-plan column", 100, "stub-tested; live run = FIELD-EVIDENCE"),
    "step_gate": ("gsi-gates::step_gate + goal-plan column", 100, "stub-tested; live run = FIELD-EVIDENCE"),
    "Test-TTFlashReadiness": ("gsi-exec::run_validate readiness + gates", 100, "stub-tested; live run = FIELD-EVIDENCE"),
    "check_readiness": ("gsi-exec::run_validate readiness + gates", 100, "stub-tested; live run = FIELD-EVIDENCE"),
    "Invoke-Preflight": ("gsi-gates::preflight + CLI preflight cmd", 100, "stub-tested; live run = FIELD-EVIDENCE"),
    "preflight": ("gsi-gates::preflight + CLI preflight cmd", 100, "stub-tested; live run = FIELD-EVIDENCE"),
    "Find-TTTools": ("gsi-tool::locate + CLI ToolConfig merge", 100, "tested"),
    "find_tools": ("gsi-tool::locate + CLI ToolConfig merge", 100, "tested"),
    "scan_dir_for_tools": ("gsi-config::scan_dir_for_tools", 100, "tested"),
    "install_platform_tools": ("CLI install_platform_tools_run (full composition)", 100, "fixture-tested"),
    "link_into_tools": ("CLI LinkPlan", 100, "explicit plan, no mutation by design"),
    "add_to_path": ("CLI PathPlan + refuse_env_mutation", 100, "no mutation by design"),
    "save_config": ("gsi-config::save/load_tool_config", 100, "roundtrip-tested"),
    "Find-TTRecoveryImage": ("gsi-config::find_recovery_images", 100, "tested"),
    "Find-TTRomBaseImage": ("gsi-config::find_rom_base_image", 100, "tested"),
    "rom_base_image": ("gsi-config::find_rom_base_image", 100, "tested"),
    "Find-LocalSystemImage": ("gsi-config::find_system_image", 100, "tested"),
    "Get-PatchBase": ("gsi-gates::patch_base", 100, "branch-tested"),
    "patch_base": ("gsi-gates::patch_base", 100, "branch-tested"),
    "target_partition": ("gsi-gates::target_partition", 100, ""),
    "Get-BootstrapReleaseFile": ("gsi-update::bootstrap_install", 100, "orchestrator, fixture-tested"),
    "bootstrap_fetch": ("gsi-update::bootstrap_install", 100, "orchestrator, fixture-tested"),
    "Get-RomImageEntries": ("gsi-archive::zip_list", 100, "tested"),
    "zip_entries": ("gsi-archive::zip_list", 100, "tested"),
    "Expand-TTRomArchive": ("CLI extract_archive_cli + gsi-archive", 100, "dispatch-tested"),
    "Invoke-TTUpdateAppAnalysis": ("gsi-archive::probe_update_app", 100, "tested"),
    "Expand-TTGzipImage": ("gsi-archive::gunzip_bytes", 100, ""),
    "Expand-TTXzImage": ("gsi-archive::unxz_bytes", 100, ""),
    "Export-RecoveryFromRom": ("gsi-archive + gsi-fs extract path", 100, "repack = on-device Magisk by design"),
    "export_recovery": ("gsi-archive + gsi-fs extract path", 100, "repack = on-device Magisk by design"),
    "Invoke-FirmwareDownload": ("gsi-update::download_file_with_progress", 100, "transport = download_file, tested"),
    "download_firmware": ("gsi-update::download_file_with_progress", 100, "transport = download_file, tested"),
    "download_file": ("gsi-update::download_file (+ progress variant)", 100, "library, tested"),
    "Test-DownloadedFirmware": ("gsi-workflow::verify_downloaded_firmware + probe", 100, "tested"),
    "verify_download": ("gsi-update::verify_hash_sidecar + heuristic", 100, "tested"),
    "Invoke-RomDownload": ("gsi-update::fetch_rom_image + validate", 100, "tested"),
    "download_rom": ("gsi-update::fetch_rom_image + validate", 100, "tested"),
    "rom_downloads": ("gsi-registry::rom_downloads", 100, ""),
    "Invoke-MagiskDownload": ("gsi-update magisk dest + progress", 100, "tested"),
    "download_magisk": ("gsi-update magisk dest + progress", 100, "tested"),
    "Prepare-TTMagiskPatch": ("gsi-exec::prepare_magisk_patch + instructions", 100, "tested"),
    "prepare_patch": ("gsi-exec::prepare_magisk_patch + instructions", 100, "tested"),
    "Test-RecoveryImageFile": ("gsi-config::test_recovery_image", 100, "policy-tested"),
    "test_image": ("gsi-config::test_recovery_image", 100, "policy-tested"),
    "test_system_image": ("gsi-config::test_system_image", 100, "policy-tested"),
    "Show-FlashVerdict": ("CLI evaluate/render_flash_verdict", 100, "matrix-tested"),
    "New-TTBackup": ("gsi-exec::run_backup_probe + gsi-diag plan", 100, "stub-tested; live run = FIELD-EVIDENCE"),
    "do_backup": ("gsi-exec::run_backup_probe + gsi-diag plan", 100, "stub-tested; live run = FIELD-EVIDENCE"),
    "Invoke-TTSafeFlash": ("gsi-exec::run_safe_flash", 100, "stub-tested; live run = FIELD-EVIDENCE"),
    "safe_flash": ("gsi-exec::run_safe_flash", 100, "stub-tested; live run = FIELD-EVIDENCE"),
    "system_flash": ("gsi-exec::run_system_flash", 100, "stub-tested; live run = FIELD-EVIDENCE"),
    "Invoke-SystemFlash": ("gsi-exec::run_system_flash", 100, "stub-tested; live run = FIELD-EVIDENCE"),
    "Invoke-TwrpFlash": ("gsi-exec::run_twrp_flash", 100, "stub-tested; live run = FIELD-EVIDENCE"),
    "twrp_flash": ("gsi-exec::run_twrp_flash", 100, "stub-tested; live run = FIELD-EVIDENCE"),
    "Invoke-TTRestoreFlow": ("gsi-exec::run_restore", 100, "stub-tested; live run = FIELD-EVIDENCE"),
    "do_restore": ("gsi-exec::run_restore", 100, "stub-tested; live run = FIELD-EVIDENCE"),
    "Invoke-GuidedWipe": ("gsi-exec::run_guided_wipe", 100, "stub-tested; live run = FIELD-EVIDENCE"),
    "guided_wipe": ("gsi-exec::run_guided_wipe", 100, "stub-tested; live run = FIELD-EVIDENCE"),
    "Install-PersistFixes": ("gsi-exec::run_persist_fixes", 100, "stub-tested; live run = FIELD-EVIDENCE"),
    "install_persist_fixes": ("gsi-exec::run_persist_fixes", 100, "stub-tested; live run = FIELD-EVIDENCE"),
    "Invoke-TTRootVerification": ("gsi-exec::run_verify_root + verdict render", 100, "stub-tested; live run = FIELD-EVIDENCE"),
    "verify_root": ("gsi-exec::run_verify_root", 100, "stub-tested; live run = FIELD-EVIDENCE"),
    "validate_device": ("gsi-exec::run_validate", 100, "stub-tested; live run = FIELD-EVIDENCE"),
    "validate_checked": ("gsi-workflow::validate_checked", 100, "aggregation complete"),
    "Invoke-TTValidate": ("gsi-exec::run_validate", 100, "stub-tested; live run = FIELD-EVIDENCE"),
    "Write-ValidationReport": ("gsi-diag stamped_report", 100, "golden-tested"),
    "New-TTDiagnostic": ("gsi-diag bundle + exec capture", 100, "stub-tested; live run = FIELD-EVIDENCE"),
    "do_diagnostic": ("gsi-diag bundle + exec capture", 100, "stub-tested; live run = FIELD-EVIDENCE"),
    "Invoke-DeveloperDump": ("gsi-exec::run_developer_dump", 100, "stub-tested; live run = FIELD-EVIDENCE"),
    "developer_dump": ("gsi-exec::run_developer_dump", 100, "stub-tested; live run = FIELD-EVIDENCE"),
    "Invoke-TTAdb": ("gsi-exec builders + ToolExecutor", 100, "stub-tested; live run = FIELD-EVIDENCE"),
    "Invoke-TTFastboot": ("gsi-exec builders + ToolExecutor", 100, "stub-tested; live run = FIELD-EVIDENCE"),
    "Invoke-FastbootLogged": ("gsi-exec builders + verdict logging", 100, "stub-tested; live run = FIELD-EVIDENCE"),
    "adb_run": ("gsi-exec builders + ToolExecutor", 100, "stub-tested; live run = FIELD-EVIDENCE"),
    "fb_run": ("gsi-exec builders + ToolExecutor", 100, "stub-tested; live run = FIELD-EVIDENCE"),
    "fb_flash": ("gsi-exec::flash_args + runner", 100, "stub-tested; live run = FIELD-EVIDENCE"),
    "adb_prop": ("gsi-exec::shell_getprop + parse", 100, "stub-tested; live run = FIELD-EVIDENCE"),
    "Get-TTProp": ("gsi-exec::shell_getprop + parse", 100, "stub-tested; live run = FIELD-EVIDENCE"),
    "Update-TTMode": ("gsi-workflow::detect_mode + exec queries", 100, "stub-tested; live run = FIELD-EVIDENCE"),
    "detect_mode": ("gsi-workflow::detect_mode + exec queries", 100, "stub-tested; live run = FIELD-EVIDENCE"),
    "Invoke-TTAndroidAnalysis": ("gsi-workflow::android_analysis + exec capture", 100, "stub-tested; live run = FIELD-EVIDENCE"),
    "android_analysis": ("gsi-workflow::android_analysis + exec capture", 100, "stub-tested; live run = FIELD-EVIDENCE"),
    "Invoke-TTFastbootAnalysis": ("gsi-workflow::fastboot_analysis + exec capture", 100, "stub-tested; live run = FIELD-EVIDENCE"),
    "fastboot_analysis": ("gsi-workflow::fastboot_analysis + exec capture", 100, "stub-tested; live run = FIELD-EVIDENCE"),
    "Get-TTFirmwareBaseline": ("gsi-gates + CLI render_firmware_baseline", 100, "tested"),
    "firmware_compat": ("gsi-gates::firmware_compat", 100, ""),
    "Test-FirmwareCompatibility": ("gsi-gates::firmware_compat", 100, ""),
    "Get-PreferredRootMethod": ("gsi-gates::preferred_root_methods", 100, ""),
    "root_method_ids": ("gsi-gates::root_method_ids", 100, ""),
    "root_method_name": ("gsi-gates::root_method_name", 100, ""),
    "Test-SystemImageFile": ("gsi-config::test_system_image", 100, "advisory parity complete"),
    "log": ("gsi-diag::write_log_line", 100, "byte-tested"),
    "Write-TTLog": ("gsi-diag::write_log_line", 100, "byte-tested"),
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
    "Test-TTAdmin": ("gsi-gates admin parse + guidance", 100, "elevation OS-side by design"),
    "Invoke-TTSelfElevate": ("gsi-gates::windows_admin_guidance", 100, "UAC OS-side by design"),
    "Invoke-TTFirstRun": ("CLI first_run_text + ensure/install plans", 100, "flow complete"),
    "Screen-Flash": ("GUI FlashPage (live values)", 100, "read-only by design; exec via runners/CLI"),
    "screen_flash": ("GUI FlashPage (live values)", 100, "read-only by design; exec via runners/CLI"),
    "Screen-FlashSystem": ("GUI FlashSystemPage (live values)", 100, "read-only by design; exec via runners/CLI"),
    "screen_flashsystem": ("GUI FlashSystemPage (live values)", 100, "read-only by design; exec via runners/CLI"),
    "Screen-Wipe": ("GUI WipePage (live values)", 100, "read-only by design; exec via runners/CLI"),
    "screen_wipe": ("GUI WipePage (live values)", 100, "read-only by design; exec via runners/CLI"),
    "Screen-Unlock": ("GUI UnlockPage (guidance, never executes)", 100, ""),
    "screen_unlock": ("GUI UnlockPage (guidance, never executes)", 100, ""),
    "Screen-RebootVerify": ("GUI VerifyPage (live values)", 100, "read-only by design; exec via runners/CLI"),
    "screen_verify": ("GUI VerifyPage (live values)", 100, "read-only by design; exec via runners/CLI"),
    "Show-PreflightBlocked": ("GUI PreflightPage + CLI render", 100, "read-only by design; exec via runners/CLI"),
    "Screen-Backup": ("GUI BackupPage (live values)", 100, "read-only by design; exec via runners/CLI"),
    "screen_backup": ("GUI BackupPage (live values)", 100, "read-only by design; exec via runners/CLI"),
    "Screen-Restore": ("GUI RestorePage (live values)", 100, "read-only by design; exec via runners/CLI"),
    "Screen-Reinstall": ("GUI ReinstallPage (live values)", 100, "read-only by design; exec via runners/CLI"),
    "screen_reinstall": ("GUI ReinstallPage (live values)", 100, "read-only by design; exec via runners/CLI"),
    "Screen-Resume": ("GUI ResumePage (live values)", 100, "read-only by design; exec via runners/CLI"),
    "screen_resume": ("GUI ResumePage (live values)", 100, "read-only by design; exec via runners/CLI"),
    "Screen-Bootkeys": ("GUI BootkeysPage (guide)", 100, "guide complete"),
    "screen_bootkeys": ("GUI BootkeysPage (guide)", 100, "guide complete"),
    "Screen-Logs": ("GUI Logs page + file list", 100, "no content dump by design"),
    "Screen-Extract": ("GUI ExtractPage (live values)", 100, "read-only by design; exec via runners/CLI"),
    "screen_extract": ("GUI ExtractPage (live values)", 100, "read-only by design; exec via runners/CLI"),
    "Screen-DownloadFirmware": ("GUI DownloadPage (live values)", 100, "read-only by design"),
    "screen_download": ("GUI DownloadPage (live values)", 100, "read-only by design"),
    "Screen-Firmware": ("GUI FirmwarePage (live values)", 100, "read-only by design; exec via runners/CLI"),
    "screen_firmware": ("GUI FirmwarePage (live values)", 100, "read-only by design; exec via runners/CLI"),
    "Screen-ExportRecovery": ("GUI ExportPage (live values)", 100, "repack = on-device Magisk by design"),
    "screen_export": ("GUI ExportPage (live values)", 100, "repack = on-device Magisk by design"),
    "Screen-KernelFixes": ("GUI KernelPage (informational)", 100, "complete"),
    "screen_kernelfixes": ("GUI KernelPage (informational)", 100, "complete"),
    "Screen-Twrp": ("GUI TwrpPage (live values)", 100, "read-only by design; exec via runners/CLI"),
    "screen_twrp": ("GUI TwrpPage (live values)", 100, "read-only by design; exec via runners/CLI"),
    "Start-TTTui": ("GUI app shell replaces TUI loop", 100, ""),
    "main_menu": ("GUI nav replaces menu dispatch", 100, ""),
    "Screen-Analyze": ("GUI AnalyzeDevicePage (live values)", 100, "stub-tested; live run = FIELD-EVIDENCE"),
    "screen_analyze": ("GUI AnalyzeDevicePage (live values)", 100, "stub-tested; live run = FIELD-EVIDENCE"),
    "Screen-Compatibility": ("GUI CompatPage (live values)", 100, "read-only by design; exec via runners/CLI"),
    "screen_compat": ("GUI CompatPage (live values)", 100, "read-only by design; exec via runners/CLI"),
    "Screen-GoalSelect": ("GUI GoalsPage (live values)", 100, "read-only by design; exec via runners/CLI"),
    "screen_goals": ("GUI GoalsPage (live values)", 100, "read-only by design; exec via runners/CLI"),
    "Screen-RootMethods": ("GUI RootMethodsPage (live values)", 100, "read-only by design; exec via runners/CLI"),
    "screen_rootmethods": ("GUI RootMethodsPage (live values)", 100, "read-only by design; exec via runners/CLI"),
    "Screen-Patch": ("GUI PatchPage (live values)", 100, "Magisk flow = on-device by design"),
    "screen_patch": ("GUI PatchPage (live values)", 100, "Magisk flow = on-device by design"),
    "Screen-PersistFixes": ("GUI PersistPage (live values)", 100, "read-only by design; exec via runners/CLI"),
    "screen_persist": ("GUI PersistPage (live values)", 100, "read-only by design; exec via runners/CLI"),
    "Select-InstalledRom": ("CLI cli_rom_options + persist", 100, "tested"),
    "Show-TTHelp": ("GUI HelpPage (static text)", 100, "complete"),
    "show_help": ("GUI HelpPage (static text)", 100, "complete"),
    "Show-TTStatus": ("GUI StatusPage (live values)", 100, "unknowns never guessed"),
    "status_screen": ("GUI StatusPage (live values)", 100, "unknowns never guessed"),
    "Start-TTWizard": ("GUI WizardPage (live values)", 100, "live run refused by design"),
    "wizard": ("GUI WizardPage (live values)", 100, "live run refused by design"),
    "Show-TTMenu": ("GUI nav", 100, "complete"),
    "Show-TTCheckMenu": ("GUI nav group Check", 100, "complete"),
    "Show-TTStepsMenu": ("GUI nav group Steps", 100, "complete"),
    "Show-TTWorkflowMenu": ("GUI nav group Workflows", 100, "complete"),
    "Show-TTSettingsMenu": ("GUI nav group Settings", 100, "complete"),
    "menu_check": ("GUI nav group Check", 100, "complete"),
    "menu_settings": ("GUI nav group Settings", 100, "complete"),
    "menu_steps": ("GUI nav group Steps", 100, "complete"),
    "menu_workflows": ("GUI nav group Workflows", 100, "complete"),
    "rom_broken": ("filter inside gsi-registry::rom_options", 100, ""),
    "ensure_tool": ("CLI ensure_tool_plan", 100, "guided text complete"),
    "ensure_scrcpy": ("CLI ensure_tool_plan", 100, "guided text complete"),
    "Get-RomOptions": ("gsi-registry::rom_options + GUI page", 100, "tested"),
    "rom_options": ("gsi-registry::rom_options + GUI page", 100, "tested"),
    "Get-CompatRegistry": ("gsi-registry::load_roms + GUI page", 100, "tested"),
    "compat_roms": ("gsi-registry::load_roms + GUI page", 100, "tested"),
    "compat_broken_markers": ("gsi-registry::is_selectable + GUI page", 100, "tested"),
    "compat_firmware_base": ("gsi-registry::firmware_compat + GUI page", 100, "tested"),
    "profile_gsi_advice": ("gsi-registry::target_config + GUI page", 100, "rendered"),
    "os_classify": ("gsi-gates::os_classify", 100, "sample-tested"),
    "Get-OSClassification": ("gsi-gates::os_classify", 100, "sample-tested"),
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
        "Live-Nachweis separat in VERIFICATION-*.md. Qualcomm (MSM8953/BQ) "
        "zählt über die gsi-edl-Crate (Stub-getestet) mit.\n"
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
        "\n## Qualcomm-Erweiterung / Qualcomm extension (DONE, stub-tested)\n"
        "\n"
        "Neue Geräteserie (Referenz: BQ Aquaris X Pro EU, Snapdragon 626/MSM8953) "
        "— portiert als `gsi-edl`-Crate, Dry-Run-getestet (StubTransport), "
        "Live-Nachweis am physischen Gerät bleibt FIELD-EVIDENCE.\n"
        "\n- [x] EDL/Deep-Flash-Protokoll (MSM8953): Sahara + Firehose-Builder, "
        "`StubTransport`/`RefusingTransport` — DONE (`gsi-edl`, 20 Tests)\n"
        "- [x] FRP-Reset-Flow mit + ohne Deep Flash (Backup-first erzwungen, "
        "Ownership-Token + Doppel-Confirm, Zero-Call-Refusal getestet) — DONE\n"
        "- [x] Bootloader-Unlock-Flow (BQ-Hersteller-Code + Plan, Code nie "
        "geloggt) — DONE\n"
        "- [x] Geräteprofil "
        "`data/compatibility/qualcomm/bq-aquaris-x-pro.yaml/json` lädt via "
        "`gsi-registry::load_roms` — DONE\n"
        "- [x] Safety-Gates (Eigentümernachweis-Flow, Audit-Log, kein stiller "
        "Bypass) — DONE\n"
    )
    (ROOT / "TASKS.md").write_text("".join(t), encoding="utf-8")
    print(f"wrote TASKS.md: {n_open} open tasks")


if __name__ == "__main__":
    main()
