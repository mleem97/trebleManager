//! TUI application state: views, keyboard flow, wizard, pure gate helpers.
//!
//! Path taken: `gsi-app` is still a shell (doc comment only — no service
//! facades, no AppState, no LogBus), so the TUI builds directly on the
//! domain crates: `gsi-device` parse/slot, `gsi-image` analyze,
//! `gsi-registry` queries, `gsi-state` plans, `gsi-gates` verdicts,
//! `gsi-tool` locate. Every fallible helper returns `Result<_, String>`
//! (the domain error style); `main` maps those to process exit codes.

use crate::mask::mask_serial;
use std::path::PathBuf;

pub const APP_NAME: &str = "gsi-root-tui";
pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Sidebar views, in display order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum View {
    Dashboard,
    Device,
    Images,
    Root,
    Flash,
    Backup,
    Firmware,
    Diagnostics,
    Tools,
    Settings,
}

impl View {
    pub fn all() -> Vec<View> {
        vec![
            View::Dashboard,
            View::Device,
            View::Images,
            View::Root,
            View::Flash,
            View::Backup,
            View::Firmware,
            View::Diagnostics,
            View::Tools,
            View::Settings,
        ]
    }

    pub fn label(self) -> &'static str {
        match self {
            View::Dashboard => "Dashboard",
            View::Device => "Device",
            View::Images => "Images",
            View::Root => "Root",
            View::Flash => "Flash",
            View::Backup => "Backup",
            View::Firmware => "Firmware",
            View::Diagnostics => "Diagnostics",
            View::Tools => "Tools",
            View::Settings => "Settings",
        }
    }

    /// Views without a hardware POC: plan only, execution refused.
    pub fn experimental(self) -> bool {
        matches!(self, View::Root | View::Flash)
    }

    /// Views that offer the plan-only wizard flow.
    pub fn wizard_domain(self) -> bool {
        matches!(self, View::Root | View::Flash | View::Backup)
    }
}

/// Focused panel (`Tab` switches).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    Sidebar,
    Main,
}

/// Wizard steps: select -> detect note -> plan -> typed confirm -> refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WizardStep {
    Select,
    DetectNote,
    Plan,
    Confirm,
    Refused,
}

fn wizard_prev(step: WizardStep) -> Option<WizardStep> {
    match step {
        WizardStep::Select => None,
        WizardStep::DetectNote => Some(WizardStep::Select),
        WizardStep::Plan => Some(WizardStep::DetectNote),
        WizardStep::Confirm => Some(WizardStep::Plan),
        WizardStep::Refused => None,
    }
}

fn wizard_step_hint(step: WizardStep) -> String {
    match step {
        WizardStep::Select => "wizard: pick an operation".to_string(),
        WizardStep::DetectNote => "wizard: detection note (plan-only wave)".to_string(),
        WizardStep::Plan => "wizard: static plan review".to_string(),
        WizardStep::Confirm => "wizard: type the confirmation word".to_string(),
        WizardStep::Refused => "wizard: refused with reason".to_string(),
    }
}

/// One plan-only wizard operation.
#[derive(Debug, Clone)]
pub struct WizardOp {
    pub id: &'static str,
    pub title: &'static str,
    pub goal: &'static str,
    pub cli: &'static str,
    pub confirm_word: &'static str,
}

/// Operations per wizard domain view. Goals reference the real
/// `gsi-state` goal table; CLI strings mirror `gsi-root --help`.
pub fn wizard_ops_for(view: View) -> Vec<WizardOp> {
    match view {
        View::Root => vec![
            WizardOp {
                id: "root-magisk",
                title: "Root via Magisk patched recovery_ramdisk",
                goal: "root",
                cli: "gsi-root device switch --to magisk --magisk <patched.img>",
                confirm_word: "FLASH",
            },
            WizardOp {
                id: "root-twrp",
                title: "Install TWRP into the shared slot",
                goal: "root",
                cli: "gsi-root device switch --to twrp --twrp <twrp.img>",
                confirm_word: "FLASH",
            },
        ],
        View::Flash => vec![
            WizardOp {
                id: "flash-gsi",
                title: "Flash GSI system image",
                goal: "custom_rom",
                cli: "gsi-root workflow run p10-lineage20",
                confirm_word: "FLASH",
            },
            WizardOp {
                id: "flash-patched",
                title: "Flash patched recovery_ramdisk",
                goal: "root",
                cli: "gsi-root device switch --to magisk --magisk <patched.img>",
                confirm_word: "FLASH",
            },
        ],
        View::Backup => vec![
            WizardOp {
                id: "backup-before-root",
                title: "Back up stock images before root",
                goal: "root",
                cli: "gsi-root goal show root",
                confirm_word: "BACKUP",
            },
            WizardOp {
                id: "backup-before-restore",
                title: "Back up before stock restore",
                goal: "restore_original",
                cli: "gsi-root goal show restore_original",
                confirm_word: "BACKUP",
            },
        ],
        _ => Vec::new(),
    }
}

#[derive(Debug, Clone)]
pub struct WizardState {
    pub ops: Vec<WizardOp>,
    pub op_idx: usize,
    pub step: WizardStep,
    pub confirm_input: String,
}

/// One attached device (serial masked unless `--show-serials`).
#[derive(Debug, Clone)]
pub struct DeviceInfo {
    pub serial: String,
    pub state: String,
}

impl DeviceInfo {
    pub fn display(&self, show_serials: bool) -> String {
        if show_serials {
            format!("{} ({})", self.serial, self.state)
        } else {
            format!("{} ({})", mask_serial(&self.serial), self.state)
        }
    }
}

/// Command-line flags.
#[derive(Debug, Clone)]
pub struct Args {
    pub dry_run: bool,
    pub yes: bool,
    pub show_serials: bool,
    pub registry: Option<PathBuf>,
    pub image: Option<PathBuf>,
    pub state_file: Option<PathBuf>,
}

impl Args {
    pub fn defaults() -> Args {
        Args {
            dry_run: true,
            yes: false,
            show_serials: false,
            registry: None,
            image: None,
            state_file: None,
        }
    }
}

#[derive(Debug)]
pub enum Action {
    Help,
    Run(Args),
}

/// Parse CLI flags. `--help`/`-h` wins immediately; anything unknown errors.
pub fn parse_args(argv: &[String]) -> Result<Action, String> {
    let mut args = Args::defaults();
    let mut i = 1;
    while i < argv.len() {
        let a = argv[i].as_str();
        if a == "--help" || a == "-h" {
            return Ok(Action::Help);
        } else if a == "--dry-run" {
            args.dry_run = true;
        } else if a == "--no-dry-run" {
            args.dry_run = false;
        } else if a == "--yes" {
            args.yes = true;
        } else if a == "--show-serials" {
            args.show_serials = true;
        } else if a == "--registry" || a == "--image" || a == "--state-file" {
            i += 1;
            match argv.get(i) {
                Some(v) => {
                    let p = PathBuf::from(v);
                    if a == "--registry" {
                        args.registry = Some(p);
                    } else if a == "--image" {
                        args.image = Some(p);
                    } else {
                        args.state_file = Some(p);
                    }
                }
                None => return Err(format!("{a} needs a value")),
            }
        } else if let Some(rest) = a.strip_prefix("--dry-run=") {
            if rest == "true" {
                args.dry_run = true;
            } else if rest == "false" {
                args.dry_run = false;
            } else {
                return Err("--dry-run takes true|false".to_string());
            }
        } else {
            return Err(format!("unknown argument: {a}"));
        }
        i += 1;
    }
    Ok(Action::Run(args))
}

pub fn usage_line() -> String {
    "usage: gsi-root-tui [--dry-run|--no-dry-run] [--yes] [--show-serials] [--registry <file>] [--image <file>] [--state-file <file>] [--help]".to_string()
}

/// Full `--help` text (printed to stdout, exit 0).
pub fn help_text() -> String {
    let mut out = Vec::new();
    out.push(format!(
        "{APP_NAME} v{APP_VERSION} - Treble Toolkit terminal UI (read-only plan wave)"
    ));
    out.push(String::new());
    out.push(usage_line());
    out.push(String::new());
    out.push("flags:".to_string());
    out.push(
        "  --dry-run (default)   destructive displays stay plans; nothing executes".to_string(),
    );
    out.push("  --no-dry-run          display-only change; execution is still refused".to_string());
    out.push("  --yes                 passed through to shown CLI commands; never".to_string());
    out.push(
        "                        authorizes execution inside the TUI (never bypassed)".to_string(),
    );
    out.push("  --show-serials        show full device serials (masked by default)".to_string());
    out.push(
        "  --registry <file>     registry profile JSON (Images/Tools/Firmware views)".to_string(),
    );
    out.push("  --image <file>        image file to analyze (Images view)".to_string());
    out.push("  --state-file <file>   workflow-state.json (default: app log dir)".to_string());
    out.push(String::new());
    out.push("keys:".to_string());
    out.push("  Up/Down navigate - Enter select - Esc back - Tab next panel".to_string());
    out.push("  / search (filter lists) - ? shortcut dialog - q quit".to_string());
    out.push("  Left back - Right select (arrow-key alternatives)".to_string());
    out.push(String::new());
    out.push("honesty:".to_string());
    out.push("  The TUI never executes destructive operations in this wave.".to_string());
    out.push("  Every wizard ends in refused-with-reason plus the exact CLI".to_string());
    out.push("  command to run instead. EXPERIMENTAL views never fake success.".to_string());
    out.join("\n")
}

/// Input keys (crossterm mapping lives in `main`; tests use this directly).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    Up,
    Down,
    Left,
    Right,
    Enter,
    Esc,
    Tab,
    Backspace,
    Help,
    Search,
    Quit,
    Char(char),
}

/// Full application state. All data fields are plain injectable values so
/// rendering and transitions stay testable without device/process I/O.
#[derive(Debug, Clone)]
pub struct App {
    pub view: View,
    pub sidebar_idx: usize,
    pub focus: Focus,
    pub searching: bool,
    pub search: String,
    pub show_help: bool,
    pub dry_run: bool,
    pub yes: bool,
    pub show_serials: bool,
    pub devices: Vec<DeviceInfo>,
    pub device_idx: usize,
    pub wizard: Option<WizardState>,
    pub status: String,
    pub state_lines: Vec<String>,
    pub slot_lines: Vec<String>,
    pub registry_lines: Vec<String>,
    pub image_lines: Vec<String>,
    pub tools_lines: Vec<String>,
    pub diag_lines: Vec<String>,
    pub firmware_lines: Vec<String>,
    pub registry_path: String,
    pub image_path: String,
    pub state_path: String,
}

impl App {
    pub fn new(args: &Args) -> App {
        let path_str = |o: &Option<PathBuf>| match o {
            Some(p) => p.display().to_string(),
            None => String::new(),
        };
        App {
            view: View::Dashboard,
            sidebar_idx: 0,
            focus: Focus::Sidebar,
            searching: false,
            search: String::new(),
            show_help: false,
            dry_run: args.dry_run,
            yes: args.yes,
            show_serials: args.show_serials,
            devices: Vec::new(),
            device_idx: 0,
            wizard: None,
            status: "ready (read-only plan wave)".to_string(),
            state_lines: Vec::new(),
            slot_lines: Vec::new(),
            registry_lines: Vec::new(),
            image_lines: Vec::new(),
            tools_lines: Vec::new(),
            diag_lines: Vec::new(),
            firmware_lines: Vec::new(),
            registry_path: path_str(&args.registry),
            image_path: path_str(&args.image),
            state_path: path_str(&args.state_file),
        }
    }
}

/// Sidebar views filtered by the `/` search query (case-insensitive).
pub fn filtered_views(search: &str) -> Vec<View> {
    let q = search.trim().to_lowercase();
    if q.is_empty() {
        return View::all();
    }
    let mut out = Vec::new();
    for v in View::all() {
        if v.label().to_lowercase().contains(&q) {
            out.push(v);
        }
    }
    out
}

/// Handle one key. Returns true when the app should quit.
pub fn handle_key(app: &mut App, key: Key) -> bool {
    if app.show_help {
        match key {
            Key::Esc | Key::Enter | Key::Help | Key::Quit => {
                app.show_help = false;
            }
            _ => {}
        }
        return false;
    }
    if app.searching {
        match key {
            Key::Esc => {
                app.searching = false;
                app.search.clear();
                app.sidebar_idx = 0;
                app.status = "search cleared".to_string();
            }
            Key::Enter => {
                app.searching = false;
                app.sidebar_idx = 0;
                app.status = "search filter applied".to_string();
            }
            Key::Backspace => {
                app.search.pop();
                app.sidebar_idx = 0;
            }
            Key::Char(c) => {
                app.search.push(c);
                app.sidebar_idx = 0;
            }
            _ => {}
        }
        return false;
    }
    if confirm_typing(app) {
        match key {
            Key::Esc => {
                if let Some(w) = app.wizard.as_mut() {
                    w.step = WizardStep::Plan;
                    w.confirm_input.clear();
                }
                app.status = "confirmation cancelled".to_string();
            }
            Key::Enter => submit_confirm(app),
            Key::Backspace => {
                if let Some(w) = app.wizard.as_mut() {
                    w.confirm_input.pop();
                }
            }
            Key::Char(c) => {
                if let Some(w) = app.wizard.as_mut() {
                    w.confirm_input.push(c);
                }
            }
            Key::Tab => {
                app.focus = match app.focus {
                    Focus::Sidebar => Focus::Main,
                    Focus::Main => Focus::Sidebar,
                };
            }
            _ => {}
        }
        return false;
    }
    match key {
        Key::Quit => return true,
        Key::Help => {
            app.show_help = true;
        }
        Key::Search => {
            app.searching = true;
            app.status = "type to filter; Enter keeps, Esc clears".to_string();
        }
        Key::Tab => {
            app.focus = match app.focus {
                Focus::Sidebar => Focus::Main,
                Focus::Main => Focus::Sidebar,
            };
        }
        Key::Esc | Key::Left => esc_back(app),
        Key::Up => move_cursor(app, -1),
        Key::Down => move_cursor(app, 1),
        Key::Enter | Key::Right => enter_action(app),
        Key::Backspace => {}
        Key::Char(_) => {}
    }
    false
}

/// True while the wizard confirm step takes typed input on the main panel.
fn confirm_typing(app: &App) -> bool {
    match app.wizard.as_ref() {
        Some(w) => w.step == WizardStep::Confirm && app.focus == Focus::Main,
        None => false,
    }
}

fn move_cursor(app: &mut App, delta: i32) {
    if app.focus == Focus::Sidebar {
        let n = filtered_views(&app.search).len();
        if n == 0 {
            return;
        }
        if app.sidebar_idx >= n {
            app.sidebar_idx = 0;
            return;
        }
        if delta < 0 {
            if app.sidebar_idx == 0 {
                app.sidebar_idx = n - 1;
            } else {
                app.sidebar_idx -= 1;
            }
        } else if app.sidebar_idx + 1 >= n {
            app.sidebar_idx = 0;
        } else {
            app.sidebar_idx += 1;
        }
        return;
    }
    // Main panel cursors: wizard op list, else device list.
    match app.wizard.as_mut() {
        Some(w) if w.step == WizardStep::Select => {
            let n = w.ops.len();
            if n == 0 {
                return;
            }
            if w.op_idx >= n {
                w.op_idx = 0;
                return;
            }
            if delta < 0 {
                if w.op_idx == 0 {
                    w.op_idx = n - 1;
                } else {
                    w.op_idx -= 1;
                }
            } else if w.op_idx + 1 >= n {
                w.op_idx = 0;
            } else {
                w.op_idx += 1;
            }
        }
        _ => {
            let n = app.devices.len();
            if n == 0 {
                return;
            }
            if app.device_idx >= n {
                app.device_idx = 0;
                return;
            }
            if delta < 0 {
                if app.device_idx == 0 {
                    app.device_idx = n - 1;
                } else {
                    app.device_idx -= 1;
                }
            } else if app.device_idx + 1 >= n {
                app.device_idx = 0;
            } else {
                app.device_idx += 1;
            }
        }
    }
}

fn enter_action(app: &mut App) {
    if app.focus == Focus::Sidebar {
        let views = filtered_views(&app.search);
        match views.get(app.sidebar_idx) {
            Some(v) => {
                app.view = *v;
                app.status = format!("view: {}", v.label());
            }
            None => {
                app.status = "no match for search".to_string();
            }
        }
        return;
    }
    if !app.view.wizard_domain() {
        app.status = "read-only view: nothing to select here".to_string();
        return;
    }
    if app.wizard.is_none() {
        let ops = wizard_ops_for(app.view);
        if ops.is_empty() {
            app.status = "no operations for this view".to_string();
            return;
        }
        app.wizard = Some(WizardState {
            ops,
            op_idx: 0,
            step: WizardStep::Select,
            confirm_input: String::new(),
        });
        app.status = "wizard started: pick an operation".to_string();
        return;
    }
    let step = match app.wizard.as_ref() {
        Some(w) => w.step,
        None => return,
    };
    match step {
        WizardStep::Select => {
            if let Some(w) = app.wizard.as_mut() {
                w.step = WizardStep::DetectNote;
            }
            app.status = wizard_step_hint(WizardStep::DetectNote);
        }
        WizardStep::DetectNote => {
            if let Some(w) = app.wizard.as_mut() {
                w.step = WizardStep::Plan;
            }
            app.status = wizard_step_hint(WizardStep::Plan);
        }
        WizardStep::Plan => {
            if let Some(w) = app.wizard.as_mut() {
                w.step = WizardStep::Confirm;
                w.confirm_input.clear();
            }
            app.status = wizard_step_hint(WizardStep::Confirm);
        }
        WizardStep::Confirm => submit_confirm(app),
        WizardStep::Refused => {
            app.wizard = None;
            app.status = "wizard closed".to_string();
        }
    }
}

fn esc_back(app: &mut App) {
    let prev = match app.wizard.as_ref() {
        Some(w) => wizard_prev(w.step),
        None => None,
    };
    match app.wizard.as_mut() {
        Some(w) => match prev {
            Some(s) => {
                w.step = s;
                if s == WizardStep::Plan {
                    w.confirm_input.clear();
                }
                app.status = wizard_step_hint(s);
            }
            None => {
                app.wizard = None;
                app.status = "wizard closed".to_string();
            }
        },
        None => {
            app.focus = Focus::Sidebar;
        }
    }
}

fn submit_confirm(app: &mut App) {
    let (word, ok) = match app.wizard.as_ref() {
        Some(w) => match w.ops.get(w.op_idx) {
            Some(op) => (
                op.confirm_word.to_string(),
                w.confirm_input.trim() == op.confirm_word,
            ),
            None => (String::new(), false),
        },
        None => return,
    };
    if ok {
        if let Some(w) = app.wizard.as_mut() {
            w.step = WizardStep::Refused;
        }
        app.status = "confirmation accepted; execution refused (plan-only wave)".to_string();
    } else {
        if word.is_empty() {
            app.status = "no operation selected".to_string();
        } else {
            app.status = format!("confirmation mismatch (type {word}); nothing planned");
        }
        if let Some(w) = app.wizard.as_mut() {
            w.confirm_input.clear();
        }
    }
}

/// CLI command to run instead of the refused TUI execution.
/// `--yes` passes through to the display; it never authorizes the TUI.
pub fn cli_for(yes: bool, op: &WizardOp) -> String {
    if yes {
        format!("{} --yes", op.cli)
    } else {
        op.cli.to_string()
    }
}

/// Refused-with-reason lines for the final wizard step.
pub fn refusal_lines(dry_run: bool, yes: bool, op: &WizardOp) -> Vec<String> {
    let mut out = vec!["live execution refused.".to_string()];
    match gsi_gates::refuse_live(op.id) {
        Ok(()) => out.push("gate passed without device (report this)".to_string()),
        Err(reason) => out.push(reason),
    }
    out.push("TUI wave is plan-only: no device writes, no tool runs.".to_string());
    out.push(format!("run instead: {}", cli_for(yes, op)));
    if yes {
        out.push("--yes is shown passed through; it never authorizes TUI execution.".to_string());
    } else {
        out.push("core destructive paths additionally require --yes on the CLI.".to_string());
    }
    if dry_run {
        out.push("dry-run: ON (default for destructive displays).".to_string());
    } else {
        out.push(
            "dry-run: OFF via --no-dry-run (display only; execution still refused).".to_string(),
        );
    }
    out
}

/// Map a workflow step id to the closest `gsi-gates` step kind.
pub fn gate_kind_for_step(step: &str) -> String {
    let s = step.trim().to_lowercase();
    if s.contains("flash") {
        "flash".to_string()
    } else if s.contains("backup") {
        "backup".to_string()
    } else if s.contains("verify") || s.contains("validat") {
        "verify".to_string()
    } else if s.contains("recon") {
        "analyze".to_string()
    } else if s.contains("patch") {
        "patch".to_string()
    } else if s.contains("restor") {
        "restore".to_string()
    } else {
        s
    }
}

/// Static plan review: real `gsi-state` plan crossed with real
/// `gsi-gates` verdicts over explicit TUI-wave inputs (no device, nothing
/// patched, profile unverified). Unknown goals stay honest errors.
pub fn plan_gate_lines(devices_empty: bool, goal: &str) -> Vec<String> {
    let plan = match gsi_state::new_workflow_plan(goal) {
        Ok(p) => p,
        Err(e) => return vec![format!("plan: {e}")],
    };
    let overall = if devices_empty {
        "NO_DEVICE".to_string()
    } else {
        "READY".to_string()
    };
    let mut out = Vec::new();
    for (k, st) in plan.steps.iter().enumerate() {
        let i = k + 1;
        let input = gsi_gates::GateInput {
            step: gate_kind_for_step(&st.id),
            states: gsi_gates::DeviceStates {
                overall: overall.clone(),
                adb: "NO_LIVE_QUERY".to_string(),
                fastboot: "NO_LIVE_QUERY".to_string(),
            },
            profile_verified: false,
            patched_present: false,
            patch_base_present: false,
            patch_base_source: "stock".to_string(),
        };
        let verdict = gsi_gates::step_gate(&input);
        if verdict.pass {
            out.push(format!(
                "{i:>2}. {:<22} READY (static; live gates run at execution)",
                st.id
            ));
        } else {
            out.push(format!(
                "{i:>2}. {:<22} BLOCKED: {}",
                st.id,
                verdict.reasons.join("; ")
            ));
        }
    }
    out.push("profile: unverified in TUI wave (no device to verify against).".to_string());
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_app() -> App {
        App::new(&Args::defaults())
    }

    fn argv(words: &[&str]) -> Vec<String> {
        let mut out = vec!["gsi-root-tui".to_string()];
        for w in words {
            out.push(w.to_string());
        }
        out
    }

    #[test]
    fn args_default_dry_run_true() {
        match parse_args(&argv(&[])) {
            Ok(Action::Run(a)) => {
                assert!(a.dry_run);
                assert!(!a.yes);
                assert!(!a.show_serials);
            }
            _ => panic!("defaults must parse"),
        }
    }

    #[test]
    fn args_flags_and_help() {
        match parse_args(&argv(&["--no-dry-run", "--yes", "--show-serials"])) {
            Ok(Action::Run(a)) => {
                assert!(!a.dry_run);
                assert!(a.yes);
                assert!(a.show_serials);
            }
            _ => panic!("flags must parse"),
        }
        match parse_args(&argv(&["--dry-run=false"])) {
            Ok(Action::Run(a)) => assert!(!a.dry_run),
            _ => panic!("dry-run=false must parse"),
        }
        match parse_args(&argv(&["--help"])) {
            Ok(Action::Help) => {}
            _ => panic!("--help must win"),
        }
        assert!(parse_args(&argv(&["--nope"])).is_err());
        assert!(parse_args(&argv(&["--registry"])).is_err());
        match parse_args(&argv(&["--registry", "p.json", "--image", "i.img"])) {
            Ok(Action::Run(a)) => {
                assert_eq!(a.registry, Some(PathBuf::from("p.json")));
                assert_eq!(a.image, Some(PathBuf::from("i.img")));
            }
            _ => panic!("paths must parse"),
        }
    }

    #[test]
    fn sidebar_nav_wraps() {
        let mut app = empty_app();
        handle_key(&mut app, Key::Up);
        assert_eq!(app.sidebar_idx, View::all().len() - 1);
        handle_key(&mut app, Key::Down);
        assert_eq!(app.sidebar_idx, 0);
    }

    #[test]
    fn enter_selects_view_and_esc_returns_focus() {
        let mut app = empty_app();
        app.focus = Focus::Sidebar;
        app.sidebar_idx = 4;
        handle_key(&mut app, Key::Enter);
        assert_eq!(app.view, View::Flash);
        app.focus = Focus::Main;
        handle_key(&mut app, Key::Esc);
        assert_eq!(app.focus, Focus::Sidebar);
    }

    #[test]
    fn tab_toggles_focus() {
        let mut app = empty_app();
        assert_eq!(app.focus, Focus::Sidebar);
        handle_key(&mut app, Key::Tab);
        assert_eq!(app.focus, Focus::Main);
        handle_key(&mut app, Key::Tab);
        assert_eq!(app.focus, Focus::Sidebar);
    }

    #[test]
    fn search_filters_sidebar() {
        let mut app = empty_app();
        handle_key(&mut app, Key::Search);
        assert!(app.searching);
        handle_key(&mut app, Key::Char('f'));
        handle_key(&mut app, Key::Char('l'));
        let views = filtered_views(&app.search);
        assert_eq!(views, vec![View::Flash]);
        handle_key(&mut app, Key::Enter);
        assert!(!app.searching);
        handle_key(&mut app, Key::Enter);
        assert_eq!(app.view, View::Flash);
    }

    #[test]
    fn esc_clears_search() {
        let mut app = empty_app();
        handle_key(&mut app, Key::Search);
        handle_key(&mut app, Key::Char('x'));
        handle_key(&mut app, Key::Esc);
        assert!(!app.searching);
        assert!(app.search.is_empty());
        assert_eq!(filtered_views(&app.search).len(), 10);
    }

    #[test]
    fn wizard_walks_to_refused() {
        let mut app = empty_app();
        app.view = View::Root;
        app.focus = Focus::Main;
        handle_key(&mut app, Key::Enter);
        handle_key(&mut app, Key::Enter);
        match app.wizard.as_ref() {
            Some(w) => assert_eq!(w.step, WizardStep::DetectNote),
            None => panic!("wizard must be active"),
        }
        handle_key(&mut app, Key::Enter);
        handle_key(&mut app, Key::Enter);
        match app.wizard.as_ref() {
            Some(w) => assert_eq!(w.step, WizardStep::Confirm),
            None => panic!("wizard must be active"),
        }
        // Type FLASH.
        for c in "FLASH".chars() {
            handle_key(&mut app, Key::Char(c));
        }
        handle_key(&mut app, Key::Enter);
        match app.wizard.as_ref() {
            Some(w) => assert_eq!(w.step, WizardStep::Refused),
            None => panic!("wizard must be active"),
        }
        // Refused shows CLI command with pass-through semantics.
        let op = wizard_ops_for(View::Root).remove(0);
        let lines = refusal_lines(false, false, &op);
        assert!(lines.iter().any(|l| l.contains("refused")));
        assert!(lines.iter().any(|l| l.contains("gsi-root device switch")));
        let with_yes = cli_for(true, &op);
        assert!(with_yes.contains("--yes"));
    }

    #[test]
    fn wizard_wrong_confirm_stays() {
        let mut app = empty_app();
        app.view = View::Flash;
        app.focus = Focus::Main;
        handle_key(&mut app, Key::Enter);
        handle_key(&mut app, Key::Enter);
        handle_key(&mut app, Key::Enter);
        handle_key(&mut app, Key::Enter);
        match app.wizard.as_ref() {
            Some(w) => assert_eq!(w.step, WizardStep::Confirm),
            None => panic!("wizard must be active"),
        }
        handle_key(&mut app, Key::Char('n'));
        handle_key(&mut app, Key::Char('o'));
        handle_key(&mut app, Key::Enter);
        match app.wizard.as_ref() {
            Some(w) => assert_eq!(w.step, WizardStep::Confirm),
            None => panic!("wizard must stay on mismatch"),
        }
        assert!(app.status.contains("mismatch"));
    }

    #[test]
    fn esc_walks_wizard_back() {
        let mut app = empty_app();
        app.view = View::Backup;
        app.focus = Focus::Main;
        handle_key(&mut app, Key::Enter);
        handle_key(&mut app, Key::Enter);
        handle_key(&mut app, Key::Esc);
        match app.wizard.as_ref() {
            Some(w) => assert_eq!(w.step, WizardStep::Select),
            None => panic!("wizard must be active"),
        }
        handle_key(&mut app, Key::Esc);
        assert!(app.wizard.is_none());
    }

    #[test]
    fn plan_lines_use_real_core_data() {
        let lines = plan_gate_lines(true, "root");
        assert_eq!(lines.len(), gsi_state::goal_steps("root").len() + 1);
        assert!(lines.iter().any(|l| l.contains("BLOCKED")));
        assert!(lines.iter().any(|l| l.contains("no device connected")));
        let bad = plan_gate_lines(true, "no-such-goal");
        assert!(bad.iter().any(|l| l.contains("unknown goal")));
        // With fixture devices the NO_DEVICE refusal clears honestly.
        let with_dev = plan_gate_lines(false, "root");
        assert!(!with_dev.iter().any(|l| l.contains("no device connected")));
    }

    #[test]
    fn device_display_masks_by_default() {
        let d = DeviceInfo {
            serial: "6PQ0217B08003446".to_string(),
            state: "device".to_string(),
        };
        assert_eq!(d.display(false), "6P...46 (device)");
        assert_eq!(d.display(true), "6PQ0217B08003446 (device)");
    }
}
