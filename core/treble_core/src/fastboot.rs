//! Fastboot flash/erase output evaluation.
//!
//! FAILED lines veto everything: progress words like `Writing`/`Erasing`
//! alone are NOT success. OK requires at least one OKAY plus a Finished line.

/// Verdict for one fastboot flash/erase output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    /// Completed: N OKAY lines plus a Finished line.
    Ok { okay: usize, total_time: String },
    /// Refused/failed: first failing lines (max 3 kept).
    Failed { lines: Vec<String> },
    /// No FAILED but no Finished either — verify manually, claim nothing.
    Unclear,
}

fn is_failed_line(line: &str) -> bool {
    let lower = line.to_lowercase();
    if lower.contains("failed") || lower.contains("remote:") {
        return true;
    }
    let t = line.trim_start().to_lowercase();
    t.starts_with("error")
}

fn total_time(line: &str) -> Option<String> {
    let lower = line.to_lowercase();
    let pos = lower.find("total time:")?;
    Some(line[pos + "total time:".len()..].trim().to_string())
}

/// Evaluate fastboot output lines (mirrors `Get-FlashVerdict`).
pub fn flash_verdict<'a, I>(lines: I) -> Verdict
where
    I: IntoIterator<Item = &'a str>,
{
    let mut okay = 0usize;
    let mut failed: Vec<String> = Vec::new();
    let mut total = String::new();
    for line in lines {
        if line.contains("OKAY") {
            okay += 1;
        }
        if is_failed_line(line) && failed.len() < 3 {
            failed.push(line.trim().to_string());
        }
        if total.is_empty() {
            if let Some(t) = total_time(line) {
                total = t;
            }
        }
    }
    if !failed.is_empty() {
        return Verdict::Failed { lines: failed };
    }
    if okay > 0 && !total.is_empty() {
        return Verdict::Ok {
            okay,
            total_time: total,
        };
    }
    Verdict::Unclear
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ok_on_full_success() {
        let v = flash_verdict([
            "Sending 'system' (1126400 KB)              OKAY [ 28.1s]",
            "Writing 'system'                                 OKAY [ 41.2s]",
            "Finished. Total time: 70.003s",
        ]);
        assert_eq!(
            v,
            Verdict::Ok {
                okay: 2,
                total_time: "70.003s".to_string()
            }
        );
    }

    #[test]
    fn failed_vetoes_writing_and_okay() {
        let v = flash_verdict([
            "Sending 'system' (1126400 KB)              OKAY [ 28.1s]",
            "Writing 'system'          FAILED (remote: 'Command not allowed')",
            "Finished. Total time: 0.010s",
        ]);
        match v {
            Verdict::Failed { lines } => assert!(
                lines.iter().any(|l| l.contains("Command not allowed")),
                "reason kept: {lines:?}"
            ),
            other => panic!("expected Failed, got {other:?}"),
        }
    }

    #[test]
    fn unclear_on_progress_only() {
        assert_eq!(flash_verdict(["Erasing 'userdata' ..."]), Verdict::Unclear);
    }

    #[test]
    fn ok_on_erase_success() {
        let v = flash_verdict([
            "Erasing 'userdata'                                 OKAY [  2.1s]",
            "Finished. Total time: 2.150s",
        ]);
        assert!(matches!(v, Verdict::Ok { .. }));
    }

    #[test]
    fn huawei_getvar_denied_is_failed_line() {
        let v = flash_verdict(["getvar:unlocked FAILED (remote: Command not allowed)"]);
        assert!(matches!(v, Verdict::Failed { .. }));
    }
}
