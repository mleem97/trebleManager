//! Slint GUI for gsi-root: same core API as the CLI, no own logic.
//!
//! Pages Dashboard/GSI/Updates/Logs/Settings. Patch/verify surface the
//! core's honest EXPERIMENTAL refusal instead of faking success.

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
    let mut out = format!("file: {path}\ncontainer: {}\n", match container {
        Container::Gzip => "gzip",
        Container::AndroidSparse => "android-sparse",
        Container::Raw => "raw",
    });
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
            let mut l = log.lock().unwrap();
            if !l.is_empty() {
                l.push('\n');
            }
            l.push_str(line.as_str());
            let text: slint::SharedString = l.clone().into();
            drop(l);
            if let Some(a) = weak.upgrade() {
                a.set_log_text(text);
            }
        });
    }
    app.on_analyze_image(|p| analyze_text(p.as_str()).into());
    app.on_patch_image(|p| patch_text(p.as_str()).into());
    app.on_check_update(|r| update_text(r.as_str()).into());
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
}
