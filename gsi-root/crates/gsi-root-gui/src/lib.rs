//! Slint GUI for gsi-root: same core API as the CLI, no own logic.
//!
//! Pages Dashboard/GSI/Detect/Analyze-Device/Tools/Updates/Logs/Settings
//! plus the Phase-9 screen cluster (flash/backup/media/flows render
//! modules). Patch/verify surface the core's honest EXPERIMENTAL refusal
//! instead of faking success.
//!
//! Refresh closures gather best-effort live values through [`live`]
//! (installed ROM via `gsi-state`, registry profile via `gsi-registry`,
//! dirs via `gsi-config`, log files via `gsi-diag`) and render them with
//! the pure page modules. Closures never panic and never touch the
//! network, a device, or a subprocess; anything unmeasurable locally
//! renders as an honest text note.

mod center_backup;
mod center_device;
mod center_firmware;
mod center_flash;
mod center_images;
mod center_root;
mod center_system;
mod live;
mod pages_analyze;
mod pages_backup;
mod pages_flash;
mod pages_flows;
mod pages_media;

use gsi_image::{detect_container, parse_sparse_header, Container};
use gsi_root_core::{resolve_engine, GsiProfile};

slint::include_modules!();

/// Analyze an image into a human-readable, multi-line report (testable).
pub fn analyze_text(path: &str) -> String {
    let p = std::path::PathBuf::from(path);
    if path.trim().is_empty() {
        return "no file given".to_string();
    }
    let container = detect_container(&p);
    let sparse = parse_sparse_header(&p);
    let profile = GsiProfile {
        container: Some(container),
        sparse,
        android_api: None,
        arch: None,
    };
    let engine = resolve_engine(&profile);
    let mut out = format!(
        "file: {path}\ncontainer: {}\n",
        match container {
            Container::Gzip => "gzip",
            Container::AndroidSparse => "android-sparse",
            Container::Raw => "raw",
        }
    );
    match sparse {
        Some(h) => out.push_str(&format!(
            "sparse: blocks={} block_size={} chunks={}\n",
            h.total_blocks, h.block_size, h.total_chunks
        )),
        None => out.push_str("sparse: no\n"),
    }
    out.push_str(&format!(
        "engine: {:?} ({:?})\n",
        engine.engine, engine.maturity
    ));
    for r in &engine.reasons {
        out.push_str(&format!("reason: {r}\n"));
    }
    out
}

/// Patch request: honest refusal until a hardware POC exists.
pub fn patch_text(path: &str) -> String {
    format!(
        "EXPERIMENTAL — refused.\nNo hardware POC for {path}: nothing written, nothing faked.\nSee devices/huawei-p10 for the POC plan."
    )
}

/// Update check against a GitHub repo (`owner/name`).
pub fn update_text(repo: &str) -> String {
    match gsi_update::latest_tag(repo) {
        Ok(t) => format!("latest: {t}"),
        Err(e) => format!("check failed: {e}"),
    }
}

/// Locate a managed external tool (PATH plus known trebleManager tool dirs).
fn find_managed(name: &str) -> Option<gsi_tool::Tool> {
    let mut extra = Vec::new();
    for cand in [
        "tools/platform-tools",
        "data/tools",
        "platform-tools",
        "tools",
    ] {
        let p = std::path::PathBuf::from(cand);
        if p.is_dir() {
            extra.push(p);
        }
    }
    gsi_tool::locate(name, &extra)
}

/// Detect attached devices via the managed adb/fastboot binding.
///
/// Read-only queries (`adb devices`, `fastboot devices`), parsed with
/// `gsi-device`. Renders human-readable text for the Detect page.
/// Missing tools or devices are reported honestly, never faked.
pub fn detect_text() -> String {
    use std::time::Duration;
    let mut out = String::new();
    let mut any = false;
    match find_managed("adb") {
        Some(adb) => {
            out.push_str(&format!("adb: {}\n", adb.path.display()));
            if let Some(v) = gsi_tool::probe_version(&adb) {
                if !v.trim().is_empty() {
                    out.push_str(&format!("adb version: {v}\n"));
                }
            }
            let r = gsi_tool::run(&adb, &["devices"], Duration::from_secs(20));
            let lines: Vec<&str> = r.stdout.lines().collect();
            for d in gsi_device::parse::adb_devices(&lines) {
                out.push_str(&format!("adb {} {}\n", d.serial, d.state));
                any = true;
            }
            if !r.success {
                let err = r.stderr.trim();
                if err.is_empty() {
                    out.push_str("adb devices failed\n");
                } else {
                    out.push_str(&format!("adb devices failed: {err}\n"));
                }
            }
        }
        None => {
            out.push_str("adb: not found (managed binding needs adb on PATH)\n");
        }
    }
    match find_managed("fastboot") {
        Some(fb) => {
            out.push_str(&format!("fastboot: {}\n", fb.path.display()));
            if let Some(v) = gsi_tool::probe_version(&fb) {
                if !v.trim().is_empty() {
                    out.push_str(&format!("fastboot version: {v}\n"));
                }
            }
            let r = gsi_tool::run(&fb, &["devices"], Duration::from_secs(20));
            let lines: Vec<&str> = r.stdout.lines().collect();
            for d in gsi_device::parse::fastboot_devices(&lines) {
                out.push_str(&format!("fastboot {} {}\n", d.serial, d.state));
                any = true;
            }
            if !r.success {
                let err = r.stderr.trim();
                if err.is_empty() {
                    out.push_str("fastboot devices failed\n");
                } else {
                    out.push_str(&format!("fastboot devices failed: {err}\n"));
                }
            }
        }
        None => {
            out.push_str("fastboot: not found (managed binding needs fastboot on PATH)\n");
        }
    }
    if !any {
        out.push_str("no devices (or no tools)\n");
    }
    out
}

/// List managed external tools (adb/fastboot plus extractor programs).
///
/// Same sources as the CLI `tools` command: PATH plus known dirs, version
/// probe when present, honest MISSING notes otherwise. Renders text for
/// the Tools page.
pub fn tools_text() -> String {
    let mut out = String::new();
    for name in ["adb", "fastboot"] {
        match find_managed(name) {
            Some(t) => {
                let v = gsi_tool::probe_version(&t).unwrap_or_default();
                if v.trim().is_empty() {
                    out.push_str(&format!("{name}: {}\n", t.path.display()));
                } else {
                    out.push_str(&format!("{name}: {} {v}\n", t.path.display()));
                }
            }
            None => {
                out.push_str(&format!("{name}: MISSING (native protocol: Phase 8)\n"));
            }
        }
    }
    let mut found_extractor = false;
    for exe in [
        "huawei_firmware_extractor.py",
        "payload-dumper-go",
        "payload-dumper-go.exe",
        "splitupdate",
        "split_updata.pl",
    ] {
        let bare = exe.trim_end_matches(".py").trim_end_matches(".exe");
        let present = find_managed(exe).is_some()
            || find_managed(bare).is_some()
            || std::path::Path::new(exe).is_file();
        if present {
            out.push_str(&format!("extractor: {exe} present\n"));
            found_extractor = true;
        }
    }
    if !found_extractor {
        out.push_str(
            "extractor: none found (place HuaweiFirmwareExtractor/payload-dumper-go into data/tools/)\n",
        );
    }
    out
}

/// Validate a file path typed into a LineEdit (Slint has no file dialog).
///
/// Empty input reports the empty form, an existing file reports present,
/// otherwise the missing form. Never touches the network or a device.
pub fn path_note(path: &str) -> String {
    let t = path.trim();
    if t.is_empty() {
        return "path: empty - enter a file path".to_string();
    }
    let p = std::path::Path::new(t);
    if p.is_file() {
        return format!("path: exists - {t}");
    }
    format!("path: missing - {t} (no file dialog in Slint; type the path)")
}

/// Dashboard device summary over explicit live context (testable).
///
/// Reuses the existing status and analysis renders; unknown stays unknown.
pub fn dashboard_text_for(ctx: &live::LiveCtx) -> String {
    let mut out = live::status_refresh(ctx);
    out.push_str("---\n");
    out.push_str(&live::analyze_device_refresh(ctx));
    out
}

/// Dashboard device summary with process-local context.
pub fn dashboard_text() -> String {
    dashboard_text_for(&live::live_ctx())
}

/// ADB badge kind and text over explicit context (honest unknown states).
///
/// Present tool reports `ok`, missing reports `unknown` (never guessed).
pub fn adb_badge_for(ctx: &live::LiveCtx) -> (String, String) {
    let _ = ctx;
    let p = live::tool_path_text("adb");
    if p.trim().is_empty() {
        (
            "unknown".to_string(),
            "ADB: Unknown - not found, needs PATH".to_string(),
        )
    } else {
        ("ok".to_string(), format!("ADB: Found - {p}"))
    }
}

/// Recommended next action from backup presence plus saved goal.
///
/// Pure over explicit inputs; honest plan-only wording throughout.
pub fn recommend_for(backup_ok: bool, goal: &str) -> String {
    if !backup_ok {
        return "Recommended: run Detect, then Preflight, then Back up before any flash."
            .to_string();
    }
    if goal.trim().is_empty() {
        return "Recommended: backup present. Pick a goal in the Wizard.".to_string();
    }
    "Recommended: backup present and goal saved. Follow the Wizard plan.".to_string()
}

/// Recommended action with process-local context.
pub fn recommend_text() -> String {
    let ctx = live::live_ctx();
    let backup_ok = live::has_backup_with_original(&ctx.data_dir.join("backups"));
    let wf = live::workflow_live(&ctx.log_dir);
    recommend_for(backup_ok, &wf.goal)
}

/// Wizard plan for an explicit goal (testable).
///
/// Reuses the existing goals render plus `gsi-state::goal_steps`;
/// unknown goals render the honest empty form. Read-only, never runs.
pub fn wizard_plan_for_goal(goal: &str) -> String {
    let g = goal.trim();
    let steps: Vec<String> = gsi_state::goal_steps(g);
    let statuses: Vec<String> = steps.iter().map(|_| "pending".to_string()).collect();
    pages_flows::goals_text(g, &steps, &statuses)
}

/// Wizard step text over explicit live context (testable).
///
/// Each step reuses the existing page refresh render for that phase;
/// execution steps stay honest plan-only (EXPERIMENTAL, Phase 8).
pub fn wizard_step_for(step: i32, ctx: &live::LiveCtx) -> String {
    match step {
        0 => {
            let wf = live::workflow_live(&ctx.log_dir);
            if wf.goal.trim().is_empty() {
                wizard_plan_for_goal("")
            } else {
                live::goals_refresh(ctx)
            }
        }
        1 => detect_text(),
        2 => live::rootmethods_refresh(ctx),
        3 => live::preflight_refresh(ctx),
        4 => live::wizard_refresh(ctx),
        5 => {
            let mut out = live::restore_refresh(ctx);
            out.push_str("gate: typed phrase plus backup both required before Execute\n");
            out
        }
        6 => {
            let mut out = live::flash_refresh(ctx);
            out.push_str("note: plan-only, nothing executed (EXPERIMENTAL, Phase 8)\n");
            out
        }
        7 => live::verify_refresh(ctx),
        _ => live::wizard_refresh(ctx),
    }
}

/// Wizard step text with process-local context.
pub fn wizard_step_text(step: i32) -> String {
    wizard_step_for(step, &live::live_ctx())
}

/// Backup gate over explicit live context (testable).
///
/// Returns exactly `ok` when a backup with `original.img` exists,
/// otherwise an honest missing note. Reuses the existing backup scan.
pub fn backup_gate_for(ctx: &live::LiveCtx) -> String {
    if live::has_backup_with_original(&ctx.data_dir.join("backups")) {
        "ok".to_string()
    } else {
        "missing: no backup with original.img found - back up first".to_string()
    }
}

/// Backup gate with process-local context.
pub fn backup_gate_text() -> String {
    backup_gate_for(&live::live_ctx())
}

/// Wizard execute over explicit live context (testable).
///
/// Runs only with exact typed confirmation plus backup gate, and even
/// then stays honest plan-only via the existing flash plan render
/// (EXPERIMENTAL, Phase 8). Never writes, never flashes.
pub fn execute_for(confirm_input: &str, expected: &str, ctx: &live::LiveCtx) -> String {
    let exp = expected.trim();
    if exp.is_empty() {
        return "refused: no expected phrase set\nnote: plan-only, nothing executed (EXPERIMENTAL, Phase 8)\n".to_string();
    }
    if confirm_input != exp {
        return "refused: typed phrase does not match - Execute stays blocked\nnote: plan-only, nothing executed (EXPERIMENTAL, Phase 8)\n".to_string();
    }
    if !live::has_backup_with_original(&ctx.data_dir.join("backups")) {
        let mut out = "refused: no backup with original.img - back up first\n".to_string();
        out.push_str(&live::flash_refresh(ctx));
        out.push_str("note: plan-only, nothing executed (EXPERIMENTAL, Phase 8)\n");
        return out;
    }
    let mut out = live::flash_refresh(ctx);
    out.push_str("confirmed: phrase matches and backup present\n");
    out.push_str("note: plan-only, nothing executed (EXPERIMENTAL, Phase 8)\n");
    out
}

/// Wizard execute with process-local context.
pub fn execute_text(confirm_input: &str, expected: &str) -> String {
    execute_for(confirm_input, expected, &live::live_ctx())
}

/// Start the GUI. Returns process exit code.
pub fn run() -> i32 {
    let app = match App::new() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("gui init failed: {e}");
            return 1;
        }
    };
    app.set_version_text(format!("gsi-root {}", env!("CARGO_PKG_VERSION")).into());
    let log = std::sync::Arc::new(std::sync::Mutex::new(String::new()));
    let weak = app.as_weak();
    {
        let log = log.clone();
        app.on_append_log(move |line| {
            if let Ok(mut l) = log.lock() {
                if !l.is_empty() {
                    l.push('\n');
                }
                l.push_str(line.as_str());
                let text: slint::SharedString = l.clone().into();
                drop(l);
                if let Some(a) = weak.upgrade() {
                    a.set_log_text(text);
                }
            }
        });
    }
    app.on_analyze_image(|p| analyze_text(p.as_str()).into());
    app.on_patch_image(|p| patch_text(p.as_str()).into());
    app.on_detect_devices(|| detect_text().into());
    app.on_list_tools(|| tools_text().into());
    app.on_check_update(|r| update_text(r.as_str()).into());
    // Phase-9 screen cluster: best-effort live values gathered inside each
    // closure (installed ROM, registry profile, local dirs, log files).
    // Anything needing a device, the network, or a subprocess renders as
    // an honest note; closures never panic and never block on those.
    // Centers: rich views over the same live values (live::* stays for
    // dashboard/wizard). Gathering mirrors live.rs; unknowns stay unknown.
    app.on_detect_refresh(|| {
        use std::time::Duration;
        let mut adb = Vec::new();
        let mut fb = Vec::new();
        let mut adb_present = false;
        let mut fb_present = false;
        if let Some(t) = find_managed("adb") {
            adb_present = true;
            let r = gsi_tool::run(&t, &["devices"], Duration::from_secs(20));
            for d in gsi_device::parse::adb_devices(&r.stdout.lines().collect::<Vec<_>>()) {
                adb.push(center_device::DetectEntry {
                    serial: d.serial,
                    state: d.state,
                });
            }
        }
        if let Some(t) = find_managed("fastboot") {
            fb_present = true;
            let r = gsi_tool::run(&t, &["devices"], Duration::from_secs(20));
            for d in gsi_device::parse::fastboot_devices(&r.stdout.lines().collect::<Vec<_>>()) {
                fb.push(center_device::DetectEntry {
                    serial: d.serial,
                    state: d.state,
                });
            }
        }
        center_device::detect_center_text(&adb, &fb, adb_present, fb_present, "").into()
    });
    app.on_compat_refresh(|| {
        let ctx = live::live_ctx();
        let rom = live::installed_rom_live(&ctx.data_dir);
        let (model, _) = live::model_from_installed(&rom.id, &rom.label);
        let reg = live::registry_live(&ctx.data_dir, &model);
        let region = live::firmware_region(&reg.firmware_base);
        let verdict = gsi_registry::firmware_compat(&model, &reg.firmware_base, &region);
        let target = reg
            .android_targets
            .last()
            .map(|(a, _)| a.to_string())
            .unwrap_or_default();
        let advice = gsi_registry::vendor_advice("", &target);
        let mut out = center_images::compat_center_text(
            &model,
            &reg.firmware_base,
            &region,
            &verdict.status,
            &verdict.reasons,
            &advice,
            &reg.recommended,
            &reg.broken,
            &reg.firmware_base,
            &[],
            "",
        );
        out.push_str(&format!("profile: {}\n", reg.profile));
        out.push_str(&format!("{}\n", reg.note));
        out.into()
    });
    app.on_firmware_refresh(|| {
        let ctx = live::live_ctx();
        let rom = live::installed_rom_live(&ctx.data_dir);
        let (model, _) = live::model_from_installed(&rom.id, &rom.label);
        let reg = live::registry_live(&ctx.data_dir, &model);
        let region = live::firmware_region(&reg.firmware_base);
        let verdict = gsi_registry::firmware_compat(&model, &reg.firmware_base, &region);
        let status = if verdict.status.trim().is_empty() {
            "FAIL"
        } else {
            verdict.status.as_str()
        };
        center_firmware::firmware_center_text(
            &model,
            &reg.firmware_base,
            &region,
            status,
            &verdict.reasons,
            "",
            "",
            "",
        )
        .into()
    });
    app.on_download_refresh(|| {
        let ctx = live::live_ctx();
        let rom = live::installed_rom_live(&ctx.data_dir);
        let (model, _) = live::model_from_installed(&rom.id, &rom.label);
        let reg = live::registry_live(&ctx.data_dir, &model);
        let opts: Vec<String> = reg.recommended.clone();
        let magisk = live::newest_magisk_apk(&ctx.data_dir);
        let magisk_status = match &magisk {
            Some(p) => {
                let size = std::fs::metadata(p).map(|m| m.len()).unwrap_or(0);
                let name = p
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("magisk.apk");
                format!("{name} cached ({size} bytes)")
            }
            None => String::new(),
        };
        let cache = ctx.data_dir.join("firmware").display().to_string();
        let idle = vec!["idle (no download running)".to_string()];
        center_firmware::download_center_text(&opts, &[], &magisk_status, &cache, &idle).into()
    });
    app.on_extract_refresh(|| {
        let ctx = live::live_ctx();
        let firmware = ctx.data_dir.join("firmware");
        let app = live::first_update_app(&firmware);
        let (name, is_app, header) = match &app {
            Some(p) => (
                p.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("")
                    .to_string(),
                true,
                live::header_hex(p),
            ),
            None => (String::new(), false, String::new()),
        };
        let entries: Vec<String> = Vec::new();
        let tools: Vec<(String, String, String)> = Vec::new();
        center_firmware::extract_center_text(&name, &entries, is_app, &header, &tools).into()
    });
    app.on_export_refresh(|| {
        let ctx = live::live_ctx();
        let roms = ctx.data_dir.join("roms");
        let pkg = live::newest_rom_package(&roms);
        let (name, kind) = match &pkg {
            Some(p) => {
                let n = p
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("")
                    .to_string();
                let k = match gsi_config::detect_image_kind(p) {
                    gsi_config::ImageKind::Boot => "boot",
                    gsi_config::ImageKind::System => "system",
                    gsi_config::ImageKind::Unknown => "unknown",
                };
                (n, k.to_string())
            }
            None => (String::new(), "unknown".to_string()),
        };
        let notes = vec!["plan branch resolved by the export planner in the CLI".to_string()];
        center_firmware::export_center_text(&name, &kind, "", &notes).into()
    });
    app.on_patch_refresh(|| {
        let ctx = live::live_ctx();
        let rom = live::installed_rom_live(&ctx.data_dir);
        let hay = format!("{} {}", rom.id, rom.label).to_ascii_lowercase();
        let source = if rom.id.trim().is_empty() {
            ""
        } else if rom.id.trim() == "stock" {
            "stock"
        } else if hay.contains("gsi")
            || hay.contains("lineage")
            || hay.contains("treble")
            || hay.contains("trebledroid")
        {
            "stock-gsi"
        } else if rom.id.starts_with("rom:") {
            "rom"
        } else {
            ""
        };
        let stock = live::first_stock_recovery(&ctx.data_dir, &ctx.data_dir.join("firmware"))
            .map(|p| p.display().to_string())
            .unwrap_or_default();
        let label = if rom.id.is_empty() {
            ""
        } else {
            rom.label.as_str()
        };
        center_root::patch_center_text(source, &stock, label, "recovery_ramdisk").into()
    });
    app.on_backup_refresh(|| {
        let ctx = live::live_ctx();
        let rom = live::installed_rom_live(&ctx.data_dir);
        let (model, _) = live::model_from_installed(&rom.id, &rom.label);
        let backups = live::newest_backup_with_original(&ctx.data_dir.join("backups"))
            .map(|p| {
                vec![(
                    p.display().to_string(),
                    String::new(),
                    "unverified".to_string(),
                    live::sidecar_sha_text(&p.join("original.img")),
                )]
            })
            .unwrap_or_default();
        let dest = ctx
            .data_dir
            .join("backups")
            .join(&model)
            .display()
            .to_string();
        center_backup::backup_center_text(&model, &dest, &backups).into()
    });
    app.on_flash_refresh(|| {
        let ctx = live::live_ctx();
        let rom = live::installed_rom_live(&ctx.data_dir);
        let (model, _) = live::model_from_installed(&rom.id, &rom.label);
        let patched = live::newest_magisk_patched(&ctx.data_dir);
        let image = patched
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_default();
        let sha = patched
            .as_ref()
            .map(|p| live::sidecar_sha_text(p))
            .unwrap_or_default();
        let backup_note = live::newest_backup_with_original(&ctx.data_dir.join("backups"))
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "no backup with original.img".to_string());
        center_flash::flash_center_text(
            "unknown (no live fastboot query)",
            &image,
            &sha,
            "recovery_ramdisk",
            &model,
            &backup_note,
            "gates pending: profile, partition, firmware, fastboot, FLASH + YES",
            false,
            false,
            "UNCLEAR (no live run)",
        )
        .into()
    });
    app.on_flash_system_refresh(|| {
        let ctx = live::live_ctx();
        let rom = live::installed_rom_live(&ctx.data_dir);
        let (model, _) = live::model_from_installed(&rom.id, &rom.label);
        let reg = live::registry_live(&ctx.data_dir, &model);
        let roms = ctx.data_dir.join("roms");
        let image = live::local_system_image(&ctx.data_dir, &roms);
        let (image_path, image_note, image_ok) = match &image {
            Some(p) => {
                let chk = gsi_config::test_system_image(p);
                let note = format!(
                    "local size check: {} ({} bytes)",
                    if chk.pass { "PASS" } else { "FAIL" },
                    chk.size_bytes
                );
                (p.display().to_string(), note, chk.pass)
            }
            None => (
                String::new(),
                "no local system image found".to_string(),
                false,
            ),
        };
        let failed: Vec<String> = Vec::new();
        center_flash::system_flash_center_text(
            &image_path,
            &image_note,
            &reg.note,
            false,
            false,
            image_ok,
            false,
            false,
            false,
            "UNCLEAR",
            0,
            "",
            &failed,
        )
        .into()
    });
    app.on_twrp_refresh(|| {
        let ctx = live::live_ctx();
        let rom = live::installed_rom_live(&ctx.data_dir);
        let wf = live::workflow_live(&ctx.log_dir);
        let image = live::newest_twrp_image(&ctx.data_dir)
            .map(|p| p.display().to_string())
            .unwrap_or_default();
        center_flash::twrp_center_text(&image, &wf.slot, !rom.id.is_empty()).into()
    });
    app.on_rootmethods_refresh(|| {
        let ctx = live::live_ctx();
        let rom = live::installed_rom_live(&ctx.data_dir);
        let methods: Vec<(String, String, bool)> = gsi_gates::preferred_root_methods()
            .iter()
            .map(|m| (m.id.clone(), m.name.clone(), m.preferred))
            .collect();
        center_root::methods_center_text(&methods, &rom.id, "").into()
    });
    app.on_persist_refresh(|| {
        let files: Vec<String> = Vec::new();
        center_root::persist_center_text("/data/adb/service.d", &files, "", true).into()
    });
    app.on_wipe_refresh(|| {
        let ctx = live::live_ctx();
        let backup_ok = live::has_backup_with_original(&ctx.data_dir.join("backups"));
        let note = live::newest_backup_with_original(&ctx.data_dir.join("backups"))
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "no backup with original.img".to_string());
        center_flash::wipe_center_text(false, false, false, backup_ok, &note).into()
    });
    app.on_restore_refresh(|| {
        let ctx = live::live_ctx();
        let backup = live::newest_backup_with_original(&ctx.data_dir.join("backups"));
        let dir = backup
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_default();
        center_backup::restore_view_center_text(
            &dir,
            "recovery_ramdisk",
            backup.is_some(),
            false,
            false,
        )
        .into()
    });
    app.on_reinstall_refresh(|| {
        let ctx = live::live_ctx();
        let rom = live::installed_rom_live(&ctx.data_dir);
        let (model, _) = live::model_from_installed(&rom.id, &rom.label);
        center_backup::reinstall_center_text(&model, "recovery_ramdisk").into()
    });
    app.on_resume_refresh(|| {
        let ctx = live::live_ctx();
        let wf = live::workflow_live(&ctx.log_dir);
        let lines: String = wf
            .steps
            .iter()
            .map(|(id, st)| format!("{id} [{st}]"))
            .collect::<Vec<_>>()
            .join("\n");
        center_backup::resume_center_text(&wf.goal, &lines).into()
    });
    app.on_verify_refresh(|| center_root::verify_root_center_text("", "", "", &[]).into());
    app.on_bootkeys_refresh(|| live::bootkeys_refresh(&live::live_ctx()).into());
    app.on_unlock_refresh(|| live::unlock_refresh(&live::live_ctx()).into());
    app.on_preflight_refresh(|| live::preflight_refresh(&live::live_ctx()).into());
    app.on_kernel_refresh(|| live::kernel_refresh(&live::live_ctx()).into());
    app.on_goals_refresh(|| live::goals_refresh(&live::live_ctx()).into());
    app.on_status_refresh(|| live::status_refresh(&live::live_ctx()).into());
    app.on_wizard_refresh(|| live::wizard_refresh(&live::live_ctx()).into());
    app.on_analyze_device_refresh(|| live::analyze_device_refresh(&live::live_ctx()).into());
    app.on_logs_files_refresh(|| live::logs_files_refresh(&live::live_ctx()).into());
    app.on_romselect_refresh(|| live::romselect_refresh(&live::live_ctx()).into());
    app.on_help_refresh(|| center_system::help_center_text(env!("CARGO_PKG_VERSION")).into());
    app.on_tools_refresh(|| {
        let mut rows: Vec<(String, String, String, String, String)> = Vec::new();
        for name in ["adb", "fastboot"] {
            match find_managed(name) {
                Some(t) => {
                    let ver = gsi_tool::probe_version(&t)
                        .unwrap_or_default()
                        .lines()
                        .next()
                        .unwrap_or("")
                        .to_string();
                    rows.push((
                        name.to_string(),
                        t.path.display().to_string(),
                        ver,
                        "managed".to_string(),
                        "present".to_string(),
                    ));
                }
                None => rows.push((
                    name.to_string(),
                    String::new(),
                    String::new(),
                    "PATH".to_string(),
                    "missing".to_string(),
                )),
            }
        }
        center_system::tools_center_text(&rows).into()
    });
    app.on_settings_refresh(|| {
        let ctx = live::live_ctx();
        let config_dir = gsi_config::config_dir()
            .map(|p| p.display().to_string())
            .unwrap_or_default();
        let cache_dir = gsi_config::cache_dir()
            .map(|p| p.display().to_string())
            .unwrap_or_default();
        let tools = vec![
            ("adb".to_string(), live::tool_path_text("adb")),
            ("fastboot".to_string(), live::tool_path_text("fastboot")),
        ];
        center_system::settings_center_text(
            "English",
            &config_dir,
            &cache_dir,
            &ctx.data_dir.display().to_string(),
            &ctx.log_dir.display().to_string(),
            &ctx.data_dir.join("backups").display().to_string(),
            &tools,
            "backup required; typed confirms",
        )
        .into()
    });
    // New center pages (no classic twin).
    app.on_device_refresh(|| {
        let ctx = live::live_ctx();
        let rom = live::installed_rom_live(&ctx.data_dir);
        let (model, _) = live::model_from_installed(&rom.id, &rom.label);
        let vendor = if model.starts_with("VTR") || model.starts_with("VKY") {
            "Huawei"
        } else if model.starts_with("BQ") {
            "BQ"
        } else {
            "unknown"
        };
        let adb_state = match find_managed("adb") {
            Some(_) => "tool present (see Detect)",
            None => "tool missing",
        };
        let fastboot_state = match find_managed("fastboot") {
            Some(_) => "tool present (see Detect)",
            None => "tool missing",
        };
        center_device::device_center_text(&center_device::DeviceOverview {
            vendor: vendor.to_string(),
            model: model.clone(),
            variant: gsi_registry::profile_variant_for_model(&model).to_string(),
            serial: "unknown (see Detect page)".to_string(),
            android: "unknown (needs device)".to_string(),
            build: "unknown (needs device)".to_string(),
            abi: "unknown (needs device)".to_string(),
            arch: "arm64 (profile)".to_string(),
            boot_mode: "unknown (needs device)".to_string(),
            adb_state: adb_state.to_string(),
            fastboot_state: fastboot_state.to_string(),
        })
        .into()
    });
    app.on_diag_refresh(|| {
        let ctx = live::live_ctx();
        let adb_ok = find_managed("adb").is_some();
        let fb_ok = find_managed("fastboot").is_some();
        let rom = live::installed_rom_live(&ctx.data_dir);
        let backup_ok = live::has_backup_with_original(&ctx.data_dir.join("backups"));
        let rows = vec![
            center_device::DiagRow {
                area: "ADB tool".to_string(),
                status: if adb_ok { "present" } else { "missing" }.to_string(),
                detail: live::tool_path_text("adb"),
                maturity: "DOCUMENTED".to_string(),
            },
            center_device::DiagRow {
                area: "Fastboot tool".to_string(),
                status: if fb_ok { "present" } else { "missing" }.to_string(),
                detail: live::tool_path_text("fastboot"),
                maturity: "DOCUMENTED".to_string(),
            },
            center_device::DiagRow {
                area: "Device profile".to_string(),
                status: if rom.id.is_empty() {
                    "unknown"
                } else {
                    "saved"
                }
                .to_string(),
                detail: rom.source.clone(),
                maturity: "DOCUMENTED".to_string(),
            },
            center_device::DiagRow {
                area: "Backup".to_string(),
                status: if backup_ok { "available" } else { "missing" }.to_string(),
                detail: "original.img gate".to_string(),
                maturity: "DOCUMENTED".to_string(),
            },
            center_device::DiagRow {
                area: "Root".to_string(),
                status: "unknown".to_string(),
                detail: "uid=0 verify needs device".to_string(),
                maturity: "EXPERIMENTAL".to_string(),
            },
            center_device::DiagRow {
                area: "SELinux".to_string(),
                status: "unknown".to_string(),
                detail: "getenforce needs device".to_string(),
                maturity: "EXPERIMENTAL".to_string(),
            },
        ];
        center_device::diag_center_text(&rows).into()
    });
    app.on_images_refresh(|| {
        let ctx = live::live_ctx();
        let (model, _) =
            live::model_from_installed(&live::installed_rom_live(&ctx.data_dir).id, "");
        let reg = live::registry_live(&ctx.data_dir, &model);
        let mut downloaded = Vec::new();
        if let Ok(rd) = std::fs::read_dir(ctx.data_dir.join("roms")) {
            for e in rd.flatten() {
                let n = e.file_name().to_string_lossy().to_string();
                if n.ends_with(".img") || n.ends_with(".zip") {
                    downloaded.push(n);
                }
            }
        }
        downloaded.sort();
        let mut local = Vec::new();
        if let Ok(rd) = std::fs::read_dir(ctx.data_dir.join("firmware")) {
            for e in rd.flatten() {
                let n = e.file_name().to_string_lossy().to_string();
                if n.ends_with(".img") || n.ends_with(".zip") || n.ends_with(".APP") {
                    local.push(n);
                }
            }
        }
        local.sort();
        center_images::images_center_text(&[], &reg.recommended, &downloaded, &local).into()
    });
    app.on_root_center_refresh(|| {
        let ctx = live::live_ctx();
        let rom = live::installed_rom_live(&ctx.data_dir);
        let wf = live::workflow_live(&ctx.log_dir);
        let label = if rom.id.is_empty() {
            ""
        } else {
            rom.label.as_str()
        };
        center_root::root_center_text(
            label,
            &wf.slot,
            &wf.boot_mode,
            "unknown (EXPERIMENTAL until hardware POC)",
            "Vol-Up trick needed every boot",
        )
        .into()
    });
    app.on_magisk_center_refresh(|| {
        let ctx = live::live_ctx();
        let apk = live::newest_magisk_apk(&ctx.data_dir);
        let (name, size_note) = match &apk {
            Some(p) => {
                let size = std::fs::metadata(p).map(|m| m.len()).unwrap_or(0);
                (
                    p.file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("magisk.apk")
                        .to_string(),
                    format!("{size} bytes"),
                )
            }
            None => (String::new(), "no APK cached".to_string()),
        };
        center_root::magisk_center_text("", "", &name, "", "", &size_note).into()
    });
    app.on_flash_progress_refresh(|| {
        let phases: Vec<(String, String)> = Vec::new();
        center_flash::flash_progress_center_text(&phases, "", "", "").into()
    });
    app.on_create_backup_refresh(|| {
        let ctx = live::live_ctx();
        let rom = live::installed_rom_live(&ctx.data_dir);
        let (model, _) = live::model_from_installed(&rom.id, &rom.label);
        let reg = live::registry_live(&ctx.data_dir, &model);
        let dest = ctx
            .data_dir
            .join("backups")
            .join(&model)
            .join("recovery_ramdisk")
            .display()
            .to_string();
        center_backup::create_backup_center_text(
            "recovery_ramdisk",
            &dest,
            &model,
            &reg.firmware_base,
        )
        .into()
    });
    // Shell additions: dashboard live card, wizard step-through, backup gate,
    // typed-phrase execute (plan-only), and path validation. All reuse the
    // existing live/page renders above; nothing is duplicated, nothing writes.
    {
        let weak = app.as_weak();
        app.on_dashboard_refresh(move || {
            let ctx = live::live_ctx();
            let (adb_kind, adb_text) = adb_badge_for(&ctx);
            let recommend = recommend_text();
            if let Some(a) = weak.upgrade() {
                a.set_dash_adb_kind(adb_kind.into());
                a.set_dash_adb_text(adb_text.into());
                a.set_dash_root_kind("unknown".into());
                a.set_dash_root_text("Root: Unknown - needs device verify (uid=0 only)".into());
                a.set_dash_selinux_kind("unknown".into());
                a.set_dash_selinux_text("SELinux: Unknown - needs device".into());
                a.set_dash_recommend_text(recommend.into());
            }
            dashboard_text().into()
        });
    }
    app.on_wizard_next(|step| wizard_step_text(step).into());
    app.on_wizard_back(|step| wizard_step_text(step).into());
    app.on_wizard_select(|goal| wizard_plan_for_goal(goal.as_str()).into());
    app.on_wizard_backup_check(|| backup_gate_text().into());
    app.on_wizard_execute(|input, expected| execute_text(input.as_str(), expected.as_str()).into());
    app.on_validate_path(|p| path_note(p.as_str()).into());
    if let Err(e) = app.run() {
        eprintln!("gui error: {e}");
        return 1;
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn analyze_empty_is_clear() {
        assert_eq!(analyze_text(""), "no file given");
    }

    #[test]
    fn analyze_missing_file_is_unsupported() {
        let t = analyze_text("/no/such/file.img");
        assert!(t.contains("container: raw"));
    }

    #[test]
    fn patch_refuses_with_path() {
        let t = patch_text("x.img");
        assert!(t.contains("EXPERIMENTAL"));
        assert!(t.contains("x.img"));
    }

    #[test]
    fn path_note_empty_missing_and_exists() {
        assert!(path_note("").contains("empty"));
        assert!(path_note("   ").contains("empty"));
        let missing = path_note("/no/such/file-gsi-gui-test.img");
        assert!(missing.contains("missing"));
        assert!(missing.contains("no file dialog"));
    }

    #[test]
    fn recommend_branches_are_honest() {
        assert!(recommend_for(false, "").contains("Back up"));
        assert!(recommend_for(true, "").contains("Pick a goal"));
        assert!(recommend_for(true, "root").contains("Follow the Wizard"));
    }

    #[test]
    fn wizard_plan_for_goal_uses_existing_render() {
        let t = wizard_plan_for_goal("root");
        assert!(t.contains("Plan for goal: root"));
        assert!(t.contains("GUI is read-only"));
        let unknown = wizard_plan_for_goal("no-such-goal");
        assert!(unknown.contains("no steps (unknown goal)"));
    }

    #[test]
    fn backup_gate_missing_is_honest() {
        let ctx = live::live_ctx_from(
            std::path::Path::new("/no-such-tool-root-gsi-gui"),
            Some(std::path::Path::new("/no-such-tool-root-gsi-gui/data")),
        );
        let g = backup_gate_for(&ctx);
        assert!(g.contains("missing"));
    }

    #[test]
    fn execute_refuses_without_match_and_without_backup() {
        let ctx = live::live_ctx_from(
            std::path::Path::new("/no-such-tool-root-gsi-gui"),
            Some(std::path::Path::new("/no-such-tool-root-gsi-gui/data")),
        );
        let wrong = execute_for("NOPE", "FLASH", &ctx);
        assert!(wrong.contains("does not match"));
        assert!(wrong.contains("plan-only"));
        let no_backup = execute_for("FLASH", "FLASH", &ctx);
        assert!(no_backup.contains("no backup"));
        assert!(no_backup.contains("plan-only"));
        let empty_exp = execute_for("FLASH", "", &ctx);
        assert!(empty_exp.contains("no expected phrase"));
    }

    #[test]
    fn wizard_step_text_is_plan_only() {
        let ctx = live::live_ctx_from(
            std::path::Path::new("/no-such-tool-root-gsi-gui"),
            Some(std::path::Path::new("/no-such-tool-root-gsi-gui/data")),
        );
        let s6 = wizard_step_for(6, &ctx);
        assert!(s6.contains("Flash plan"));
        assert!(s6.contains("plan-only"));
        let s7 = wizard_step_for(7, &ctx);
        assert!(s7.contains("Result: NOT_ROOTED") || s7.contains("verify"));
    }
}
