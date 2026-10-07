//! Typed native workflows: plan once, run everywhere (CLI/GUI/scripts).
//!
//! GUI, CLI and scripts share these definitions — no duplicate logic.
//! Destructive steps never run without explicit confirmation (the runner
//! refuses them unless `allow_destructive` is set, i.e. CLI `--yes`).

use gsi_image::{detect_container, Container};
use gsi_root_core::Maturity;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

/// One workflow step (spec §11).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkflowStep {
    Download { url: String, dest: PathBuf },
    VerifyHash { file: PathBuf, sha256: String },
    Analyze { file: PathBuf },
    Extract,
    Patch,
    Repack,
    VerifyOutput,
    Backup,
    DeviceDetect,
    Flash,
    Reboot,
    Test,
    Cleanup,
}

/// A named, inspectable workflow.
#[derive(Debug, Clone)]
pub struct Workflow {
    pub id: String,
    pub name: String,
    pub steps: Vec<WorkflowStep>,
}

/// Progress events for CLI bars and GUI status (spec §31).
#[derive(Debug, Clone)]
pub enum ProgressEvent {
    Started { operation: String },
    Progress { current: u64, total: u64 },
    Message { message: String },
    Warning { message: String },
    Completed,
}

/// Step outcome (honest: unimplemented engines refuse, never fake).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StepOutcome {
    Done,
    Skipped(String),
    RefusedExperimental(String),
    Failed(String),
}

/// P10 → LineageOS 20 reference workflow (spec §34 pipeline, device part
/// still manual until the hardware POC exists).
pub fn p10_lineage20(input: &Path, _output: &Path) -> Workflow {
    Workflow {
        id: "p10-lineage20".to_string(),
        name: "Huawei P10 → LineageOS 20 Root".to_string(),
        steps: vec![
            WorkflowStep::Analyze {
                file: input.to_path_buf(),
            },
            WorkflowStep::Patch,
            WorkflowStep::Repack,
            WorkflowStep::VerifyOutput,
            WorkflowStep::Backup,
            WorkflowStep::Flash,
        ],
    }
}

/// SHA-256 of a file, lowercase hex.
pub fn sha256_file(path: &Path) -> Result<String, String> {
    let data = std::fs::read(path).map_err(|e| format!("read: {e}"))?;
    let mut h = Sha256::new();
    h.update(&data);
    Ok(format!("{:x}", h.finalize()))
}

/// Analyze step: real container detection + maturity verdict.
pub fn run_analyze(file: &Path) -> (Vec<ProgressEvent>, StepOutcome) {
    let mut ev = vec![ProgressEvent::Started {
        operation: format!("analyze {}", file.display()),
    }];
    let container = detect_container(file);
    ev.push(ProgressEvent::Message {
        message: format!(
            "container: {}",
            match container {
                Container::Gzip => "gzip",
                Container::AndroidSparse => "android-sparse",
                Container::Raw => "raw",
            }
        ),
    });
    ev.push(ProgressEvent::Completed);
    (
        ev,
        StepOutcome::Done,
    )
}

/// Patch step: refuses until a hardware POC exists (never faked).
pub fn run_patch(_plan: &str) -> (Vec<ProgressEvent>, StepOutcome) {
    (
        vec![ProgressEvent::Started {
            operation: "patch".to_string(),
        }],
        StepOutcome::RefusedExperimental(
            "no hardware POC: patching would be unproven. See devices/huawei-p10.".to_string(),
        ),
    )
}

/// Verify a file hash against an expected SHA-256.
pub fn run_verify_hash(file: &Path, expected: &str) -> (Vec<ProgressEvent>, StepOutcome) {
    let mut ev = vec![ProgressEvent::Started {
        operation: format!("verify {}", file.display()),
    }];
    match sha256_file(file) {
        Ok(got) => {
            if got.eq_ignore_ascii_case(expected) {
                ev.push(ProgressEvent::Completed);
                (ev, StepOutcome::Done)
            } else {
                (
                    ev,
                    StepOutcome::Failed(format!(
                        "hash mismatch: got {got}, want {expected}"
                    )),
                )
            }
        }
        Err(e) => (ev, StepOutcome::Failed(e)),
    }
}

/// Maturity of the whole workflow path (worst link decides).
pub fn workflow_maturity() -> Maturity {
    // Patch engine unproven on hardware → whole root path is Experimental.
    Maturity::Experimental
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn tmp(name: &str, bytes: &[u8]) -> PathBuf {
        let mut p = std::env::temp_dir();
        p.push(format!("gsi-workflow-test-{name}"));
        let mut f = std::fs::File::create(&p).unwrap();
        f.write_all(bytes).unwrap();
        p
    }

    #[test]
    fn p10_workflow_lists_steps_in_order() {
        let w = p10_lineage20(Path::new("in.img"), Path::new("out.img"));
        assert_eq!(w.id, "p10-lineage20");
        assert!(matches!(w.steps[0], WorkflowStep::Analyze { .. }));
        assert!(matches!(w.steps[1], WorkflowStep::Patch));
        assert!(w.steps.len() >= 5);
    }

    #[test]
    fn analyze_is_real_and_reports_container() {
        let p = tmp("a.gz", &[0x1F, 0x8B, 0x08, 0x00]);
        let (ev, out) = run_analyze(&p);
        assert_eq!(out, StepOutcome::Done);
        assert!(ev.iter().any(|e| matches!(
            e,
            ProgressEvent::Message { message } if message.contains("gzip")
        )));
        std::fs::remove_file(p).ok();
    }

    #[test]
    fn patch_refuses_honestly() {
        let (_, out) = run_patch("anything");
        assert!(matches!(out, StepOutcome::RefusedExperimental(_)));
    }

    #[test]
    fn hash_verify_roundtrip() {
        let p = tmp("h.bin", b"hello treble");
        let h = sha256_file(&p).unwrap();
        let (_, out) = run_verify_hash(&p, &h);
        assert_eq!(out, StepOutcome::Done);
        let (_, out2) = run_verify_hash(&p, "deadbeef");
        assert!(matches!(out2, StepOutcome::Failed(_)));
        std::fs::remove_file(p).ok();
    }

    #[test]
    fn destructive_steps_are_distinct_variants() {
        // The type system (not a boolean) separates destructive steps.
        let w = p10_lineage20(Path::new("i"), Path::new("o"));
        assert!(w.steps.contains(&WorkflowStep::Flash));
        assert!(w.steps.contains(&WorkflowStep::Backup));
    }
}
