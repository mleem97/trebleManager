//! Managed external-tool binding.
//!
//! Until native ADB/fastboot protocols land (Phase 8), tools the project
//! cannot reimplement (adb/fastboot binaries, extractor programs) are
//! called through this layer — never through shell scripts:
//! explicit lookup (PATH + known dirs), version probe, timeout-guarded
//! execution, captured output. Callers parse output with `gsi-device`.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::time::Duration;

/// A located tool binary.
#[derive(Debug, Clone)]
pub struct Tool {
    /// Logical name (`adb`, `fastboot`, ...).
    pub name: String,
    /// Resolved executable path.
    pub path: PathBuf,
}

/// Execution result (mirrors what the scripts log).
#[derive(Debug, Clone)]
pub struct RunResult {
    pub success: bool,
    pub code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub timed_out: bool,
}

/// Locate `name[.exe]` in PATH plus `extra_dirs` (no shell involved).
pub fn locate(name: &str, extra_dirs: &[PathBuf]) -> Option<Tool> {
    let exe = if cfg!(windows) {
        format!("{name}.exe")
    } else {
        name.to_string()
    };
    let mut dirs: Vec<PathBuf> = Vec::new();
    if let Some(path) = std::env::var_os("PATH") {
        dirs.extend(std::env::split_paths(&path));
    }
    dirs.extend(extra_dirs.iter().cloned());
    for d in dirs {
        let cand = d.join(&exe);
        if is_executable(&cand) {
            return Some(Tool {
                name: name.to_string(),
                path: cand,
            });
        }
        // Windows also resolves bare names; Unix needs exact match.
        #[cfg(windows)]
        {
            let bare = d.join(name);
            if bare != cand && is_executable(&bare) {
                return Some(Tool {
                    name: name.to_string(),
                    path: bare,
                });
            }
        }
    }
    None
}

#[cfg(unix)]
fn is_executable(p: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    p.is_file()
        && std::fs::metadata(p)
            .map(|m| m.permissions().mode() & 0o111 != 0)
            .unwrap_or(false)
}

#[cfg(windows)]
fn is_executable(p: &Path) -> bool {
    p.is_file()
}

/// Run a tool with argv and a timeout. Never panics on tool failure —
/// failures are data (`success: false`, captured output).
pub fn run(tool: &Tool, args: &[&str], timeout: Duration) -> RunResult {
    let mut child = match Command::new(&tool.path)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(c) => c,
        Err(e) => {
            return RunResult {
                success: false,
                code: None,
                stdout: String::new(),
                stderr: format!("spawn: {e}"),
                timed_out: false,
            }
        }
    };
    let timed_out = Arc::new(AtomicBool::new(false));
    let killer = timed_out.clone();
    // Drain pipes on reader threads BEFORE waiting: a child filling the OS
    // pipe buffer must never deadlock the wait, however verbose it gets.
    let (stdout, stderr, status) = std::thread::scope(|s| {
        let mut out_h = child.stdout.take();
        let mut err_h = child.stderr.take();
        let ho = s.spawn(move || {
            let mut buf = String::new();
            if let Some(ref mut o) = out_h {
                use std::io::Read;
                let _ = o.read_to_string(&mut buf);
            }
            buf
        });
        let he = s.spawn(move || {
            let mut buf = String::new();
            if let Some(ref mut e) = err_h {
                use std::io::Read;
                let _ = e.read_to_string(&mut buf);
            }
            buf
        });
        let deadline = std::time::Instant::now() + timeout;
        let status = loop {
            match child.try_wait() {
                Ok(Some(st)) => break Some(st),
                Ok(None) => {
                    if std::time::Instant::now() >= deadline {
                        killer.store(true, Ordering::SeqCst);
                        let _ = child.kill();
                        match child.wait() {
                            Ok(st) => break Some(st),
                            Err(_) => break None,
                        }
                    } else {
                        std::thread::sleep(Duration::from_millis(50));
                    }
                }
                Err(_) => break None,
            }
        };
        drop(killer);
        (ho.join().unwrap_or_default(), he.join().unwrap_or_default(), status)
    });
    let was_timeout = timed_out.load(Ordering::SeqCst);
    RunResult {
        success: !was_timeout && status.map(|s| s.success()).unwrap_or(false),
        code: status.and_then(|s| s.code()),
        stdout,
        stderr,
        timed_out: was_timeout,
    }
}

/// Probe `--version` output (first line, trimmed). `None` when unusable.
pub fn probe_version(tool: &Tool) -> Option<String> {
    let r = run(tool, &["--version"], Duration::from_secs(15));
    if !r.success {
        // fastboot has no --version on old builds; try `version` verb.
        let r2 = run(tool, &["version"], Duration::from_secs(15));
        if !r2.success {
            return None;
        }
        return r2.stdout.lines().next().map(|l| l.trim().to_string());
    }
    r.stdout.lines().next().map(|l| l.trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_tool_is_none() {
        assert!(locate("gsi-tool-test-no-such-binary-xyz", &[]).is_none());
    }

    #[test]
    fn failing_spawn_is_data_not_panic() {
        let t = Tool {
            name: "x".to_string(),
            path: PathBuf::from("/no/such/file"),
        };
        let r = run(&t, &[], Duration::from_secs(5));
        assert!(!r.success);
        assert!(!r.timed_out);
    }

    #[test]
    fn timeout_kills_and_reports() {
        #[cfg(unix)]
        {
            // sleep exists everywhere on unix test hosts.
            if let Some(t) = locate("sleep", &[]) {
                let r = run(&t, &["30"], Duration::from_millis(300));
                assert!(!r.success);
                assert!(r.timed_out);
                return;
            }
        }
        // No sleep binary: still a valid (skipped) outcome, never a failure.
    }

    #[test]
    fn quick_command_succeeds_with_output() {
        #[cfg(unix)]
        {
            if let Some(t) = locate("echo", &[]) {
                let r = run(&t, &["hello"], Duration::from_secs(5));
                assert!(r.success);
                assert!(r.stdout.contains("hello"));
                return;
            }
        }
        #[cfg(windows)]
        {
            if let Some(t) = locate("cmd", &[]) {
                let r = run(&t, &["/c", "echo", "hello"], Duration::from_secs(5));
                assert!(r.success);
                assert!(r.stdout.contains("hello"));
                return;
            }
        }
    }
}
