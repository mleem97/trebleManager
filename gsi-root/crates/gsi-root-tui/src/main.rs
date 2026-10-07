//! `gsi-root-tui` binary: ratatui + crossterm terminal UI over the domain crates.
//!
//! Read-only views first; the wizard is plan-only and always ends in
//! refused-with-reason plus the exact CLI command to run instead.

mod app;
mod mask;
mod ui;

use app::{Action, App};
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::{backend::Backend, backend::CrosstermBackend, Terminal};
use std::path::PathBuf;
use std::time::Duration;

fn main() {
    std::process::exit(run());
}

fn run() -> i32 {
    let argv: Vec<String> = std::env::args().collect();
    let args = match app::parse_args(&argv) {
        Ok(Action::Help) => {
            println!("{}", app::help_text());
            return 0;
        }
        Ok(Action::Run(a)) => a,
        Err(e) => {
            eprintln!("gsi-root-tui: {e}");
            eprintln!("{}", app::usage_line());
            return 2;
        }
    };
    let mut data = App::new(&args);
    load_runtime(&mut data);
    if enable_raw_mode().is_err() {
        eprintln!("gsi-root-tui: terminal raw mode unavailable");
        return 1;
    }
    let mut stdout = std::io::stdout();
    if stdout.execute(EnterAlternateScreen).is_err() {
        let _ = disable_raw_mode();
        eprintln!("gsi-root-tui: alternate screen unavailable");
        return 1;
    }
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = match Terminal::new(backend) {
        Ok(t) => t,
        Err(e) => {
            let _ = disable_raw_mode();
            eprintln!("gsi-root-tui: terminal unavailable: {e}");
            return 1;
        }
    };
    let code = event_loop(&mut terminal, &mut data);
    let _ = disable_raw_mode();
    let _ = terminal.backend_mut().execute(LeaveAlternateScreen);
    let _ = terminal.show_cursor();
    code
}

fn event_loop<B: Backend>(terminal: &mut Terminal<B>, data: &mut App) -> i32 {
    loop {
        if terminal.draw(|f| ui::render(f, data)).is_err() {
            return 1;
        }
        let ready = event::poll(Duration::from_millis(150)).unwrap_or_default();
        if !ready {
            continue;
        }
        match event::read() {
            Ok(Event::Key(k)) => {
                if k.kind != KeyEventKind::Press {
                    continue;
                }
                if let Some(key) = map_key(k.code, k.modifiers) {
                    if app::handle_key(data, key) {
                        return 0;
                    }
                }
            }
            Ok(_) => {}
            Err(_) => {}
        }
    }
}

fn map_key(code: KeyCode, mods: KeyModifiers) -> Option<app::Key> {
    if mods.contains(KeyModifiers::CONTROL) {
        match code {
            KeyCode::Char('c') | KeyCode::Char('C') => return Some(app::Key::Quit),
            _ => return None,
        }
    }
    match code {
        KeyCode::Up => Some(app::Key::Up),
        KeyCode::Down => Some(app::Key::Down),
        KeyCode::Left => Some(app::Key::Left),
        KeyCode::Right => Some(app::Key::Right),
        KeyCode::Enter => Some(app::Key::Enter),
        KeyCode::Esc => Some(app::Key::Esc),
        KeyCode::Tab => Some(app::Key::Tab),
        KeyCode::Backspace => Some(app::Key::Backspace),
        KeyCode::Char('?') => Some(app::Key::Help),
        KeyCode::Char('/') => Some(app::Key::Search),
        KeyCode::Char('q') | KeyCode::Char('Q') => Some(app::Key::Quit),
        KeyCode::Char(c) => Some(app::Key::Char(c)),
        _ => None,
    }
}

/// Fill read-only view data (filesystem scans only: state files, registry
/// JSON, image magic, PATH lookup). Failures become honest lines, never
/// hard errors; the TUI spawns no processes and touches no device.
fn load_runtime(app: &mut App) {
    load_state(app);
    load_registry(app);
    load_image(app);
    load_tools(app);
}

fn load_state(app: &mut App) {
    let path = if app.state_path.is_empty() {
        match gsi_state::default_workflow_state_file() {
            Some(p) => p,
            None => PathBuf::from("workflow-state.json"),
        }
    } else {
        PathBuf::from(app.state_path.as_str())
    };
    app.state_path = path.display().to_string();
    match gsi_state::read_workflow_state(&path) {
        Ok(Some(wf)) => {
            let mut done = 0;
            for s in wf.steps.iter() {
                if s.status == "done" {
                    done += 1;
                }
            }
            if wf.goal.is_empty() {
                app.state_lines.push("goal: (none)".to_string());
            } else {
                app.state_lines.push(format!("goal: {}", wf.goal));
            }
            app.state_lines
                .push(format!("steps: {done}/{} done", wf.steps.len()));
            if wf.updated.is_empty() {
                app.state_lines.push("updated: unknown".to_string());
            } else {
                app.state_lines.push(format!("updated: {}", wf.updated));
            }
        }
        Ok(None) => app
            .state_lines
            .push("workflow-state: no file (fresh setup)".to_string()),
        Err(e) => app.state_lines.push(format!("workflow-state: {e}")),
    }
    match gsi_state::default_installed_rom_file() {
        Some(p) => match gsi_state::read_installed_rom(&p) {
            Ok(Some(id)) => app.state_lines.push(format!("installed-rom: {id}")),
            Ok(None) => app
                .state_lines
                .push("installed-rom: none recorded".to_string()),
            Err(e) => app.state_lines.push(format!("installed-rom: {e}")),
        },
        None => app
            .state_lines
            .push("installed-rom: no home dir (skipped)".to_string()),
    }
    match gsi_state::read_root_state(&path) {
        Ok(Some(r)) => {
            let boot = match r.boot_mode {
                Some(b) => format!(" ({b})"),
                None => String::new(),
            };
            app.state_lines
                .push(format!("last-root: {}{boot}", r.state));
        }
        Ok(None) => app.state_lines.push("last-root: none recorded".to_string()),
        Err(e) => app.state_lines.push(format!("last-root: {e}")),
    }
    // Slot record is read-only in this wave (no `write_slot` call anywhere).
    let slot = gsi_device::read_slot(&path);
    if slot.occupant == "unknown" {
        app.slot_lines
            .push("occupant: unknown (fresh setup or no state file)".to_string());
    } else {
        app.slot_lines.push(format!("occupant: {}", slot.occupant));
    }
    if !slot.detail.is_empty() {
        app.slot_lines.push(format!("detail: {}", slot.detail));
    }
    app.slot_lines
        .push("shared slot: TWRP and Magisk overwrite each other.".to_string());
}

fn load_registry(app: &mut App) {
    if app.registry_path.is_empty() {
        app.registry_lines
            .push("registry: none loaded (pass --registry <profile.json>).".to_string());
        app.firmware_lines
            .push("firmware: no registry loaded.".to_string());
        return;
    }
    let path = PathBuf::from(app.registry_path.as_str());
    let entries = match gsi_registry::load_roms(&path) {
        Ok(e) => e,
        Err(e) => {
            app.registry_lines.push(format!("registry: {e}"));
            app.firmware_lines.push(format!("firmware: {e}"));
            return;
        }
    };
    let opts = gsi_registry::rom_options(&entries);
    let androids = gsi_registry::target_androids(&entries);
    let broken = gsi_registry::rom_broken(&entries);
    app.registry_lines
        .push(format!("entries: {}", entries.len()));
    app.registry_lines
        .push(format!("selectable systems: {}", opts.len()));
    let mut vers = Vec::new();
    for (a, n) in androids.iter() {
        vers.push(format!("android {a}: {n}"));
    }
    if vers.is_empty() {
        app.registry_lines
            .push("androids: none selectable".to_string());
    } else {
        app.registry_lines
            .push(format!("androids: {}", vers.join(", ")));
    }
    app.registry_lines
        .push(format!("researched broken: {}", broken.len()));
    let mut shown = 0;
    for (_, label) in opts.iter() {
        if shown >= 8 {
            break;
        }
        app.registry_lines.push(format!("  - {label}"));
        shown += 1;
    }
    if opts.len() > shown {
        app.registry_lines
            .push(format!("  ... and {} more", opts.len() - shown));
    }
    let base = gsi_registry::firmware_base(&path);
    if base.is_empty() {
        app.firmware_lines
            .push("required base: not stated in profile".to_string());
    } else {
        app.firmware_lines.push(format!("required base: {base}"));
        let verdict = gsi_gates::firmware_compat("", &base, "");
        app.firmware_lines.push(format!(
            "verdict on base string (model unknown): {}",
            verdict.status
        ));
        if !verdict.fw_cust.is_empty() {
            app.firmware_lines
                .push(format!("region tag: {}", verdict.fw_cust));
        }
        for r in verdict.reasons.iter() {
            app.firmware_lines.push(format!("  - {r}"));
        }
    }
    match gsi_registry::tools_block(&path) {
        Ok(tb) => {
            for os in ["linux", "darwin", "windows"] {
                match gsi_registry::platform_tools_url(&tb, os) {
                    Ok(u) => app.tools_lines.push(format!("platform-tools [{os}]: {u}")),
                    Err(e) => app.tools_lines.push(format!("platform-tools [{os}]: {e}")),
                }
            }
            match gsi_registry::firmware_extractor_url(&tb) {
                Ok(u) => app.tools_lines.push(format!("extractor: {u}")),
                Err(e) => app.tools_lines.push(format!("extractor: {e}")),
            }
        }
        Err(e) => app.tools_lines.push(format!("registry tools: {e}")),
    }
}

fn load_image(app: &mut App) {
    if app.image_path.is_empty() {
        return;
    }
    let path = PathBuf::from(app.image_path.as_str());
    let label = match gsi_image::detect_container(&path) {
        gsi_image::Container::Gzip => "gzip",
        gsi_image::Container::AndroidSparse => "android-sparse",
        gsi_image::Container::Raw => "raw",
    };
    app.image_lines.push(format!("container: {label}"));
    match gsi_image::parse_sparse_header(&path) {
        Some(h) => app.image_lines.push(format!(
            "sparse: blocks={} block_size={} chunks={}",
            h.total_blocks, h.block_size, h.total_chunks
        )),
        None => app.image_lines.push("sparse: no".to_string()),
    }
}

fn load_tools(app: &mut App) {
    let adb = gsi_tool::locate("adb", &[]);
    let fastboot = gsi_tool::locate("fastboot", &[]);
    let adb_s = match adb.as_ref() {
        Some(t) => t.path.display().to_string(),
        None => String::new(),
    };
    let fb_s = match fastboot.as_ref() {
        Some(t) => t.path.display().to_string(),
        None => String::new(),
    };
    if adb_s.is_empty() {
        app.tools_lines
            .push("adb: not found (needs platform-tools on PATH)".to_string());
    } else {
        app.tools_lines.push(format!("adb: {adb_s}"));
    }
    if fb_s.is_empty() {
        app.tools_lines
            .push("fastboot: not found (needs platform-tools on PATH)".to_string());
    } else {
        app.tools_lines.push(format!("fastboot: {fb_s}"));
    }
    app.tools_lines
        .push("version probe skipped (no process spawn in this wave).".to_string());
    let input = gsi_gates::PreflightInput {
        adb_path: adb_s,
        fastboot_path: fb_s,
        shell_ok: true,
        writables: Vec::new(),
        files: Vec::new(),
        hashes: Vec::new(),
    };
    let rep = gsi_gates::preflight(&input);
    app.diag_lines
        .push("shell: assumed compatible (full preflight runs via CLI).".to_string());
    if rep.go {
        app.diag_lines
            .push("preflight inputs: no blocks.".to_string());
    } else {
        app.diag_lines.push("preflight inputs: blocks:".to_string());
        for b in rep.blocks.iter() {
            app.diag_lines.push(format!("  - {b}"));
        }
    }
}
