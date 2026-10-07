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
    eprintln!("  device slot|switch            slot state + switch plan (no flashing yet)");
    eprintln!("  device detect|info            devices via managed adb/fastboot binding");
    eprintln!("  adb devices                   list adb devices (needs adb on PATH)");
    eprintln!("  fastboot devices|getvar       fastboot queries (needs fastboot on PATH)");
    eprintln!("  tools                         managed external tools status");
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
        "tools" => tools_status(),
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
