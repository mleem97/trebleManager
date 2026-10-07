//! Device execution layer with injected executors.
//!
//! Scripts shell out to platform tools; this crate does the same via the
//! managed `gsi-tool` binding. No native protocols here, parity only.
//! All destructive work needs double confirm tokens and is refused
//! before any call when tokens are missing.

use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::time::Duration;

// ------------------------------------------------------------ core types

/// Captured process output, mirrors `gsi-tool` run result data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecOut {
    /// Captured stdout text.
    pub stdout: String,
    /// Captured stderr text.
    pub stderr: String,
    /// Exit code when known.
    pub code: Option<i32>,
}

impl ExecOut {
    /// Build an output record.
    pub fn new(stdout: String, stderr: String, code: Option<i32>) -> Self {
        Self {
            stdout,
            stderr,
            code,
        }
    }

    /// True when code is zero.
    pub fn success(&self) -> bool {
        matches!(self.code, Some(0))
    }

    /// stdout plus stderr, for verdict scans.
    pub fn combined(&self) -> String {
        let mut s = self.stdout.clone();
        if !self.stderr.is_empty() {
            if !s.is_empty() {
                s.push('\n');
            }
            s.push_str(self.stderr.as_str());
        }
        s
    }

    /// Split combined text into lines.
    pub fn lines(&self) -> Vec<String> {
        let mut out = Vec::new();
        for l in self.combined().lines() {
            out.push(l.to_string());
        }
        out
    }
}

/// Typed failure: refused safety gate or invalid input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecError {
    /// Safety gate refused, no call was made.
    Refused(String),
    /// Invalid input, no call was made.
    Invalid(String),
}

impl ExecError {
    /// Reason text.
    pub fn reason(&self) -> &str {
        match self {
            Self::Refused(s) => s.as_str(),
            Self::Invalid(s) => s.as_str(),
        }
    }

    /// Refused gate helper.
    pub fn refused(msg: String) -> Self {
        Self::Refused(msg)
    }

    /// Invalid input helper.
    pub fn invalid(msg: String) -> Self {
        Self::Invalid(msg)
    }
}

impl std::fmt::Display for ExecError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Refused(s) => write!(f, "refused: {s}"),
            Self::Invalid(s) => write!(f, "invalid: {s}"),
        }
    }
}

impl std::error::Error for ExecError {}

/// One recorded stub call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecordedCall {
    /// fastboot argv.
    Fastboot(Vec<String>),
    /// adb argv (non-shell).
    Adb(Vec<String>),
    /// adb shell command string.
    AdbShell(String),
}

impl RecordedCall {
    /// Kind id: fastboot, adb, adb-shell.
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Fastboot(_) => "fastboot",
            Self::Adb(_) => "adb",
            Self::AdbShell(_) => "adb-shell",
        }
    }
}

/// Device access point, injected for dry-run tests.
pub trait Executor {
    /// Run `fastboot <args>`, timeout inside impl.
    fn run_fastboot(&mut self, args: &[String]) -> ExecOut;
    /// Run `adb <args>`, timeout inside impl.
    fn run_adb(&mut self, args: &[String]) -> ExecOut;
    /// Run `adb shell <cmd>`.
    fn run_adb_shell(&mut self, cmd: &str) -> ExecOut;
}

// ------------------------------------------------------------ stub

/// Dry-run stub: records calls, replays scripted outputs.
#[derive(Debug)]
pub struct StubExecutor {
    calls: Vec<RecordedCall>,
    queue: VecDeque<ExecOut>,
}

impl StubExecutor {
    /// Empty stub, no scripted output.
    pub fn new() -> Self {
        Self {
            calls: Vec::new(),
            queue: VecDeque::new(),
        }
    }

    /// Stub with scripted outputs in order.
    pub fn with_outputs(outputs: Vec<ExecOut>) -> Self {
        let mut q = VecDeque::new();
        for o in outputs {
            q.push_back(o);
        }
        Self {
            calls: Vec::new(),
            queue: q,
        }
    }

    /// Append one scripted output.
    pub fn push_output(&mut self, out: ExecOut) {
        self.queue.push_back(out);
    }

    /// All recorded calls in order.
    pub fn calls(&self) -> &[RecordedCall] {
        self.calls.as_slice()
    }

    /// Count of recorded calls.
    pub fn call_count(&self) -> usize {
        self.calls.len()
    }

    /// Clear recorded calls.
    pub fn clear(&mut self) {
        self.calls.clear();
    }

    /// Scripted OK output helper.
    pub fn ok_output(stdout: &str) -> ExecOut {
        ExecOut::new(stdout.to_string(), String::new(), Some(0))
    }

    /// Scripted unclear output helper.
    pub fn unclear_output(stdout: &str) -> ExecOut {
        ExecOut::new(stdout.to_string(), String::new(), Some(0))
    }

    /// Scripted failed output helper.
    pub fn failed_output(stdout: &str) -> ExecOut {
        ExecOut::new(stdout.to_string(), String::new(), Some(1))
    }

    fn next_out(&mut self) -> ExecOut {
        match self.queue.pop_front() {
            Some(o) => o,
            None => ExecOut::new(String::new(), String::new(), Some(1)),
        }
    }
}

impl Default for StubExecutor {
    fn default() -> Self {
        Self::new()
    }
}

impl Executor for StubExecutor {
    fn run_fastboot(&mut self, args: &[String]) -> ExecOut {
        self.calls.push(RecordedCall::Fastboot(args.to_vec()));
        self.next_out()
    }
    fn run_adb(&mut self, args: &[String]) -> ExecOut {
        self.calls.push(RecordedCall::Adb(args.to_vec()));
        self.next_out()
    }
    fn run_adb_shell(&mut self, cmd: &str) -> ExecOut {
        self.calls.push(RecordedCall::AdbShell(cmd.to_string()));
        self.next_out()
    }
}

// ------------------------------------------------------------ real

/// Real executor via `gsi-tool` run with script timeouts.
#[derive(Debug, Clone)]
pub struct ToolExecutor {
    /// Resolved adb tool.
    pub adb: gsi_tool::Tool,
    /// Resolved fastboot tool.
    pub fastboot: gsi_tool::Tool,
}

impl ToolExecutor {
    /// Build from resolved tools.
    pub fn new(adb: gsi_tool::Tool, fastboot: gsi_tool::Tool) -> Self {
        Self { adb, fastboot }
    }

    /// Locate adb plus fastboot in PATH and extra dirs.
    pub fn locate_pair(extra_dirs: &[PathBuf]) -> Option<Self> {
        let adb = gsi_tool::locate("adb", extra_dirs);
        let fastboot = gsi_tool::locate("fastboot", extra_dirs);
        match (adb, fastboot) {
            (Some(a), Some(f)) => Some(Self {
                adb: a,
                fastboot: f,
            }),
            _ => None,
        }
    }

    fn to_out(r: gsi_tool::RunResult) -> ExecOut {
        ExecOut::new(r.stdout, r.stderr, r.code)
    }
}

impl Executor for ToolExecutor {
    fn run_fastboot(&mut self, args: &[String]) -> ExecOut {
        let mut refs: Vec<&str> = Vec::new();
        for a in args.iter() {
            refs.push(a.as_str());
        }
        // fb_flash 600s for flash, fb_run 120s else (scripts).
        let mut is_flash = false;
        if let Some(first) = refs.first() {
            if *first == "flash" {
                is_flash = true;
            }
        }
        let timeout = if is_flash {
            Duration::from_secs(600)
        } else {
            Duration::from_secs(120)
        };
        let r = gsi_tool::run(&self.fastboot, refs.as_slice(), timeout);
        Self::to_out(r)
    }
    fn run_adb(&mut self, args: &[String]) -> ExecOut {
        let mut refs: Vec<&str> = Vec::new();
        for a in args.iter() {
            refs.push(a.as_str());
        }
        let r = gsi_tool::run(&self.adb, refs.as_slice(), Duration::from_secs(30));
        Self::to_out(r)
    }
    fn run_adb_shell(&mut self, cmd: &str) -> ExecOut {
        let args = ["shell", cmd];
        let r = gsi_tool::run(&self.adb, &args, Duration::from_secs(30));
        Self::to_out(r)
    }
}

// ------------------------------------------------------------ partitions

/// Allowed flash target, type-level allow list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Partition {
    /// recovery_ramdisk slot, always allowed.
    RecoveryRamdisk,
    /// system slot, needs explicit override token.
    System,
}

impl Partition {
    /// Fastboot name.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::RecoveryRamdisk => "recovery_ramdisk",
            Self::System => "system",
        }
    }

    /// Parse fastboot name, only the two allowed values pass.
    pub fn parse(s: &str) -> Result<Self, ExecError> {
        let t = s.trim();
        if t == "recovery_ramdisk" {
            Ok(Self::RecoveryRamdisk)
        } else if t == "system" {
            Ok(Self::System)
        } else {
            Err(ExecError::invalid(format!(
                "refuse partition '{t}': only recovery_ramdisk or system-with-override"
            )))
        }
    }
}

/// Explicit second-token override for system flash.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SystemOverride {
    token: String,
}

impl SystemOverride {
    /// Accept only YES or JA as second token.
    pub fn confirm(second: &str) -> Result<Self, ExecError> {
        let t = second.trim();
        if t == "YES" || t == "JA" {
            Ok(Self {
                token: t.to_string(),
            })
        } else {
            Err(ExecError::refused(
                "system flash needs explicit second token YES".to_string(),
            ))
        }
    }

    /// Stored token.
    pub fn token(&self) -> &str {
        self.token.as_str()
    }
}

/// Reboot target for adb reboot shapes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RebootTarget {
    /// plain reboot to system.
    System,
    /// reboot to bootloader/fastboot.
    Bootloader,
    /// reboot to recovery.
    Recovery,
    /// reboot to fastbootd.
    Fastbootd,
}

impl RebootTarget {
    /// Arg tail after `reboot`.
    pub fn as_arg(self) -> Option<&'static str> {
        match self {
            Self::System => None,
            Self::Bootloader => Some("bootloader"),
            Self::Recovery => Some("recovery"),
            Self::Fastbootd => Some("fastboot"),
        }
    }
}

// ------------------------------------------------------------ builders

fn has_forbidden_flag(args: &[String]) -> Option<String> {
    for a in args.iter() {
        let low = a.to_lowercase();
        if low.contains("--force") {
            return Some("--force".to_string());
        }
        if low.contains("verity") || low.contains("disable-verification") {
            return Some("verity".to_string());
        }
        if low == "oem" {
            return Some("oem".to_string());
        }
        if low == "unlock" {
            return Some("unlock".to_string());
        }
        if low == "flashing" {
            return Some("flashing".to_string());
        }
        if low == "format" {
            return Some("format".to_string());
        }
    }
    None
}

/// Validate fastboot argv, refuse dangerous ops always.
pub fn check_fastboot_args(args: &[String]) -> Result<(), ExecError> {
    if args.is_empty() {
        return Err(ExecError::invalid("empty fastboot argv".to_string()));
    }
    if let Some(flag) = has_forbidden_flag(args) {
        return Err(ExecError::refused(format!("refuse fastboot flag '{flag}'")));
    }
    let head = args[0].trim();
    if head == "flash" {
        if args.len() != 3 {
            return Err(ExecError::invalid(
                "flash needs exactly partition plus image".to_string(),
            ));
        }
        let part = args[1].trim();
        if part != "recovery_ramdisk" && part != "system" {
            return Err(ExecError::refused(format!("refuse flash target '{part}'")));
        }
        let img = args[2].trim();
        if img.is_empty() {
            return Err(ExecError::invalid("empty image path".to_string()));
        }
        if img.starts_with('-') {
            return Err(ExecError::refused(
                "refuse image starting with '-'".to_string(),
            ));
        }
        return Ok(());
    }
    if head == "erase" {
        if args.len() != 2 {
            return Err(ExecError::invalid("erase needs one target".to_string()));
        }
        if args[1].trim() != "userdata" {
            return Err(ExecError::refused(format!(
                "refuse erase target '{}': only userdata allowed",
                args[1].trim()
            )));
        }
        return Ok(());
    }
    if head == "reboot" {
        if args.len() != 1 {
            return Err(ExecError::invalid(
                "fastboot reboot takes no tail".to_string(),
            ));
        }
        return Ok(());
    }
    if head == "devices" {
        if args.len() != 1 {
            return Err(ExecError::invalid("devices takes no tail".to_string()));
        }
        return Ok(());
    }
    if head == "getvar" {
        if args.len() != 2 {
            return Err(ExecError::invalid("getvar needs one var".to_string()));
        }
        if args[1].trim().is_empty() {
            return Err(ExecError::invalid("empty getvar name".to_string()));
        }
        return Ok(());
    }
    Err(ExecError::refused(format!("refuse fastboot verb '{head}'")))
}

/// Build `fastboot flash <part> <image>`, system needs override.
pub fn flash_args(
    partition: Partition,
    image: &str,
    sys_override: Option<&SystemOverride>,
) -> Result<Vec<String>, ExecError> {
    let img = image.trim();
    if img.is_empty() {
        return Err(ExecError::invalid("empty image path".to_string()));
    }
    if img.starts_with('-') {
        return Err(ExecError::refused(
            "refuse image starting with '-'".to_string(),
        ));
    }
    match partition {
        Partition::RecoveryRamdisk => {
            let v = vec![
                "flash".to_string(),
                "recovery_ramdisk".to_string(),
                img.to_string(),
            ];
            check_fastboot_args(&v)?;
            Ok(v)
        }
        Partition::System => match sys_override {
            Some(_) => {
                let v = vec!["flash".to_string(), "system".to_string(), img.to_string()];
                check_fastboot_args(&v)?;
                Ok(v)
            }
            None => Err(ExecError::refused(
                "system flash needs explicit override with YES".to_string(),
            )),
        },
    }
}

/// Build `fastboot erase userdata`, the only allowed erase.
pub fn erase_userdata_args() -> Vec<String> {
    vec!["erase".to_string(), "userdata".to_string()]
}

/// Build `fastboot devices`.
pub fn fastboot_devices_args() -> Vec<String> {
    vec!["devices".to_string()]
}

/// Build `fastboot getvar <var>`.
pub fn fastboot_getvar_args(var: &str) -> Result<Vec<String>, ExecError> {
    let v = var.trim();
    if v.is_empty() {
        return Err(ExecError::invalid("empty getvar name".to_string()));
    }
    let out = vec!["getvar".to_string(), v.to_string()];
    check_fastboot_args(&out)?;
    Ok(out)
}

/// Build `fastboot reboot`.
pub fn fastboot_reboot_args() -> Vec<String> {
    vec!["reboot".to_string()]
}

/// Build `adb devices [-l]`.
pub fn adb_devices_args(long: bool) -> Vec<String> {
    if long {
        vec!["devices".to_string(), "-l".to_string()]
    } else {
        vec!["devices".to_string()]
    }
}

/// Build `adb wait-for-device`.
pub fn adb_wait_for_device_args() -> Vec<String> {
    vec!["wait-for-device".to_string()]
}

/// Build `adb reboot [tail]`.
pub fn adb_reboot_args(target: RebootTarget) -> Vec<String> {
    match target.as_arg() {
        Some(tail) => vec!["reboot".to_string(), tail.to_string()],
        None => vec!["reboot".to_string()],
    }
}

/// Build `adb push <local> <remote>`.
pub fn adb_push_args(local: &str, remote: &str) -> Result<Vec<String>, ExecError> {
    let l = local.trim();
    let r = remote.trim();
    if l.is_empty() || r.is_empty() {
        return Err(ExecError::invalid(
            "push needs local plus remote".to_string(),
        ));
    }
    if l.starts_with('-') || r.starts_with('-') {
        return Err(ExecError::refused("refuse push path with '-'".to_string()));
    }
    if !r.starts_with('/') {
        return Err(ExecError::invalid(
            "remote push path must be absolute".to_string(),
        ));
    }
    Ok(vec!["push".to_string(), l.to_string(), r.to_string()])
}

// ------------------------------------------------------------ shell strings

/// Shell `getprop <name>`, mirrors `adb_prop`.
pub fn shell_getprop(name: &str) -> Result<String, ExecError> {
    let n = name.trim();
    if n.is_empty() {
        return Err(ExecError::invalid("empty prop name".to_string()));
    }
    if n.contains(' ') || n.contains(';') || n.contains('&') || n.contains('|') {
        return Err(ExecError::refused(
            "refuse prop with shell chars".to_string(),
        ));
    }
    Ok(format!("getprop {n}"))
}

/// Shell `getprop` full dump.
pub fn shell_getprop_all() -> String {
    "getprop".to_string()
}

/// Shell `su -c id`, root gate.
pub fn shell_su_id() -> String {
    "su -c id".to_string()
}

/// Shell `which su`.
pub fn shell_which_su() -> String {
    "which su".to_string()
}

/// Shell `magisk -v`.
pub fn shell_magisk_version() -> String {
    "magisk -v".to_string()
}

/// Shell `getenforce`.
pub fn shell_getenforce() -> String {
    "getenforce".to_string()
}

/// Shell `mount`.
pub fn shell_mount() -> String {
    "mount".to_string()
}

/// Shell `dumpsys wifi`.
pub fn shell_dumpsys_wifi() -> String {
    "dumpsys wifi".to_string()
}

/// Shell bluetooth flag.
pub fn shell_bluetooth_state() -> String {
    "settings get global bluetooth_on".to_string()
}

/// Shell `dumpsys battery`.
pub fn shell_dumpsys_battery() -> String {
    "dumpsys battery".to_string()
}

/// Shell `dumpsys sensorservice`.
pub fn shell_dumpsys_sensors() -> String {
    "dumpsys sensorservice".to_string()
}

/// Shell by-name listing.
pub fn shell_byname_listing() -> String {
    "ls -l /dev/block/by-name/".to_string()
}

/// Shell cmdline read.
pub fn shell_cat_cmdline() -> String {
    "cat /proc/cmdline".to_string()
}

/// Shell partitions read.
pub fn shell_cat_partitions() -> String {
    "cat /proc/partitions".to_string()
}

/// Shell vendor listing plus build prop.
pub fn shell_cat_vendor() -> String {
    "ls -l /vendor/etc/ 2>&1; cat /vendor/build.prop 2>&1".to_string()
}

/// Shell logcat tail.
pub fn shell_logcat() -> String {
    "logcat -d -t 200 2>&1".to_string()
}

/// Shell immediate aptouch stop.
pub fn shell_stop_aptouch() -> String {
    "su -c 'stop aptouch' 2>&1".to_string()
}

/// Shell persist copy plus chmod for one service.d file.
pub fn shell_persist_cp(name: &str) -> Result<String, ExecError> {
    let n = name.trim();
    if n.is_empty() {
        return Err(ExecError::invalid("empty persist name".to_string()));
    }
    if n.contains('/') || n.contains(' ') || n.contains(';') {
        return Err(ExecError::refused("refuse persist name".to_string()));
    }
    Ok(format!(
        "su -c 'cp /sdcard/Download/{n} /data/adb/service.d/{n} && chmod 755 /data/adb/service.d/{n}' 2>&1"
    ))
}

/// Shell persist presence check.
pub fn shell_persist_ls(name: &str) -> Result<String, ExecError> {
    let n = name.trim();
    if n.is_empty() {
        return Err(ExecError::invalid("empty persist name".to_string()));
    }
    if n.contains('/') || n.contains(' ') || n.contains(';') {
        return Err(ExecError::refused("refuse persist name".to_string()));
    }
    Ok(format!("su -c 'ls -l /data/adb/service.d/{n}' 2>&1"))
}

/// Exact dd probe from backup flow.
pub fn backup_dd_cmd(target: &str) -> Result<String, ExecError> {
    let t = target.trim();
    if t.is_empty() {
        return Err(ExecError::invalid("empty dd target".to_string()));
    }
    if !t.starts_with("/dev/block/") {
        return Err(ExecError::invalid(
            "dd target must be under /dev/block/".to_string(),
        ));
    }
    if t.contains(' ') || t.contains(';') {
        return Err(ExecError::refused("refuse dd target".to_string()));
    }
    Ok(format!(
        "dd if={t} of=/sdcard/recovery_ramdisk_dump.img bs=4096 count=8192 2>&1; echo EXIT:$?"
    ))
}

// ------------------------------------------------------------ verdict

/// Flash output verdict, same rules as scripts and treble_core.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FlashVerdict {
    /// N OKAY lines plus Finished time.
    Ok { okay: usize, total_time: String },
    /// FAILED lines veto all, first lines kept.
    Failed { lines: Vec<String> },
    /// No FAILED yet no Finished, verify by hand.
    Unclear,
}

impl FlashVerdict {
    /// True for Ok.
    pub fn is_ok(&self) -> bool {
        matches!(self, Self::Ok { .. })
    }

    /// Short id OK, FAILED, UNCLEAR.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Ok { .. } => "OK",
            Self::Failed { .. } => "FAILED",
            Self::Unclear => "UNCLEAR",
        }
    }
}

fn is_failed_line(line: &str) -> bool {
    let low = line.to_lowercase();
    if low.contains("failed") || low.contains("remote:") {
        return true;
    }
    let t = line.trim_start().to_lowercase();
    t.starts_with("error")
}

fn total_time_of(line: &str) -> Option<String> {
    let low = line.to_lowercase();
    let pos = low.find("total time:")?;
    let start = pos + "total time:".len();
    let raw = line.get(start..)?;
    Some(raw.trim().to_string())
}

/// Evaluate fastboot output lines, FAILED vetoes all.
pub fn flash_verdict<S: AsRef<str>>(lines: &[S]) -> FlashVerdict {
    let mut okay = 0usize;
    let mut failed: Vec<String> = Vec::new();
    let mut total = String::new();
    for item in lines.iter() {
        let line = item.as_ref();
        if line.contains("OKAY") {
            okay += 1;
        }
        if is_failed_line(line) && failed.len() < 3 {
            failed.push(line.trim().to_string());
        }
        if total.is_empty() {
            if let Some(t) = total_time_of(line) {
                total = t;
            }
        }
    }
    if !failed.is_empty() {
        return FlashVerdict::Failed { lines: failed };
    }
    if okay > 0 && !total.is_empty() {
        return FlashVerdict::Ok {
            okay,
            total_time: total,
        };
    }
    FlashVerdict::Unclear
}

// ------------------------------------------------------------ runners

/// Typed run report for one destructive op.
#[derive(Debug, Clone)]
pub struct RunReport {
    /// Op id: safe-flash, system-flash, twrp-flash, restore, wipe.
    pub op: String,
    /// Target partition or userdata.
    pub partition: String,
    /// Image path or empty for wipe.
    pub image: String,
    /// Executed fastboot argv.
    pub argv: Vec<String>,
    /// Raw tool output.
    pub output: ExecOut,
    /// Parsed verdict.
    pub verdict: FlashVerdict,
    /// True only for Ok verdict.
    pub success: bool,
}

impl RunReport {
    /// True when verdict is Ok.
    pub fn is_ok(&self) -> bool {
        self.success
    }
}

fn first_ok(want: &str, given: &str) -> bool {
    let w = want.trim();
    let g = given.trim();
    if w == "FLASH" {
        return g == "FLASH" || g == "FLASHEN";
    }
    if w == "RESTORE" {
        return g == "RESTORE";
    }
    if w == "WIPE" {
        return g == "WIPE";
    }
    if w == "SET" {
        return g == "SET";
    }
    w == g
}

fn second_ok(want: &str, given: &str) -> bool {
    let w = want.trim();
    let g = given.trim();
    if w == "YES" {
        return g == "YES" || g == "JA";
    }
    w == g
}

fn plan_tokens_ok(
    plan: &gsi_workflow::DestructivePlan,
    first: &str,
    second: &str,
) -> Result<(), ExecError> {
    if !first_ok(plan.first_word.as_str(), first) {
        return Err(ExecError::refused(format!(
            "missing first token '{}'",
            plan.first_word.as_str()
        )));
    }
    if !second_ok(plan.second_word.as_str(), second) {
        return Err(ExecError::refused(format!(
            "missing second token '{}'",
            plan.second_word.as_str()
        )));
    }
    if !plan.confirm_first || !plan.confirm_second {
        return Err(ExecError::refused(
            "plan confirms are off, refuse exec".to_string(),
        ));
    }
    if !plan.ready {
        let mut why = String::from("plan not ready");
        if let Some(b) = plan.blocks.first() {
            why.push_str(": ");
            why.push_str(b.as_str());
        }
        return Err(ExecError::refused(why));
    }
    Ok(())
}

/// Execute a workflow destructive plan, refuse before any call on bad tokens.
pub fn run_plan(
    plan: &gsi_workflow::DestructivePlan,
    exec: &mut dyn Executor,
    first: &str,
    second: &str,
) -> Result<RunReport, ExecError> {
    plan_tokens_ok(plan, first, second)?;
    let op = plan.op.trim().to_string();
    if op == "wipe" {
        if plan.partition.trim() != "userdata" {
            return Err(ExecError::invalid(
                "wipe partition must be userdata".to_string(),
            ));
        }
        let argv = erase_userdata_args();
        check_fastboot_args(&argv)?;
        let out = exec.run_fastboot(&argv);
        let lines = out.lines();
        let verdict = flash_verdict(lines.as_slice());
        let ok = verdict.is_ok();
        return Ok(RunReport {
            op,
            partition: "userdata".to_string(),
            image: String::new(),
            argv,
            output: out,
            verdict,
            success: ok,
        });
    }
    if op == "safe-flash" || op == "twrp-flash" {
        let part = Partition::parse(plan.partition.trim())?;
        match part {
            Partition::RecoveryRamdisk => {}
            Partition::System => {
                return Err(ExecError::refused(
                    "safe path refuses system target".to_string(),
                ));
            }
        }
        let argv = flash_args(part, plan.image.trim(), None)?;
        let out = exec.run_fastboot(&argv);
        let lines = out.lines();
        let verdict = flash_verdict(lines.as_slice());
        let ok = verdict.is_ok();
        return Ok(RunReport {
            op,
            partition: part.as_str().to_string(),
            image: plan.image.trim().to_string(),
            argv,
            output: out,
            verdict,
            success: ok,
        });
    }
    if op == "system-flash" {
        let ov = SystemOverride::confirm(second)?;
        let argv = flash_args(Partition::System, plan.image.trim(), Some(&ov))?;
        let out = exec.run_fastboot(&argv);
        let lines = out.lines();
        let verdict = flash_verdict(lines.as_slice());
        let ok = verdict.is_ok();
        return Ok(RunReport {
            op,
            partition: "system".to_string(),
            image: plan.image.trim().to_string(),
            argv,
            output: out,
            verdict,
            success: ok,
        });
    }
    if op == "restore" {
        let raw = plan.partition.trim();
        if raw.is_empty() {
            return Err(ExecError::invalid("restore needs partition".to_string()));
        }
        if raw == "userdata" {
            return Err(ExecError::refused(
                "restore refuses userdata target".to_string(),
            ));
        }
        let part = Partition::parse(raw)?;
        let argv = match part {
            Partition::RecoveryRamdisk => flash_args(part, plan.image.trim(), None)?,
            Partition::System => {
                let ov = SystemOverride::confirm(second)?;
                flash_args(part, plan.image.trim(), Some(&ov))?
            }
        };
        let out = exec.run_fastboot(&argv);
        let lines = out.lines();
        let verdict = flash_verdict(lines.as_slice());
        let ok = verdict.is_ok();
        return Ok(RunReport {
            op,
            partition: part.as_str().to_string(),
            image: plan.image.trim().to_string(),
            argv,
            output: out,
            verdict,
            success: ok,
        });
    }
    Err(ExecError::invalid(format!("unknown op '{op}'")))
}

/// Run safe-flash plan, checks op id first.
pub fn run_safe_flash(
    plan: &gsi_workflow::DestructivePlan,
    exec: &mut dyn Executor,
    first: &str,
    second: &str,
) -> Result<RunReport, ExecError> {
    if plan.op.trim() != "safe-flash" {
        return Err(ExecError::invalid("not a safe-flash plan".to_string()));
    }
    run_plan(plan, exec, first, second)
}

/// Run system-flash plan, checks op id first.
pub fn run_system_flash(
    plan: &gsi_workflow::DestructivePlan,
    exec: &mut dyn Executor,
    first: &str,
    second: &str,
) -> Result<RunReport, ExecError> {
    if plan.op.trim() != "system-flash" {
        return Err(ExecError::invalid("not a system-flash plan".to_string()));
    }
    run_plan(plan, exec, first, second)
}

/// Run twrp-flash plan, checks op id first.
pub fn run_twrp_flash(
    plan: &gsi_workflow::DestructivePlan,
    exec: &mut dyn Executor,
    first: &str,
    second: &str,
) -> Result<RunReport, ExecError> {
    if plan.op.trim() != "twrp-flash" {
        return Err(ExecError::invalid("not a twrp-flash plan".to_string()));
    }
    run_plan(plan, exec, first, second)
}

/// Run restore plan, checks op id first.
pub fn run_restore(
    plan: &gsi_workflow::DestructivePlan,
    exec: &mut dyn Executor,
    first: &str,
    second: &str,
) -> Result<RunReport, ExecError> {
    if plan.op.trim() != "restore" {
        return Err(ExecError::invalid("not a restore plan".to_string()));
    }
    run_plan(plan, exec, first, second)
}

/// Run guided-wipe plan, checks op id first.
pub fn run_guided_wipe(
    plan: &gsi_workflow::DestructivePlan,
    exec: &mut dyn Executor,
    first: &str,
    second: &str,
) -> Result<RunReport, ExecError> {
    if plan.op.trim() != "wipe" {
        return Err(ExecError::invalid("not a wipe plan".to_string()));
    }
    run_plan(plan, exec, first, second)
}

// ------------------------------------------------------------ validate

/// One read-only check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationCheck {
    /// Check name.
    pub name: String,
    /// True for pass.
    pub pass: bool,
    /// Detail text.
    pub detail: String,
}

/// Read-only post-flash report.
#[derive(Debug, Clone)]
pub struct ValidationReport {
    /// Ordered checks.
    pub checks: Vec<ValidationCheck>,
    /// True when all pass.
    pub ok: bool,
}

impl ValidationReport {
    /// True when all pass.
    pub fn is_ok(&self) -> bool {
        self.ok
    }
}

fn check_item(name: &str, pass: bool, detail: &str) -> ValidationCheck {
    ValidationCheck {
        name: name.to_string(),
        pass,
        detail: detail.to_string(),
    }
}

/// Run read-only validate sequence over adb shell builders.
pub fn run_validate(exec: &mut dyn Executor) -> ValidationReport {
    let mut checks: Vec<ValidationCheck> = Vec::new();
    // ADB presence is modeled by devices call.
    let dev_argv = adb_devices_args(false);
    let dev_out = exec.run_adb(dev_argv.as_slice());
    let dev_text = dev_out.combined();
    let has_device = dev_text.contains("device") && !dev_text.trim().is_empty();
    // When no device, single ADB row fails and sequence stops.
    if !has_device && dev_text.trim().is_empty() {
        checks.push(check_item("ADB", false, "no android device"));
        return ValidationReport { checks, ok: false };
    }
    // Serial is first token when present.
    let mut serial = String::new();
    for line in dev_text.lines() {
        let t = line.trim();
        if t.is_empty() || t.starts_with("List") {
            continue;
        }
        let mut parts = t.split_whitespace();
        if let Some(s) = parts.next() {
            if !s.is_empty() {
                serial = s.to_string();
                break;
            }
        }
    }
    if serial.is_empty() {
        serial = "device".to_string();
    }
    checks.push(check_item("ADB", true, serial.as_str()));
    // OS release plus display.
    let mut rel = String::new();
    let mut disp = String::new();
    if let Ok(cmd) = shell_getprop("ro.build.version.release") {
        let o = exec.run_adb_shell(cmd.as_str());
        rel = o.stdout.trim().to_string();
        if rel.is_empty() {
            rel = o.combined().trim().to_string();
        }
    }
    if let Ok(cmd) = shell_getprop("ro.build.display.id") {
        let o = exec.run_adb_shell(cmd.as_str());
        disp = o.stdout.trim().to_string();
        if disp.is_empty() {
            disp = o.combined().trim().to_string();
        }
    }
    if rel.is_empty() {
        checks.push(check_item("OS", false, "empty"));
    } else {
        let mut d = rel.clone();
        d.push_str(" / ");
        d.push_str(disp.as_str());
        checks.push(check_item("OS", true, d.as_str()));
    }
    // SELinux.
    {
        let o = exec.run_adb_shell(shell_getenforce().as_str());
        let v = o.combined().trim().to_string();
        if v.is_empty() {
            checks.push(check_item("SELinux", false, "empty"));
        } else {
            checks.push(check_item("SELinux", true, v.as_str()));
        }
    }
    // ROOT via su id.
    {
        let o = exec.run_adb_shell(shell_su_id().as_str());
        let v = o.combined();
        if is_uid_root(v.as_str()) {
            checks.push(check_item("ROOT", true, v.trim()));
        } else {
            checks.push(check_item("ROOT", false, v.trim()));
        }
    }
    // Mounts.
    {
        let o = exec.run_adb_shell(shell_mount().as_str());
        let v = o.combined();
        if v.contains("/system") {
            checks.push(check_item("MOUNTS-system", true, "mounted"));
        } else {
            checks.push(check_item("MOUNTS-system", false, "missing"));
        }
        if v.contains("/vendor") {
            checks.push(check_item("MOUNTS-vendor", true, "mounted"));
        } else {
            checks.push(check_item("MOUNTS-vendor", false, "missing"));
        }
    }
    // WIFI.
    {
        let o = exec.run_adb_shell(shell_dumpsys_wifi().as_str());
        let v = o.combined();
        let mut hit = String::new();
        for line in v.lines() {
            if line.to_lowercase().contains("wi-fi is") || line.to_lowercase().contains("wifi is") {
                hit = line.trim().to_string();
                break;
            }
        }
        if hit.is_empty() && !v.trim().is_empty() {
            let first_line = v.lines().next().unwrap_or("");
            hit = first_line.trim().to_string();
        }
        if hit.is_empty() {
            checks.push(check_item("WIFI", false, "empty"));
        } else {
            checks.push(check_item("WIFI", true, hit.as_str()));
        }
    }
    // Bluetooth.
    {
        let o = exec.run_adb_shell(shell_bluetooth_state().as_str());
        let v = o.combined().trim().to_string();
        if v == "1" || v == "0" {
            let mut d = String::from("state=");
            d.push_str(v.as_str());
            checks.push(check_item("BLUETOOTH", true, d.as_str()));
        } else {
            checks.push(check_item("BLUETOOTH", false, v.as_str()));
        }
    }
    // Battery.
    {
        let o = exec.run_adb_shell(shell_dumpsys_battery().as_str());
        let v = o.combined();
        let mut hit = String::new();
        for line in v.lines() {
            if line.to_lowercase().contains("level") {
                hit = line.trim().to_string();
                break;
            }
        }
        if hit.is_empty() {
            checks.push(check_item("BATTERY", false, "empty"));
        } else {
            checks.push(check_item("BATTERY", true, hit.as_str()));
        }
    }
    // Sensors.
    {
        let o = exec.run_adb_shell(shell_dumpsys_sensors().as_str());
        let v = o.combined();
        let mut n = 0usize;
        for line in v.lines() {
            if line.to_lowercase().contains("sensor") {
                n += 1;
            }
        }
        if n > 0 {
            checks.push(check_item("SENSORS", true, format!("entries={n}").as_str()));
        } else {
            checks.push(check_item("SENSORS", false, "none"));
        }
    }
    let mut ok = true;
    for c in checks.iter() {
        if !c.pass {
            ok = false;
            break;
        }
    }
    ValidationReport { checks, ok }
}

// ------------------------------------------------------------ verify

/// Root verify verdict.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerifyVerdict {
    /// su id shows uid=0.
    Rooted,
    /// su present yet no uid=0.
    Inconclusive,
    /// No root sign.
    NotRooted,
}

impl VerifyVerdict {
    /// Uppercase id.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Rooted => "ROOTED",
            Self::Inconclusive => "INCONCLUSIVE",
            Self::NotRooted => "NOT_ROOTED",
        }
    }
}

/// True when su id output holds uid=0.
pub fn is_uid_root(su_id: &str) -> bool {
    su_id.contains("uid=0")
}

/// Pure classifier for verify transcripts.
pub fn classify_verify_root(which_su: &str, su_id: &str) -> VerifyVerdict {
    if is_uid_root(su_id) {
        return VerifyVerdict::Rooted;
    }
    let w = which_su.trim();
    if !w.is_empty() && !w.contains("not found") && !w.contains("No such") {
        return VerifyVerdict::Inconclusive;
    }
    VerifyVerdict::NotRooted
}

/// Live verify report.
#[derive(Debug, Clone)]
pub struct VerifyReport {
    /// which su text.
    pub which_su: String,
    /// su id text.
    pub su_id: String,
    /// magisk version text.
    pub magisk_v: String,
    /// Verdict.
    pub verdict: VerifyVerdict,
}

/// Run verify sequence: which su, su id, magisk version.
pub fn run_verify_root(exec: &mut dyn Executor) -> VerifyReport {
    let w_out = exec.run_adb_shell(shell_which_su().as_str());
    let w = w_out.combined();
    let id_out = exec.run_adb_shell(shell_su_id().as_str());
    let id = id_out.combined();
    let m_out = exec.run_adb_shell(shell_magisk_version().as_str());
    let m = m_out.combined();
    let v = classify_verify_root(w.as_str(), id.as_str());
    VerifyReport {
        which_su: w,
        su_id: id,
        magisk_v: m,
        verdict: v,
    }
}

// ------------------------------------------------------------ persist

/// Persist install report.
#[derive(Debug, Clone)]
pub struct PersistReport {
    /// Installed file names.
    pub installed: Vec<String>,
    /// True when all files verified.
    pub ok: bool,
}

/// Run persist install: root gate plus push plus shell copy.
pub fn run_persist_fixes(exec: &mut dyn Executor) -> Result<PersistReport, ExecError> {
    let id_out = exec.run_adb_shell(shell_su_id().as_str());
    let id = id_out.combined();
    if !is_uid_root(id.as_str()) {
        return Err(ExecError::refused("no live root, need uid=0".to_string()));
    }
    // Immediate relief, result is info only.
    let _ = exec.run_adb_shell(shell_stop_aptouch().as_str());
    let plan = gsi_workflow::plan_persist_fixes();
    let mut installed: Vec<String> = Vec::new();
    for f in plan.files.iter() {
        let name = f.name.trim().to_string();
        if name.is_empty() {
            return Err(ExecError::invalid("empty persist name".to_string()));
        }
        // Push local temp path to sdcard path.
        let mut local = String::from("/tmp/");
        local.push_str(name.as_str());
        let mut remote = String::from("/sdcard/Download/");
        remote.push_str(name.as_str());
        let push = adb_push_args(local.as_str(), remote.as_str())?;
        let _ = exec.run_adb(push.as_slice());
        // Copy into service.d.
        let cp = shell_persist_cp(name.as_str())?;
        let _ = exec.run_adb_shell(cp.as_str());
        // Verify presence.
        let ls = shell_persist_ls(name.as_str())?;
        let chk = exec.run_adb_shell(ls.as_str());
        let text = chk.combined();
        if text.contains(name.as_str()) {
            installed.push(name);
        } else {
            return Err(ExecError::invalid(format!("persist verify lost {name}")));
        }
    }
    let ok = installed.len() == plan.files.len();
    Ok(PersistReport { installed, ok })
}

// ------------------------------------------------------------ goal orchestrator

/// Goal-level confirm tokens, gate for whole goal run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoalConfirms {
    /// First token text.
    pub first: String,
    /// Second token text.
    pub second: String,
}

impl GoalConfirms {
    /// Build from two token texts.
    pub fn new(first: &str, second: &str) -> Self {
        Self {
            first: first.to_string(),
            second: second.to_string(),
        }
    }

    /// Empty tokens, always treated as missing.
    pub fn missing() -> Self {
        Self {
            first: String::new(),
            second: String::new(),
        }
    }

    /// True when either token is blank.
    pub fn is_missing(&self) -> bool {
        self.first.trim().is_empty() || self.second.trim().is_empty()
    }
}

/// One executed goal step.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoalStepOutcome {
    /// Step id from plan.
    pub step: String,
    /// Runner id: safe-flash, system-flash, twrp-flash, restore, wipe,
    /// validate, verify, persist, info, plan.
    pub op: String,
    /// Status: ok, refused, failed, skipped.
    pub status: String,
    /// Detail: verdict id or reason text.
    pub detail: String,
}

/// Aggregated goal report.
#[derive(Debug, Clone)]
pub struct GoalReport {
    /// Normalized goal id.
    pub goal: String,
    /// Per-step outcomes in plan order.
    pub steps: Vec<GoalStepOutcome>,
    /// True when no refused and no failed steps.
    pub ok: bool,
}

impl GoalReport {
    /// True when all steps ok or skipped.
    pub fn is_ok(&self) -> bool {
        self.ok
    }
}

fn goal_outcome(step: &str, op: &str, status: &str, detail: &str) -> GoalStepOutcome {
    GoalStepOutcome {
        step: step.to_string(),
        op: op.to_string(),
        status: status.to_string(),
        detail: detail.to_string(),
    }
}

fn goal_op_for(step_id: &str) -> &'static str {
    let s = step_id.trim().to_lowercase();
    let b = s.as_str();
    if b == "flash_system" || b == "rom_flash" || b == "rom_installation" {
        return "system-flash";
    }
    if b == "recovery_flash" {
        return "twrp-flash";
    }
    if b == "flash" || b == "root_flash" {
        return "safe-flash";
    }
    if b == "restore" || b == "rollback_plan" || b == "identify_original_artifact" {
        return "restore";
    }
    if b.contains("wipe") {
        return "wipe";
    }
    if b.contains("persist") {
        return "persist";
    }
    if b == "validate_backup" {
        return "restore";
    }
    if b.contains("validat") {
        return "validate";
    }
    if b.contains("verify") {
        return "verify";
    }
    "info"
}

fn goal_safe_plan() -> gsi_workflow::DestructivePlan {
    gsi_workflow::plan_safe_flash(
        "goal-safe.img",
        "recovery_ramdisk",
        true,
        true,
        true,
        true,
        true,
    )
}

fn goal_system_plan() -> gsi_workflow::DestructivePlan {
    gsi_workflow::plan_system_flash("goal-system.img", true, true, true, false, true, true)
}

fn goal_twrp_plan() -> gsi_workflow::DestructivePlan {
    gsi_workflow::plan_twrp_flash(
        "goal-twrp.img",
        "recovery_ramdisk",
        true,
        true,
        true,
        true,
        true,
    )
}

fn goal_restore_plan() -> gsi_workflow::DestructivePlan {
    gsi_workflow::plan_restore("backup", "recovery_ramdisk", true, true, true, true)
}

fn goal_wipe_plan() -> gsi_workflow::DestructivePlan {
    gsi_workflow::plan_guided_wipe(true, true, true)
}

fn goal_refused_detail(err: &ExecError) -> (String, String) {
    match err {
        ExecError::Refused(s) => ("refused".to_string(), s.clone()),
        ExecError::Invalid(s) => ("failed".to_string(), s.clone()),
    }
}

/// Run a goal plan stepwise over injected executor.
///
/// Loads ordered plan via gsi-state, then dispatches each Ready step to
/// the matching runner from this crate. Refused plan steps stay refused
/// with reasons and make no calls. Missing goal-level confirms refuse
/// the whole goal before any call, so stub call count stays zero.
/// Read-only info steps are marked skipped with no calls.
pub fn run_goal(goal_id: &str, exec: &mut dyn Executor, confirms: &GoalConfirms) -> GoalReport {
    let plan = match gsi_state::new_workflow_plan(goal_id) {
        Ok(p) => p,
        Err(e) => {
            return GoalReport {
                goal: goal_id.trim().to_string(),
                steps: vec![goal_outcome("plan", "plan", "refused", e.as_str())],
                ok: false,
            };
        }
    };
    let goal_name = plan.goal.clone();
    if confirms.is_missing() {
        let mut steps: Vec<GoalStepOutcome> = Vec::new();
        for s in plan.steps.iter() {
            let op = goal_op_for(s.id.as_str());
            match &s.disposition {
                gsi_state::StepDisposition::Refused(reason) => {
                    steps.push(goal_outcome(s.id.as_str(), op, "refused", reason.as_str()));
                }
                gsi_state::StepDisposition::Ready => {
                    steps.push(goal_outcome(
                        s.id.as_str(),
                        op,
                        "refused",
                        "missing confirms",
                    ));
                }
            }
        }
        return GoalReport {
            goal: goal_name,
            steps,
            ok: false,
        };
    }
    let mut steps: Vec<GoalStepOutcome> = Vec::new();
    for s in plan.steps.iter() {
        match &s.disposition {
            gsi_state::StepDisposition::Refused(reason) => {
                let op = goal_op_for(s.id.as_str());
                steps.push(goal_outcome(s.id.as_str(), op, "refused", reason.as_str()));
            }
            gsi_state::StepDisposition::Ready => {
                let op = goal_op_for(s.id.as_str());
                if op == "safe-flash" {
                    let p = goal_safe_plan();
                    match run_safe_flash(&p, exec, "FLASH", "YES") {
                        Ok(r) => {
                            if r.success {
                                steps.push(goal_outcome(
                                    s.id.as_str(),
                                    op,
                                    "ok",
                                    r.verdict.as_str(),
                                ));
                            } else {
                                steps.push(goal_outcome(
                                    s.id.as_str(),
                                    op,
                                    "failed",
                                    r.verdict.as_str(),
                                ));
                            }
                        }
                        Err(e) => {
                            let (st, d) = goal_refused_detail(&e);
                            steps.push(goal_outcome(s.id.as_str(), op, st.as_str(), d.as_str()));
                        }
                    }
                } else if op == "system-flash" {
                    let p = goal_system_plan();
                    match run_system_flash(&p, exec, "FLASH", "YES") {
                        Ok(r) => {
                            if r.success {
                                steps.push(goal_outcome(
                                    s.id.as_str(),
                                    op,
                                    "ok",
                                    r.verdict.as_str(),
                                ));
                            } else {
                                steps.push(goal_outcome(
                                    s.id.as_str(),
                                    op,
                                    "failed",
                                    r.verdict.as_str(),
                                ));
                            }
                        }
                        Err(e) => {
                            let (st, d) = goal_refused_detail(&e);
                            steps.push(goal_outcome(s.id.as_str(), op, st.as_str(), d.as_str()));
                        }
                    }
                } else if op == "twrp-flash" {
                    let p = goal_twrp_plan();
                    match run_twrp_flash(&p, exec, "FLASH", "YES") {
                        Ok(r) => {
                            if r.success {
                                steps.push(goal_outcome(
                                    s.id.as_str(),
                                    op,
                                    "ok",
                                    r.verdict.as_str(),
                                ));
                            } else {
                                steps.push(goal_outcome(
                                    s.id.as_str(),
                                    op,
                                    "failed",
                                    r.verdict.as_str(),
                                ));
                            }
                        }
                        Err(e) => {
                            let (st, d) = goal_refused_detail(&e);
                            steps.push(goal_outcome(s.id.as_str(), op, st.as_str(), d.as_str()));
                        }
                    }
                } else if op == "restore" {
                    let p = goal_restore_plan();
                    match run_restore(&p, exec, "RESTORE", "YES") {
                        Ok(r) => {
                            if r.success {
                                steps.push(goal_outcome(
                                    s.id.as_str(),
                                    op,
                                    "ok",
                                    r.verdict.as_str(),
                                ));
                            } else {
                                steps.push(goal_outcome(
                                    s.id.as_str(),
                                    op,
                                    "failed",
                                    r.verdict.as_str(),
                                ));
                            }
                        }
                        Err(e) => {
                            let (st, d) = goal_refused_detail(&e);
                            steps.push(goal_outcome(s.id.as_str(), op, st.as_str(), d.as_str()));
                        }
                    }
                } else if op == "wipe" {
                    let p = goal_wipe_plan();
                    match run_guided_wipe(&p, exec, "WIPE", "YES") {
                        Ok(r) => {
                            if r.success {
                                steps.push(goal_outcome(
                                    s.id.as_str(),
                                    op,
                                    "ok",
                                    r.verdict.as_str(),
                                ));
                            } else {
                                steps.push(goal_outcome(
                                    s.id.as_str(),
                                    op,
                                    "failed",
                                    r.verdict.as_str(),
                                ));
                            }
                        }
                        Err(e) => {
                            let (st, d) = goal_refused_detail(&e);
                            steps.push(goal_outcome(s.id.as_str(), op, st.as_str(), d.as_str()));
                        }
                    }
                } else if op == "validate" {
                    let rep = run_validate(exec);
                    if rep.is_ok() {
                        let mut d = String::from("all pass ");
                        d.push_str(rep.checks.len().to_string().as_str());
                        steps.push(goal_outcome(s.id.as_str(), op, "ok", d.as_str()));
                    } else {
                        let mut bad = String::new();
                        for c in rep.checks.iter() {
                            if !c.pass {
                                bad = c.name.clone();
                                break;
                            }
                        }
                        if bad.is_empty() {
                            bad = "check failed".to_string();
                        }
                        steps.push(goal_outcome(s.id.as_str(), op, "failed", bad.as_str()));
                    }
                } else if op == "verify" {
                    let rep = run_verify_root(exec);
                    if rep.verdict == VerifyVerdict::Rooted {
                        steps.push(goal_outcome(s.id.as_str(), op, "ok", "ROOTED"));
                    } else {
                        steps.push(goal_outcome(
                            s.id.as_str(),
                            op,
                            "failed",
                            rep.verdict.as_str(),
                        ));
                    }
                } else if op == "persist" {
                    match run_persist_fixes(exec) {
                        Ok(r) => {
                            if r.ok {
                                let mut d = String::from("installed ");
                                d.push_str(r.installed.len().to_string().as_str());
                                steps.push(goal_outcome(s.id.as_str(), op, "ok", d.as_str()));
                            } else {
                                steps.push(goal_outcome(s.id.as_str(), op, "failed", "partial"));
                            }
                        }
                        Err(e) => {
                            let (st, d) = goal_refused_detail(&e);
                            steps.push(goal_outcome(s.id.as_str(), op, st.as_str(), d.as_str()));
                        }
                    }
                } else {
                    steps.push(goal_outcome(
                        s.id.as_str(),
                        "info",
                        "skipped",
                        "informational/manual step",
                    ));
                }
            }
        }
    }
    let mut ok = true;
    for o in steps.iter() {
        if o.status == "refused" || o.status == "failed" {
            ok = false;
            break;
        }
    }
    GoalReport {
        goal: goal_name,
        steps,
        ok,
    }
}

// ------------------------------------------------------------ developer dump

/// One text bundle entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DumpEntry {
    /// Entry name: byname, partitions, getprop, vendor, mounts, logcat.
    pub name: String,
    /// Captured text.
    pub content: String,
}

/// Developer dump bundle.
#[derive(Debug, Clone)]
pub struct DeveloperDumpReport {
    /// Normalized kind.
    pub kind: String,
    /// Ordered text entries.
    pub entries: Vec<DumpEntry>,
}

fn dump_entry(name: &str, content: &str) -> DumpEntry {
    DumpEntry {
        name: name.to_string(),
        content: content.to_string(),
    }
}

fn normalize_dump_kind(kind: &str) -> Result<String, ExecError> {
    let k = kind.trim().to_lowercase();
    if k == "dump-partitions" || k == "partitions" {
        return Ok("dump-partitions".to_string());
    }
    if k == "dump-properties" || k == "properties" {
        return Ok("dump-properties".to_string());
    }
    if k == "dump-vendor" || k == "vendor" {
        return Ok("dump-vendor".to_string());
    }
    if k == "dump-logs" || k == "logs" {
        return Ok("dump-logs".to_string());
    }
    Err(ExecError::invalid(format!("unknown dump kind '{kind}'")))
}

/// Run read-only developer dump over adb shell builders.
///
/// Kinds: dump-partitions, dump-properties, dump-vendor, dump-logs
/// (short forms partitions, properties, vendor, logs also work).
/// Uses shell builders for by-name, partitions, getprop, vendor,
/// mount and logcat. Returns text bundle entries.
pub fn run_developer_dump(
    kind: &str,
    exec: &mut dyn Executor,
) -> Result<DeveloperDumpReport, ExecError> {
    let norm = normalize_dump_kind(kind)?;
    if norm == "dump-partitions" {
        let a_out = exec.run_adb_shell(shell_byname_listing().as_str());
        let b_out = exec.run_adb_shell(shell_cat_partitions().as_str());
        return Ok(DeveloperDumpReport {
            kind: norm,
            entries: vec![
                dump_entry("byname", a_out.combined().as_str()),
                dump_entry("partitions", b_out.combined().as_str()),
            ],
        });
    }
    if norm == "dump-properties" {
        let o = exec.run_adb_shell(shell_getprop_all().as_str());
        return Ok(DeveloperDumpReport {
            kind: norm,
            entries: vec![dump_entry("getprop", o.combined().as_str())],
        });
    }
    if norm == "dump-vendor" {
        let a_out = exec.run_adb_shell(shell_cat_vendor().as_str());
        let b_out = exec.run_adb_shell(shell_mount().as_str());
        return Ok(DeveloperDumpReport {
            kind: norm,
            entries: vec![
                dump_entry("vendor", a_out.combined().as_str()),
                dump_entry("mounts", b_out.combined().as_str()),
            ],
        });
    }
    if norm == "dump-logs" {
        let o = exec.run_adb_shell(shell_logcat().as_str());
        return Ok(DeveloperDumpReport {
            kind: norm,
            entries: vec![dump_entry("logcat", o.combined().as_str())],
        });
    }
    Err(ExecError::invalid(format!("unknown dump kind '{kind}'")))
}

// ------------------------------------------------------------ magisk staging

/// Stage a copy for on-device Magisk patching.
///
/// Copies src image into staging dir under its own file name and
/// returns the staged path. Refuses when src is missing or when
/// dest equals src. Creates staging dir as needed.
pub fn prepare_magisk_patch(src_image: &Path, staging_dir: &Path) -> Result<PathBuf, ExecError> {
    if src_image.as_os_str().is_empty() {
        return Err(ExecError::invalid("empty src image path".to_string()));
    }
    if staging_dir.as_os_str().is_empty() {
        return Err(ExecError::invalid("empty staging dir".to_string()));
    }
    let meta = match std::fs::metadata(src_image) {
        Ok(m) => m,
        Err(e) => {
            return Err(ExecError::refused(format!("missing src image: {e}")));
        }
    };
    if !meta.is_file() {
        return Err(ExecError::refused("src image is not a file".to_string()));
    }
    let name = match src_image.file_name() {
        Some(n) => n,
        None => {
            return Err(ExecError::invalid("no file name in src".to_string()));
        }
    };
    let dest = staging_dir.join(name);
    if dest.as_path() == src_image {
        return Err(ExecError::refused(
            "src equals dest, refuse copy".to_string(),
        ));
    }
    match std::fs::create_dir_all(staging_dir) {
        Ok(()) => {}
        Err(e) => {
            return Err(ExecError::invalid(format!("mkdir staging: {e}")));
        }
    }
    match std::fs::copy(src_image, dest.as_path()) {
        Ok(_) => {}
        Err(e) => {
            return Err(ExecError::invalid(format!("copy staged: {e}")));
        }
    }
    Ok(dest)
}

/// Exact on-device Magisk patch steps, mirrors script wording.
pub fn magisk_patch_instructions() -> String {
    let mut s = String::new();
    s.push_str("Magisk patch (real, never copy = claim patched)\n");
    s.push_str("1. Install Magisk APK from official source:\n");
    s.push_str("   https://github.com/topjohnwu/Magisk/releases\n");
    s.push_str("2. Copy staged file to device:\n");
    s.push_str("   adb push \"<staged>\" /sdcard/Download/\n");
    s.push_str("3. On device (any OS, stock or GSI/custom):\n");
    s.push_str("   Open Magisk -> Install -> Select and Patch a File\n");
    s.push_str(
        "   -> select RECOVERY_RAMDIS(K).img (exactly this file, NOT boot.img/recovery.img)\n",
    );
    s.push_str("4. Result on device: /sdcard/Download/magisk_patched-*.img\n");
    s.push_str("   Copy back: adb pull /sdcard/Download/magisk_patched-XXXX.img data/magisk/\n");
    s.push_str("5. Validation: hash != stock, size plausible, header documented (hashes must differ; patched == base with identical hash is rejected, NO fake patch).\n");
    s.push_str("6. Boot rooted: Vol-Up + Power until Huawei logo, then release (Magisk boot cheat, not persistent).\n");
    s
}

// ------------------------------------------------------------ backup probe

/// Run backup dd probe over adb shell.
pub fn run_backup_probe(exec: &mut dyn Executor, target: &str) -> Result<ExecOut, ExecError> {
    let cmd = backup_dd_cmd(target)?;
    Ok(exec.run_adb_shell(cmd.as_str()))
}

// ------------------------------------------------------------ tests

#[cfg(test)]
mod tests {
    use super::*;

    fn flash_ok_text() -> String {
        "Sending 'recovery_ramdisk' (12345 KB) OKAY [ 1.0s]\nWriting 'recovery_ramdisk' OKAY [ 1.0s]\nFinished. Total time: 2.000s\n".to_string()
    }

    #[test]
    fn builder_golden_argv() {
        let v = flash_args(Partition::RecoveryRamdisk, "magisk.img", None);
        match v {
            Ok(a) => {
                assert_eq!(a.len(), 3);
                assert_eq!(a[0].as_str(), "flash");
                assert_eq!(a[1].as_str(), "recovery_ramdisk");
                assert_eq!(a[2].as_str(), "magisk.img");
            }
            Err(_) => panic!(),
        }
        let ov = SystemOverride::confirm("YES");
        match ov {
            Ok(ref o) => {
                let s = flash_args(Partition::System, "gsi.img", Some(o));
                match s {
                    Ok(a) => {
                        assert_eq!(a[1].as_str(), "system");
                        assert_eq!(a[2].as_str(), "gsi.img");
                    }
                    Err(_) => panic!(),
                }
            }
            Err(_) => panic!(),
        }
        assert_eq!(
            erase_userdata_args(),
            vec!["erase".to_string(), "userdata".to_string()]
        );
        assert_eq!(fastboot_reboot_args(), vec!["reboot".to_string()]);
        assert_eq!(
            adb_reboot_args(RebootTarget::Bootloader),
            vec!["reboot".to_string(), "bootloader".to_string()]
        );
        assert_eq!(
            adb_reboot_args(RebootTarget::System),
            vec!["reboot".to_string()]
        );
        let p = adb_push_args("/tmp/a.sh", "/sdcard/Download/a.sh");
        match p {
            Ok(a) => {
                assert_eq!(a[0].as_str(), "push");
                assert_eq!(a[2].as_str(), "/sdcard/Download/a.sh");
            }
            Err(_) => panic!(),
        }
        let g = shell_getprop("ro.build.version.release");
        match g {
            Ok(s) => assert_eq!(s.as_str(), "getprop ro.build.version.release"),
            Err(_) => panic!(),
        }
        assert_eq!(shell_su_id().as_str(), "su -c id");
        assert_eq!(shell_getenforce().as_str(), "getenforce");
        let dd = backup_dd_cmd("/dev/block/mmcblk0p30");
        match dd {
            Ok(s) => assert!(s.contains("dd if=/dev/block/mmcblk0p30")),
            Err(_) => panic!(),
        }
    }

    #[test]
    fn refusal_matrix() {
        // System needs override.
        let r = flash_args(Partition::System, "x.img", None);
        assert!(r.is_err());
        // Bad override token.
        let bad = SystemOverride::confirm("NO");
        assert!(bad.is_err());
        // Bad partition parse.
        assert!(Partition::parse("boot").is_err());
        assert!(Partition::parse("userdata").is_err());
        assert!(Partition::parse("").is_err());
        // Empty image.
        assert!(flash_args(Partition::RecoveryRamdisk, "  ", None).is_err());
        // Raw dangerous verbs.
        let cases: Vec<Vec<String>> = vec![
            vec!["erase".to_string(), "system".to_string()],
            vec!["format".to_string(), "userdata".to_string()],
            vec!["flashing".to_string(), "unlock".to_string()],
            vec!["oem".to_string(), "unlock".to_string()],
            vec![
                "flash".to_string(),
                "recovery_ramdisk".to_string(),
                "--force".to_string(),
            ],
            vec!["flash".to_string(), "boot".to_string(), "x.img".to_string()],
            vec!["erase".to_string(), "cache".to_string()],
        ];
        for c in cases {
            assert!(check_fastboot_args(&c).is_err());
        }
        // Allowed shapes pass.
        assert!(check_fastboot_args(&erase_userdata_args()).is_ok());
        assert!(check_fastboot_args(&fastboot_reboot_args()).is_ok());
        assert!(check_fastboot_args(&fastboot_devices_args()).is_ok());
        let gv = fastboot_getvar_args("product");
        assert!(gv.is_ok());
        // Push refuses relative remote.
        assert!(adb_push_args("/tmp/a", "rel/path").is_err());
        // Persist name with slash refused.
        assert!(shell_persist_cp("a/b").is_err());
        assert!(shell_persist_ls("").is_err());
        // dd target outside block refused.
        assert!(backup_dd_cmd("/sdcard/x").is_err());
        // getprop with shell chars refused.
        assert!(shell_getprop("a; rm").is_err());
    }

    #[test]
    fn stub_safe_flash_run_ok() {
        let plan = gsi_workflow::plan_safe_flash(
            "m.img",
            "recovery_ramdisk",
            true,
            true,
            true,
            true,
            true,
        );
        assert!(plan.ready);
        let mut stub =
            StubExecutor::with_outputs(vec![StubExecutor::ok_output(flash_ok_text().as_str())]);
        let r = run_plan(&plan, &mut stub, "FLASH", "YES");
        match r {
            Ok(rep) => {
                assert!(rep.success);
                assert!(rep.verdict.is_ok());
                assert_eq!(rep.argv[1].as_str(), "recovery_ramdisk");
                assert_eq!(stub.call_count(), 1);
                match stub.calls().first() {
                    Some(RecordedCall::Fastboot(a)) => {
                        assert_eq!(a[0].as_str(), "flash");
                    }
                    _ => panic!(),
                }
            }
            Err(_) => panic!(),
        }
    }

    #[test]
    fn stub_missing_confirms_zero_calls() {
        let plan = gsi_workflow::plan_safe_flash(
            "m.img",
            "recovery_ramdisk",
            true,
            true,
            true,
            false,
            false,
        );
        // Plan itself not ready, runner must refuse before any call.
        let mut stub = StubExecutor::new();
        let r = run_plan(&plan, &mut stub, "", "");
        assert!(r.is_err());
        assert_eq!(stub.call_count(), 0);
        // Ready plan yet wrong tokens also zero calls.
        let plan2 = gsi_workflow::plan_safe_flash(
            "m.img",
            "recovery_ramdisk",
            true,
            true,
            true,
            true,
            true,
        );
        let mut stub2 = StubExecutor::new();
        let r2 = run_plan(&plan2, &mut stub2, "NO", "NO");
        assert!(r2.is_err());
        assert_eq!(stub2.call_count(), 0);
        // Wipe missing second also zero calls.
        let wplan = gsi_workflow::plan_guided_wipe(true, true, true);
        let mut stub3 = StubExecutor::new();
        let r3 = run_plan(&wplan, &mut stub3, "WIPE", "");
        assert!(r3.is_err());
        assert_eq!(stub3.call_count(), 0);
    }

    #[test]
    fn verdict_mapping_failed_unclear() {
        let bad = vec![
            "Sending 'system' (1 KB) OKAY [ 0.1s]".to_string(),
            "Writing 'system' FAILED (remote: 'Command not allowed')".to_string(),
            "Finished. Total time: 0.010s".to_string(),
        ];
        let v = flash_verdict(bad.as_slice());
        match &v {
            FlashVerdict::Failed { lines } => {
                assert!(lines.iter().any(|l| l.contains("Command not allowed")));
            }
            _ => panic!(),
        }
        assert_eq!(v.as_str(), "FAILED");
        let prog = vec!["Erasing 'userdata' ...".to_string()];
        let u = flash_verdict(prog.as_slice());
        assert_eq!(u, FlashVerdict::Unclear);
        assert_eq!(u.as_str(), "UNCLEAR");
        let ok_lines = vec![
            "Erasing 'userdata' OKAY [  2.1s]".to_string(),
            "Finished. Total time: 2.150s".to_string(),
        ];
        let o = flash_verdict(ok_lines.as_slice());
        assert!(o.is_ok());
        assert_eq!(o.as_str(), "OK");
    }

    #[test]
    fn validate_sequence_scripted() {
        let mut stub = StubExecutor::with_outputs(vec![
            StubExecutor::ok_output("List of devices attached\nABC123\tdevice\n"),
            StubExecutor::ok_output("13\n"),
            StubExecutor::ok_output("trebledroid 13 arm64_bvN\n"),
            StubExecutor::ok_output("Enforcing\n"),
            StubExecutor::ok_output("uid=0(root) gid=0\n"),
            StubExecutor::ok_output(
                "/dev/block/system /system ext4\n/dev/block/vendor /vendor ext4\n",
            ),
            StubExecutor::ok_output("Wi-Fi is enabled\n"),
            StubExecutor::ok_output("1\n"),
            StubExecutor::ok_output("level: 88\n"),
            StubExecutor::ok_output("Sensor0\nSensor1\n"),
        ]);
        let rep = run_validate(&mut stub);
        assert!(rep.is_ok());
        assert!(rep.checks.len() >= 9);
        let mut found_root = false;
        for c in rep.checks.iter() {
            if c.name == "ROOT" && c.pass {
                found_root = true;
            }
        }
        assert!(found_root);
        // Classifier alone.
        assert_eq!(
            classify_verify_root("", "uid=0(root)"),
            VerifyVerdict::Rooted
        );
        assert_eq!(VerifyVerdict::Rooted.as_str(), "ROOTED");
        assert!(is_uid_root("uid=0"));
        assert!(!is_uid_root("uid=2000"));
    }

    #[test]
    fn verify_and_wipe_and_persist_shapes() {
        // Verify sequence.
        let mut stub = StubExecutor::with_outputs(vec![
            StubExecutor::ok_output("/system/xbin/su\n"),
            StubExecutor::ok_output("uid=0(root) gid=0 groups=0\n"),
            StubExecutor::ok_output("27.0:MAGISK\n"),
        ]);
        let vr = run_verify_root(&mut stub);
        assert_eq!(vr.verdict, VerifyVerdict::Rooted);
        assert_eq!(stub.call_count(), 3);
        // Wipe run ok.
        let wplan = gsi_workflow::plan_guided_wipe(true, true, true);
        let mut wstub = StubExecutor::with_outputs(vec![StubExecutor::ok_output(
            "Erasing 'userdata' OKAY [  2.1s]\nFinished. Total time: 2.150s\n",
        )]);
        let wrep = run_guided_wipe(&wplan, &mut wstub, "WIPE", "YES");
        match wrep {
            Ok(r) => {
                assert!(r.success);
                assert_eq!(r.argv, erase_userdata_args());
            }
            Err(_) => panic!(),
        }
        // System flash bilingual JA passes.
        let splan = gsi_workflow::plan_system_flash("gsi.img", true, true, true, false, true, true);
        let mut sstub =
            StubExecutor::with_outputs(vec![StubExecutor::ok_output(flash_ok_text().as_str())]);
        let srep = run_system_flash(&splan, &mut sstub, "FLASH", "JA");
        assert!(srep.is_ok());
        // Persist refuses without root, one gate call only.
        let mut pstub = StubExecutor::with_outputs(vec![StubExecutor::ok_output("uid=2000\n")]);
        let pr = run_persist_fixes(&mut pstub);
        assert!(pr.is_err());
        assert_eq!(pstub.call_count(), 1);
    }

    #[test]
    fn goal_missing_confirms_zero_calls() {
        let missing = GoalConfirms::missing();
        assert!(missing.is_missing());
        let mut stub = StubExecutor::new();
        let rep = run_goal("root", &mut stub, &missing);
        assert!(!rep.is_ok());
        assert_eq!(stub.call_count(), 0);
        assert!(!rep.steps.is_empty());
        let mut any_refused = false;
        for o in rep.steps.iter() {
            if o.status == "refused" {
                any_refused = true;
            }
        }
        assert!(any_refused);
        // Empty tokens also count as missing.
        let empty = GoalConfirms::new("", "");
        let mut stub2 = StubExecutor::new();
        let rep2 = run_goal("root", &mut stub2, &empty);
        assert!(!rep2.is_ok());
        assert_eq!(stub2.call_count(), 0);
    }

    #[test]
    fn goal_unknown_zero_calls() {
        let ok_c = GoalConfirms::new("FLASH", "YES");
        assert!(!ok_c.is_missing());
        let mut stub = StubExecutor::new();
        let rep = run_goal("no-such-goal", &mut stub, &ok_c);
        assert!(!rep.is_ok());
        assert_eq!(stub.call_count(), 0);
    }

    #[test]
    fn goal_root_dispatch_ok() {
        let c = GoalConfirms::new("FLASH", "YES");
        let mut stub = StubExecutor::with_outputs(vec![
            StubExecutor::ok_output(flash_ok_text().as_str()),
            StubExecutor::ok_output("/system/xbin/su\n"),
            StubExecutor::ok_output("uid=0(root) gid=0\n"),
            StubExecutor::ok_output("27.0:MAGISK\n"),
            StubExecutor::ok_output("List of devices attached\nABC123\tdevice\n"),
            StubExecutor::ok_output("13\n"),
            StubExecutor::ok_output("trebledroid 13 arm64_bvN\n"),
            StubExecutor::ok_output("Enforcing\n"),
            StubExecutor::ok_output("uid=0(root) gid=0\n"),
            StubExecutor::ok_output(
                "/dev/block/system /system ext4\n/dev/block/vendor /vendor ext4\n",
            ),
            StubExecutor::ok_output("Wi-Fi is enabled\n"),
            StubExecutor::ok_output("1\n"),
            StubExecutor::ok_output("level: 88\n"),
            StubExecutor::ok_output("Sensor0\nSensor1\n"),
        ]);
        let rep = run_goal("root", &mut stub, &c);
        assert_eq!(rep.goal.as_str(), "root");
        assert!(rep.is_ok());
        assert_eq!(stub.call_count(), 14);
        let mut saw_flash = false;
        let mut saw_verify = false;
        let mut saw_validate = false;
        for o in rep.steps.iter() {
            if o.op == "safe-flash" && o.status == "ok" {
                saw_flash = true;
            }
            if o.op == "verify" && o.status == "ok" {
                saw_verify = true;
            }
            if o.op == "validate" && o.status == "ok" {
                saw_validate = true;
            }
        }
        assert!(saw_flash);
        assert!(saw_verify);
        assert!(saw_validate);
        // Info steps are skipped with no extra calls.
        let mut skipped = 0;
        for o in rep.steps.iter() {
            if o.status == "skipped" {
                skipped += 1;
            }
        }
        assert!(skipped > 0);
    }

    #[test]
    fn developer_dump_kinds_scripted() {
        // Partitions: byname plus partitions.
        let mut p_stub = StubExecutor::with_outputs(vec![
            StubExecutor::ok_output("recovery_ramdisk -> /dev/block/mmcblk0p30\n"),
            StubExecutor::ok_output("major minor blocks\n179 0 1234\n"),
        ]);
        let p_rep = run_developer_dump("dump-partitions", &mut p_stub);
        match p_rep {
            Ok(r) => {
                assert_eq!(r.kind.as_str(), "dump-partitions");
                assert_eq!(r.entries.len(), 2);
                assert_eq!(r.entries[0].name.as_str(), "byname");
                assert!(r.entries[0].content.contains("recovery_ramdisk"));
                assert_eq!(r.entries[1].name.as_str(), "partitions");
                assert_eq!(p_stub.call_count(), 2);
            }
            Err(_) => panic!(),
        }
        // Properties short form.
        let mut g_stub = StubExecutor::with_outputs(vec![StubExecutor::ok_output(
            "[ro.product.model]: [VTR-L29]\n",
        )]);
        let g_rep = run_developer_dump("properties", &mut g_stub);
        match g_rep {
            Ok(r) => {
                assert_eq!(r.entries.len(), 1);
                assert!(r.entries[0].content.contains("VTR-L29"));
                assert_eq!(g_stub.call_count(), 1);
            }
            Err(_) => panic!(),
        }
        // Vendor: vendor plus mounts.
        let mut v_stub = StubExecutor::with_outputs(vec![
            StubExecutor::ok_output("audio_effects.xml\n"),
            StubExecutor::ok_output("/vendor /vendor ext4\n"),
        ]);
        let v_rep = run_developer_dump("dump-vendor", &mut v_stub);
        match v_rep {
            Ok(r) => {
                assert_eq!(r.entries.len(), 2);
                assert_eq!(r.entries[0].name.as_str(), "vendor");
                assert_eq!(r.entries[1].name.as_str(), "mounts");
                assert_eq!(v_stub.call_count(), 2);
            }
            Err(_) => panic!(),
        }
        // Logs.
        let mut l_stub = StubExecutor::with_outputs(vec![StubExecutor::ok_output("log line 1\n")]);
        let l_rep = run_developer_dump("logs", &mut l_stub);
        match l_rep {
            Ok(r) => {
                assert_eq!(r.entries.len(), 1);
                assert!(r.entries[0].content.contains("log line"));
                assert_eq!(l_stub.call_count(), 1);
            }
            Err(_) => panic!(),
        }
    }

    #[test]
    fn developer_dump_invalid_zero_calls() {
        let mut stub = StubExecutor::new();
        let r = run_developer_dump("nope", &mut stub);
        assert!(r.is_err());
        assert_eq!(stub.call_count(), 0);
    }

    #[test]
    fn patch_staging_roundtrip_and_refusals() {
        let mut base = std::env::temp_dir();
        base.push(format!("gsi-exec-patch-{}", std::process::id()));
        let mut src = base.clone();
        src.push("RECOVERY_RAMDIS.img");
        let mut stage = base.clone();
        stage.push("to-patch");
        // Fresh base.
        let _ = std::fs::remove_dir_all(base.as_path());
        let mk = std::fs::create_dir_all(stage.as_path());
        assert!(mk.is_ok());
        let wr = std::fs::write(src.as_path(), b"stock-bytes");
        assert!(wr.is_ok());
        let staged = prepare_magisk_patch(src.as_path(), stage.as_path());
        match staged {
            Ok(dest) => {
                assert_eq!(dest.as_path().parent(), Some(stage.as_path()));
                let back = std::fs::read(dest.as_path());
                match back {
                    Ok(b) => assert_eq!(b, b"stock-bytes"),
                    Err(_) => panic!(),
                }
            }
            Err(_) => panic!(),
        }
        // Same src and dest refuses.
        let mut same_stage = base.clone();
        same_stage.push("same");
        let _ = std::fs::create_dir_all(same_stage.as_path());
        let mut same_src = same_stage.clone();
        same_src.push("a.img");
        let wr2 = std::fs::write(same_src.as_path(), b"x");
        assert!(wr2.is_ok());
        // Stage into its own parent dir with same file name hits src==dest only
        // when staging dir holds the file directly; use parent as stage.
        let parent = match same_src.parent() {
            Some(p) => p.to_path_buf(),
            None => base.clone(),
        };
        let dup = prepare_magisk_patch(same_src.as_path(), parent.as_path());
        assert!(dup.is_err());
        // Missing src refuses.
        let mut missing = base.clone();
        missing.push("absent.img");
        let m = prepare_magisk_patch(missing.as_path(), stage.as_path());
        assert!(m.is_err());
        let _ = std::fs::remove_dir_all(base.as_path());
    }

    #[test]
    fn patch_instructions_wording() {
        let t = magisk_patch_instructions();
        assert!(t.contains("Select and Patch a File"));
        assert!(t.contains("Vol-Up + Power"));
        assert!(t.contains("Magisk boot cheat"));
        assert!(t.contains("hash != stock"));
        assert!(t.contains("hashes must differ"));
        assert!(t.contains("magisk_patched"));
        assert!(t.contains("RECOVERY_RAMDIS"));
    }
}
