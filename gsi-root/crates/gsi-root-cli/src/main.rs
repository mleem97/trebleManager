//! `gsi-root` CLI — thin layer over `gsi-root-core` (no own logic).
//!
//! `analyze`/`inspect` work today. `patch`/`verify` refuse honestly until a
//! hardware POC exists (EXPERIMENTAL, never faked as done).

use gsi_android::Props;
use gsi_image::{detect_container, parse_sparse_header, Container};
use gsi_root_core::{resolve_engine, GsiProfile, Maturity};
use std::path::PathBuf;

fn usage() -> ! {
    eprintln!("gsi-root: analyze|inspect|patch|verify <image> | version");
    eprintln!("  analyze <image>   static GSI profile (no writes)");
    eprintln!("  inspect <image>   alias of analyze");
    eprintln!("  patch <image>     EXPERIMENTAL: refused until hardware POC");
    eprintln!("  verify <image>    EXPERIMENTAL: refused until hardware POC");
    std::process::exit(2);
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
    println!(
        "engine: {:?} ({:?})",
        engine.engine, engine.maturity
    );
    for r in &engine.reasons {
        println!("reason: {r}");
    }
    // Props demo hook: analyzer reads build.prop from mounted images in
    // later phases; the parser itself is unit-tested (see gsi-android).
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

fn main() {
    let mut args = std::env::args().skip(1);
    let cmd = args.next().unwrap_or_else(|| {
        eprintln!("gsi-root: no command runs the GUI (Phase 9, Slint) — CLI only for now.");
        usage();
    });
    let code = match cmd.as_str() {
        "analyze" | "inspect" => match args.next() {
            Some(p) => analyze(&p),
            None => usage(),
        },
        "patch" | "verify" => experimental(&cmd),
        "version" => {
            println!("gsi-root {}", env!("CARGO_PKG_VERSION"));
            0
        }
        _ => usage(),
    };
    std::process::exit(code);
}
