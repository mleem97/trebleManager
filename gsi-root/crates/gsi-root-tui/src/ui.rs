//! Ratatui rendering: header | sidebar | main panel | status bar, plus the
//! `?` shortcut dialog. Pure views over `App`; every view shows core data
//! or honest empty states (never fake success).

use crate::app::{
    cli_for, filtered_views, plan_gate_lines, refusal_lines, wizard_ops_for, App, Focus, View,
    WizardStep, APP_NAME, APP_VERSION,
};
use crate::mask::mask_serial;
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
    Frame,
};

/// Render the full TUI frame.
pub fn render(f: &mut Frame, app: &App) {
    let area = f.area();
    let header_h: u16 = if app.view.experimental() { 5 } else { 4 };
    let chunks = Layout::vertical([
        Constraint::Length(header_h),
        Constraint::Min(0),
        Constraint::Length(1),
    ])
    .split(area);
    render_header(f, app, chunks[0]);
    let body = Layout::horizontal([Constraint::Length(26), Constraint::Min(0)]).split(chunks[1]);
    render_sidebar(f, app, body[0]);
    render_main(f, app, body[1]);
    render_status(f, app, chunks[2]);
    if app.show_help {
        render_help_dialog(f, area);
    }
}

fn flags_line(app: &App) -> String {
    let dry = if app.dry_run {
        "dry-run: ON"
    } else {
        "dry-run: OFF (display only)"
    };
    let yes = if app.yes {
        "--yes: set (pass-through)"
    } else {
        "--yes: not set"
    };
    let ser = if app.show_serials {
        "serials: shown"
    } else {
        "serials: masked"
    };
    format!("{dry} | {yes} | {ser} | devices: {}", app.devices.len())
}

fn render_header(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .title(format!(" {APP_NAME} "))
        .borders(Borders::ALL);
    let inner = block.inner(area);
    f.render_widget(block, area);
    let mut lines = vec![
        Line::from(vec![
            Span::styled(
                format!("{APP_NAME} v{APP_VERSION}"),
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::raw(" - Treble Toolkit (read-only plan wave)"),
        ]),
        Line::from(flags_line(app)),
    ];
    if app.view.experimental() {
        lines.push(Line::from(vec![Span::styled(
            "EXPERIMENTAL - plan only, execution refused (no hardware POC)",
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        )]));
    }
    f.render_widget(Paragraph::new(Text::from(lines)), inner);
}

fn render_sidebar(f: &mut Frame, app: &App, area: Rect) {
    let views = filtered_views(&app.search);
    let mut items = Vec::new();
    if views.is_empty() {
        items.push(ListItem::new(Line::from("(no match)".to_string())));
    }
    for (i, v) in views.iter().enumerate() {
        let marker = if *v == app.view { "> " } else { "  " };
        let style = if i == app.sidebar_idx && app.focus == Focus::Sidebar {
            Style::default().fg(Color::Black).bg(Color::White)
        } else if *v == app.view {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };
        items.push(ListItem::new(Line::from(vec![Span::styled(
            format!("{marker}{}", v.label()),
            style,
        )])));
    }
    let title = if app.searching {
        format!(" Views /{} ", app.search)
    } else {
        " Views ".to_string()
    };
    let list = List::new(items).block(Block::default().title(title).borders(Borders::ALL));
    f.render_widget(list, area);
}

fn render_main(f: &mut Frame, app: &App, area: Rect) {
    let (title, strings) = main_content(app);
    let block = Block::default()
        .title(format!(" {title} "))
        .borders(Borders::ALL);
    let inner = block.inner(area);
    f.render_widget(block, area);
    let lines: Vec<Line> = strings.into_iter().collect();
    f.render_widget(
        Paragraph::new(Text::from(lines)).wrap(Wrap { trim: true }),
        inner,
    );
}

fn render_status(f: &mut Frame, app: &App, area: Rect) {
    let hints = "q quit | ? help | / search | Tab panel | Enter select | Esc back";
    let text = if app.searching {
        format!("/{} (Enter keeps, Esc clears)", app.search)
    } else if app.status.is_empty() {
        hints.to_string()
    } else {
        format!("{hints} || {}", app.status)
    };
    f.render_widget(Paragraph::new(Line::from(text)), area);
}

fn centered_rect(area: Rect) -> Rect {
    let vert = Layout::vertical([
        Constraint::Percentage(10),
        Constraint::Percentage(80),
        Constraint::Percentage(10),
    ])
    .split(area);
    let horiz = Layout::horizontal([
        Constraint::Percentage(15),
        Constraint::Percentage(70),
        Constraint::Percentage(15),
    ])
    .split(vert[1]);
    horiz[1]
}

fn help_strings() -> Vec<String> {
    vec![
        "Shortcuts (press ? or Esc to close)".to_string(),
        String::new(),
        "  Up / Down      move cursor (sidebar, operations, devices)".to_string(),
        "  Left / Right   back / select (Esc / Enter alternatives)".to_string(),
        "  Enter          select view, or advance the wizard".to_string(),
        "  Esc            back: dialog, search, wizard step, focus".to_string(),
        "  Tab            next panel (Sidebar <-> Main)".to_string(),
        "  / search       filter lists; Enter keeps, Esc clears".to_string(),
        "  ?              this shortcut dialog".to_string(),
        "  q quit         quit from anywhere except text input".to_string(),
        String::new(),
        "wizard: pick op, read detect note, review plan, type the".to_string(),
        "shown word, then read refused-with-reason plus the exact".to_string(),
        "CLI command to run instead.".to_string(),
        String::new(),
        "flags: --dry-run default ON; --yes passes through and".to_string(),
        "never authorizes TUI execution.".to_string(),
    ]
}

fn render_help_dialog(f: &mut Frame, area: Rect) {
    let popup = centered_rect(area);
    f.render_widget(Clear, popup);
    let block = Block::default()
        .title(" Shortcuts (?) ")
        .borders(Borders::ALL);
    let inner = block.inner(popup);
    f.render_widget(block, popup);
    let lines: Vec<Line> = help_strings().into_iter().map(Line::from).collect();
    f.render_widget(
        Paragraph::new(Text::from(lines)).wrap(Wrap { trim: true }),
        inner,
    );
}

fn setup_hint_strings() -> Vec<String> {
    vec![
        "no device connected.".to_string(),
        "setup hints:".to_string(),
        "  1. install platform-tools (adb, fastboot)".to_string(),
        "  2. enable USB debugging on the device".to_string(),
        "  3. run: gsi-root device detect".to_string(),
        "  4. Huawei quirk: 'Command not allowed' is not lock proof".to_string(),
    ]
}

fn dashboard_content(app: &App) -> (String, Vec<String>) {
    let mut out = vec![
        "Treble Toolkit - read-only plan wave.".to_string(),
        format!("build: {APP_NAME} v{APP_VERSION}"),
        String::new(),
    ];
    if app.devices.is_empty() {
        out.extend(setup_hint_strings());
    } else {
        out.push(format!("devices: {}", app.devices.len()));
        for d in app.devices.iter() {
            out.push(format!("  - {}", d.display(app.show_serials)));
        }
    }
    out.push(String::new());
    if app.state_lines.is_empty() {
        out.push("workflow-state: not loaded (no state file read yet).".to_string());
    } else {
        out.push("workflow-state:".to_string());
        for l in app.state_lines.iter() {
            out.push(format!("  {l}"));
        }
    }
    out.push(String::new());
    out.push("TUI honesty: read-only views first; destructive flows end".to_string());
    out.push("refused-with-reason plus the exact CLI command.".to_string());
    ("Dashboard".to_string(), out)
}

fn device_content(app: &App) -> (String, Vec<String>) {
    let mut out = Vec::new();
    if app.devices.is_empty() {
        out.extend(setup_hint_strings());
    } else {
        out.push("attached devices (serials masked by default):".to_string());
        for (i, d) in app.devices.iter().enumerate() {
            let marker = if i == app.device_idx && app.focus == Focus::Main {
                ">"
            } else {
                " "
            };
            out.push(format!("{marker} {}", d.display(app.show_serials)));
        }
    }
    out.push(String::new());
    out.push("detection note: live detection runs outside the TUI via".to_string());
    out.push("  gsi-root device detect".to_string());
    out.push("output parses with gsi-device (adb/fastboot/getprop/".to_string());
    out.push("by-name/getvar; Huawei denial is a quirk, not lock proof).".to_string());
    out.push(String::new());
    if app.slot_lines.is_empty() {
        out.push("slot: unknown (no state file read yet).".to_string());
    } else {
        out.push("shared recovery_ramdisk slot:".to_string());
        for l in app.slot_lines.iter() {
            out.push(format!("  {l}"));
        }
    }
    ("Device".to_string(), out)
}

fn images_content(app: &App) -> (String, Vec<String>) {
    let mut out = Vec::new();
    if app.image_lines.is_empty() {
        out.push("image: none analyzed (pass --image <file>).".to_string());
        out.push("run: gsi-root analyze <image>".to_string());
    } else {
        out.push(format!("image: {}", app.image_path));
        for l in app.image_lines.iter() {
            out.push(format!("  {l}"));
        }
    }
    out.push(String::new());
    if app.registry_lines.is_empty() {
        out.push("registry: none loaded (pass --registry <profile.json>).".to_string());
        out.push("run: gsi-root select image --registry <file>".to_string());
    } else {
        out.push(format!("registry: {}", app.registry_path));
        for l in app.registry_lines.iter() {
            out.push(format!("  {l}"));
        }
    }
    ("Images".to_string(), out)
}

fn root_method_strings() -> Vec<String> {
    let mut out = vec!["root methods (gsi-gates table):".to_string()];
    for m in gsi_gates::preferred_root_methods().iter() {
        let name = match gsi_gates::root_method_name(&m.id) {
            Ok(n) => n,
            Err(_) => m.id.clone(),
        };
        let star = if m.preferred { "*" } else { " " };
        out.push(format!(" {star} {} - {name}", m.id));
    }
    out.push("shared slot: TWRP and Magisk overwrite each other.".to_string());
    out
}

fn ops_strings(view: View) -> Vec<String> {
    let mut out = vec!["operations (Enter starts the plan-only wizard):".to_string()];
    for op in wizard_ops_for(view).iter() {
        out.push(format!("  - {} [{}]", op.title, op.goal));
    }
    out
}

fn root_content(app: &App) -> (String, Vec<String>) {
    let mut out = vec![
        "EXPERIMENTAL: root has no hardware POC; plan only.".to_string(),
        String::new(),
    ];
    out.extend(root_method_strings());
    out.push(String::new());
    if app.slot_lines.is_empty() {
        out.push("slot: unknown (no state file read yet).".to_string());
    } else {
        for l in app.slot_lines.iter() {
            out.push(format!("slot: {l}"));
        }
    }
    out.push(String::new());
    out.extend(ops_strings(View::Root));
    ("Root".to_string(), out)
}

fn readiness_strings(app: &App) -> Vec<String> {
    let parts = gsi_gates::ReadinessParts {
        model_ok: false,
        profile_verified: false,
        partition_ok: false,
        image_exists: !app.image_path.is_empty(),
        hash_known: false,
        size_ok: false,
        firmware_ok: false,
        backup_ok: false,
        fastboot_ok: false,
        differs_from_stock: false,
    };
    let rep = gsi_gates::check_readiness(&parts);
    let mut out = vec!["flash readiness (explicit TUI-wave inputs):".to_string()];
    for c in rep.checks.iter() {
        let mark = if c.pass { "ok" } else { "missing" };
        out.push(format!("  [{mark}] {} ({})", c.name, c.detail));
    }
    let verdict = if rep.go { "READY" } else { "NOT READY" };
    out.push(format!("verdict: {verdict} (no device, nothing verified)."));
    out.push("inputs: model unknown; profile unverified; no hash;".to_string());
    out.push("no backup; no fastboot device.".to_string());
    out
}

fn flash_content(app: &App) -> (String, Vec<String>) {
    let mut out = vec![
        "EXPERIMENTAL: flashing has no hardware POC; plan only.".to_string(),
        String::new(),
    ];
    out.extend(readiness_strings(app));
    out.push(String::new());
    out.extend(ops_strings(View::Flash));
    ("Flash".to_string(), out)
}

fn backup_content(app: &App) -> (String, Vec<String>) {
    let mut out = vec![
        "backups need a connected device; the TUI shows the plan only.".to_string(),
        String::new(),
    ];
    out.extend(ops_strings(View::Backup));
    out.push(String::new());
    if app.state_lines.is_empty() {
        out.push("workflow-state: not loaded (no state file read yet).".to_string());
    } else {
        for l in app.state_lines.iter() {
            out.push(format!("state: {l}"));
        }
    }
    ("Backup".to_string(), out)
}

fn partition_rule_strings() -> Vec<String> {
    let mut out = vec!["partition rules (gsi-gates):".to_string()];
    for p in ["VTR-L29", "VKY-L29", "GENERIC-TREBLE"] {
        out.push(format!("  {p} -> '{}'", gsi_gates::target_partition(p)));
    }
    let mut forbidden = Vec::new();
    for p in [
        "boot",
        "recovery",
        "system",
        "vendor",
        "userdata",
        "recovery_ramdisk",
    ] {
        if gsi_gates::is_forbidden_partition(p) {
            forbidden.push(p.to_string());
        }
    }
    out.push(format!("never flash directly: {}", forbidden.join(", ")));
    out
}

fn firmware_content(app: &App) -> (String, Vec<String>) {
    let mut out = Vec::new();
    if app.firmware_lines.is_empty() {
        out.push("firmware: no registry loaded (pass --registry <profile.json>).".to_string());
        out.push("run: gsi-root verdict flash --what firmware".to_string());
        out.push(String::new());
    } else {
        out.push(format!("firmware (registry {}):", app.registry_path));
        for l in app.firmware_lines.iter() {
            out.push(format!("  {l}"));
        }
        out.push(String::new());
    }
    out.push("model family is strict (VTR vs VKY); submodel and region".to_string());
    out.push("mismatches are WARN; EMUI base notes are advisory.".to_string());
    out.push(String::new());
    out.extend(partition_rule_strings());
    ("Firmware".to_string(), out)
}

fn diagnostics_content(app: &App) -> (String, Vec<String>) {
    let mut out = Vec::new();
    if app.diag_lines.is_empty() {
        out.push("diagnostics: no host scan ran yet.".to_string());
    } else {
        out.push("host diagnostics (explicit inputs, listed verbatim):".to_string());
        for l in app.diag_lines.iter() {
            out.push(format!("  {l}"));
        }
    }
    out.push(String::new());
    if app.tools_lines.is_empty() {
        out.push("tools: not scanned.".to_string());
    } else {
        for l in app.tools_lines.iter() {
            out.push(format!("tools: {l}"));
        }
    }
    ("Diagnostics".to_string(), out)
}

fn tools_content(app: &App) -> (String, Vec<String>) {
    let mut out = Vec::new();
    if app.tools_lines.is_empty() {
        out.push("tools: adb/fastboot not scanned.".to_string());
        out.push("install platform-tools, then restart the TUI.".to_string());
    } else {
        out.push("managed tools (locate only, no process spawn):".to_string());
        for l in app.tools_lines.iter() {
            out.push(format!("  {l}"));
        }
    }
    out.push(String::new());
    out.push("version probe skipped in the read-only wave;".to_string());
    out.push("run: gsi-root tools".to_string());
    ("Tools".to_string(), out)
}

fn settings_content(app: &App) -> (String, Vec<String>) {
    let out = vec![
        "settings (flags only in this wave):".to_string(),
        format!(
            "  dry-run: {}",
            if app.dry_run {
                "ON (default)"
            } else {
                "OFF (display only)"
            }
        ),
        format!(
            "  --yes: {}",
            if app.yes {
                "set (pass-through)"
            } else {
                "not set"
            }
        ),
        format!(
            "  serials: {}",
            if app.show_serials {
                "shown"
            } else {
                "masked (default)"
            }
        ),
        format!(
            "  registry: {}",
            if app.registry_path.is_empty() {
                "(none)"
            } else {
                app.registry_path.as_str()
            }
        ),
        format!(
            "  image: {}",
            if app.image_path.is_empty() {
                "(none)"
            } else {
                app.image_path.as_str()
            }
        ),
        format!(
            "  state: {}",
            if app.state_path.is_empty() {
                "(default log dir)"
            } else {
                app.state_path.as_str()
            }
        ),
        String::new(),
        format!(
            "  masking demo: 6PQ0217B08003446 -> {}",
            mask_serial("6PQ0217B08003446")
        ),
        String::new(),
        "language: English only (i18n pending; see MISSING-I18N).".to_string(),
        String::new(),
        "keys: arrows move, Enter selects, Esc goes back,".to_string(),
        "Tab changes panel, / searches, ? helps, q quits.".to_string(),
    ];
    ("Settings".to_string(), out)
}

fn wizard_content(app: &App) -> (String, Vec<String>) {
    let w = match app.wizard.as_ref() {
        Some(w) => w,
        None => return ("Wizard".to_string(), vec!["no wizard active".to_string()]),
    };
    let op = match w.ops.get(w.op_idx) {
        Some(o) => o.clone(),
        None => {
            return (
                "Wizard".to_string(),
                vec!["no operation selected".to_string()],
            )
        }
    };
    let dry = if app.dry_run {
        "dry-run: ON (default for destructive displays)"
    } else {
        "dry-run: OFF via --no-dry-run (display only)"
    };
    match w.step {
        WizardStep::Select => {
            let mut out = vec![
                "Step 1/5 - select operation (plan only).".to_string(),
                String::new(),
            ];
            for (i, o) in w.ops.iter().enumerate() {
                let marker = if i == w.op_idx { ">" } else { " " };
                out.push(format!("{marker} {} [{}]", o.title, o.goal));
            }
            out.push(String::new());
            out.push("Enter continues; Esc closes the wizard.".to_string());
            ("Wizard".to_string(), out)
        }
        WizardStep::DetectNote => {
            let mut out = vec![
                "Step 2/5 - device detection note.".to_string(),
                String::new(),
                format!("operation: {}", op.title),
                "detection runs outside the TUI:".to_string(),
                "  gsi-root device detect".to_string(),
                "the TUI ships no live detection in this wave;".to_string(),
                "parsers (gsi-device) stay ready for CLI output.".to_string(),
                String::new(),
            ];
            if app.devices.is_empty() {
                out.push("devices: none (honest empty state).".to_string());
            } else {
                out.push(format!("devices: {}", app.devices.len()));
                for d in app.devices.iter() {
                    out.push(format!("  - {}", d.display(app.show_serials)));
                }
            }
            out.push(String::new());
            out.push("Enter continues; Esc goes back.".to_string());
            ("Wizard".to_string(), out)
        }
        WizardStep::Plan => {
            let mut out = vec![
                "Step 3/5 - plan display (static gates).".to_string(),
                String::new(),
                format!("operation: {}", op.title),
                format!("goal: {}", op.goal),
                String::new(),
            ];
            out.extend(plan_gate_lines(app.devices.is_empty(), op.goal));
            out.push(String::new());
            out.push(dry.to_string());
            out.push("Enter continues; Esc goes back.".to_string());
            ("Wizard".to_string(), out)
        }
        WizardStep::Confirm => {
            let mut out = vec![
                "Step 4/5 - typed confirmation.".to_string(),
                String::new(),
                format!("operation: {}", op.title),
                "DESTRUCTIVE display: nothing executes from the TUI.".to_string(),
                format!(
                    "type {} then Enter (anything else cancels).",
                    op.confirm_word
                ),
                format!("> {}", w.confirm_input),
                String::new(),
                dry.to_string(),
            ];
            if app.yes {
                out.push("--yes is set: passed through to the CLI text;".to_string());
                out.push("it never authorizes TUI execution.".to_string());
            } else {
                out.push("core destructive paths require --yes on the CLI.".to_string());
            }
            ("Wizard".to_string(), out)
        }
        WizardStep::Refused => {
            let mut out = vec!["Step 5/5 - refused with reason.".to_string(), String::new()];
            out.extend(refusal_lines(app.dry_run, app.yes, &op));
            out.push(String::new());
            out.push(format!("exact command: {}", cli_for(app.yes, &op)));
            out.push("Enter closes the wizard; Esc goes back.".to_string());
            ("Wizard".to_string(), out)
        }
    }
}

fn main_content(app: &App) -> (String, Vec<Line<'_>>) {
    let (title, strings) = if app.wizard.is_some() && app.view.wizard_domain() {
        wizard_content(app)
    } else {
        match app.view {
            View::Dashboard => dashboard_content(app),
            View::Device => device_content(app),
            View::Images => images_content(app),
            View::Root => root_content(app),
            View::Flash => flash_content(app),
            View::Backup => backup_content(app),
            View::Firmware => firmware_content(app),
            View::Diagnostics => diagnostics_content(app),
            View::Tools => tools_content(app),
            View::Settings => settings_content(app),
        }
    };
    let lines: Vec<Line> = strings.into_iter().map(Line::from).collect();
    (title, lines)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::{handle_key, Args, DeviceInfo, Key};
    use ratatui::{backend::TestBackend, Terminal};

    fn empty_app() -> App {
        App::new(&Args::defaults())
    }

    fn screen_text(app: &App) -> String {
        let backend = TestBackend::new(100, 30);
        let mut terminal = match Terminal::new(backend) {
            Ok(t) => t,
            Err(_) => return String::new(),
        };
        if terminal.draw(|f| render(f, app)).is_err() {
            return String::new();
        }
        let backend = terminal.backend();
        let buf = backend.buffer();
        let area = buf.area;
        let mut out = String::new();
        let mut y = 0;
        while y < area.height {
            let mut line = String::new();
            let mut x = 0;
            while x < area.width {
                if let Some(c) = buf.cell((x, y)) {
                    line.push_str(c.symbol())
                }
                x += 1;
            }
            out.push_str(line.trim_end());
            out.push('\n');
            y += 1;
        }
        out
    }

    #[test]
    fn golden_dashboard_no_device() {
        let app = empty_app();
        let s = screen_text(&app);
        assert!(s.contains(APP_NAME), "header app name");
        assert!(s.contains(APP_VERSION), "header version");
        assert!(s.contains("Dashboard"), "sidebar + panel");
        assert!(s.contains("Diagnostics"), "all sidebar views listed");
        assert!(s.contains("no device"), "honest empty state");
        assert!(s.contains("setup hints"), "setup hints shown");
        assert!(s.contains("quit"), "status bar hints");
        assert!(s.contains("serials: masked"), "mask default visible");
    }

    #[test]
    fn golden_multi_device_select() {
        let mut app = empty_app();
        app.view = View::Device;
        app.focus = Focus::Main;
        app.devices = vec![
            DeviceInfo {
                serial: "6PQ0217B08003446".to_string(),
                state: "device".to_string(),
            },
            DeviceInfo {
                serial: "ABCDEF123456".to_string(),
                state: "fastboot".to_string(),
            },
        ];
        app.device_idx = 1;
        let s = screen_text(&app);
        assert!(s.contains("6P...46"), "first serial masked");
        assert!(s.contains("AB...56"), "second serial masked");
        assert!(!s.contains("6PQ0217B08003446"), "no raw serial leaks");
        assert!(!s.contains("ABCDEF123456"), "no raw serial leaks");
    }

    #[test]
    fn golden_destructive_confirm() {
        let mut app = empty_app();
        app.view = View::Flash;
        app.focus = Focus::Main;
        handle_key(&mut app, Key::Enter);
        handle_key(&mut app, Key::Enter);
        handle_key(&mut app, Key::Enter);
        handle_key(&mut app, Key::Enter);
        let s = screen_text(&app);
        assert!(s.contains("Step 4/5"), "confirm step shown");
        assert!(s.contains("FLASH"), "typed word shown");
        assert!(s.contains("dry-run: ON"), "dry-run default shown");
        assert!(s.contains("--yes"), "--yes note shown");
        assert!(s.contains("nothing executes"), "honesty kept");
    }

    #[test]
    fn golden_experimental_banner() {
        let mut app = empty_app();
        app.view = View::Root;
        let s = screen_text(&app);
        assert!(s.contains("EXPERIMENTAL"), "banner rendered");
        assert!(s.contains("execution refused"), "refusal stated");
        assert!(s.contains("magisk-recovery"), "real gate table shown");
    }

    #[test]
    fn golden_help_dialog() {
        let mut app = empty_app();
        handle_key(&mut app, Key::Help);
        let s = screen_text(&app);
        assert!(s.contains("Shortcuts"), "dialog title");
        assert!(s.contains("quit"), "quit row");
        assert!(s.contains("search"), "search row");
        assert!(s.contains("next panel"), "tab row");
        assert!(s.contains("wizard"), "wizard flow covered");
    }

    #[test]
    fn golden_search_filters_and_refused_cli() {
        let mut app = empty_app();
        app.view = View::Root;
        app.focus = Focus::Main;
        handle_key(&mut app, Key::Enter);
        handle_key(&mut app, Key::Enter);
        handle_key(&mut app, Key::Enter);
        handle_key(&mut app, Key::Enter);
        for c in "FLASH".chars() {
            handle_key(&mut app, Key::Char(c));
        }
        handle_key(&mut app, Key::Enter);
        let s = screen_text(&app);
        assert!(s.contains("Step 5/5"), "refused step shown");
        assert!(s.contains("gsi-root device switch"), "exact CLI shown");
    }
}
