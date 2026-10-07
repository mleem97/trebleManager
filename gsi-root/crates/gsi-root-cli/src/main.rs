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
        Some(pick) => match resolve_root_target_choice(&pick) {
            Ok(tag) => {
                if tag == "current" {
                    if current.is_empty() {
                        eprintln!("select root-target: no current system known (pass --current or --installed-file)");
                        return 1;
                    }
                    println!("target kept: {label} -> keep system, root only");
                    0
                } else if tag == "stock" {
                    println!("target: Stock EMUI (choose 8 / 9.0 / 9.1 explicitly in guided flow)");
                    0
                } else {
                    println!("target: other system (run `select image` to resolve Android -> system -> variant)");
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
}
