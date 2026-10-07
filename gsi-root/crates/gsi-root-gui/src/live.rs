//! Best-effort live values for the GUI refresh closures.
//!
//! The page renderers (`pages_flash`, `pages_backup`, `pages_media`,
//! `pages_flows`, `pages_analyze`) stay pure over explicit inputs. This
//! module gathers those inputs from the local machine without ever
//! touching a device, the network, or a subprocess:
//! installed ROM via `gsi-state`, registry profile via `gsi-registry`,
//! directories via `gsi-config`, log files via `gsi-diag`.
//!
//! Override: `TT_DATA_DIR` points at the tool `data/` dir (which holds
//! `installed-rom.txt`, `compatibility/`, `firmware/`, `roms/` and
//! `backups/`). Without it the tool root derived from
//! `gsi-config::script_root` is used, exactly like the scripts.
//!
//! Every function here is total: missing files, unreadable dirs and
//! unknown values render as honest text notes, never as panics. Tests use
//! temporary dirs only (filesystem reads are local and hermetic; no
//! network, device, or process is involved).

use std::path::{Path, PathBuf};

use crate::{pages_analyze, pages_backup, pages_flash, pages_flows, pages_media};

// ---------------------------------------------------------------- context

/// Resolved local directories for one refresh pass.
#[derive(Debug, Clone)]
pub struct LiveCtx {
    /// Tool root (parent of `scripts/`, or the script dir itself).
    pub tool_root: PathBuf,
    /// Tool `data/` dir (installed ROM, registry, firmware, roms, backups).
    pub data_dir: PathBuf,
    /// Log dir (workflow state, log files).
    pub log_dir: PathBuf,
}

/// Tool root from the running executable / `TT_SCRIPT_ROOT` / CWD.
///
/// Reads process state; call only from refresh closures, never from tests.
pub fn tool_root_dir() -> PathBuf {
    gsi_config::tool_root(&gsi_config::script_root(None))
}

/// Pure context construction over explicit inputs (testable).
///
/// `override_data` is the `TT_DATA_DIR` value when set and non-empty.
/// Without it the layout mirrors the scripts: `<tool_root>/data` and
/// `<tool_root>/logs`.
pub fn live_ctx_from(tool_root: &Path, override_data: Option<&Path>) -> LiveCtx {
    match override_data {
        Some(d) if !d.as_os_str().is_empty() => LiveCtx {
            tool_root: tool_root.to_path_buf(),
            data_dir: d.to_path_buf(),
            log_dir: d.join("logs"),
        },
        _ => LiveCtx {
            tool_root: tool_root.to_path_buf(),
            data_dir: tool_root.join("data"),
            log_dir: tool_root.join("logs"),
        },
    }
}

/// Resolve one refresh context, honoring `TT_DATA_DIR` when set.
///
/// Never panics; missing values fall back to the tool-root layout.
pub fn live_ctx() -> LiveCtx {
    let root = tool_root_dir();
    let over = std::env::var_os("TT_DATA_DIR")
        .map(PathBuf::from)
        .filter(|p| !p.as_os_str().is_empty());
    live_ctx_from(&root, over.as_deref())
}

// ------------------------------------------------------------ pure mappers

/// Display label for an installed-ROM id (`stock`, `rom:<label>`, other).
///
/// Empty input yields an empty label (callers render the unknown form).
pub fn rom_label_live(installed_id: &str) -> String {
    let t = installed_id.trim();
    if t.is_empty() {
        return String::new();
    }
    if t == "stock" {
        return "Stock EMUI".to_string();
    }
    if let Some(label) = t.strip_prefix("rom:") {
        let l = label.trim();
        if l.is_empty() {
            return t.to_string();
        }
        return l.to_string();
    }
    t.to_string()
}

/// Known P10 model tokens, most specific first.
const MODEL_TOKENS: &[&str] = &[
    "VTR-L29", "VTR-L09", "VKY-L29", "VKY-L09", "VTR-AL00", "VTR-TL00", "VKY-AL00", "VKY-TL00",
];

/// Derive the registry model from the installed ROM id/label.
///
/// Returns the model plus whether it is a default guess (`true` when no
/// token was found and `VTR-L29` was assumed, mirroring the scripts).
pub fn model_from_installed(installed_id: &str, label: &str) -> (String, bool) {
    let hay = format!("{} {}", installed_id, label).to_ascii_uppercase();
    for tok in MODEL_TOKENS {
        if hay.contains(tok) {
            return (tok.to_string(), false);
        }
    }
    ("VTR-L29".to_string(), true)
}

/// Extract the `Cxxx` region from a firmware baseline (`(C432` -> `C432`).
///
/// Mirrors the `(C\d+` grep in both script sides. Empty when absent.
pub fn firmware_region(baseline: &str) -> String {
    let b = baseline.as_bytes();
    let mut i = 0;
    while i + 1 < b.len() {
        if b[i] == b'(' && (b[i + 1] == b'C' || b[i + 1] == b'c') {
            let mut j = i + 2;
            while j < b.len() && b[j].is_ascii_digit() {
                j += 1;
            }
            if j > i + 2 {
                return format!("C{}", &baseline[i + 2..j]);
            }
        }
        i += 1;
    }
    String::new()
}

// ---------------------------------------------------------- fs-backed live

/// Saved installed ROM plus where it came from.
#[derive(Debug, Clone)]
pub struct RomLive {
    /// Raw id (`stock`, `rom:<label>`, ...; empty when unknown).
    pub id: String,
    /// Display label (empty when unknown).
    pub label: String,
    /// Source file shown to the user, or the absence note.
    pub source: String,
}

/// Read `<data_dir>/installed-rom.txt` via `gsi-state`.
///
/// Missing, empty, or corrupt files yield the unknown form plus a note;
/// corrupt content is reported, never hidden.
pub fn installed_rom_live(data_dir: &Path) -> RomLive {
    let path = data_dir.join("installed-rom.txt");
    match gsi_state::read_installed_rom(&path) {
        Ok(Some(id)) => RomLive {
            label: rom_label_live(&id),
            source: format!("saved: {}", path.display()),
            id,
        },
        Ok(None) => RomLive {
            id: String::new(),
            label: String::new(),
            source: format!("no saved system ({} missing or empty)", path.display()),
        },
        Err(e) => RomLive {
            id: String::new(),
            label: String::new(),
            source: format!("saved system unreadable ({}: {e})", path.display()),
        },
    }
}

/// Live registry data for one profile.
#[derive(Debug, Clone)]
pub struct RegistryLive {
    /// Profile id actually used.
    pub profile: String,
    /// Selectable ROM labels.
    pub recommended: Vec<String>,
    /// `name -- reason` lines for researched-broken entries.
    pub broken: Vec<String>,
    /// Advisory `required_base` firmware string (may be empty).
    pub firmware_base: String,
    /// Android targets with working-image counts, ascending.
    pub android_targets: Vec<(i64, usize)>,
    /// Where the data came from, or why defaults render.
    pub note: String,
}

/// Load the compat profile under `<data_dir>/compatibility/huawei/p10/`.
///
/// `profile` empty falls back to `VTR-L29`. Any failure yields empty lists
/// plus an honest note; nothing is guessed.
pub fn registry_live(data_dir: &Path, profile: &str) -> RegistryLive {
    let prof = if profile.trim().is_empty() {
        "VTR-L29"
    } else {
        profile.trim()
    };
    let empty = |note: String| RegistryLive {
        profile: prof.to_string(),
        recommended: Vec::new(),
        broken: Vec::new(),
        firmware_base: String::new(),
        android_targets: Vec::new(),
        note,
    };
    let path = match gsi_config::resolve_compat_profile(data_dir, prof) {
        Some(p) => p,
        None => {
            return empty(format!(
                "no registry profile for '{prof}' under {}",
                data_dir.join("compatibility").display()
            ));
        }
    };
    let entries = match gsi_registry::load_roms(&path) {
        Ok(e) => e,
        Err(e) => return empty(format!("registry '{}' unreadable: {e}", path.display())),
    };
    let recommended: Vec<String> = gsi_registry::rom_options(&entries)
        .into_iter()
        .map(|(_, label)| label)
        .collect();
    let broken: Vec<String> = gsi_registry::rom_broken(&entries)
        .into_iter()
        .map(|(name, reason)| {
            if reason.trim().is_empty() {
                name
            } else {
                format!("{name} -- {}", reason.trim())
            }
        })
        .collect();
    RegistryLive {
        profile: prof.to_string(),
        recommended,
        broken,
        firmware_base: gsi_registry::firmware_base(&path),
        android_targets: gsi_registry::target_androids(&entries),
        note: format!("registry: {}", path.display()),
    }
}

/// Live workflow-state data.
#[derive(Debug, Clone)]
pub struct WorkflowLive {
    /// Saved goal id (empty when none).
    pub goal: String,
    /// `(id, status)` steps in file order.
    pub steps: Vec<(String, String)>,
    /// Shared-slot occupant (`unknown` when unrecorded).
    pub slot: String,
    /// Persisted boot mode (empty when unrecorded).
    pub boot_mode: String,
    /// Source file or absence note.
    pub note: String,
}

/// Read `<log_dir>/workflow-state.json` via `gsi-state`.
///
/// Missing or corrupt files yield the empty form plus a note.
pub fn workflow_live(log_dir: &Path) -> WorkflowLive {
    let path = gsi_state::workflow_state_file(log_dir);
    let empty = |note: String| WorkflowLive {
        goal: String::new(),
        steps: Vec::new(),
        slot: "unknown".to_string(),
        boot_mode: String::new(),
        note,
    };
    let wf = match gsi_state::read_workflow_state(&path) {
        Ok(Some(w)) => w,
        Ok(None) => {
            return empty(format!("no saved workflow ({} missing)", path.display()));
        }
        Err(e) => {
            return empty(format!(
                "workflow state unreadable ({}: {e})",
                path.display()
            ))
        }
    };
    let steps: Vec<(String, String)> = wf
        .steps
        .iter()
        .map(|s| {
            let st = if s.status.trim().is_empty() {
                "pending".to_string()
            } else {
                s.status.clone()
            };
            (s.id.clone(), st)
        })
        .collect();
    let boot_mode = wf
        .last_root
        .as_ref()
        .and_then(|r| r.boot_mode.clone())
        .unwrap_or_default();
    let slot = wf
        .slot
        .as_ref()
        .map(|s| {
            if s.occupant.trim().is_empty() {
                "unknown".to_string()
            } else {
                s.occupant.clone()
            }
        })
        .unwrap_or_else(|| "unknown".to_string());
    WorkflowLive {
        goal: wf.goal,
        steps,
        slot,
        boot_mode,
        note: format!("state: {}", path.display()),
    }
}

/// Status for one step id from saved steps, `pending` when unrecorded.
pub fn saved_step_status(steps: &[(String, String)], id: &str) -> String {
    for (sid, st) in steps {
        if sid == id {
            return st.clone();
        }
    }
    "pending".to_string()
}

/// True when any `original.img` exists anywhere under `backups_dir`.
///
/// Bounded local scan (depth cap 4, symlinks skipped); missing dirs read
/// as false.
pub fn has_backup_with_original(backups_dir: &Path) -> bool {
    newest_backup_with_original(backups_dir).is_some()
}

/// Newest directory directly holding `original.img` (by dir mtime).
///
/// Scans at most 4 levels under `backups_dir`, skips symlinks, returns
/// `None` when no backup qualifies.
pub fn newest_backup_with_original(backups_dir: &Path) -> Option<PathBuf> {
    fn walk(dir: &Path, depth: u32, out: &mut Vec<PathBuf>) {
        if depth > 4 {
            return;
        }
        let entries = match std::fs::read_dir(dir) {
            Ok(e) => e,
            Err(_) => return,
        };
        for entry in entries {
            let entry = match entry {
                Ok(e) => e,
                Err(_) => continue,
            };
            let ftype = match entry.file_type() {
                Ok(t) => t,
                Err(_) => continue,
            };
            if ftype.is_symlink() {
                continue;
            }
            if ftype.is_dir() {
                let p = entry.path();
                if p.join("original.img").is_file() && !out.contains(&p) {
                    out.push(p.clone());
                }
                walk(&p, depth + 1, out);
            }
        }
    }
    fn mtime(p: &Path) -> std::time::SystemTime {
        std::fs::metadata(p)
            .and_then(|m| m.modified())
            .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
    }
    let mut cands = Vec::new();
    walk(backups_dir, 0, &mut cands);
    cands.sort_by(|a, b| mtime(b).cmp(&mtime(a)).then_with(|| a.cmp(b)));
    cands.into_iter().next()
}

/// Newest Magisk-patched image under `data_dir` (recursive, may be `None`).
pub fn newest_magisk_patched(data_dir: &Path) -> Option<PathBuf> {
    gsi_config::find_magisk_patched(&[data_dir.to_path_buf()])
}

/// First stock recovery image found (exact names first, then fallback).
pub fn first_stock_recovery(data_dir: &Path, firmware_dir: &Path) -> Option<PathBuf> {
    let names = gsi_config::stock_recovery_names();
    let dirs = vec![data_dir.to_path_buf(), firmware_dir.to_path_buf()];
    gsi_config::find_recovery_images(&dirs, &names)
        .into_iter()
        .next()
}

/// Local system image: exact registry name first, else newest `*-arm64_*`.
pub fn local_system_image(data_dir: &Path, roms_dir: &Path) -> Option<PathBuf> {
    gsi_config::find_system_image(&[roms_dir.to_path_buf(), data_dir.to_path_buf()], None)
}

/// Newest `*.APP` firmware container (sorted first hit, may be `None`).
pub fn first_update_app(firmware_dir: &Path) -> Option<PathBuf> {
    gsi_config::find_update_apps(&[firmware_dir.to_path_buf()])
        .into_iter()
        .next()
}

/// Newest file in `dir` whose name satisfies `pred` (non-recursive top
/// level plus one nested level via `gsi-config`; symlinks skipped).
pub fn newest_file_matching(dir: &Path, pred: impl Fn(&str) -> bool) -> Option<PathBuf> {
    gsi_config::newest_matching_file(dir, pred)
}

/// First line token of the `<image>.sha256` sidecar (empty when unreadable).
pub fn sidecar_sha_text(image: &Path) -> String {
    let mut p = image.as_os_str().to_owned();
    p.push(".sha256");
    let sidecar = PathBuf::from(p);
    match std::fs::read_to_string(&sidecar) {
        Ok(t) => t.split_whitespace().next().unwrap_or("").to_string(),
        Err(_) => String::new(),
    }
}

/// First 4 bytes of a file as upper hex (`3A FF 26 ED`; empty on error).
pub fn header_hex(path: &Path) -> String {
    use std::io::Read;
    let mut f = match std::fs::File::open(path) {
        Ok(f) => f,
        Err(_) => return String::new(),
    };
    let mut buf = [0u8; 4];
    let mut n = 0;
    while n < buf.len() {
        match f.read(&mut buf[n..]) {
            Ok(0) => break,
            Ok(r) => n += r,
            Err(_) => return String::new(),
        }
    }
    if n == 0 {
        return String::new();
    }
    buf[..n]
        .iter()
        .map(|b| format!("{b:02X}"))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Newest TWRP image (`*twrp*.img`, case-insensitive) under `data_dir`.
pub fn newest_twrp_image(data_dir: &Path) -> Option<PathBuf> {
    newest_file_matching(data_dir, |n| {
        let l = n.to_ascii_lowercase();
        l.ends_with(".img") && l.contains("twrp")
    })
}

/// Newest ROM package in `roms_dir` (zip/tar/img/app containers).
pub fn newest_rom_package(roms_dir: &Path) -> Option<PathBuf> {
    newest_file_matching(roms_dir, |n| {
        let l = n.to_ascii_lowercase();
        l.ends_with(".zip")
            || l.ends_with(".tar.gz")
            || l.ends_with(".tar.xz")
            || l.ends_with(".tgz")
            || l.ends_with(".tar")
            || l.ends_with(".img")
            || l.ends_with(".img.gz")
            || l.ends_with(".img.xz")
            || l.ends_with(".app")
    })
}

/// Newest Magisk APK in `<data_dir>/magisk` (`*.apk`).
pub fn newest_magisk_apk(data_dir: &Path) -> Option<PathBuf> {
    newest_file_matching(&data_dir.join("magisk"), |n| {
        n.to_ascii_lowercase().ends_with(".apk")
    })
}

/// Dirs that are missing or not writable (for the preflight render).
pub fn unwritable_dirs(dirs: &[PathBuf]) -> Vec<String> {
    let mut out = Vec::new();
    for d in dirs {
        let ok = match std::fs::metadata(d) {
            Ok(m) => m.is_dir() && !m.permissions().readonly(),
            Err(_) => false,
        };
        if !ok {
            out.push(format!("{} (missing or not writable)", d.display()));
        }
    }
    out
}

/// Locate `adb`/`fastboot` without probing or running anything.
///
/// Returns the display path or an empty string; PATH plus the known
/// trebleManager tool dirs are searched, mirroring the scripts.
pub fn tool_path_text(name: &str) -> String {
    let mut extra = Vec::new();
    for cand in [
        "tools/platform-tools",
        "data/tools",
        "platform-tools",
        "tools",
    ] {
        let p = PathBuf::from(cand);
        if p.is_dir() {
            extra.push(p);
        }
    }
    match gsi_tool::locate(name, &extra) {
        Some(t) => t.path.display().to_string(),
        None => String::new(),
    }
}

// ----------------------------------------------------------------- renders

/// One-line provenance footer shared by the live page renders.
pub fn ctx_note(ctx: &LiveCtx) -> String {
    format!(
        "live: data_dir={} log_dir={}",
        ctx.data_dir.display(),
        ctx.log_dir.display()
    )
}

/// Render app directories (tool root, data, logs, config, cache, firmware,
///
/// Pure over explicit inputs: `config_dir`/`cache_dir` come from
/// `gsi-config` in the closure and are `None` when unresolvable.
pub fn dirs_text(ctx: &LiveCtx, config_dir: Option<&Path>, cache_dir: Option<&Path>) -> String {
    let mut out = String::new();
    out.push_str(&format!("tool root: {}\n", ctx.tool_root.display()));
    out.push_str(&format!("data: {}\n", ctx.data_dir.display()));
    out.push_str(&format!("logs: {}\n", ctx.log_dir.display()));
    match config_dir {
        Some(p) => out.push_str(&format!("config: {}\n", p.display())),
        None => out.push_str("config: unknown\n"),
    }
    match cache_dir {
        Some(p) => out.push_str(&format!("cache: {}\n", p.display())),
        None => out.push_str("cache: unknown\n"),
    }
    out.push_str(&format!(
        "firmware: {}\n",
        ctx.data_dir.join("firmware").display()
    ));
    out.push_str(&format!("roms: {}\n", ctx.data_dir.join("roms").display()));
    out.push_str(&format!(
        "backups: {}\n",
        ctx.data_dir.join("backups").display()
    ));
    out
}

/// Render log files (names plus sizes only, never content).
///
/// Pure over an explicit dir via `gsi-diag::list_log_files`; a missing dir
/// renders the honest empty form.
pub fn log_files_text(log_dir: &Path) -> String {
    let mut out = String::new();
    out.push_str(&format!("log files ({}):\n", log_dir.display()));
    let files = gsi_diag::list_log_files(log_dir);
    if files.is_empty() {
        out.push_str("no log files\n");
    } else {
        for f in &files {
            out.push_str(&format!(" - {} ({} bytes)\n", f.name, f.size));
        }
    }
    out.push_str(
        "names + sizes only; open or bundle them via the scripts (diagnostic ZIP in the TUI)\n",
    );
    out
}

// ------------------------------------------------------- per-page refreshes

/// Analyze page over injected values (Detect pattern).
///
/// Transcripts stay empty here: capturing them needs live adb queries,
/// which refresh closures never run. The saved system label is live;
/// anything else renders the honest `no device` form via
/// `pages_analyze::analyze_device_text`.
pub fn analyze_device_refresh(ctx: &LiveCtx) -> String {
    let rom = installed_rom_live(&ctx.data_dir);
    let label = if rom.id.is_empty() {
        ""
    } else {
        rom.label.as_str()
    };
    let mut out = pages_analyze::analyze_device_text("", "", "", label);
    out.push_str(&format!("{}\n", rom.source));
    out.push_str(&format!("{}\n", ctx_note(ctx)));
    out
}

/// Logs file list over the resolved log dir.
pub fn logs_files_refresh(ctx: &LiveCtx) -> String {
    let mut out = log_files_text(&ctx.log_dir);
    out.push_str(&format!("{}\n", ctx_note(ctx)));
    out
}

/// Goals page: saved goal plus saved step states over the goal table.
pub fn goals_refresh(ctx: &LiveCtx) -> String {
    let wf = workflow_live(&ctx.log_dir);
    let steps: Vec<String> = gsi_state::goal_steps(&wf.goal);
    let statuses: Vec<String> = steps
        .iter()
        .map(|s| saved_step_status(&wf.steps, s))
        .collect();
    let mut out = pages_flows::goals_text(&wf.goal, &steps, &statuses);
    if wf.goal.trim().is_empty() {
        let known = gsi_state::goal_ids().join(", ");
        out.push_str(&format!("known goals: {known}\n"));
    }
    out.push_str(&format!("{}\n", wf.note));
    out.push_str(&format!("{}\n", ctx_note(ctx)));
    out
}

/// Status page: live profile/system/state values, unknowns stay unknown.
pub fn status_refresh(ctx: &LiveCtx) -> String {
    let rom = installed_rom_live(&ctx.data_dir);
    let (model, _) = model_from_installed(&rom.id, &rom.label);
    let reg = registry_live(&ctx.data_dir, &model);
    let wf = workflow_live(&ctx.log_dir);
    let installed_label = if rom.id.is_empty() {
        ""
    } else {
        rom.label.as_str()
    };
    let stock = first_stock_recovery(&ctx.data_dir, &ctx.data_dir.join("firmware"))
        .map(|p| p.display().to_string())
        .unwrap_or_default();
    let patched = newest_magisk_patched(&ctx.data_dir)
        .map(|p| p.display().to_string())
        .unwrap_or_default();
    let backup = newest_backup_with_original(&ctx.data_dir.join("backups"))
        .map(|p| p.display().to_string())
        .unwrap_or_default();
    let firmware = if reg.firmware_base.trim().is_empty() {
        ""
    } else {
        reg.firmware_base.as_str()
    };
    let mut out = pages_flows::status_text(
        &model,
        "",
        "",
        "",
        "none",
        firmware,
        installed_label,
        &stock,
        &patched,
        &backup,
        &wf.slot,
        "",
    );
    out.push_str("os note: OS class needs live analysis (Analyze Device page)\n");
    out.push_str(&format!("{}\n", reg.note));
    out.push_str(&format!("{}\n", wf.note));
    out.push_str(&format!("{}\n", rom.source));
    out.push_str(&format!("{}\n", ctx_note(ctx)));
    out
}

/// Wizard page: saved system plus saved goal over the goal table.
pub fn wizard_refresh(ctx: &LiveCtx) -> String {
    let rom = installed_rom_live(&ctx.data_dir);
    let wf = workflow_live(&ctx.log_dir);
    let installed_label = if rom.id.is_empty() {
        ""
    } else {
        rom.label.as_str()
    };
    let steps: Vec<String> = gsi_state::goal_steps(&wf.goal);
    let patch_note = if rom.id.is_empty() {
        String::new()
    } else {
        format!(
            "Your system decides the patch base (see Patch page): {}.",
            rom.label
        )
    };
    let mut out = pages_flows::wizard_text(installed_label, &wf.goal, &steps, &[], &patch_note);
    out.push_str(&format!("{}\n", wf.note));
    out.push_str(&format!("{}\n", rom.source));
    out.push_str(&format!("{}\n", ctx_note(ctx)));
    out
}

/// Flash page: measurable readiness live, device gates honestly pending.
///
/// Measured locally: patched image present, `.sha256` sidecar, size
/// policy, backup with `original.img`. Model token, profile verification,
/// partition presence, firmware verdict, fastboot state, and the double
/// confirmation need a live device and render as pending.
pub fn flash_refresh(ctx: &LiveCtx) -> String {
    let rom = installed_rom_live(&ctx.data_dir);
    let (model, model_defaulted) = model_from_installed(&rom.id, &rom.label);
    let wf = workflow_live(&ctx.log_dir);
    let patched = newest_magisk_patched(&ctx.data_dir);
    let image_exists = patched.is_some();
    let (sha, size_ok) = match &patched {
        Some(p) => {
            let chk = gsi_config::test_recovery_image(p);
            (sidecar_sha_text(p), chk.size_ok)
        }
        None => (String::new(), false),
    };
    let hash_known = !sha.trim().is_empty();
    let backups = ctx.data_dir.join("backups");
    let backup = newest_backup_with_original(&backups);
    let backup_ok = has_backup_with_original(&backups);
    let stock = first_stock_recovery(&ctx.data_dir, &ctx.data_dir.join("firmware"));
    let differs = match (&patched, &stock) {
        (Some(p), Some(s)) => match (std::fs::metadata(p), std::fs::metadata(s)) {
            (Ok(mp), Ok(ms)) => mp.len() != ms.len(),
            _ => false,
        },
        _ => false,
    };
    let image = patched
        .as_ref()
        .map(|p| p.display().to_string())
        .unwrap_or_default();
    let backup_dir = backup
        .as_ref()
        .map(|p| p.display().to_string())
        .unwrap_or_default();
    let failed: Vec<String> = Vec::new();
    let mut out = pages_flash::flash_text(
        "recovery_ramdisk",
        &image,
        &sha,
        &backup_dir,
        &wf.slot,
        !rom.id.is_empty() && !model_defaulted,
        false,
        false,
        image_exists,
        hash_known,
        size_ok,
        false,
        backup_ok,
        false,
        differs,
        false,
        false,
        "UNCLEAR",
        0,
        "",
        &failed,
    );
    out.push_str(&format!("model: {model}\n"));
    out.push_str("device gates pending: profile verification, partition presence, firmware verdict, fastboot state, FLASH + YES (live device work, Phase 8)\n");
    if differs {
        out.push_str("differs note: patched and stock differ by size; the hash comparison runs in script verify\n");
    }
    out.push_str(&format!("{}\n", wf.note));
    out.push_str(&format!("{}\n", rom.source));
    out.push_str(&format!("{}\n", ctx_note(ctx)));
    out
}

/// Verify page: no live adb here, so transcripts stay empty.
///
/// Renders the classifier over empty input (honest `NOT_ROOTED` form)
/// plus the saved slot for context.
pub fn verify_refresh(ctx: &LiveCtx) -> String {
    let wf = workflow_live(&ctx.log_dir);
    let checks: Vec<String> = Vec::new();
    let mut out = pages_flash::verify_text("", "", "", &checks);
    out.push_str(&format!("saved slot: {}\n", wf.slot));
    out.push_str("device note: live adb verify (which su / su -c id / magisk -v) needs a device; run the Verify screen in the scripts\n");
    out.push_str(&format!("{}\n", wf.note));
    out.push_str(&format!("{}\n", ctx_note(ctx)));
    out
}

/// Unlock page: static guidance plus live context.
pub fn unlock_refresh(ctx: &LiveCtx) -> String {
    let rom = installed_rom_live(&ctx.data_dir);
    let mut out = pages_flash::unlock_text();
    out.push_str(&format!("{}\n", rom.source));
    out.push_str(&format!("{}\n", ctx_note(ctx)));
    out
}

/// Preflight page: live tool presence plus writable-dir probes.
///
/// No version probes and no device queries run here (those would spawn
/// processes); the scripts own the full gate.
pub fn preflight_refresh(ctx: &LiveCtx) -> String {
    let adb = tool_path_text("adb");
    let fastboot = tool_path_text("fastboot");
    let dirs = vec![
        ctx.data_dir.clone(),
        ctx.log_dir.clone(),
        ctx.data_dir.join("backups"),
    ];
    let unwritable = unwritable_dirs(&dirs);
    let missing: Vec<String> = Vec::new();
    let hashes: Vec<String> = Vec::new();
    let mut out = pages_flash::preflight_blocked_text(
        &adb,
        &fastboot,
        true,
        &unwritable,
        &missing,
        &hashes,
        "unknown (no device query in GUI)",
        "unknown (no device query in GUI)",
    );
    out.push_str("shell note: shell capability is evaluated by the scripts; the GUI assumes a capable host\n");
    let config_dir = gsi_config::config_dir();
    let cache_dir = gsi_config::cache_dir();
    out.push_str(&dirs_text(ctx, config_dir.as_deref(), cache_dir.as_deref()));
    out.push_str(&format!("{}\n", ctx_note(ctx)));
    out
}

/// Restore page: newest backup with `original.img`, when one exists.
pub fn restore_refresh(ctx: &LiveCtx) -> String {
    let backup = newest_backup_with_original(&ctx.data_dir.join("backups"));
    let dir = backup
        .as_ref()
        .map(|p| p.display().to_string())
        .unwrap_or_default();
    let mut out = pages_backup::restore_text(&dir, "recovery_ramdisk", backup.is_some());
    out.push_str(&format!("{}\n", ctx_note(ctx)));
    out
}

/// Bootkeys page: persisted boot mode when one was saved.
pub fn bootkeys_refresh(ctx: &LiveCtx) -> String {
    let wf = workflow_live(&ctx.log_dir);
    let mut out = pages_backup::bootkeys_text(&wf.boot_mode);
    out.push_str(&format!("{}\n", wf.note));
    out.push_str(&format!("{}\n", ctx_note(ctx)));
    out
}

/// Kernel page: wiki defaults plus live slot context.
pub fn kernel_refresh(ctx: &LiveCtx) -> String {
    let wf = workflow_live(&ctx.log_dir);
    let kernels: Vec<&str> = Vec::new();
    let fixes: Vec<&str> = Vec::new();
    let mut out = pages_media::kernel_text(&kernels, &fixes);
    out.push_str(&format!("saved slot: {}\n", wf.slot));
    out.push_str(&format!("{}\n", ctx_note(ctx)));
    out
}

/// Root-methods page: saved selection; the table lives script-side.
pub fn rootmethods_refresh(ctx: &LiveCtx) -> String {
    let rom = installed_rom_live(&ctx.data_dir);
    let methods: Vec<(String, String, bool)> = Vec::new();
    let mut out = pages_flows::rootmethods_text(&methods, &rom.id);
    out.push_str("methods note: the ordered method table lives in the scripts; the GUI only shows the saved selection\n");
    out.push_str(&format!("{}\n", rom.source));
    out.push_str(&format!("{}\n", ctx_note(ctx)));
    out
}

/// ROM-select page: live registry options plus saved/detected markers.
pub fn romselect_refresh(ctx: &LiveCtx) -> String {
    let rom = installed_rom_live(&ctx.data_dir);
    let (model, _) = model_from_installed(&rom.id, &rom.label);
    let reg = registry_live(&ctx.data_dir, &model);
    let mut options = vec!["Stock EMUI".to_string()];
    for r in &reg.recommended {
        if !options.contains(r) {
            options.push(r.clone());
        }
    }
    let broken: Vec<String> = reg
        .broken
        .iter()
        .map(|b| b.split(" -- ").next().unwrap_or(b).trim().to_string())
        .collect();
    let current = if rom.id.trim() == "stock" {
        "Stock EMUI"
    } else if options.iter().any(|o| o == &rom.label) {
        rom.label.as_str()
    } else {
        ""
    };
    let mut out = pages_flows::romselect_text(&options, current, "", &broken);
    out.push_str(&format!("{}\n", reg.note));
    out.push_str(&format!("{}\n", rom.source));
    out.push_str(&format!("{}\n", ctx_note(ctx)));
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static N: AtomicUsize = AtomicUsize::new(0);

    fn tmp_root(tag: &str) -> PathBuf {
        let n = N.fetch_add(1, Ordering::SeqCst);
        let mut p = std::env::temp_dir();
        p.push(format!("gsi-gui-live-{tag}-{}-{n}", std::process::id()));
        let _ = std::fs::create_dir_all(&p);
        p
    }

    fn tool_tree(tag: &str) -> (PathBuf, LiveCtx) {
        let root = tmp_root(tag);
        let data = root.join("data");
        let _ = std::fs::create_dir_all(data.join("compatibility").join("huawei").join("p10"));
        let _ = std::fs::create_dir_all(data.join("firmware"));
        let _ = std::fs::create_dir_all(data.join("roms"));
        let _ = std::fs::create_dir_all(data.join("magisk"));
        let _ = std::fs::create_dir_all(data.join("backups"));
        let _ = std::fs::create_dir_all(root.join("logs"));
        let ctx = live_ctx_from(&root, Some(&data));
        (root, ctx)
    }

    fn write(path: &Path, bytes: &[u8]) {
        if let Some(p) = path.parent() {
            let _ = std::fs::create_dir_all(p);
        }
        let _ = std::fs::write(path, bytes);
    }

    const PROFILE_JSON: &str = r#"{
        "firmware": {"required_base": "VTR-L29 9.1.0.297(C432E5R1P9)"},
        "roms": [
            {"name": "LineageOS", "version": 20, "android": 13, "status": "working",
             "gsi": "arm64_bgN", "file": "l20.img.gz", "url": "https://x/l20.img.gz"},
            {"name": "LineageOS 20 Light", "version": 20, "status": "broken",
             "reason": "non-booting"}
        ]
    }"#;

    fn seed_registry(ctx: &LiveCtx) {
        write(
            &ctx.data_dir
                .join("compatibility")
                .join("huawei")
                .join("p10")
                .join("VTR-L29.json"),
            PROFILE_JSON.as_bytes(),
        );
    }

    #[test]
    fn ctx_from_override_and_tool_root() {
        let root = PathBuf::from("/tool");
        let over = PathBuf::from("/override");
        let c = live_ctx_from(&root, Some(&over));
        assert_eq!(c.data_dir, over);
        assert_eq!(c.log_dir, over.join("logs"));
        let c2 = live_ctx_from(&root, None);
        assert_eq!(c2.data_dir, PathBuf::from("/tool/data"));
        assert_eq!(c2.log_dir, PathBuf::from("/tool/logs"));
        let empty = PathBuf::from("");
        let c3 = live_ctx_from(&root, Some(&empty));
        assert_eq!(c3.data_dir, PathBuf::from("/tool/data"));
    }

    #[test]
    fn rom_label_mapping() {
        assert_eq!(rom_label_live("stock"), "Stock EMUI");
        assert_eq!(rom_label_live("rom:LineageOS 20"), "LineageOS 20");
        assert_eq!(rom_label_live("custom"), "custom");
        assert_eq!(rom_label_live("  "), "");
    }

    #[test]
    fn model_tokens_and_default() {
        assert_eq!(
            model_from_installed("rom:VTR-L09 GSI", "x"),
            ("VTR-L09".to_string(), false)
        );
        assert_eq!(
            model_from_installed("stock", "Stock EMUI"),
            ("VTR-L29".to_string(), true)
        );
        assert_eq!(model_from_installed("", ""), ("VTR-L29".to_string(), true));
    }

    #[test]
    fn region_parse() {
        assert_eq!(firmware_region("VTR-L29 9.1.0.297(C432E5R1P9)"), "C432");
        assert_eq!(firmware_region("no region here"), "");
        assert_eq!(firmware_region(""), "");
    }

    #[test]
    fn installed_rom_saved_missing_and_corrupt() {
        let (_root, ctx) = tool_tree("rom");
        let missing = installed_rom_live(&ctx.data_dir);
        assert!(missing.id.is_empty());
        assert!(missing.source.contains("no saved system"));
        write(&ctx.data_dir.join("installed-rom.txt"), b"stock");
        let saved = installed_rom_live(&ctx.data_dir);
        assert_eq!(saved.id, "stock");
        assert_eq!(saved.label, "Stock EMUI");
        write(&ctx.data_dir.join("installed-rom.txt"), b"two tokens here");
        let bad = installed_rom_live(&ctx.data_dir);
        assert!(bad.id.is_empty());
        assert!(bad.source.contains("unreadable"));
    }

    #[test]
    fn registry_load_and_missing() {
        let (_root, ctx) = tool_tree("reg");
        seed_registry(&ctx);
        let reg = registry_live(&ctx.data_dir, "VTR-L29");
        assert_eq!(reg.profile, "VTR-L29");
        assert_eq!(reg.recommended, vec!["LineageOS 20".to_string()]);
        assert!(reg.broken.iter().any(|b| b.contains("LineageOS 20 Light")));
        assert!(reg.firmware_base.contains("C432"));
        assert_eq!(reg.android_targets, vec![(13, 1)]);
        let miss = registry_live(&ctx.data_dir, "NOPE");
        assert!(miss.recommended.is_empty());
        assert!(miss.note.contains("no registry profile"));
        let empty_prof = registry_live(&ctx.data_dir, "");
        assert_eq!(empty_prof.profile, "VTR-L29");
    }

    #[test]
    fn workflow_saved_missing_and_parsed() {
        let (_root, ctx) = tool_tree("wf");
        let missing = workflow_live(&ctx.log_dir);
        assert!(missing.goal.is_empty());
        assert_eq!(missing.slot, "unknown");
        write(
            &ctx.log_dir.join("workflow-state.json"),
            br#"{"goal":"root","steps":[{"id":"reconnaissance","status":"done"}],"slot":{"occupant":"magisk","detail":"","timestamp":""},"last_root":{"state":"ROOTED","timestamp":"","boot_mode":"cheat"},"updated":""}"#,
        );
        let wf = workflow_live(&ctx.log_dir);
        assert_eq!(wf.goal, "root");
        assert_eq!(
            wf.steps,
            vec![("reconnaissance".to_string(), "done".to_string())]
        );
        assert_eq!(wf.slot, "magisk");
        assert_eq!(wf.boot_mode, "cheat");
        assert_eq!(saved_step_status(&wf.steps, "reconnaissance"), "done");
        assert_eq!(saved_step_status(&wf.steps, "other"), "pending");
        write(&ctx.log_dir.join("workflow-state.json"), b"{oops");
        let bad = workflow_live(&ctx.log_dir);
        assert!(bad.goal.is_empty());
        assert!(bad.note.contains("unreadable"));
    }

    #[test]
    fn backups_scan() {
        let (_root, ctx) = tool_tree("bak");
        let backups = ctx.data_dir.join("backups");
        assert!(!has_backup_with_original(&backups));
        assert!(newest_backup_with_original(&backups).is_none());
        let good = backups.join("VTR-L29").join("recovery_ramdisk").join("s1");
        write(&good.join("original.img"), b"img");
        assert!(has_backup_with_original(&backups));
        assert_eq!(newest_backup_with_original(&backups), Some(good));
        let _ = std::fs::remove_dir_all(ctx.log_dir.parent().unwrap_or(&backups));
    }

    #[test]
    fn sidecar_and_header_helpers() {
        let (_root, ctx) = tool_tree("hex");
        let img = ctx.data_dir.join("m.img");
        write(&img, &[0x3A, 0xFF, 0x26, 0xED, 0x00]);
        assert_eq!(header_hex(&img), "3A FF 26 ED");
        assert_eq!(sidecar_sha_text(&img), "");
        write(&ctx.data_dir.join("m.img.sha256"), b"ab12  m.img\n");
        assert_eq!(sidecar_sha_text(&img), "ab12");
        assert_eq!(header_hex(&ctx.data_dir.join("nope.img")), "");
    }

    #[test]
    fn log_files_render_names_and_sizes() {
        let (_root, ctx) = tool_tree("logs");
        write(&ctx.log_dir.join("b.log"), b"12345");
        write(&ctx.log_dir.join("a.log"), b"12");
        let t = log_files_text(&ctx.log_dir);
        assert!(t.contains("a.log (2 bytes)"));
        assert!(t.contains("b.log (5 bytes)"));
        assert!(t.contains("names + sizes only"));
        assert!(t.find("a.log") < t.find("b.log"));
        let missing = ctx.data_dir.join("no-logs-here");
        let e = log_files_text(&missing);
        assert!(e.contains("no log files"));
    }

    #[test]
    fn dirs_render() {
        let (_root, ctx) = tool_tree("dirs");
        let cfg = PathBuf::from("/cfg");
        let t = dirs_text(&ctx, Some(&cfg), None);
        assert!(t.contains("tool root:"));
        assert!(t.contains("firmware"));
        assert!(t.contains("/cfg"));
        assert!(t.contains("cache: unknown"));
    }

    #[test]
    fn unwritable_lists_missing() {
        let (_root, ctx) = tool_tree("wro");
        let out = unwritable_dirs(&[ctx.data_dir.join("nope")]);
        assert_eq!(out.len(), 1);
        assert!(out[0].contains("missing or not writable"));
        let ok = unwritable_dirs(std::slice::from_ref(&ctx.data_dir));
        assert!(ok.is_empty());
    }

    #[test]
    fn analyze_refresh_live_label_and_honest_absence() {
        let (_root, ctx) = tool_tree("ana");
        seed_registry(&ctx);
        let t = analyze_device_refresh(&ctx);
        assert!(t.contains("no device connected."));
        assert!(t.contains("Phone runs: unknown"));
        assert!(t.contains("no saved system"));
        write(&ctx.data_dir.join("installed-rom.txt"), b"stock");
        let t2 = analyze_device_refresh(&ctx);
        assert!(t2.contains("Phone runs: Stock EMUI"));
        assert!(t2.contains("OS class:"));
    }

    #[test]
    fn goals_status_wizard_bootkeys_roundtrip() {
        let (_root, ctx) = tool_tree("gsw");
        seed_registry(&ctx);
        write(&ctx.data_dir.join("installed-rom.txt"), b"stock");
        write(
            &ctx.log_dir.join("workflow-state.json"),
            br#"{"goal":"root","steps":[{"id":"reconnaissance","status":"done"}],"updated":""}"#,
        );
        let g = goals_refresh(&ctx);
        assert!(g.contains("Plan for goal: root"));
        assert!(g.contains("reconnaissance [done]"));
        assert!(g.contains("compatibility [pending]"));
        let s = status_refresh(&ctx);
        assert!(s.contains("Phone runs     : Stock EMUI"));
        assert!(s.contains("Device profile : VTR-L29"));
        let w = wizard_refresh(&ctx);
        assert!(w.contains("Your system: Stock EMUI"));
        assert!(w.contains("Goal: root"));
        let b = bootkeys_refresh(&ctx);
        assert!(b.contains("Vol-Up + Power until Huawei logo"));
        assert!(b.contains("persisted boot mode: unknown"));
    }

    #[test]
    fn flash_verify_backup_refresh_paths() {
        let (_root, ctx) = tool_tree("fvb");
        seed_registry(&ctx);
        write(&ctx.data_dir.join("installed-rom.txt"), b"rom:VTR-L29 GSI");
        let f = flash_refresh(&ctx);
        assert!(f.contains("Flash plan"));
        assert!(f.contains("BLOCKED"));
        assert!(f.contains("device gates pending"));
        assert!(f.contains("model: VTR-L29"));
        let v = verify_refresh(&ctx);
        assert!(v.contains("Result: NOT_ROOTED"));
        assert!(v.contains("needs a device"));
    }

    #[test]
    fn media_flow_refresh_paths() {
        let (_root, ctx) = tool_tree("mf");
        seed_registry(&ctx);
        write(&ctx.data_dir.join("installed-rom.txt"), b"stock");
        let k = kernel_refresh(&ctx);
        assert!(k.contains("Proto8"));
        let rm = rootmethods_refresh(&ctx);
        assert!(rm.contains("Current: stock"));
        let rs = romselect_refresh(&ctx);
        assert!(rs.contains("Stock EMUI  <-- current") || rs.contains("[1] Stock EMUI"));
        assert!(rs.contains("LineageOS 20"));
        assert!(restore_refresh(&ctx).contains("no backup with original.img found"));
        assert!(unlock_refresh(&ctx).contains("PotatoNV"));
        let pre = preflight_refresh(&ctx);
        assert!(pre.contains("Device states right now"));
        let lf = logs_files_refresh(&ctx);
        assert!(lf.contains("log files"));
    }
}
