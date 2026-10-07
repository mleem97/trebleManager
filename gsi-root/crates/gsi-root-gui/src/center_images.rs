//! Images GUI center renders (read-only, never executing).
//!
//! Covers the Images center cluster: `ImagesCenter` (installed / available
//! / downloaded / local tables), `AnalyzerCenter` (path echo + analysis
//! table) and `CompatCenter` (per-area verdicts + recommended workflow).
//!
//! Reuse, never duplicate:
//! - container and sparse fields mirror `gsi-image::detect_container` and
//!   `gsi-image::parse_sparse_header` plus the engine verdict from
//!   `gsi-root-core::resolve_engine` (live values are caller-measured; this
//!   module only renders text, it never opens a file).
//! - archive kinds mirror `gsi-archive::classify` as a local label only:
//!   `gsi-archive` is not a `gsi-root-gui` dependency, so the GUI keeps the
//!   same script-order mapping without taking the dependency (see
//!   `pages_media::extract_text` for the same rule).
//! - the compat base delegates to `crate::pages_flows::compat_text` (which
//!   mirrors `Screen-Compatibility` / `screen_compat`); per-area reasons,
//!   prerequisites and limitations are caller-measured via
//!   `gsi-registry::firmware_compat` / `vendor_advice` /
//!   `test_rom_against_registry`.
//! - future live wiring (shell-owned): registry lists via `live` scans,
//!   cache dirs via `gsi-config`, no downloads started here.
//!
//! Slint side lives in `ui/center_images.slint` (`ImagesCenter`,
//! `AnalyzerCenter`, `CompatCenter`) with the same visual pattern as the
//! Detect page (baked title, note, Refresh button, read-only log view) and
//! the component contract `in-out property <string> result; callback
//! refresh() -> string;` plus an `append-log` passthrough. Drag and drop is
//! not supported by Slint: the analyzer uses a path `LineEdit` plus an
//! honest note. No device, no network, no file access, no `unwrap` /
//! `expect` / `panic` outside tests.

/// Archive kind label by file name (case-insensitive, script order).
///
/// Local mirror of `gsi-archive::classify` without taking that dependency
/// (same rule as `pages_media`): `.tar.gz` / `.tgz` count as tar, not gzip.
fn archive_kind_label(name: &str) -> &'static str {
    let mut lower = String::new();
    for c in name.chars() {
        lower.push(c.to_ascii_lowercase());
    }
    if lower.ends_with(".tar.gz") {
        "tar.gz"
    } else if lower.ends_with(".tar.xz") {
        "tar.xz"
    } else if lower.ends_with(".tgz") {
        "tgz"
    } else if lower.ends_with(".tar") {
        "tar"
    } else if lower.ends_with(".zip") {
        "zip"
    } else if lower.ends_with(".gz") {
        "gzip-single"
    } else if lower.ends_with(".xz") {
        "xz-single"
    } else if lower.ends_with(".app") {
        "update-app"
    } else if lower.ends_with(".img") {
        "plain-img"
    } else {
        "plain"
    }
}

fn list_block(out: &mut String, label: &str, rows: &[String], empty_hint: &str) {
    out.push_str(label);
    if rows.is_empty() {
        out.push_str(empty_hint);
        return;
    }
    out.push('\n');
    let mut shown: usize = 0;
    for r in rows.iter() {
        let t = r.trim();
        if t.is_empty() {
            continue;
        }
        out.push_str(&format!(" - {t}\n"));
        shown += 1;
        if shown >= 8 {
            break;
        }
    }
    if shown == 0 {
        out.push_str(empty_hint);
    } else if rows.len() > shown {
        out.push_str(" - ... (truncated)\n");
    }
}

/// Images center render over four explicit lists.
///
/// Each list holds preformatted display lines (caller-built from the
/// registry and the local `data/roms`, `data/firmware` scans). Empty lists
/// render honest empty states, never guesses. Read-only; no download or
/// extraction starts here.
pub fn images_center_text(
    installed: &[String],
    available: &[String],
    downloaded: &[String],
    local: &[String],
) -> String {
    let mut out = String::new();
    out.push_str("Images center (read-only, nothing executed)\n");
    list_block(
        &mut out,
        "installed:",
        installed,
        " (none recorded - pick a system)\n",
    );
    list_block(
        &mut out,
        "available (registry):",
        available,
        " none listed (submit device data first)\n",
    );
    list_block(
        &mut out,
        "downloaded (cache):",
        downloaded,
        " none cached (drop package into data/roms/ manually)\n",
    );
    if local.is_empty() {
        out.push_str("local files: none found\n");
    } else {
        out.push_str("local files:\n");
        let mut shown: usize = 0;
        for f in local.iter() {
            let t = f.trim();
            if t.is_empty() {
                continue;
            }
            let base = t.rsplit('/').next().unwrap_or(t);
            let base2 = base.rsplit('\\').next().unwrap_or(base);
            out.push_str(&format!(" - {t} [{}]\n", archive_kind_label(base2)));
            shown += 1;
            if shown >= 8 {
                break;
            }
        }
        if shown == 0 {
            out.push_str("local files: none found\n");
        } else if local.len() > shown {
            out.push_str(" - ... (truncated)\n");
        }
    }
    out.push_str("refused: no download or extraction started here\n");
    out
}

/// One compatibility area verdict (caller-measured via `gsi-registry`).
#[derive(Debug, Clone, Default)]
pub struct CompatArea {
    pub area: String,
    pub verdict: String,
    pub reasons: Vec<String>,
    pub prerequisites: Vec<String>,
    pub limitations: Vec<String>,
}

/// Compat center render: registry base plus per-area verdicts and workflow.
///
/// The registry base delegates to `crate::pages_flows::compat_text`
/// (recommended / researched-broken / firmware verdict / vendor advice).
/// `areas` carry the per-area `PASS` / `WARN` / `FAIL` verdicts with
/// reasons, prerequisites and limitations; `workflow` names the recommended
/// next flow. Empty areas render honestly. Read-only.
#[allow(clippy::too_many_arguments)]
pub fn compat_center_text(
    model: &str,
    firmware_baseline: &str,
    region: &str,
    firmware_status: &str,
    firmware_reasons: &[String],
    vendor_advice: &str,
    recommended: &[String],
    not_recommended: &[String],
    firmware_base_note: &str,
    areas: &[CompatArea],
    workflow: &str,
) -> String {
    let mut out = String::new();
    out.push_str("Compat center (read-only, nothing executed)\n");
    out.push_str(&crate::pages_flows::compat_text(
        model,
        firmware_baseline,
        region,
        firmware_status,
        firmware_reasons,
        vendor_advice,
        recommended,
        not_recommended,
        firmware_base_note,
    ));
    if areas.is_empty() {
        out.push_str("\nareas: none listed (registry verdict above is the full picture)\n");
    } else {
        out.push_str("\nareas:\n");
        for a in areas.iter() {
            let name = if a.area.trim().is_empty() {
                "(unnamed)"
            } else {
                a.area.trim()
            };
            let v = if a.verdict.trim().is_empty() {
                "UNKNOWN"
            } else {
                a.verdict.trim()
            };
            out.push_str(&format!(" [{v}] {name}\n"));
            for r in a.reasons.iter() {
                let t = r.trim();
                if t.is_empty() {
                    continue;
                }
                out.push_str(&format!("   reason: {t}\n"));
            }
            for p in a.prerequisites.iter() {
                let t = p.trim();
                if t.is_empty() {
                    continue;
                }
                out.push_str(&format!("   needs: {t}\n"));
            }
            for l in a.limitations.iter() {
                let t = l.trim();
                if t.is_empty() {
                    continue;
                }
                out.push_str(&format!("   limit: {t}\n"));
            }
        }
    }
    let w = workflow.trim();
    if w.is_empty() {
        out.push_str("recommended workflow: (none - resolve blockers first)\n");
    } else {
        out.push_str(&format!("recommended workflow: {w}\n"));
    }
    out.push_str("refused: flashing or downloading from a verdict alone is Phase 8 shell work\n");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strings(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    fn area(
        name: &str,
        verdict: &str,
        reasons: &[&str],
        needs: &[&str],
        limits: &[&str],
    ) -> CompatArea {
        CompatArea {
            area: name.to_string(),
            verdict: verdict.to_string(),
            reasons: strings(reasons),
            prerequisites: strings(needs),
            limitations: strings(limits),
        }
    }

    #[test]
    fn images_all_lists_render() {
        let t = images_center_text(
            &strings(&["Stock EMUI (current)"]),
            &strings(&["LineageOS 20 UNOFFICIAL"]),
            &strings(&["lineage-20.img.gz cached"]),
            &strings(&["data/roms/lineage-20.img.gz"]),
        );
        assert!(t.contains("Images center"));
        assert!(t.contains("Stock EMUI"));
        assert!(t.contains("LineageOS 20"));
        assert!(t.contains("cached"));
        assert!(t.contains("lineage-20.img.gz [gzip-single]"));
        assert!(t.contains("no download or extraction started here"));
    }

    #[test]
    fn images_empty_states_are_honest() {
        let t = images_center_text(&[], &[], &[], &[]);
        assert!(t.contains("(none recorded - pick a system)"));
        assert!(t.contains("none listed (submit device data first)"));
        assert!(t.contains("drop package into data/roms/ manually"));
        assert!(t.contains("local files: none found"));
    }

    #[test]
    fn images_local_kind_labels() {
        let t = images_center_text(
            &[],
            &[],
            &[],
            &strings(&["rom.zip", "fw.tar.gz", "upd.APP", "sys.img"]),
        );
        assert!(t.contains("rom.zip [zip]"));
        assert!(t.contains("fw.tar.gz [tar.gz]"));
        assert!(t.contains("upd.APP [update-app]"));
        assert!(t.contains("sys.img [plain-img]"));
    }

    #[test]
    fn compat_delegates_and_adds_areas() {
        let t = compat_center_text(
            "VTR-L29",
            "VTR-L29 9.1.0.297(C432E5R1P9)",
            "C432",
            "PASS",
            &strings(&["EMUI 9.1 base ok (recovery_ramdisk method)."]),
            "Pie vendor: Q/R/S boot. Target Android 13 expected to boot.",
            &strings(&["LineageOS 20 UNOFFICIAL"]),
            &strings(&["LineageOS 20 Light - non-booting"]),
            "required_base EMUI 9.1",
            &[area(
                "system",
                "PASS",
                &["header ok"],
                &["fastboot mode"],
                &["userdata stays"],
            )],
            "backup -> flash-system -> eRecovery wipe -> verify",
        );
        assert!(t.contains("Compat center"));
        assert!(t.contains("model: VTR-L29"));
        assert!(t.contains("[+] LineageOS 20 UNOFFICIAL"));
        assert!(t.contains("[PASS] system"));
        assert!(t.contains("reason: header ok"));
        assert!(t.contains("needs: fastboot mode"));
        assert!(t.contains("limit: userdata stays"));
        assert!(t.contains("recommended workflow: backup -> flash-system"));
        assert!(t.contains("Phase 8"));
    }

    #[test]
    fn compat_empty_areas_and_workflow_is_honest() {
        let t = compat_center_text("", "", "", "", &[], "", &[], &[], "", &[], "");
        assert!(t.contains("model: unknown"));
        assert!(t.contains("none listed (submit device data first)"));
        assert!(t.contains("areas: none listed"));
        assert!(t.contains("resolve blockers first"));
    }

    #[test]
    fn compat_unknown_verdict_renders_unknown() {
        let t = compat_center_text(
            "VTR-L29",
            "base",
            "C432",
            "WARN",
            &[],
            "adv",
            &[],
            &[],
            "",
            &[area("vendor", "", &[], &[], &[])],
            "check vendor",
        );
        assert!(t.contains("[UNKNOWN] vendor"));
        assert!(t.contains("recommended workflow: check vendor"));
    }
}
