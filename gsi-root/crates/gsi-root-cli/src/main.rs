//! `gsi-root` CLI — thin layer over `gsi-root-core` (no own logic).
//!
//! `analyze`/`inspect`/`workflow`/`config` work today. `patch`/`verify`
//! refuse honestly until a hardware POC exists (EXPERIMENTAL, never faked
//! as done). Without arguments: GUI note (Slint is Phase 9).

use gsi_android::Props;
use gsi_image::{detect_container, parse_sparse_header, Container};
use gsi_root_core::{resolve_engine, GsiProfile, Maturity};
use gsi_workflow::{p10_lineage20, run_analyze, run_patch, ProgressEvent, StepOutcome, WorkflowStep};
use std::path::PathBuf;

fn usage() -> ! {
    eprintln!("gsi-root <command> [args]");
    eprintln!("  analyze|inspect <image>      static GSI profile (no writes)");
    eprintln!("  workflow list                show built-in workflows");
    eprintln!("  workflow run p10-lineage20 [--yes]   walk the reference workflow");
    eprintln!("  patch <image>                EXPERIMENTAL: refused until hardware POC");
    eprintln!("  verify <image>               EXPERIMENTAL: refused until hardware POC");
    eprintln!("  update check --repo <o/r>    latest GitHub release tag");
    eprintln!("  update download --repo <o/r> --asset <name> --out <file> [--sha256 <h>]");
    eprintln!("  update install --staged <file> --target <path>");
    eprintln!("  update rollback --target <path>");
    eprintln!("  device slot [--state-file <path>]");
    eprintln!("  device switch --to magisk|twrp [--magisk <img>] [--twrp <img>] [--state-file <path>]");
    eprintln!("  config show                  resolved app directories");
    eprintln!("  config tool-root --script-dir <path>   tool root from script dir");
    eprintln!("  device slot|switch            slot state + switch plan (no flashing yet)");
    eprintln!("  device detect|info            devices via managed adb/fastboot binding");
    eprintln!("  adb devices                   list adb devices (needs adb on PATH)");
    eprintln!("  fastboot devices|getvar       fastboot queries (needs fastboot on PATH)");
    eprintln!("  tools                         managed external tools status");
    eprintln!("  tools config [--config <path>]            merged tool paths (config + PATH)");
    eprintln!("  tools save-config --out <path> [--adb <p> --fastboot <p> --scrcpy <p>]");
    eprintln!("  tools path-plan --dir <dir>               honest PATH plan (no mutation)");
    eprintln!("  tools link-plan --src <dir> --central <dir>");
    eprintln!("  tools install-plan --registry <file> [--profile <id>] --os <os> --dest <dir>");
    eprintln!("  select rom --registry <file>|--profile <id> [--pick <n|id|label>] [--current <id>]");
    eprintln!("  select image --registry <file>|--profile <id> [--android <n|ver> --system <n|label> --variant <n|label>]");
    eprintln!("  select device [--pick <n|serial>]         plan only, never sets env here");
    eprintln!("  select root-target [--pick <1|2|3>] [--current <id>]");
    eprintln!("  goal show <goal>              text plan from workflow state table");
    eprintln!("  goal screen <step>            screen for one plan step");
    eprintln!("  progress demo                render ProgressEvent lines");
    eprintln!("  verdict flash [--what <op>] [--file <path>]   evaluate flash output");
    eprintln!("  verdict readiness [--all-ok]  render readiness table");
    eprintln!("  install [--yes]              self-copy to user bin dir (--yes: persist PATH on Unix)");
    eprintln!("  version");
    std::process::exit(2);
}

fn flag(args: &[String], name: &str) -> bool {
    args.iter().any(|a| a == name)
}

fn opt(args: &[String], name: &str) -> Option<String> {
    let mut it = args.iter().peekable();
    while let Some(a) = it.next() {
        if a == name {
            return it.next().cloned();
        }
    }
    None
}

fn analyze(path: &str) -> i32 {
    let p = PathBuf::from(path);
    let container = detect_container(&p);
    let sparse = parse_sparse_header(&p);
    let profile = GsiProfile {
        container: Some(container),
        sparse,
        android_api: None,
        arch: None,
    };
    let engine = resolve_engine(&profile);
    println!("file: {path}");
    println!(
        "container: {}",
        match container {
            Container::Gzip => "gzip",
            Container::AndroidSparse => "android-sparse",
            Container::Raw => "raw",
        }
    );
    match sparse {
        Some(h) => println!(
            "sparse: blocks={} block_size={} chunks={}",
            h.total_blocks, h.block_size, h.total_chunks
        ),
        None => println!("sparse: no"),
    }
    println!("engine: {:?} ({:?})", engine.engine, engine.maturity);
    for r in &engine.reasons {
        println!("reason: {r}");
    }
    // Props hook: analyzer reads build.prop from mounted images in later
    // phases; the parser itself is unit-tested (see gsi-android).
    let _ = Props::parse("");
    if engine.maturity == Maturity::Unsupported {
        1
    } else {
        0
    }
}

fn experimental(what: &str) -> i32 {
    eprintln!("gsi-root {what}: EXPERIMENTAL — refused.");
    eprintln!("No hardware POC exists yet: nothing is written, nothing is faked as done.");
    eprintln!("See README.md (phases) and devices/huawei-p10 for the POC plan.");
    3
}

fn workflow_list() -> i32 {
    let w = p10_lineage20(
        &PathBuf::from("input.img"),
        &PathBuf::from("output.img"),
    );
    println!("{}: {}", w.id, w.name);
    for (i, s) in w.steps.iter().enumerate() {
        println!("  {}. {s:?}", i + 1);
    }
    0
}

fn print_events(evs: &[ProgressEvent]) {
    for e in evs {
        match e {
            ProgressEvent::Started { operation } => println!("> {operation}"),
            ProgressEvent::Progress { current, total } => {
                println!("  {current}/{total}")
            }
            ProgressEvent::Message { message } => println!("  {message}"),
            ProgressEvent::Warning { message } => println!("  WARN {message}"),
            ProgressEvent::Completed => println!("  done"),
        }
    }
}

fn workflow_run(id: &str, yes: bool) -> i32 {
    if id != "p10-lineage20" {
        eprintln!("unknown workflow: {id} (try: workflow list)");
        return 2;
    }
    let w = p10_lineage20(
        &PathBuf::from("input.img"),
        &PathBuf::from("output.img"),
    );
    println!("workflow {}: {}", w.id, w.name);
    let mut failed = false;
    for s in &w.steps {
        match s {
            WorkflowStep::Analyze { file } => {
                let (ev, out) = run_analyze(file);
                print_events(&ev);
                println!("analyze: {out:?}");
            }
            WorkflowStep::Patch => {
                let (ev, out) = run_patch("plan");
                print_events(&ev);
                println!("patch: {out:?}");
                if out != StepOutcome::Done {
                    failed = true;
                }
            }
            WorkflowStep::Flash | WorkflowStep::Reboot | WorkflowStep::Test => {
                if yes {
                    println!("{s:?}: refused (no device layer yet)");
                } else {
                    println!("{s:?}: skipped (needs --yes AND a device layer)");
                }
                failed = true;
            }
            other => println!("{other:?}: planned (not executed in this phase)"),
        }
    }
    if failed {
        3
    } else {
        0
    }
}

fn update_check(repo: &str) -> i32 {
    match gsi_update::latest_tag(repo) {
        Ok(t) => {
            println!("latest: {t}");
            0
        }
        Err(e) => {
            eprintln!("update check failed: {e}");
            1
        }
    }
}

fn update_download(repo: &str, asset: &str, out: &str, sha: &str) -> i32 {
    // Resolve asset URL via the release metadata would need JSON parsing of
    // assets; this phase takes the exact asset path segment instead:
    // --asset <path-in-release, e.g. v1.0/file.zip> is appended honestly.
    let url = format!("https://github.com/{repo}/releases/download/{asset}");
    println!("download: {url}");
    match gsi_update::download_to(&url, PathBuf::from(out).as_path()) {
        Ok(()) => {
            if sha.is_empty() {
                eprintln!("downloaded (no hash given -- NOT verified, refusing to stage)");
                return 3;
            }
            match gsi_update::verify_staged(PathBuf::from(out).as_path(), sha) {
                Ok(sig) => {
                    println!("verified SHA-256; signature: {sig:?}");
                    0
                }
                Err(e) => {
                    eprintln!("verify failed: {e}");
                    1
                }
            }
        }
        Err(e) => {
            eprintln!("download failed: {e}");
            1
        }
    }
}

fn update_install(staged: &str, target: &str) -> i32 {
    match gsi_update::install_staged(
        PathBuf::from(target).as_path(),
        PathBuf::from(staged).as_path(),
    ) {
        Ok(bak) => {
            println!("installed; backup: {}", bak.display());
            0
        }
        Err(e) => {
            eprintln!("install failed (rolled back): {e}");
            1
        }
    }
}

fn update_rollback(target: &str) -> i32 {
    let t = PathBuf::from(target);
    let bak = t.with_extension("bak");
    if !bak.exists() {
        eprintln!("no backup found: {}", bak.display());
        return 1;
    }
    match std::fs::rename(&bak, &t) {
        Ok(()) => {
            println!("rolled back to {}", t.display());
            0
        }
        Err(e) => {
            eprintln!("rollback failed: {e}");
            1
        }
    }
}

fn config_show() -> i32 {
    println!("config: {}", opt_path(gsi_config::config_dir()));
    println!("cache: {}", opt_path(gsi_config::cache_dir()));
    println!("bin: {}", opt_path(gsi_config::user_bin_dir()));
    0
}

fn opt_path(p: Option<PathBuf>) -> String {
    p.map(|x| x.display().to_string())
        .unwrap_or_else(|| "(unresolvable)".to_string())
}

/// Managed tool lookup: PATH plus trebleManager tool dirs when present.
fn find_managed(name: &str) -> Option<gsi_tool::Tool> {
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
    gsi_tool::locate(name, &extra)
}

fn device_detect() -> i32 {
    use std::time::Duration;
    let mut any = false;
    if let Some(adb) = find_managed("adb") {
        println!("adb: {}", adb.path.display());
        if let Some(v) = gsi_tool::probe_version(&adb) {
            println!("adb version: {v}");
        }
        let r = gsi_tool::run(&adb, &["devices"], Duration::from_secs(20));
        for d in gsi_device::parse::adb_devices(
            &r.stdout.lines().collect::<Vec<_>>(),
        ) {
            println!("adb {} {}", d.serial, d.state);
            any = true;
        }
        if !r.success {
            println!("adb devices failed: {}", r.stderr.trim());
        }
    } else {
        println!("adb: not found (managed binding needs adb on PATH)");
    }
    if let Some(fb) = find_managed("fastboot") {
        println!("fastboot: {}", fb.path.display());
        if let Some(v) = gsi_tool::probe_version(&fb) {
            println!("fastboot version: {v}");
        }
        let r = gsi_tool::run(&fb, &["devices"], Duration::from_secs(20));
        for d in gsi_device::parse::fastboot_devices(
            &r.stdout.lines().collect::<Vec<_>>(),
        ) {
            println!("fastboot {} {}", d.serial, d.state);
            any = true;
        }
        if !r.success {
            println!("fastboot devices failed: {}", r.stderr.trim());
        }
    } else {
        println!("fastboot: not found (managed binding needs fastboot on PATH)");
    }
    if any {
        0
    } else {
        println!("no devices (or no tools)");
        1
    }
}

fn device_info() -> i32 {
    use std::time::Duration;
    let adb = match find_managed("adb") {
        Some(a) => a,
        None => {
            println!("adb: not found");
            return 1;
        }
    };
    let r = gsi_tool::run(
        &adb,
        &["shell", "getprop"],
        Duration::from_secs(30),
    );
    if !r.success {
        println!("getprop failed (no Android device?)");
        return 1;
    }
    let props =
        gsi_device::parse::getprop(&r.stdout.lines().collect::<Vec<_>>());
    for k in [
        "ro.product.model",
        "ro.product.device",
        "ro.build.version.release",
        "ro.build.display.id",
        "ro.treble.enabled",
    ] {
        if let Some(v) = props.get(k) {
            println!("{k} = {v}");
        }
    }
    0
}

fn tools_status() -> i32 {
    // Managed externals: present/version or honest absence. Never faked.
    let mut missing = false;
    for name in ["adb", "fastboot"] {
        match find_managed(name) {
            Some(t) => {
                let v = gsi_tool::probe_version(&t).unwrap_or_default();
                println!("{name}: {} {v}", t.path.display());
            }
            None => {
                println!("{name}: MISSING (native protocol: Phase 8)");
                missing = true;
            }
        }
    }
    // Extractor programs (data/tools style drops + PATH).
    let mut found_extractor = false;
    for exe in [
        "huawei_firmware_extractor.py",
        "payload-dumper-go",
        "payload-dumper-go.exe",
        "splitupdate",
        "split_updata.pl",
    ] {
        let bare = exe.trim_end_matches(".py").trim_end_matches(".exe");
        if find_managed(exe).is_some()
            || find_managed(bare).is_some()
            || std::path::Path::new(exe).is_file()
        {
            println!("extractor: {exe} present");
            found_extractor = true;
        }
    }
    if !found_extractor {
        println!("extractor: none found (place HuaweiFirmwareExtractor/payload-dumper-go into data/tools/)");
    }
    if missing {
        1
    } else {
        0
    }
}

fn self_install(add_path: bool) -> i32 {
    let dir = match gsi_config::user_bin_dir() {
        Some(d) => d,
        None => {
            eprintln!("cannot resolve user bin dir on this platform");
            return 1;
        }
    };
    match gsi_update::self_install(&dir) {
        Ok(dest) => {
            println!("installed: {}", dest.display());
            if add_path {
                #[cfg(unix)]
                {
                    let rc = std::env::var_os("HOME")
                        .map(|h| PathBuf::from(h).join(".profile"));
                    if let Some(rc) = rc {
                        let line = format!(
                            "\n# gsi-root (idempotent)\ncase \":$PATH:\" in *\":{}:\"*) ;; *) export PATH=\"$PATH:{}\" ;; esac\n",
                            dir.display(),
                            dir.display()
                        );
                        let cur = std::fs::read_to_string(&rc).unwrap_or_default();
                        if !cur.contains(dir.to_str().unwrap_or("\0")) {
                            use std::io::Write;
                            if let Ok(mut f) =
                                std::fs::OpenOptions::new().create(true).append(true).open(&rc)
                            {
                                let _ = f.write_all(line.as_bytes());
                                println!("PATH persisted in {}", rc.display());
                            }
                        } else {
                            println!("PATH already contains {}", dir.display());
                        }
                    }
                }
                #[cfg(windows)]
                {
                    println!("Windows: add {} to your user PATH manually (Settings > Environment).", dir.display());
                    println!("Automatic setx is skipped: it truncates long PATH values.");
                }
            } else {
                println!("ensure {} is on your PATH (--yes persists it on Unix).", dir.display());
            }
            0
        }
        Err(e) => {
            eprintln!("install failed: {e}");
            1
        }
    }
}

// ------------------------------------------------------------ script parity
// Pure CLI helpers mirroring scripts/Treble-Toolkit.ps1 + treble-toolkit.sh.
// These return Result/String/structs and never mutate process env.

/// Tool paths merged from a config file (owned by the CLI crate).
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ToolConfig {
    /// Configured adb path (may be empty).
    #[serde(default)]
    pub adb: String,
    /// Configured fastboot path (may be empty).
    #[serde(default)]
    pub fastboot: String,
    /// Configured scrcpy path (may be empty, optional).
    #[serde(default)]
    pub scrcpy: String,
}

/// Honest plan for a PATH addition (no env mutation here).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathPlan {
    /// Directory that would be added.
    pub dir: String,
    /// Action tag (`add_to_path`).
    pub action: String,
    /// Always true: explicit user confirm required.
    pub needs_confirm: bool,
    /// Why mutation is refused in library context.
    pub message: String,
}

/// Honest plan for one symlink into the central tools folder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkPlan {
    /// Source binary path.
    pub src: String,
    /// Destination link path.
    pub dest: String,
    /// Always true: explicit user confirm required.
    pub needs_confirm: bool,
}

/// Honest plan for a platform-tools install (URL from registry).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstallPlan {
    /// Official download URL (registry `tools` block, per OS).
    pub url: String,
    /// Destination base directory.
    pub dest_dir: String,
    /// Always true: explicit user confirm required.
    pub needs_confirm: bool,
    /// Human note.
    pub note: String,
}

/// Flash output verdict (mirrors Get-FlashVerdict / flash_verdict).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlashVerdict {
    /// `OK` | `FAILED` | `UNCLEAR`.
    pub verdict: String,
    /// Count of `OKAY` lines.
    pub okay: usize,
    /// Trimmed FAILED lines (veto everything).
    pub failed: Vec<String>,
    /// `Total time` value when present.
    pub total_time: String,
}

/// Parse a 1-based numeric selection into a 0-based index.
pub fn parse_selection(input: &str, len: usize) -> Result<usize, String> {
    let t = input.trim();
    if t.is_empty() {
        return Err("empty selection".to_string());
    }
    let mut digits_only = true;
    for ch in t.chars() {
        if !ch.is_ascii_digit() {
            digits_only = false;
            break;
        }
    }
    if !digits_only {
        return Err(format!("not a number: '{t}'"));
    }
    let mut val: usize = 0;
    for ch in t.chars() {
        let d = (ch as usize).saturating_sub('0' as usize);
        val = val.saturating_mul(10).saturating_add(d);
    }
    if val < 1 || val > len {
        return Err(format!("out of range 1-{len}: '{t}'"));
    }
    Ok(val - 1)
}

/// Resolve a list choice by number (1-based) or exact label.
pub fn resolve_list_choice(options: &[String], input: &str) -> Result<String, String> {
    if options.is_empty() {
        return Err("no options".to_string());
    }
    let t = input.trim();
    if t.is_empty() {
        return Err("empty selection".to_string());
    }
    let mut numeric = true;
    for ch in t.chars() {
        if !ch.is_ascii_digit() {
            numeric = false;
            break;
        }
    }
    if numeric {
        match parse_selection(t, options.len()) {
            Ok(i) => match options.get(i) {
                Some(s) => return Ok(s.clone()),
                None => return Err(format!("out of range 1-{}: '{t}'", options.len())),
            },
            Err(e) => return Err(e),
        }
    }
    for o in options {
        if o == t {
            return Ok(o.clone());
        }
    }
    Err(format!("unknown choice: '{t}'"))
}

/// ROM options for the CLI: stock first, registry, other last.
pub fn cli_rom_options(entries: &[gsi_registry::RomEntry]) -> Vec<(String, String)> {
    let mut out = Vec::new();
    out.push((
        "stock".to_string(),
        "Stock EMUI (Huawei original)".to_string(),
    ));
    for (id, label) in gsi_registry::rom_options(entries) {
        if id == "stock" {
            continue;
        }
        let mut dup = false;
        for (eid, _) in &out {
            if eid == &id {
                dup = true;
                break;
            }
        }
        if !dup {
            out.push((id, label));
        }
    }
    out.push((
        "other".to_string(),
        "Other custom ROM (not in list)".to_string(),
    ));
    out
}

/// Resolve a ROM choice by number, id (`rom:...`/`stock`/`other`) or label.
pub fn resolve_rom_choice(
    options: &[(String, String)],
    input: &str,
) -> Result<String, String> {
    if options.is_empty() {
        return Err("no ROM options".to_string());
    }
    let t = input.trim();
    if t.is_empty() {
        return Err("empty selection".to_string());
    }
    let mut numeric = true;
    for ch in t.chars() {
        if !ch.is_ascii_digit() {
            numeric = false;
            break;
        }
    }
    if numeric {
        match parse_selection(t, options.len()) {
            Ok(i) => match options.get(i) {
                Some((id, _)) => return Ok(id.clone()),
                None => return Err(format!("out of range 1-{}: '{t}'", options.len())),
            },
            Err(e) => return Err(e),
        }
    }
    for (id, label) in options {
        if id == t || label == t {
            return Ok(id.clone());
        }
    }
    Err(format!("unknown ROM choice: '{t}'"))
}

/// Numbered ROM menu text (golden-renderable).
pub fn render_rom_menu(options: &[(String, String)], current: &str) -> String {
    let mut s = String::from("Which system is on your phone right now?\n");
    for (i, (_, label)) in options.iter().enumerate() {
        let mut mark = String::new();
        if let Some((id, _)) = options.get(i) {
            if id == current {
                mark = "  <-- current, Enter keeps it".to_string();
            }
        }
        s.push_str(&format!(" [{}] {label}{mark}\n", i + 1));
    }
    s
}

/// Short display name for an installed-ROM id.
pub fn installed_rom_label(id: &str) -> String {
    gsi_gates::rom_label(id)
}

/// Variant display: variant, else build, else gsi (mirrors resolver screens).
pub fn variant_label(entry: &gsi_registry::RomEntry) -> String {
    if !entry.variant.trim().is_empty() {
        return entry.variant.trim().to_string();
    }
    let b = match &entry.build {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Number(n) => n.to_string(),
        _ => String::new(),
    };
    if !b.trim().is_empty() {
        return b.trim().to_string();
    }
    entry.gsi.clone()
}

/// Numbered Android menu text from registry entries.
pub fn render_android_menu(entries: &[gsi_registry::RomEntry]) -> String {
    let mut s = String::from("Which Android version should be installed?\n");
    for (i, (ver, count)) in gsi_registry::target_androids(entries).iter().enumerate() {
        s.push_str(&format!(" [{}] Android {ver}   ({count} images)\n", i + 1));
    }
    s
}

/// Resolve an Android choice by menu number or version label.
pub fn resolve_android_choice(
    entries: &[gsi_registry::RomEntry],
    input: &str,
) -> Result<i64, String> {
    let androids = gsi_registry::target_androids(entries);
    if androids.is_empty() {
        return Err("no working images in registry".to_string());
    }
    let t = input.trim();
    if t.is_empty() {
        return Err("empty selection".to_string());
    }
    let mut numeric = true;
    for ch in t.chars() {
        if !ch.is_ascii_digit() {
            numeric = false;
            break;
        }
    }
    if numeric {
        if let Ok(i) = parse_selection(t, androids.len()) {
            if let Some((v, _)) = androids.get(i) {
                return Ok(*v);
            }
        }
        for (v, _) in &androids {
            if v.to_string() == t {
                return Ok(*v);
            }
        }
        return Err(format!("unknown Android choice: '{t}'"));
    }
    let low = t.to_lowercase();
    let mut digits = String::new();
    for ch in low.chars() {
        if ch.is_ascii_digit() {
            digits.push(ch);
        }
    }
    if !digits.is_empty() {
        for (v, _) in &androids {
            if v.to_string() == digits {
                return Ok(*v);
            }
        }
    }
    Err(format!("unknown Android choice: '{t}'"))
}

/// Numbered system menu text.
pub fn render_system_menu(systems: &[String]) -> String {
    let mut s = String::from("Systems:\n");
    for (i, l) in systems.iter().enumerate() {
        s.push_str(&format!(" [{}] {l}\n", i + 1));
    }
    s
}

/// Resolve a system choice by number or exact label.
pub fn resolve_system_choice(systems: &[String], input: &str) -> Result<String, String> {
    resolve_list_choice(systems, input)
}

/// Render a resolved target config as CLI text.
pub fn render_target_config(cfg: &gsi_registry::TargetConfig) -> String {
    let mut s = String::new();
    s.push_str("Device   : Huawei\n");
    s.push_str(&format!("Android  : {}\n", cfg.android));
    s.push_str(&format!("System   : {}\n", cfg.system));
    s.push_str(&format!("Base     : {}\n", cfg.firmware_base));
    s.push_str(&format!(
        "Root     : Magisk / {} ({})\n",
        cfg.root_type, cfg.root_source
    ));
    if !cfg.system_file.is_empty() {
        s.push_str(&format!("System   : {}\n", cfg.system_file));
    }
    if !cfg.system_url.is_empty() {
        s.push_str(&format!("Download : {}\n", cfg.system_url));
    } else {
        s.push_str("Download : no verified direct link (manual package into data/roms/).\n");
    }
    s
}

/// Build `serial (transport)` choices from adb/fastboot serial lists.
pub fn render_device_choices(adb_serials: &[String], fb_serials: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    for s in adb_serials {
        let t = s.trim();
        if !t.is_empty() {
            out.push(format!("{t} (adb)"));
        }
    }
    for s in fb_serials {
        let t = s.trim();
        if !t.is_empty() {
            out.push(format!("{t} (fastboot)"));
        }
    }
    out
}

/// Resolve a device choice by number, serial, or full entry; returns serial.
pub fn resolve_device_serial(choices: &[String], input: &str) -> Result<String, String> {
    if choices.is_empty() {
        return Err("no devices".to_string());
    }
    let t = input.trim();
    if t.is_empty() {
        return Err("empty selection".to_string());
    }
    let mut numeric = true;
    for ch in t.chars() {
        if !ch.is_ascii_digit() {
            numeric = false;
            break;
        }
    }
    if numeric {
        match parse_selection(t, choices.len()) {
            Ok(i) => match choices.get(i) {
                Some(c) => {
                    let mut serial = String::new();
                    for part in c.split_whitespace() {
                        serial = part.to_string();
                        break;
                    }
                    if serial.is_empty() {
                        return Err("invalid device entry".to_string());
                    }
                    return Ok(serial);
                }
                None => return Err(format!("out of range 1-{}: '{t}'", choices.len())),
            },
            Err(e) => return Err(e),
        }
    }
    for c in choices {
        if c == t {
            let mut serial = String::new();
            for part in c.split_whitespace() {
                serial = part.to_string();
                break;
            }
            if serial.is_empty() {
                return Err("invalid device entry".to_string());
            }
            return Ok(serial);
        }
        let mut serial = String::new();
        for part in c.split_whitespace() {
            serial = part.to_string();
            break;
        }
        if serial == t {
            return Ok(serial);
        }
    }
    Err(format!("unknown device choice: '{t}'"))
}

/// Root-target menu text (current label shown).
pub fn render_root_target_menu(current_label: &str) -> String {
    format!(
        "Root target: which system stays on the phone?\n [1] Current system: {current_label}\n [2] Stock EMUI (choose version)\n [3] Other system / ROM (choose Android -> ROM -> variant)\n"
    )
}

/// Resolve a root-target choice (`1`/`2`/`3` or words) to a tag.
pub fn resolve_root_target_choice(input: &str) -> Result<String, String> {
    let t = input.trim().to_lowercase();
    if t == "1" || t == "current" || t == "keep" {
        return Ok("current".to_string());
    }
    if t == "2" || t == "stock" {
        return Ok("stock".to_string());
    }
    if t == "3" || t == "other" {
        return Ok("other".to_string());
    }
    Err(format!(
        "unknown root target choice: '{}' (use 1/2/3)",
        input.trim()
    ))
}

/// Map one plan step to its screen (mirrors `goal_screen` dispatch).
pub fn goal_screen_target(step: &str) -> &'static str {
    match step.trim().to_lowercase().as_str() {
        "reconnaissance" => "Screen-Analyze",
        "compatibility" | "custom_rom_compatibility" | "rom_compatibility" => {
            "Screen-Compatibility"
        }
        "firmware" | "firmware_selection" | "firmware_validation" | "stock_firmware_validation" => {
            "Screen-Firmware"
        }
        "extract" | "artifact_extraction" | "rom_validation" => "Screen-Extract",
        "magisk_patch" | "root_preparation" | "root_image_preparation" => "Screen-Patch",
        "backup" | "backup_if_required" => "Screen-Backup",
        "flash" | "root_flash" | "safety_gate" | "flash_plan" => "Screen-Flash",
        "rom_installation" | "rom_flash" | "flash_system" => "Screen-FlashSystem",
        "recovery_compatibility" => "Screen-RootMethods",
        "recovery_validation" | "recovery_flash" => "Screen-Twrp",
        "reboot" | "boot" | "root_verify" | "verify" | "validate" => "Screen-RebootVerify",
        "restore" | "identify_original_artifact" | "validate_backup" | "rollback_plan" => {
            "Screen-Restore"
        }
        "rom_export" | "artifact_export" => "Screen-ExportRecovery",
        "wipe" => "Screen-Wipe",
        _ => "Manual-Step",
    }
}

/// Render a goal plan as CLI text (goal + steps + screens).
pub fn render_goal_plan(goal: &str) -> Result<String, String> {
    let plan = gsi_state::new_workflow_plan(goal).map_err(|e| e)?;
    let mut s = format!("Plan for {}:\n", plan.goal);
    for st in &plan.steps {
        let screen = goal_screen_target(&st.id);
        let gate = match gsi_state::step_gate(&st.id) {
            Some(g) => g,
            None => "-".to_string(),
        };
        s.push_str(&format!(" - {} -> {screen} [gate:{gate}]\n", st.id));
    }
    Ok(s)
}

/// Render one `ProgressEvent` as a text line (matches CLI `print_events`).
pub fn render_progress_event(ev: &gsi_workflow::ProgressEvent) -> String {
    match ev {
        gsi_workflow::ProgressEvent::Started { operation } => format!("> {operation}"),
        gsi_workflow::ProgressEvent::Progress { current, total } => {
            format!("  {current}/{total}")
        }
        gsi_workflow::ProgressEvent::Message { message } => format!("  {message}"),
        gsi_workflow::ProgressEvent::Warning { message } => format!("  WARN {message}"),
        gsi_workflow::ProgressEvent::Completed => "  done".to_string(),
    }
}

/// Render many `ProgressEvent`s as newline-joined text lines.
pub fn render_progress_events(evs: &[gsi_workflow::ProgressEvent]) -> String {
    let mut out = Vec::new();
    for e in evs {
        out.push(render_progress_event(e));
    }
    out.join("\n")
}

/// Evaluate fastboot flash/erase output (pure, mirrors scripts).
///
/// `FAILED`/`remote:`/`error` lines veto everything; `OK` needs at
/// least one `OKAY` plus a `Finished ... Total time:` line.
pub fn evaluate_flash_output(lines: &[&str]) -> FlashVerdict {
    let mut okay: usize = 0;
    let mut failed: Vec<String> = Vec::new();
    let mut total = String::new();
    for line in lines {
        if line.contains("OKAY") {
            okay = okay.saturating_add(1);
        }
        let low = line.to_lowercase();
        let trimmed_start = line.trim_start().to_lowercase();
        if low.contains("failed")
            || low.contains("remote:")
            || trimmed_start.starts_with("error")
        {
            failed.push(line.trim().to_string());
        }
        if low.contains("finished") && low.contains("total time:") {
            let needle = "total time:";
            if let Some(pos) = low.find(needle) {
                let start = pos.saturating_add(needle.len());
                if let Some(tail) = line.get(start..) {
                    total = tail.trim().to_string();
                }
            }
        }
    }
    let verdict = if !failed.is_empty() {
        "FAILED".to_string()
    } else if okay > 0 && !total.is_empty() {
        "OK".to_string()
    } else {
        "UNCLEAR".to_string()
    };
    FlashVerdict {
        verdict,
        okay,
        failed,
        total_time: total,
    }
}

/// Render a flash verdict as CLI text (mirrors Show-FlashVerdict).
pub fn render_flash_verdict(v: &FlashVerdict, what: &str) -> String {
    let w = if what.trim().is_empty() {
        "flash".to_string()
    } else {
        what.trim().to_string()
    };
    if v.verdict == "OK" {
        let mut s = format!("FLASH RESULT: OK ({} OKAY", v.okay);
        if !v.total_time.is_empty() {
            s.push_str(&format!(", {}", v.total_time));
        }
        s.push_str(&format!(") [{w}]\n"));
        s
    } else if v.verdict == "FAILED" {
        let mut s = String::from("FLASH RESULT: FAILED - nothing claimed as done.\n");
        let mut n = 0;
        for f in &v.failed {
            if n >= 3 {
                break;
            }
            s.push_str(&format!(" ! {f}\n"));
            n += 1;
        }
        s.push_str("Hints: 'Command not allowed' = Huawei refused (retry, cable, TROUBLESHOOTING). 'too large' = image bigger than partition. Full output is in the log.\n");
        s
    } else {
        String::from(
            "FLASH RESULT: UNCLEAR - no FAILED, but no Finished either. Verify manually before rebooting.\n",
        )
    }
}

/// Render a `FlashReadiness` table as CLI text.
pub fn render_readiness(r: &gsi_gates::FlashReadiness) -> String {
    let mut s = String::new();
    for c in &r.checks {
        let mark = if c.pass { "[OK]" } else { "[FAIL]" };
        s.push_str(&format!("{mark} {}: {}\n", c.name, c.detail));
    }
    if r.go {
        s.push_str("READY: all checks pass.\n");
    } else {
        s.push_str("BLOCKED: fix FAILED checks first.\n");
    }
    s
}

/// Render firmware baseline (mirrors Get-TTFirmwareBaseline display).
pub fn render_firmware_baseline(display: &str, incremental: &str, override_baseline: &str) -> String {
    let base =
        gsi_gates::firmware_baseline_from_parts(display, incremental, override_baseline);
    format!("firmware baseline: {base}\n")
}

/// Render a preflight report (mirrors Show-PreflightBlocked text side).
pub fn render_preflight_report(r: &gsi_gates::PreflightReport) -> String {
    let mut s = String::new();
    if r.go {
        s.push_str("PREFLIGHT: GO — no blockers.\n");
    } else {
        s.push_str("PREFLIGHT: BLOCKED\n");
        for b in &r.blocks {
            s.push_str(&format!("  - {b}\n"));
        }
    }
    s
}

/// Probe directory writability (create + remove probe file; no leftovers).
fn dir_writable(dir: &std::path::Path) -> bool {
    let probe = dir.join(".tt-write-probe");
    match std::fs::write(&probe, b"tt") {
        Ok(()) => {
            let _ = std::fs::remove_file(&probe);
            true
        }
        Err(_) => false,
    }
}

fn preflight_cmd(args: &[String]) -> i32 {
    let adb = opt(args, "--adb")
        .or_else(|| find_managed("adb").map(|t| t.path.display().to_string()))
        .unwrap_or_default();
    let fastboot = opt(args, "--fastboot")
        .or_else(|| find_managed("fastboot").map(|t| t.path.display().to_string()))
        .unwrap_or_default();
    let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    let mut writables = vec![("workdir".to_string(), dir_writable(&cwd))];
    for (i, w) in args.iter().enumerate() {
        if w == "--writable" {
            if let Some(d) = args.get(i + 1) {
                writables.push((d.clone(), dir_writable(std::path::Path::new(d))));
            }
        }
    }
    let mut files = Vec::new();
    for (i, w) in args.iter().enumerate() {
        if w == "--file" {
            if let Some(f) = args.get(i + 1) {
                files.push((f.clone(), std::path::Path::new(f).is_file()));
            }
        }
    }
    let mut hashes = Vec::new();
    for (i, w) in args.iter().enumerate() {
        if w == "--expect-hash" {
            if let Some(spec) = args.get(i + 1) {
                let mut it = spec.splitn(2, '=');
                let name = it.next().unwrap_or("").to_string();
                let expected = it.next().unwrap_or("").to_string();
                let actual = gsi_workflow::sha256_file(std::path::Path::new(&name))
                    .unwrap_or_default();
                hashes.push((name, expected, actual));
            }
        }
    }
    let report = gsi_gates::preflight(&gsi_gates::PreflightInput {
        adb_path: adb,
        fastboot_path: fastboot,
        shell_ok: true,
        writables,
        files,
        hashes,
    });
    print!("{}", render_preflight_report(&report));
    if report.go {
        0
    } else {
        1
    }
}
/// First-run flow text (mirrors Invoke-TTFirstRun): missing-tools notice +
/// setup guidance + config note. No execution, no prompts in lib.
pub fn first_run_text(adb_ok: bool, fastboot_ok: bool) -> String {
    if adb_ok && fastboot_ok {
        return "first run: required tools present (adb/fastboot).\n".to_string();
    }
    let mut s = String::new();
    s.push_str("FIRST RUN: required tools are missing (adb/fastboot).\n");
    if !adb_ok {
        s.push_str("  - adb: missing — place platform-tools on PATH or run setup\n");
    }
    if !fastboot_ok {
        s.push_str("  - fastboot: missing — place platform-tools on PATH or run setup\n");
    }
    s.push_str("Remote run: execute Setup-TrebleToolkit.bat from a local copy once,\n");
    s.push_str("or place adb/fastboot on PATH. Tool paths are saved to data/config.json.\n");
    s
}

/// Parse tool-path JSON (tolerates missing keys, rejects invalid JSON).
pub fn parse_tool_config(text: &str) -> Result<ToolConfig, String> {
    if text.trim().is_empty() {
        return Ok(ToolConfig::default());
    }
    let v: serde_json::Value =
        serde_json::from_str(text).map_err(|e| format!("config: invalid json: {e}"))?;
    let get = |k: &str| -> String {
        match v.get(k).and_then(|x| x.as_str()) {
            Some(s) => s.to_string(),
            None => String::new(),
        }
    };
    Ok(ToolConfig {
        adb: get("adb"),
        fastboot: get("fastboot"),
        scrcpy: get("scrcpy"),
    })
}

/// Load tool-path JSON; missing file yields empty config.
pub fn load_tool_config(path: &std::path::Path) -> Result<ToolConfig, String> {
    let text = match std::fs::read_to_string(path) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Ok(ToolConfig::default())
        }
        Err(e) => return Err(format!("read: {e}")),
    };
    parse_tool_config(&text)
}

/// Write tool-path JSON (creates parent dirs; stable shape for tests).
pub fn save_tool_config(path: &std::path::Path, cfg: &ToolConfig) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).map_err(|e| format!("mkdir: {e}"))?;
        }
    }
    let v = serde_json::json!({
        "adb": cfg.adb,
        "fastboot": cfg.fastboot,
        "scrcpy": cfg.scrcpy,
    });
    let text = serde_json::to_string_pretty(&v).map_err(|e| format!("json: {e}"))?;
    std::fs::write(path, text).map_err(|e| format!("write: {e}"))?;
    Ok(())
}

/// Tool root from a script dir (`scripts` leaf goes to parent).
pub fn tool_root_from_script_dir(script_dir: &std::path::Path) -> PathBuf {
    let is_scripts = match script_dir.file_name().and_then(|s| s.to_str()) {
        Some(n) => n == "scripts",
        None => false,
    };
    if is_scripts {
        match script_dir.parent() {
            Some(p) => p.to_path_buf(),
            None => script_dir.to_path_buf(),
        }
    } else {
        script_dir.to_path_buf()
    }
}

/// Resolve one tool: configured file wins, else PATH scan via `gsi-tool`.
pub fn resolve_tool_with_config(
    configured: &str,
    name: &str,
    extra_dirs: &[PathBuf],
) -> Option<PathBuf> {
    let t = configured.trim();
    if !t.is_empty() {
        let p = PathBuf::from(t);
        if p.is_file() {
            return Some(p);
        }
    }
    gsi_tool::locate(name, extra_dirs).map(|tool| tool.path)
}

/// Clear refusal for env mutation in library context.
pub fn refuse_env_mutation(op: &str) -> String {
    let name = if op.trim().is_empty() {
        "operation".to_string()
    } else {
        op.trim().to_string()
    };
    format!(
        "refused: '{name}' mutates process env/PATH; library context never mutates env. Confirm explicitly in the CLI (show plan, ask [y/N]), then apply in the caller."
    )
}

/// Honest PATH plan (no mutation).
pub fn plan_add_to_path(dir: &str) -> PathPlan {
    PathPlan {
        dir: dir.trim().to_string(),
        action: "add_to_path".to_string(),
        needs_confirm: true,
        message: refuse_env_mutation("add_to_path"),
    }
}

/// Render a PATH plan with explicit confirm representation.
pub fn render_path_plan(p: &PathPlan) -> String {
    format!(
        "PLAN: {} '{}' (session only, needs confirm [y/N]).\nNote: {}\n",
        p.action, p.dir, p.message
    )
}

/// Honest link plan for `adb`/`fastboot` (no symlinks created here).
pub fn plan_link_into_tools(
    src_dir: &std::path::Path,
    central_dir: &std::path::Path,
) -> Vec<LinkPlan> {
    let mut out = Vec::new();
    for name in ["adb", "fastboot"] {
        out.push(LinkPlan {
            src: src_dir.join(name).display().to_string(),
            dest: central_dir.join(name).display().to_string(),
            needs_confirm: true,
        });
    }
    out
}

/// Render one link plan with explicit confirm representation.
pub fn render_link_plan(p: &LinkPlan) -> String {
    format!(
        "PLAN: symlink '{}' -> '{}' (needs confirm [y/N]).\n",
        p.src, p.dest
    )
}

/// Platform-tools install plan with URL from the registry `tools` block.
pub fn platform_tools_install_plan(
    tools: &gsi_registry::ToolsBlock,
    os: &str,
    dest_dir: &std::path::Path,
) -> Result<InstallPlan, String> {
    let url = gsi_registry::platform_tools_url(tools, os).map_err(|e| e)?;
    if dest_dir.as_os_str().is_empty() {
        return Err("install: empty dest dir".to_string());
    }
    Ok(InstallPlan {
        url,
        dest_dir: dest_dir.display().to_string(),
        needs_confirm: true,
        note: "official Google platform-tools, portable zip; extract then PATH (confirm first)"
            .to_string(),
    })
}

/// Render an install plan with explicit confirm representation.
pub fn render_install_plan(p: &InstallPlan) -> String {
    format!(
        "PLAN: download official platform-tools\n URL : {}\n Dest: {}\nNote: {} (needs confirm [y/N]).\n",
        p.url, p.dest_dir, p.note
    )
}

// ------------------------------------------------------------ thin remainders
// Pure CLI compositions over registry/archive/update/workflow. No spawn,
// no device access. Hermetic tests use local fixtures only.

/// Stock EMUI version menu (guided text, mirrors Select-RootTarget).
pub fn render_stock_version_menu() -> String {
    String::from("Which Stock EMUI version?\n [1] Android 8 / EMUI 8\n [2] Android 9 / EMUI 9.0\n [3] Android 9 / EMUI 9.1\n")
}

/// Resolve a stock version pick to its EMUI label.
pub fn resolve_stock_version_choice(input: &str) -> Result<String, String> {
    let low = input.trim().to_lowercase();
    let s: &str = match low.strip_prefix("stock:") {
        Some(rest) => rest.trim(),
        None => low.trim(),
    };
    if s == "1" || s == "8" || s == "emui 8" || s == "android 8" || s == "android 8 / emui 8" {
        return Ok("EMUI 8 (Android 8)".to_string());
    }
    if s == "2" || s == "9.0" || s == "emui 9.0" || s == "android 9 / emui 9.0" {
        return Ok("EMUI 9.0 (Android 9)".to_string());
    }
    if s == "3" || s == "9.1" || s == "emui 9.1" || s == "android 9 / emui 9.1" {
        return Ok("EMUI 9.1 (Android 9)".to_string());
    }
    Err(format!(
        "unknown stock version: '{}' (use 1/2/3 or 8/9.0/9.1)",
        input.trim()
    ))
}

/// Root-target detail with stock sub-choice as guided text.
pub fn resolve_root_target_detail(choice: &str) -> Result<String, String> {
    let t = choice.trim().to_lowercase();
    if t == "1" || t == "current" || t == "keep" {
        return Ok(
            "target kept: keep system, root only (patch base from installed ROM)".to_string(),
        );
    }
    if t == "2" || t == "stock" {
        return Ok(render_stock_version_menu());
    }
    if t == "3" || t == "other" {
        return Ok("target: other system (run `select image` to resolve Android -> system -> variant; system is NOT replaced for root)\n".to_string());
    }
    match resolve_stock_version_choice(choice) {
        Ok(label) => {
            let note = if label.contains("9.1") {
                "EMUI 9.1 base ok (recovery_ramdisk method)"
            } else if label.contains("9.0") {
                "EMUI 9.0 instead of 9.1, method possible but prefer full 9.1 firmware"
            } else {
                "EMUI 8 base, different boot chain possible (see wiki/kernel notes)"
            };
            Ok(format!(
                "Target: Stock {label}\nNote: {note}; full UPDATE.APP/ZIP into data/firmware/.\n"
            ))
        }
        Err(_) => Err(format!(
            "unknown root target choice: '{}' (use 1/2/3)",
            choice.trim()
        )),
    }
}

/// Magisk APK facts with SHA-256 plus SHA-512.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MagiskInfoDetail {
    /// APK path as text.
    pub path: String,
    /// Version from file name.
    pub version: String,
    /// SHA-256 hex.
    pub sha256: String,
    /// SHA-512 hex.
    pub sha512: String,
    /// File size in bytes.
    pub size: u64,
    /// Source hint.
    pub source: String,
}

/// Local Magisk APK facts (never more than the file shows).
pub fn magisk_info_detail(apk_path: &std::path::Path) -> Result<MagiskInfoDetail, String> {
    let base = gsi_update::magisk_info(apk_path)?;
    let sha512 = gsi_workflow::sha512_file(apk_path)?;
    Ok(MagiskInfoDetail {
        path: base.path.display().to_string(),
        version: base.version,
        sha256: base.sha256,
        sha512,
        size: base.size,
        source: base.source,
    })
}

/// Render Magisk facts as CLI text.
pub fn render_magisk_info(info: &MagiskInfoDetail) -> String {
    format!(
        "APK: {}\nVersion: {}\nSHA256: {}\nSHA512: {}\nSize: {}\nSource: {}\n",
        info.path, info.version, info.sha256, info.sha512, info.size, info.source
    )
}

/// Registry context for the Magisk stable channel.
#[derive(Debug, Clone)]
pub struct MagiskRegistry {
    /// Resolved profile file.
    pub registry_path: PathBuf,
    /// Stable.json URL (registry override wins).
    pub stable_url: String,
    /// Count of ROM entries.
    pub rom_count: usize,
}

/// Load registry file for Magisk (registry load plus stable URL).
pub fn load_registry_for_magisk(
    data_dir: &std::path::Path,
    model: &str,
) -> Result<MagiskRegistry, String> {
    let m = model.trim();
    if m.is_empty() {
        return Err("magisk registry: empty model".to_string());
    }
    if m.contains('/') || m.contains('\\') || m.contains("..") {
        return Err(format!("magisk registry: bad model '{m}'"));
    }
    if data_dir.as_os_str().is_empty() {
        return Err("magisk registry: empty data dir".to_string());
    }
    let c1 = data_dir
        .join("data")
        .join("compatibility")
        .join("huawei")
        .join("p10")
        .join(format!("{m}.json"));
    let c2 = data_dir.join(format!("{m}.json"));
    let picked = if c1.is_file() { c1 } else { c2 };
    let text = match std::fs::read_to_string(&picked) {
        Ok(t) => t,
        Err(e) => return Err(format!("read {}: {e}", picked.display())),
    };
    let roms = gsi_registry::load_roms(&picked)?;
    let url = gsi_update::magisk_stable_url(Some(text.as_str()));
    Ok(MagiskRegistry {
        registry_path: picked,
        stable_url: url,
        rom_count: roms.len(),
    })
}

/// Thin path wrapper over gsi-archive (mirrors Expand-TTRomArchive).
pub fn extract_archive_cli(
    path: &std::path::Path,
    dest: &std::path::Path,
) -> Result<Vec<PathBuf>, String> {
    let name = match path.file_name().and_then(|n| n.to_str()) {
        Some(n) => n.to_string(),
        None => return Err("archive: bad file name".to_string()),
    };
    if dest.as_os_str().is_empty() {
        return Err("archive: empty dest dir".to_string());
    }
    match gsi_archive::classify(name.as_str()) {
        gsi_archive::ArchiveKind::Tar
        | gsi_archive::ArchiveKind::TarGz
        | gsi_archive::ArchiveKind::TarXz
        | gsi_archive::ArchiveKind::Tgz => gsi_archive::extract_tar(path, dest),
        gsi_archive::ArchiveKind::Zip => {
            let data = match std::fs::read(path) {
                Ok(d) => d,
                Err(e) => return Err(format!("read {}: {e}", path.display())),
            };
            gsi_archive::extract_zip(data.as_slice(), dest, false)
        }
        gsi_archive::ArchiveKind::GzipSingle | gsi_archive::ArchiveKind::XzSingle => {
            let out = gsi_archive::decompress_single(path, dest)?;
            let rel = match out.file_name().and_then(|n| n.to_str()) {
                Some(n) => PathBuf::from(n),
                None => out,
            };
            Ok(vec![rel])
        }
        gsi_archive::ArchiveKind::Plain => Err(format!("unsupported archive: {name}")),
    }
}

/// Guided install text (mirrors ensure_tool plus ensure_scrcpy).
pub fn ensure_tool_plan(tool: &str) -> String {
    let t = tool.trim().to_lowercase();
    if t == "scrcpy" {
        return String::from(
            "scrcpy (screen mirror) is optional. Want it?\n [1] Install hint for your OS  [2] Select executable (scans folder)  [3] Skip (stays optional)\nHints:\n Debian/Ubuntu: sudo apt install scrcpy\n Fedora: sudo dnf install scrcpy\n Arch: sudo pacman -S scrcpy\n macOS: brew install scrcpy\n Windows: Setup-TrebleToolkit.bat or https://github.com/Genymobile/scrcpy/releases\nNote: scrcpy stays optional forever; skipping is valid.\n",
        );
    }
    let name = if t.is_empty() {
        "tool".to_string()
    } else {
        tool.trim().to_string()
    };
    format!(
        "{name} missing. Install or point to it?\n [1] Install into PATH (official Google platform-tools, portable)\n [2] Select one executable (scans its folder for the others)\n [3] Select every needed executable manually\n [4] Use custom folder as PATH (adds folder to PATH / symlinks into central tools)  [q] Abort\nNote: library context never mutates env; confirm explicitly in the CLI.\n"
    )
}

fn platform_zip_name(url: &str) -> String {
    let no_q: &str = match url.split(&['?', '#'][..]).next() {
        Some(s) => s,
        None => "",
    };
    let trimmed = no_q.trim_end_matches('/');
    let name: &str = match trimmed.rsplit('/').next() {
        Some(s) => s,
        None => "",
    };
    if name.is_empty() {
        "platform-tools.zip".to_string()
    } else {
        name.to_string()
    }
}

/// Full platform-tools install (URL plus fetch plus zip extract).
pub fn install_platform_tools_run(
    registry_path: &std::path::Path,
    os: &str,
    dest: &std::path::Path,
) -> Result<Vec<PathBuf>, String> {
    if dest.as_os_str().is_empty() {
        return Err("install: empty dest dir".to_string());
    }
    let tools = gsi_registry::tools_block(registry_path)?;
    let url = gsi_registry::platform_tools_url(&tools, os)?;
    let zip_name = platform_zip_name(url.as_str());
    let zip_dest = dest.join(zip_name.as_str());
    let _ = gsi_update::download_file(url.as_str(), &zip_dest)?;
    let data = match std::fs::read(&zip_dest) {
        Ok(d) => d,
        Err(e) => return Err(format!("read {}: {e}", zip_dest.display())),
    };
    gsi_archive::extract_auto(zip_name.as_str(), data.as_slice(), dest)
}

// ------------------------------------------------------------ dispatch
// Thin CLI dispatch over the pure helpers above (I/O only here).

fn registry_path_from_args(args: &[String]) -> Result<PathBuf, String> {
    match opt(args, "--registry") {
        Some(p) if !p.trim().is_empty() => Ok(PathBuf::from(p)),
        _ => match opt(args, "--profile") {
            Some(id) if !id.trim().is_empty() => {
                Ok(gsi_registry::repo_profile_path(id.trim()))
            }
            _ => Err("needs --registry <file> or --profile <id>".to_string()),
        },
    }
}

fn current_rom_from_args(args: &[String]) -> String {
    match opt(args, "--current") {
        Some(c) if !c.trim().is_empty() => return c.trim().to_string(),
        _ => {}
    }
    match opt(args, "--installed-file") {
        Some(f) if !f.trim().is_empty() => {
            match gsi_state::read_installed_rom(std::path::Path::new(&f)) {
                Ok(Some(id)) => id,
                _ => String::new(),
            }
        }
        _ => String::new(),
    }
}

fn select_rom_cmd(args: &[String]) -> i32 {
    let reg = match registry_path_from_args(args) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("select rom: {e}");
            return 2;
        }
    };
    let entries = match gsi_registry::load_roms(&reg) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("select rom: load failed: {e}");
            return 1;
        }
    };
    let options = cli_rom_options(&entries);
    let current = current_rom_from_args(args);
    match opt(args, "--pick") {
        Some(pick) => match resolve_rom_choice(&options, &pick) {
            Ok(id) => {
                println!("selected: {id} ({})", installed_rom_label(&id));
                0
            }
            Err(e) => {
                eprintln!("select rom: {e}");
                1
            }
        },
        None => {
            print!("{}", render_rom_menu(&options, &current));
            eprintln!("re-run with --pick <n|id|label>");
            2
        }
    }
}

fn select_image_cmd(args: &[String]) -> i32 {
    let reg = match registry_path_from_args(args) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("select image: {e}");
            return 2;
        }
    };
    let entries = match gsi_registry::load_roms(&reg) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("select image: load failed: {e}");
            return 1;
        }
    };
    let fw_base = gsi_registry::firmware_base(&reg);
    let android = match opt(args, "--android") {
        Some(a) => match resolve_android_choice(&entries, &a) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("select image: {e}");
                return 1;
            }
        },
        None => {
            print!("{}", render_android_menu(&entries));
            eprintln!("re-run with --android <n|ver> --system <n|label> [--variant <n|label>]");
            return 2;
        }
    };
    let systems = gsi_registry::systems_for(&entries, android);
    if systems.is_empty() {
        eprintln!("select image: no systems for Android {android}");
        return 1;
    }
    let system = match opt(args, "--system") {
        Some(s) => match resolve_system_choice(&systems, &s) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("select image: {e}");
                return 1;
            }
        },
        None => {
            print!("{}", render_system_menu(&systems));
            eprintln!("re-run with --system <n|label> [--variant <n|label>]");
            return 2;
        }
    };
    let variants = gsi_registry::variants_for(&entries, &system);
    if variants.is_empty() {
        eprintln!("select image: no variants for '{system}'");
        return 1;
    }
    let entry: &gsi_registry::RomEntry = if variants.len() == 1 {
        match variants.get(0) {
            Some(e) => *e,
            None => {
                eprintln!("select image: internal index error");
                return 1;
            }
        }
    } else {
        match opt(args, "--variant") {
            Some(vpick) => {
                let labels: Vec<String> =
                    variants.iter().map(|e| variant_label(e)).collect();
                let mut dedup: Vec<String> = Vec::new();
                for l in &labels {
                    if !dedup.contains(l) {
                        dedup.push(l.clone());
                    }
                }
                let want = match resolve_list_choice(&dedup, &vpick) {
                    Ok(w) => w,
                    Err(e) => {
                        eprintln!("select image: {e}");
                        return 1;
                    }
                };
                let mut found: Option<&gsi_registry::RomEntry> = None;
                for e in &variants {
                    if variant_label(e) == want {
                        found = Some(*e);
                        break;
                    }
                }
                match found {
                    Some(e) => e,
                    None => {
                        eprintln!("select image: variant not found");
                        return 1;
                    }
                }
            }
            None => {
                let mut s = String::from("Variants:\n");
                for (i, e) in variants.iter().enumerate() {
                    s.push_str(&format!(" [{}] {}\n", i + 1, variant_label(e)));
                }
                print!("{s}");
                eprintln!("re-run with --variant <n|label>");
                return 2;
            }
        }
    };
    let cfg = gsi_registry::target_config(entry, &fw_base);
    print!("{}", render_target_config(&cfg));
    0
}

fn select_device_cmd(args: &[String]) -> i32 {
    use std::time::Duration;
    let mut adb_serials: Vec<String> = Vec::new();
    let mut fb_serials: Vec<String> = Vec::new();
    if let Some(adb) = find_managed("adb") {
        let r = gsi_tool::run(&adb, &["devices"], Duration::from_secs(20));
        for d in gsi_device::parse::adb_devices(&r.stdout.lines().collect::<Vec<_>>()) {
            if d.state == "device" {
                adb_serials.push(d.serial);
            }
        }
    }
    if let Some(fb) = find_managed("fastboot") {
        let r = gsi_tool::run(&fb, &["devices"], Duration::from_secs(20));
        for d in gsi_device::parse::fastboot_devices(&r.stdout.lines().collect::<Vec<_>>()) {
            fb_serials.push(d.serial);
        }
    }
    let choices = render_device_choices(&adb_serials, &fb_serials);
    if choices.is_empty() {
        println!("no devices (or no tools)");
        return 1;
    }
    if choices.len() == 1 {
        match choices.get(0) {
            Some(c) => {
                println!("only one target, no selection needed: {c}");
                0
            }
            None => {
                eprintln!("select device: internal error");
                1
            }
        }
    } else {
        match opt(args, "--pick") {
            Some(pick) => match resolve_device_serial(&choices, &pick) {
                Ok(serial) => {
                    println!("plan: use ANDROID_SERIAL={serial} (not set here; confirm then export in your shell)");
                    println!("note: {}", refuse_env_mutation("ANDROID_SERIAL"));
                    0
                }
                Err(e) => {
                    eprintln!("select device: {e}");
                    1
                }
            },
            None => {
                for (i, c) in choices.iter().enumerate() {
                    println!(" [{}] {c}", i + 1);
                }
                eprintln!("multiple devices: re-run with --pick <n|serial> (plan only, env not set here)");
                2
            }
        }
    }
}

fn select_root_target_cmd(args: &[String]) -> i32 {
    let current = current_rom_from_args(args);
    let label = if current.is_empty() {
        "(unknown - analyze first)".to_string()
    } else {
        installed_rom_label(&current)
    };
    match opt(args, "--pick") {
        Some(pick) => match resolve_root_target_detail(pick.as_str()) {
            Ok(detail) => {
                if detail.starts_with("target kept") {
                    if current.is_empty() {
                        eprintln!("select root-target: no current system known (pass --current or --installed-file)");
                        return 1;
                    }
                    println!("target kept: {label} -> keep system, root only");
                    0
                } else {
                    print!("{detail}");
                    0
                }
            }
            Err(e) => {
                eprintln!("select root-target: {e}");
                1
            }
        },
        None => {
            print!("{}", render_root_target_menu(&label));
            eprintln!("re-run with --pick <1|2|3>");
            2
        }
    }
}

fn goal_show_cmd(args: &[String]) -> i32 {
    let goal = match opt(args, "--goal") {
        Some(g) if !g.trim().is_empty() => g,
        _ => match args.first() {
            Some(a) if a == "show" => match args.get(1) {
                Some(g) => g.clone(),
                None => String::new(),
            },
            Some(a) => a.clone(),
            None => String::new(),
        },
    };
    let g = goal.trim();
    if g.is_empty() || g == "show" {
        eprintln!("goal show needs <goal> (try: root, custom_rom, stock_rom, ...)");
        return 2;
    }
    match render_goal_plan(g) {
        Ok(s) => {
            print!("{s}");
            0
        }
        Err(e) => {
            eprintln!("goal show: {e}");
            1
        }
    }
}

fn goal_screen_cmd(args: &[String]) -> i32 {
    let step = match opt(args, "--step") {
        Some(s) => s,
        _ => match args.get(1) {
            Some(s) => s.clone(),
            None => String::new(),
        },
    };
    if step.trim().is_empty() || step.trim() == "screen" {
        eprintln!("goal screen needs <step>");
        return 2;
    }
    println!("{}", goal_screen_target(&step));
    0
}

fn progress_demo_cmd() -> i32 {
    let evs = vec![
        gsi_workflow::ProgressEvent::Started {
            operation: "download firmware.zip".to_string(),
        },
        gsi_workflow::ProgressEvent::Progress {
            current: 42,
            total: 100,
        },
        gsi_workflow::ProgressEvent::Message {
            message: "writing firmware.zip".to_string(),
        },
        gsi_workflow::ProgressEvent::Warning {
            message: "slow mirror".to_string(),
        },
        gsi_workflow::ProgressEvent::Completed,
    ];
    println!("{}", render_progress_events(&evs));
    0
}

fn verdict_flash_cmd(args: &[String]) -> i32 {
    let what = match opt(args, "--what") {
        Some(w) => w,
        None => "flash".to_string(),
    };
    let lines: Vec<String> = match opt(args, "--file") {
        Some(f) if !f.trim().is_empty() => match std::fs::read_to_string(&f) {
            Ok(t) => t.lines().map(|l| l.to_string()).collect(),
            Err(e) => {
                eprintln!("verdict flash: read failed: {e}");
                return 1;
            }
        },
        _ => match std::io::read_to_string(std::io::stdin()) {
            Ok(t) => t.lines().map(|l| l.to_string()).collect(),
            Err(e) => {
                eprintln!("verdict flash: stdin failed: {e}");
                return 1;
            }
        },
    };
    let refs: Vec<&str> = lines.iter().map(|s| s.as_str()).collect();
    let v = evaluate_flash_output(&refs);
    print!("{}", render_flash_verdict(&v, &what));
    if v.verdict == "OK" {
        0
    } else if v.verdict == "FAILED" {
        1
    } else {
        2
    }
}

fn verdict_readiness_cmd(args: &[String]) -> i32 {
    let all = flag(args, "--all-ok");
    let parts = gsi_gates::ReadinessParts {
        model_ok: all || flag(args, "--model-ok"),
        profile_verified: all || flag(args, "--profile-ok"),
        partition_ok: all || flag(args, "--partition-ok"),
        image_exists: all || flag(args, "--image-ok"),
        hash_known: all || flag(args, "--hash-ok"),
        size_ok: all || flag(args, "--size-ok"),
        firmware_ok: all || flag(args, "--firmware-ok"),
        backup_ok: all || flag(args, "--backup-ok"),
        fastboot_ok: all || flag(args, "--fastboot-ok"),
        differs_from_stock: all || flag(args, "--differs-ok"),
    };
    let r = gsi_gates::check_readiness(&parts);
    print!("{}", render_readiness(&r));
    if r.go {
        0
    } else {
        1
    }
}

fn managed_extra_dirs() -> Vec<PathBuf> {
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
    extra
}

fn tools_config_cmd(args: &[String]) -> i32 {
    let cfg = match opt(args, "--config") {
        Some(p) if !p.trim().is_empty() => match load_tool_config(std::path::Path::new(&p)) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("tools config: {e}");
                return 1;
            }
        },
        _ => ToolConfig::default(),
    };
    let mut extra = managed_extra_dirs();
    if let Some(d) = opt(args, "--extra-dir") {
        if !d.trim().is_empty() {
            extra.push(PathBuf::from(d));
        }
    }
    let adb = resolve_tool_with_config(&cfg.adb, "adb", &extra);
    let fb = resolve_tool_with_config(&cfg.fastboot, "fastboot", &extra);
    match adb {
        Some(p) => println!("adb: {}", p.display()),
        None => println!("adb: MISSING (native protocol: Phase 8)"),
    }
    match fb {
        Some(p) => println!("fastboot: {}", p.display()),
        None => println!("fastboot: MISSING (native protocol: Phase 8)"),
    }
    if cfg.scrcpy.trim().is_empty() {
        println!("scrcpy: (optional, not configured)");
    } else {
        println!("scrcpy(config): {}", cfg.scrcpy.trim());
    }
    0
}

fn tools_save_config_cmd(args: &[String]) -> i32 {
    let out = match opt(args, "--out") {
        Some(o) if !o.trim().is_empty() => o,
        _ => {
            eprintln!("tools save-config needs --out <path>");
            return 2;
        }
    };
    let adb = match opt(args, "--adb") {
        Some(v) => v,
        None => String::new(),
    };
    let fastboot = match opt(args, "--fastboot") {
        Some(v) => v,
        None => String::new(),
    };
    let scrcpy = match opt(args, "--scrcpy") {
        Some(v) => v,
        None => String::new(),
    };
    let cfg = ToolConfig {
        adb,
        fastboot,
        scrcpy,
    };
    match save_tool_config(std::path::Path::new(&out), &cfg) {
        Ok(()) => {
            println!("saved: {out}");
            0
        }
        Err(e) => {
            eprintln!("tools save-config: {e}");
            1
        }
    }
}

fn tools_path_plan_cmd(args: &[String]) -> i32 {
    let dir = match opt(args, "--dir") {
        Some(d) if !d.trim().is_empty() => d,
        _ => {
            eprintln!("tools path-plan needs --dir <dir>");
            return 2;
        }
    };
    print!("{}", render_path_plan(&plan_add_to_path(&dir)));
    0
}

fn tools_link_plan_cmd(args: &[String]) -> i32 {
    let src = match opt(args, "--src") {
        Some(s) if !s.trim().is_empty() => s,
        _ => {
            eprintln!("tools link-plan needs --src <dir> --central <dir>");
            return 2;
        }
    };
    let central = match opt(args, "--central") {
        Some(c) if !c.trim().is_empty() => c,
        _ => {
            eprintln!("tools link-plan needs --src <dir> --central <dir>");
            return 2;
        }
    };
    for p in plan_link_into_tools(
        std::path::Path::new(&src),
        std::path::Path::new(&central),
    ) {
        print!("{}", render_link_plan(&p));
    }
    0
}

fn tools_install_plan_cmd(args: &[String]) -> i32 {
    let reg = match registry_path_from_args(args) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("tools install-plan: {e}");
            return 2;
        }
    };
    let os = match opt(args, "--os") {
        Some(o) if !o.trim().is_empty() => o,
        _ => std::env::consts::OS.to_string(),
    };
    let dest = match opt(args, "--dest") {
        Some(d) if !d.trim().is_empty() => d,
        _ => {
            eprintln!("tools install-plan needs --dest <dir>");
            return 2;
        }
    };
    let tools = match gsi_registry::tools_block(&reg) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("tools install-plan: {e}");
            return 1;
        }
    };
    match platform_tools_install_plan(&tools, &os, std::path::Path::new(&dest)) {
        Ok(p) => {
            print!("{}", render_install_plan(&p));
            0
        }
        Err(e) => {
            eprintln!("tools install-plan: {e}");
            1
        }
    }
}

fn config_tool_root_cmd(args: &[String]) -> i32 {
    let dir = match opt(args, "--script-dir") {
        Some(d) if !d.trim().is_empty() => d,
        _ => {
            eprintln!("config tool-root needs --script-dir <path>");
            return 2;
        }
    };
    let root = tool_root_from_script_dir(std::path::Path::new(&dir));
    println!("{}", root.display());
    0
}

fn main() {
    let raw: Vec<String> = std::env::args().skip(1).collect();
    let cmd = raw.first().cloned().unwrap_or_default();
    let args = if raw.is_empty() {
        vec![]
    } else {
        raw[1..].to_vec()
    };
    let code = match cmd.as_str() {
        "" | "gui" => gsi_root_gui::run(),
        "analyze" | "inspect" => match args.first() {
            Some(p) => analyze(p),
            None => usage(),
        },
        "patch" | "verify" => experimental(&cmd),
        "version" => {
            println!("gsi-root {}", env!("CARGO_PKG_VERSION"));
            0
        }
        "workflow" => match args.first().map(String::as_str) {
            Some("list") => workflow_list(),
            Some("run") => {
                let id = args.get(1).cloned().unwrap_or_default();
                workflow_run(&id, flag(&args, "--yes"))
            }
            _ => usage(),
        },
        "update" => match args.first().map(String::as_str) {
            Some("check") => match opt(&args, "--repo") {
                Some(r) => update_check(&r),
                None => {
                    eprintln!("update check needs --repo <owner/name>");
                    2
                }
            },
            Some("download") => {
                let repo = opt(&args, "--repo").unwrap_or_default();
                let asset = opt(&args, "--asset").unwrap_or_default();
                let out = opt(&args, "--out").unwrap_or_default();
                let sha = opt(&args, "--sha256").unwrap_or_default();
                if repo.is_empty() || asset.is_empty() || out.is_empty() {
                    eprintln!("needs --repo, --asset, --out (and --sha256 to verify)");
                    2
                } else {
                    update_download(&repo, &asset, &out, &sha)
                }
            }
            Some("install") => {
                let staged = opt(&args, "--staged").unwrap_or_default();
                let target = opt(&args, "--target").unwrap_or_default();
                if staged.is_empty() || target.is_empty() {
                    eprintln!("needs --staged <file> --target <path>");
                    2
                } else {
                    update_install(&staged, &target)
                }
            }
            Some("rollback") => match opt(&args, "--target") {
                Some(t) => update_rollback(&t),
                None => {
                    eprintln!("needs --target <path>");
                    2
                }
            },
            _ => usage(),
        },
        "config" => match args.first().map(String::as_str) {
            Some("show") => config_show(),
            Some("tool-root") => config_tool_root_cmd(&args),
            _ => usage(),
        },
        "select" => match args.first().map(String::as_str) {
            Some("rom") => select_rom_cmd(&args),
            Some("image") => select_image_cmd(&args),
            Some("device") => select_device_cmd(&args),
            Some("root-target") => select_root_target_cmd(&args),
            _ => usage(),
        },
        "goal" => match args.first().map(String::as_str) {
            Some("show") => goal_show_cmd(&args),
            Some("screen") => goal_screen_cmd(&args),
            _ => usage(),
        },
        "progress" => match args.first().map(String::as_str) {
            Some("demo") => progress_demo_cmd(),
            None => progress_demo_cmd(),
            _ => usage(),
        },
        "verdict" => match args.first().map(String::as_str) {
            Some("flash") => verdict_flash_cmd(&args),
            Some("readiness") => verdict_readiness_cmd(&args),
            _ => usage(),
        },
        "preflight" => preflight_cmd(&args),
        "firmware-baseline" => {
            let d = opt(&args, "--display").unwrap_or_default();
            let i = opt(&args, "--incremental").unwrap_or_default();
            let o = opt(&args, "--baseline").unwrap_or_default();
            print!("{}", render_firmware_baseline(&d, &i, &o));
            0
        }
        "install" => self_install(flag(&args, "--yes")),
        "adb" => match args.first().map(String::as_str) {
            Some("devices") => match find_managed("adb") {
                Some(a) => {
                    use std::time::Duration;
                    let r = gsi_tool::run(&a, &["devices"], Duration::from_secs(20));
                    println!("{}", r.stdout.trim_end());
                    if r.success {
                        0
                    } else {
                        1
                    }
                }
                None => {
                    println!("adb: not found");
                    1
                }
            },
            _ => usage(),
        },
        "fastboot" => match args.first().map(String::as_str) {
            Some("devices") => match find_managed("fastboot") {
                Some(f) => {
                    use std::time::Duration;
                    let r = gsi_tool::run(&f, &["devices"], Duration::from_secs(20));
                    println!("{}", r.stdout.trim_end());
                    if r.success {
                        0
                    } else {
                        1
                    }
                }
                None => {
                    println!("fastboot: not found");
                    1
                }
            },
            Some("getvar") => match find_managed("fastboot") {
                Some(f) => {
                    use std::time::Duration;
                    let what = args.get(1).cloned().unwrap_or_else(|| "all".to_string());
                    let r = gsi_tool::run(&f, &["getvar", &what], Duration::from_secs(30));
                    println!("{}", r.stdout.trim_end());
                    let g = gsi_device::parse::getvar(
                        &r.stdout.lines().collect::<Vec<_>>(),
                    );
                    if g.command_denied {
                        println!("note: command denied is a Huawei quirk, not a lock proof");
                    }
                    if r.success {
                        0
                    } else {
                        1
                    }
                }
                None => {
                    println!("fastboot: not found");
                    1
                }
            },
            _ => usage(),
        },
        "tools" => match args.first().map(String::as_str) {
            None => tools_status(),
            Some("config") => tools_config_cmd(&args),
            Some("save-config") => tools_save_config_cmd(&args),
            Some("path-plan") => tools_path_plan_cmd(&args),
            Some("link-plan") => tools_link_plan_cmd(&args),
            Some("install-plan") => tools_install_plan_cmd(&args),
            _ => usage(),
        },
        "device" => match args.first().map(String::as_str) {
            Some("slot") => {
                let f = opt(&args, "--state-file")
                    .unwrap_or_else(|| "workflow-state.json".to_string());
                let r = gsi_device::read_slot(std::path::Path::new(&f));
                println!("slot: {} ({})", r.occupant, r.detail);
                0
            }
            Some("detect") => device_detect(),
            Some("info") => device_info(),
            Some("switch") => {
                let to = gsi_device::SlotOccupant::parse(
                    &opt(&args, "--to").unwrap_or_default(),
                );
                let f = opt(&args, "--state-file").unwrap_or_default();
                let cur = if f.is_empty() {
                    gsi_device::SlotOccupant::Unknown
                } else {
                    gsi_device::SlotOccupant::parse(
                        &gsi_device::read_slot(std::path::Path::new(&f)).occupant,
                    )
                };
                let plan = gsi_device::plan_switch(
                    cur,
                    to,
                    opt(&args, "--magisk").as_deref(),
                    opt(&args, "--twrp").as_deref(),
                );
                for w in &plan.warnings {
                    println!("warn: {w}");
                }
                match &plan.action {
                    gsi_device::SwitchAction::AlreadyThere => {
                        println!("already there: {}", to.as_str());
                        0
                    }
                    gsi_device::SwitchAction::Flash { image } => {
                        println!("plan: flash {image} (execution needs native fastboot, Phase 8)");
                        3
                    }
                    gsi_device::SwitchAction::NeedInput(what) => {
                        println!("need input: {what}");
                        2
                    }
                }
            }
            _ => usage(),
        },
        _ => usage(),
    };
    std::process::exit(code);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_entries() -> Vec<gsi_registry::RomEntry> {
        let v: serde_json::Value = serde_json::json!([
            {"name": "LineageOS", "version": 20, "android": 13, "status": "working",
             "gsi": "arm64_bgN", "file": "l20.img.gz", "url": "https://x/l20.img.gz",
             "root_artifact": {"type": "recovery_ramdisk", "source": "stock_firmware"}},
            {"name": "LineageOS", "version": 20, "android": 13, "variant": "vndklite",
             "build": 20251021, "status": "working", "gsi": "arm64_bgN",
             "root_artifact": {"type": "recovery_ramdisk", "source": "stock_firmware"}},
            {"name": "AOSP", "android": "12", "status": "working"},
            {"name": "LineageOS 20 Light", "version": 20, "status": "broken",
             "reason": "non-booting", "markers": ["light"]}
        ]);
        v.as_array()
            .unwrap()
            .iter()
            .map(|r| serde_json::from_value(r.clone()).unwrap())
            .collect()
    }

    fn tools_fixture() -> gsi_registry::ToolsBlock {
        gsi_registry::parse_tools_block(&serde_json::json!({
            "tools": {
                "platform_tools": {
                    "source": "direct-official",
                    "url_pattern": "https://dl.google.com/android/repository/platform-tools-latest-{os}.zip",
                    "provides": ["adb", "fastboot"]
                },
                "extractors": [
                    {"name": "HuaweiFirmwareExtractor",
                     "file": "huawei_firmware_extractor.py",
                     "url": "https://raw.githubusercontent.com/Natsume324/HuaweiFirmwareExtractor/main/huawei_firmware_extractor.py"}
                ]
            }
        }))
    }

    fn tmp_dir(tag: &str) -> PathBuf {
        static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let n = N.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let mut p = std::env::temp_dir();
        p.push(format!("gsi-cli-test-{tag}-{}-{n}", std::process::id()));
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn selection_numbers() {
        assert_eq!(parse_selection("1", 3).unwrap(), 0);
        assert_eq!(parse_selection(" 3 ", 3).unwrap(), 2);
        assert!(parse_selection("", 3).is_err());
        assert!(parse_selection("x", 3).is_err());
        assert!(parse_selection("0", 3).is_err());
        assert!(parse_selection("4", 3).is_err());
        assert!(parse_selection("1", 0).is_err());
    }

    #[test]
    fn list_choice_number_and_label() {
        let opts = vec!["a".to_string(), "b".to_string()];
        assert_eq!(resolve_list_choice(&opts, "1").unwrap(), "a");
        assert_eq!(resolve_list_choice(&opts, "b").unwrap(), "b");
        assert!(resolve_list_choice(&opts, "").is_err());
        assert!(resolve_list_choice(&opts, "z").is_err());
        assert!(resolve_list_choice(&[], "1").is_err());
    }

    #[test]
    fn rom_options_and_resolve() {
        let opts = cli_rom_options(&sample_entries());
        assert_eq!(opts.first().unwrap().0, "stock");
        assert_eq!(opts.last().unwrap().0, "other");
        assert!(opts.iter().all(|(id, _)| !id.contains("Light")));
        assert_eq!(resolve_rom_choice(&opts, "1").unwrap(), "stock");
        assert_eq!(
            resolve_rom_choice(&opts, "Other custom ROM (not in list)").unwrap(),
            "other"
        );
        let id = resolve_rom_choice(&opts, "2").unwrap();
        assert!(id.starts_with("rom:"));
        assert_eq!(resolve_rom_choice(&opts, &id).unwrap(), id);
        assert!(resolve_rom_choice(&opts, "nope").is_err());
        let menu = render_rom_menu(&opts, "stock");
        assert!(menu.contains("Which system is on your phone"));
        assert!(menu.contains("current, Enter keeps it"));
        assert_eq!(installed_rom_label("stock"), "Stock EMUI");
        assert_eq!(installed_rom_label("rom:LineageOS 20"), "LineageOS 20");
    }

    #[test]
    fn android_menus_and_resolve() {
        let e = sample_entries();
        let menu = render_android_menu(&e);
        assert!(menu.contains("Android 12"));
        assert!(menu.contains("Android 13"));
        assert_eq!(resolve_android_choice(&e, "1").unwrap(), 12);
        assert_eq!(resolve_android_choice(&e, "2").unwrap(), 13);
        assert_eq!(resolve_android_choice(&e, "13").unwrap(), 13);
        assert_eq!(resolve_android_choice(&e, "Android 12").unwrap(), 12);
        assert!(resolve_android_choice(&e, "99").is_err());
        assert!(resolve_android_choice(&[], "1").is_err());
        let systems = gsi_registry::systems_for(&e, 13);
        assert!(!systems.is_empty());
        let sys_menu = render_system_menu(&systems);
        assert!(sys_menu.contains("Systems:"));
        assert_eq!(resolve_system_choice(&systems, "1").unwrap(), systems[0]);
    }

    #[test]
    fn target_config_renders() {
        let e = sample_entries();
        let cfg = gsi_registry::target_config(&e[0], "EMUI 9.1");
        let s = render_target_config(&cfg);
        assert!(s.contains("Android  : 13"));
        assert!(s.contains("System   : LineageOS 20"));
        assert!(s.contains("Base     : EMUI 9.1"));
        assert!(s.contains("Download : https://x/l20.img.gz"));
        let cfg2 = gsi_registry::target_config(&e[2], "EMUI 9.1");
        let s2 = render_target_config(&cfg2);
        assert!(s2.contains("no verified direct link"));
    }

    #[test]
    fn variant_labels() {
        let e = sample_entries();
        assert_eq!(variant_label(&e[1]), "vndklite");
        assert_eq!(variant_label(&e[0]), "arm64_bgN");
    }

    #[test]
    fn device_choices_and_resolve() {
        let c = render_device_choices(&["ABC123".to_string()], &["XYZ999".to_string()]);
        assert_eq!(c, vec!["ABC123 (adb)".to_string(), "XYZ999 (fastboot)".to_string()]);
        assert_eq!(resolve_device_serial(&c, "1").unwrap(), "ABC123");
        assert_eq!(resolve_device_serial(&c, "XYZ999").unwrap(), "XYZ999");
        assert_eq!(
            resolve_device_serial(&c, "XYZ999 (fastboot)").unwrap(),
            "XYZ999"
        );
        assert!(resolve_device_serial(&c, "nope").is_err());
        assert!(resolve_device_serial(&[], "1").is_err());
    }

    #[test]
    fn root_target_menu_and_resolve() {
        let m = render_root_target_menu("Stock EMUI");
        assert!(m.contains("Root target"));
        assert!(m.contains("[1]"));
        assert_eq!(resolve_root_target_choice("1").unwrap(), "current");
        assert_eq!(resolve_root_target_choice("stock").unwrap(), "stock");
        assert_eq!(resolve_root_target_choice("3").unwrap(), "other");
        assert!(resolve_root_target_choice("9").is_err());
    }

    #[test]
    fn goal_screen_dispatch() {
        assert_eq!(goal_screen_target("reconnaissance"), "Screen-Analyze");
        assert_eq!(goal_screen_target("compatibility"), "Screen-Compatibility");
        assert_eq!(goal_screen_target("firmware"), "Screen-Firmware");
        assert_eq!(goal_screen_target("firmware_selection"), "Screen-Firmware");
        assert_eq!(goal_screen_target("extract"), "Screen-Extract");
        assert_eq!(goal_screen_target("magisk_patch"), "Screen-Patch");
        assert_eq!(goal_screen_target("backup"), "Screen-Backup");
        assert_eq!(goal_screen_target("flash"), "Screen-Flash");
        assert_eq!(goal_screen_target("flash_system"), "Screen-FlashSystem");
        assert_eq!(goal_screen_target("recovery_compatibility"), "Screen-RootMethods");
        assert_eq!(goal_screen_target("recovery_flash"), "Screen-Twrp");
        assert_eq!(goal_screen_target("root_verify"), "Screen-RebootVerify");
        assert_eq!(goal_screen_target("validate"), "Screen-RebootVerify");
        assert_eq!(goal_screen_target("restore"), "Screen-Restore");
        assert_eq!(goal_screen_target("wipe"), "Screen-Wipe");
        assert_eq!(goal_screen_target("nope"), "Manual-Step");
    }

    #[test]
    fn goal_plan_golden() {
        let s = render_goal_plan("root").unwrap();
        assert!(s.starts_with("Plan for root:\n"));
        assert!(s.contains(" - reconnaissance -> Screen-Analyze"));
        assert!(s.contains(" - validate -> Screen-RebootVerify"));
        assert!(render_goal_plan("nope").is_err());
    }

    #[test]
    fn progress_renderer_golden() {
        let evs = vec![
            gsi_workflow::ProgressEvent::Started {
                operation: "download a".to_string(),
            },
            gsi_workflow::ProgressEvent::Progress {
                current: 1,
                total: 2,
            },
            gsi_workflow::ProgressEvent::Message {
                message: "m".to_string(),
            },
            gsi_workflow::ProgressEvent::Warning {
                message: "w".to_string(),
            },
            gsi_workflow::ProgressEvent::Completed,
        ];
        assert_eq!(render_progress_event(&evs[0]), "> download a");
        assert_eq!(render_progress_event(&evs[1]), "  1/2");
        assert_eq!(render_progress_event(&evs[4]), "  done");
        let all = render_progress_events(&evs);
        assert_eq!(all, "> download a\n  1/2\n  m\n  WARN w\n  done");
    }

    #[test]
    fn flash_verdict_matrix() {
        // first-run flow text (guided, no execution).
        assert!(first_run_text(true, true).contains("tools present"));
        let fr = first_run_text(false, false);
        assert!(fr.contains("FIRST RUN"));
        assert!(fr.contains("config.json"));
        // firmware-baseline + preflight renders (dedicated CLI coverage).
        let b = render_firmware_baseline("9.1.0.297(C432E5R1P9)", "", "");
        assert!(b.contains("firmware baseline:"));
        assert!(b.contains("9.1.0.297"));
        let go = gsi_gates::preflight(&gsi_gates::PreflightInput {
            adb_path: "/usr/bin/adb".to_string(),
            fastboot_path: "/usr/bin/fastboot".to_string(),
            shell_ok: true,
            writables: vec![("workdir".to_string(), true)],
            files: vec![("x.img".to_string(), true)],
            hashes: vec![("x.img".to_string(), "ab".to_string(), "ab".to_string())],
        });
        assert!(go.go);
        assert!(render_preflight_report(&go).contains("PREFLIGHT: GO"));
        let blocked = gsi_gates::preflight(&gsi_gates::PreflightInput {
            adb_path: String::new(),
            fastboot_path: String::new(),
            shell_ok: false,
            writables: vec![],
            files: vec![],
            hashes: vec![],
        });
        assert!(!blocked.go);
        let bs = render_preflight_report(&blocked);
        assert!(bs.contains("PREFLIGHT: BLOCKED"));
        assert!(bs.contains("adb missing"));
        let ok = evaluate_flash_output(&["Sending 'x' (1 KB)", "OKAY [ 0.1s]", "Finished. Total time: 0.2s"]);
        assert_eq!(ok.verdict, "OK");
        assert_eq!(ok.okay, 1);
        let bad = evaluate_flash_output(&["OKAY [ 0.1s]", "FAILED (remote: denied)", "Finished. Total time: 0.1s"]);
        assert_eq!(bad.verdict, "FAILED");
        assert_eq!(bad.failed.len(), 1);
        let unclear = evaluate_flash_output(&["Writing 'x'"]);
        assert_eq!(unclear.verdict, "UNCLEAR");
        let s_ok = render_flash_verdict(&ok, "flash recovery_ramdisk");
        assert!(s_ok.starts_with("FLASH RESULT: OK (1 OKAY"));
        let s_bad = render_flash_verdict(&bad, "flash");
        assert!(s_bad.contains("FLASH RESULT: FAILED"));
        assert!(s_bad.contains("Hints:"));
        let s_unc = render_flash_verdict(&unclear, "");
        assert!(s_unc.contains("UNCLEAR"));
        let parts = gsi_gates::ReadinessParts {
            model_ok: true,
            profile_verified: true,
            partition_ok: true,
            image_exists: true,
            hash_known: true,
            size_ok: true,
            firmware_ok: true,
            backup_ok: true,
            fastboot_ok: true,
            differs_from_stock: true,
        };
        let r = gsi_gates::check_readiness(&parts);
        let rs = render_readiness(&r);
        assert!(rs.contains("READY: all checks pass."));
        let mut one = parts.clone();
        one.backup_ok = false;
        let r2 = gsi_gates::check_readiness(&one);
        assert!(render_readiness(&r2).contains("BLOCKED"));
    }

    #[test]
    fn tool_config_roundtrip() {
        assert_eq!(
            parse_tool_config("").unwrap(),
            ToolConfig::default()
        );
        assert!(parse_tool_config("{oops").is_err());
        let c = parse_tool_config(r#"{"adb":"/a/adb","fastboot":"/a/fb"}"#).unwrap();
        assert_eq!(c.adb, "/a/adb");
        assert_eq!(c.scrcpy, "");
        let d = tmp_dir("cfg");
        let f = d.join("config.json");
        assert_eq!(load_tool_config(&f).unwrap(), ToolConfig::default());
        save_tool_config(&f, &c).unwrap();
        let back = load_tool_config(&f).unwrap();
        assert_eq!(back, c);
        std::fs::write(&f, "{bad").unwrap();
        assert!(load_tool_config(&f).is_err());
    }

    #[test]
    fn resolve_prefers_config_file() {
        let d = tmp_dir("tool");
        let fake = d.join("adb");
        std::fs::write(&fake, "x").unwrap();
        let got = resolve_tool_with_config(fake.to_str().unwrap(), "gsi-test-no-such-tool-xyz", &[]).unwrap();
        assert_eq!(got, fake);
        assert!(resolve_tool_with_config("", "gsi-test-no-such-tool-xyz", &[]).is_none());
        assert!(resolve_tool_with_config("/no/such/file", "gsi-test-no-such-tool-xyz", &[]).is_none());
    }

    #[test]
    fn honest_plans_need_confirm() {
        let p = plan_add_to_path("/tmp/x");
        assert!(p.needs_confirm);
        assert!(render_path_plan(&p).contains("[y/N]"));
        assert!(refuse_env_mutation("add_to_path").contains("never mutates env"));
        let links = plan_link_into_tools(
            std::path::Path::new("/src"),
            std::path::Path::new("/central"),
        );
        assert_eq!(links.len(), 2);
        assert!(render_link_plan(&links[0]).contains("symlink"));
        let t = tools_fixture();
        let plan = platform_tools_install_plan(&t, "linux", std::path::Path::new("/tmp/tools")).unwrap();
        assert!(plan.url.contains("linux"));
        assert!(plan.needs_confirm);
        assert!(render_install_plan(&plan).contains("needs confirm"));
        assert_eq!(
            gsi_registry::platform_tools_url(&t, "linux").unwrap(),
            plan.url
        );
        assert_eq!(
            gsi_registry::firmware_extractor_url(&t).unwrap(),
            "https://raw.githubusercontent.com/Natsume324/HuaweiFirmwareExtractor/main/huawei_firmware_extractor.py"
        );
    }

    #[test]
    fn tool_root_shapes() {
        assert_eq!(
            tool_root_from_script_dir(std::path::Path::new("/r/scripts")),
            PathBuf::from("/r")
        );
        assert_eq!(
            tool_root_from_script_dir(std::path::Path::new("/r")),
            PathBuf::from("/r")
        );
    }

    #[test]
    fn root_target_detail_stock_subchoice() {
        let menu = render_stock_version_menu();
        assert!(menu.contains("Android 8 / EMUI 8"));
        assert!(menu.contains("Android 9 / EMUI 9.0"));
        assert!(menu.contains("Android 9 / EMUI 9.1"));
        assert_eq!(
            resolve_stock_version_choice("1").unwrap(),
            "EMUI 8 (Android 8)"
        );
        assert_eq!(
            resolve_stock_version_choice("stock:2").unwrap(),
            "EMUI 9.0 (Android 9)"
        );
        assert_eq!(
            resolve_stock_version_choice("9.1").unwrap(),
            "EMUI 9.1 (Android 9)"
        );
        assert_eq!(
            resolve_stock_version_choice("EMUI 8").unwrap(),
            "EMUI 8 (Android 8)"
        );
        assert!(resolve_stock_version_choice("9").is_err());
        assert!(resolve_stock_version_choice("").is_err());
        let keep = resolve_root_target_detail("1").unwrap();
        assert!(keep.contains("target kept"));
        let stock_menu = resolve_root_target_detail("stock").unwrap();
        assert!(stock_menu.contains("Which Stock EMUI version?"));
        let s91 = resolve_root_target_detail("stock:3").unwrap();
        assert!(s91.contains("EMUI 9.1"));
        assert!(s91.contains("recovery_ramdisk"));
        let s80 = resolve_root_target_detail("emui 8").unwrap();
        assert!(s80.contains("EMUI 8"));
        let other = resolve_root_target_detail("other").unwrap();
        assert!(other.contains("select image"));
        assert!(resolve_root_target_detail("9").is_err());
        assert!(resolve_root_target_detail("nope").is_err());
    }

    fn raw_tar_cli_bytes(name: &str, data: &[u8]) -> Vec<u8> {
        let mut hdr = [0u8; 512];
        let nb = name.as_bytes();
        let n = if nb.len() < 100 { nb.len() } else { 100 };
        hdr[..n].copy_from_slice(&nb[..n]);
        hdr[100..108].copy_from_slice(b"0000777\0");
        hdr[108..116].copy_from_slice(b"0000000\0");
        hdr[116..124].copy_from_slice(b"0000000\0");
        let size = format!("{:011o}\0", data.len());
        hdr[124..136].copy_from_slice(size.as_bytes());
        hdr[136..148].copy_from_slice(b"00000000000\0");
        hdr[156] = b'0';
        hdr[257..262].copy_from_slice(b"ustar");
        let mut sum: u32 = 8 * 0x20;
        for (i, b) in hdr.iter().enumerate() {
            if !(148..156).contains(&i) {
                sum += *b as u32;
            }
        }
        let chk = format!("{:06o}\0 ", sum);
        hdr[148..156].copy_from_slice(chk.as_bytes());
        let mut out = Vec::from(&hdr[..]);
        out.extend_from_slice(data);
        let pad = (512 - data.len() % 512) % 512;
        out.extend(std::iter::repeat(0u8).take(pad));
        out.extend_from_slice(&[0u8; 1024]);
        out
    }

    fn le16_cli(v: u16) -> [u8; 2] {
        v.to_le_bytes()
    }

    fn le32_cli(v: u32) -> [u8; 4] {
        v.to_le_bytes()
    }

    fn crc32_cli(data: &[u8]) -> u32 {
        let mut crc: u32 = 0xFFFF_FFFF;
        for &b in data {
            crc ^= b as u32;
            for _ in 0..8 {
                if crc & 1 != 0 {
                    crc = (crc >> 1) ^ 0xEDB8_8320;
                } else {
                    crc >>= 1;
                }
            }
        }
        !crc
    }

    fn raw_zip_cli_bytes(name: &str, data: &[u8]) -> Vec<u8> {
        let nb = name.as_bytes();
        let crc = crc32_cli(data);
        let mut out = Vec::new();
        out.extend_from_slice(&[0x50, 0x4b, 0x03, 0x04]);
        out.extend_from_slice(&[0x14, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);
        out.extend_from_slice(&crc.to_le_bytes());
        out.extend_from_slice(&le32_cli(data.len() as u32));
        out.extend_from_slice(&le32_cli(data.len() as u32));
        out.extend_from_slice(&le16_cli(nb.len() as u16));
        out.extend_from_slice(&[0x00, 0x00]);
        out.extend_from_slice(nb);
        out.extend_from_slice(data);
        let cd_start = out.len() as u32;
        let cdh_start = out.len();
        out.extend_from_slice(&[0x50, 0x4b, 0x01, 0x02]);
        out.extend_from_slice(&[
            0x14, 0x00, 0x14, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ]);
        out.extend_from_slice(&crc.to_le_bytes());
        out.extend_from_slice(&le32_cli(data.len() as u32));
        out.extend_from_slice(&le32_cli(data.len() as u32));
        out.extend_from_slice(&le16_cli(nb.len() as u16));
        out.extend_from_slice(&[0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);
        out.extend_from_slice(&le32_cli(0));
        out.extend_from_slice(nb);
        let cd_size = (out.len() - cdh_start) as u32;
        out.extend_from_slice(&[0x50, 0x4b, 0x05, 0x06]);
        out.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]);
        out.extend_from_slice(&le16_cli(1));
        out.extend_from_slice(&le16_cli(1));
        out.extend_from_slice(&le32_cli(cd_size));
        out.extend_from_slice(&le32_cli(cd_start));
        out.extend_from_slice(&[0x00, 0x00]);
        out
    }

    #[test]
    fn archive_cli_wrapper_dispatch() {
        let d = tmp_dir("archive-cli");
        // tar success.
        let tar_bytes = raw_tar_cli_bytes("boot.img", b"ANDROID!fake");
        let tar_path = d.join("rom.tar");
        std::fs::write(&tar_path, &tar_bytes).unwrap();
        let out = extract_archive_cli(&tar_path, &d.join("out-tar")).unwrap();
        assert_eq!(out.len(), 1);
        assert!(d.join("out-tar").join("boot.img").is_file());
        // zip success.
        let zip_bytes = raw_zip_cli_bytes("platform-tools/adb", b"fake-adb");
        let zip_path = d.join("tools.zip");
        std::fs::write(&zip_path, &zip_bytes).unwrap();
        let out2 = extract_archive_cli(&zip_path, &d.join("out-zip")).unwrap();
        assert_eq!(out2.len(), 1);
        assert!(d.join("out-zip").join("platform-tools/adb").is_file());
        // gzip single success (embedded gzip for "hello treble test 123\n").
        let gz: Vec<u8> = vec![
            31, 139, 8, 0, 0, 0, 0, 0, 2, 255, 203, 72, 205, 201, 201, 87, 40, 41, 74, 77,
            202, 73, 85, 40, 73, 45, 46, 81, 48, 52, 50, 230, 2, 0, 33, 102, 123, 183, 22,
            0, 0, 0,
        ];
        let gz_path = d.join("hello.img.gz");
        std::fs::write(&gz_path, &gz).unwrap();
        let out3 = extract_archive_cli(&gz_path, &d.join("out-gz")).unwrap();
        assert_eq!(out3.len(), 1);
        assert_eq!(
            std::fs::read(d.join("out-gz").join("hello.img")).unwrap(),
            b"hello treble test 123\n"
        );
        // xz single success (embedded xz for "hello treble xz 456\n").
        let xz: Vec<u8> = vec![
            253, 55, 122, 88, 90, 0, 0, 4, 230, 214, 180, 70, 2, 0, 33, 1, 22, 0, 0, 0,
            116, 47, 229, 163, 1, 0, 19, 104, 101, 108, 108, 111, 32, 116, 114, 101, 98,
            108, 101, 32, 120, 122, 32, 52, 53, 54, 10, 0, 78, 187, 117, 255, 10, 122, 74,
            139, 0, 1, 44, 20, 248, 10, 109, 3, 31, 182, 243, 125, 1, 0, 0, 0, 0, 4, 89,
            90,
        ];
        let xz_path = d.join("hello.img.xz");
        std::fs::write(&xz_path, &xz).unwrap();
        let out4 = extract_archive_cli(&xz_path, &d.join("out-xz")).unwrap();
        assert_eq!(out4.len(), 1);
        assert_eq!(
            std::fs::read(d.join("out-xz").join("hello.img")).unwrap(),
            b"hello treble xz 456\n"
        );
        // plain refused.
        let plain = d.join("rom.img");
        std::fs::write(&plain, b"raw").unwrap();
        assert!(extract_archive_cli(&plain, &d.join("out-plain")).is_err());
        // missing file refused.
        assert!(extract_archive_cli(&d.join("nope.zip"), &d.join("out-miss")).is_err());
        // empty dest refused.
        assert!(extract_archive_cli(&zip_path, std::path::Path::new("")).is_err());
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn magisk_info_with_sha512() {
        let d = tmp_dir("magisk512");
        let apk = d.join("Magisk-v28.1.apk");
        std::fs::write(&apk, b"fakapk").unwrap();
        let info = magisk_info_detail(&apk).unwrap();
        assert_eq!(info.version, "28.1");
        assert_eq!(info.size, 6);
        assert_eq!(info.sha256, gsi_update::sha256_file(&apk).unwrap());
        assert_eq!(info.sha512.len(), 128);
        assert_eq!(info.sha512, gsi_workflow::sha512_file(&apk).unwrap());
        let text = render_magisk_info(&info);
        assert!(text.contains("Version: 28.1"));
        assert!(text.contains("SHA256: "));
        assert!(text.contains("SHA512: "));
        assert!(text.contains(&info.sha512));
        assert!(magisk_info_detail(&d.join("missing.apk")).is_err());
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn registry_for_magisk_loads() {
        let d = tmp_dir("magreg");
        let reg = d.join("VTR-L29.json");
        let body = serde_json::json!({
            "roms": [
                {"name": "LineageOS", "version": 20, "android": 13, "status": "working",
                 "gsi": "arm64_bgN", "file": "l.img.gz", "url": "https://x/l.img.gz"}
            ],
            "magisk": {"stable_json": "https://example.invalid/x/stable.json"}
        });
        std::fs::write(&reg, serde_json::to_string(&body).unwrap()).unwrap();
        let ctx = load_registry_for_magisk(&d, "VTR-L29").unwrap();
        assert_eq!(ctx.stable_url, "https://example.invalid/x/stable.json");
        assert_eq!(ctx.rom_count, 1);
        assert_eq!(ctx.registry_path, reg);
        // default URL when no override.
        let reg2 = d.join("VTR-L09.json");
        let body2 = serde_json::json!({"roms": []});
        std::fs::write(&reg2, serde_json::to_string(&body2).unwrap()).unwrap();
        let ctx2 = load_registry_for_magisk(&d, "VTR-L09").unwrap();
        assert_eq!(ctx2.stable_url, gsi_update::MAGISK_STABLE_DEFAULT_URL);
        // nested tool-root layout also resolves.
        let nested = d.join("nested");
        let pdir = nested.join("data/compatibility/huawei/p10");
        std::fs::create_dir_all(&pdir).unwrap();
        std::fs::write(pdir.join("VTR-L29.json"), serde_json::to_string(&body).unwrap())
            .unwrap();
        let ctx3 = load_registry_for_magisk(&nested, "VTR-L29").unwrap();
        assert_eq!(ctx3.stable_url, "https://example.invalid/x/stable.json");
        assert!(load_registry_for_magisk(&d, "").is_err());
        assert!(load_registry_for_magisk(&d, "a/b").is_err());
        assert!(load_registry_for_magisk(&d, "MISSING").is_err());
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn tool_plan_guided_text() {
        let adb = ensure_tool_plan("adb");
        assert!(adb.contains("adb missing"));
        assert!(adb.contains("[1] Install into PATH"));
        assert!(adb.contains("[q] Abort"));
        let fb = ensure_tool_plan("fastboot");
        assert!(fb.contains("fastboot missing"));
        let scr = ensure_tool_plan("scrcpy");
        assert!(scr.contains("optional"));
        assert!(scr.contains("apt install scrcpy"));
        assert!(scr.contains("brew install scrcpy"));
        assert!(scr.contains("Setup-TrebleToolkit.bat"));
        assert!(ensure_tool_plan("Scrcpy").contains("optional"));
        let empty = ensure_tool_plan("");
        assert!(empty.contains("tool missing"));
        assert!(empty.contains("[q] Abort"));
    }

    #[test]
    fn platform_tools_install_full_composition() {
        let d = tmp_dir("ptinstall");
        let reg = d.join("VTR-L29.json");
        let body = serde_json::json!({
            "roms": [],
            "tools": {
                "platform_tools": {
                    "source": "direct-official",
                    "url_pattern": "https://example.invalid/platform-tools-latest-{os}.zip",
                    "provides": ["adb", "fastboot"]
                }
            }
        });
        std::fs::write(&reg, serde_json::to_string(&body).unwrap()).unwrap();
        let dest = d.join("tools");
        std::fs::create_dir_all(&dest).unwrap();
        // Pre-seed cached zip so no network happens (download_file returns Cached).
        let zip_name = "platform-tools-latest-linux.zip";
        let zip_bytes = raw_zip_cli_bytes("platform-tools/adb", b"fake-adb");
        std::fs::write(dest.join(zip_name), &zip_bytes).unwrap();
        let out = install_platform_tools_run(&reg, "linux", &dest).unwrap();
        assert_eq!(out.len(), 1);
        assert!(dest.join("platform-tools/adb").is_file());
        assert_eq!(
            std::fs::read(dest.join("platform-tools/adb")).unwrap(),
            b"fake-adb"
        );
        assert!(install_platform_tools_run(&reg, "linux", std::path::Path::new("")).is_err());
        assert!(install_platform_tools_run(&d.join("missing.json"), "linux", &dest).is_err());
        std::fs::remove_dir_all(&d).ok();
    }
}
