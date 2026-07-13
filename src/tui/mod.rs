mod screens;
mod overlays;
pub mod widgets;

use crate::catalog::resolver::Resolver;
use crate::catalog::CatalogSource;
use crossterm::event::{Event, KeyCode, KeyEventKind};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::ExecutableCommand;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Layout};
use ratatui::style::{Color, Style};
use ratatui::text::Span;
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::{Frame, Terminal};
use screens::catalog::{CatalogAction, CatalogScreen, DoctorLine, DownloadItem};
use std::collections::HashSet;
use std::io::stdout;
use std::sync::{Arc, Mutex};
use std::thread;

enum Overlay {
    Help,
    Confirm(CatalogAction, String),
}

struct App {
    resolver: Resolver,
    catalog: CatalogScreen,
    overlay: Option<Overlay>,
    quit: bool,
    msg: String,
    msg_ticks: u8,
    progress_tx: std::sync::mpsc::Sender<ProgressEvent>,
    progress_rx: std::sync::mpsc::Receiver<ProgressEvent>,
    downloads: Arc<Mutex<Vec<DownloadItem>>>,
    last_key_time: std::time::Instant,
}

#[derive(Debug, Clone)]
struct ProgressEvent {
    tool: String,
    stage: String,
    done: bool,
    error: Option<String>,
}

fn resolve_tool_names(resolver: &Resolver, env_name: &str) -> Vec<String> {
    let mut names = Vec::new();
    if let Ok(items) = resolver.resolve(env_name) {
        let mut seen = HashSet::new();
        for item in &items {
            if let crate::catalog::index::ResolvedItem::Tool(r) = item {
                if seen.insert(r.name.clone()) {
                    names.push(r.name.clone());
                }
            }
        }
    }
    names
}

/// Which other envs (besides the given one) use this tool?
fn other_envs_using(resolver: &Resolver, tool: &str, current_env: &str) -> Vec<String> {
    resolver
        .which_envs(tool)
        .into_iter()
        .filter(|e| e != current_env)
        .collect()
}

/// Build a confirmation message for destructive actions.
fn confirm_message(resolver: &Resolver, action: &CatalogAction) -> Option<String> {
    match action {
        CatalogAction::RemoveEnv(name) => {
            let tool_names = resolve_tool_names(resolver, name);
            let total = tool_names.len();
            let shared: Vec<&String> = tool_names.iter().filter(|t| !other_envs_using(resolver, t, name).is_empty()).collect();
            let remove = total - shared.len();
            Some(format!(
                "Remove profile \"{}\"?\n\n{} packages will be removed.\n{} shared packages will be kept.",
                name, remove, shared.len()
            ))
        }
        CatalogAction::RemoveTool(name) => {
            let others = other_envs_using(resolver, name, "");
            if others.is_empty() {
                Some(format!("Remove \"{}\"?", name))
            } else {
                let mut msg = format!("Remove \"{}\"?\n\nRequired by:\n", name);
                for env in &others {
                    msg.push_str(&format!("  \u{2022} {}\n", env));
                }
                msg.push_str("\nRemoving it will make these profiles incomplete.");
                Some(msg)
            }
        }
        CatalogAction::RemovePdk(name) => Some(format!("Remove PDK \"{}\"?", name)),
        CatalogAction::RemoveAllPdks => Some("Remove all installed PDKs?".into()),
        _ => None,
    }
}

impl App {
    fn new(source: CatalogSource) -> Result<Self, Box<dyn std::error::Error>> {
        let resolver = Resolver::load_from(&source)?;
        let mut envs = resolver.list_environments();
        envs.sort();
        let mut catalog = CatalogScreen::new(envs.clone(), &source);
        catalog.rebuild_sidebar();
        for env in &envs {
            let names = resolve_tool_names(&resolver, env);
            catalog.refresh_tools_for(env, names);
        }
        catalog.load_lockfile();
        let (tx, rx) = std::sync::mpsc::channel();
        Ok(Self {
            resolver,
            catalog,
            overlay: None,
            quit: false,
            msg: String::new(),
            msg_ticks: 0,
            progress_tx: tx,
            progress_rx: rx,
            downloads: Arc::new(Mutex::new(Vec::new())),
            last_key_time: std::time::Instant::now(),
        })
    }
}

pub fn run(source: CatalogSource) -> Result<Option<String>, Box<dyn std::error::Error>> {
    let mut app = App::new(source.clone())?;
    enable_raw_mode()?;
    let mut out = stdout();
    out.execute(EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(out);
    let mut terminal = Terminal::new(backend)?;

    let mut tick: u64 = 0;
    loop {
        tick += 1;
        app.catalog.tick = tick;

        // Drain progress events from all background threads
        while let Ok(ev) = app.progress_rx.try_recv() {
            let stage = ev.stage.clone();

            // Doctor events are handled separately — never touch downloads
            let is_doctor = app.catalog.doctor_running
                || stage.starts_with("PASS|") || stage.starts_with("FAIL|")
                || stage == "PNF" || stage == "DONE";
            if is_doctor {
                if stage == "DONE" {
                    app.catalog.doctor_running = false;
                } else if stage == "PNF" {
                    app.catalog.doctor_results.push(DoctorLine {
                        name: ev.tool.clone(), passed: false, detail: "binary not found".into(), elapsed: 0.0,
                    });
                } else if let Some(rest) = stage.strip_prefix("PASS|") {
                    let parts: Vec<&str> = rest.splitn(2, '|').collect();
                    app.catalog.doctor_results.push(DoctorLine {
                        name: ev.tool.clone(), passed: true,
                        detail: parts.get(1).unwrap_or(&"").to_string(),
                        elapsed: parts[0].parse().unwrap_or(0.0),
                    });
                } else if let Some(rest) = stage.strip_prefix("FAIL|") {
                    let parts: Vec<&str> = rest.splitn(2, '|').collect();
                    app.catalog.doctor_results.push(DoctorLine {
                        name: ev.tool.clone(), passed: false,
                        detail: parts.get(1).unwrap_or(&"").to_string(),
                        elapsed: parts[0].parse().unwrap_or(0.0),
                    });
                }
                continue;
            }

            if let Ok(mut dls) = app.downloads.lock() {
                if let Some(dl) = dls.iter_mut().find(|d| d.name == ev.tool) {
                    dl.stage = stage.clone();
                    dl.progress = if ev.done { 100 } else { dl.progress.saturating_add(3).min(95) };
                    dl.done_ticks = if ev.done { 10 } else { 0 };
                } else if !ev.done {
                    dls.push(DownloadItem { name: ev.tool.clone(), progress: 5, stage: stage.clone(), done_ticks: 0 });
                }
            }
            if ev.done && ev.error.is_none() {
                app.catalog.load_lockfile();
            }
        }

        // Update downloads: count down done_ticks, remove expired
        if let Ok(mut dls) = app.downloads.lock() {
            dls.retain(|dl| dl.done_ticks > 0 || dl.progress < 100);
            for dl in dls.iter_mut() {
                if dl.done_ticks > 0 {
                    dl.done_ticks -= 1;
                }
            }
        }

        if let Ok(dls) = app.downloads.lock() {
            app.catalog.downloads = dls.clone();
        }
        app.catalog.rebuild_sidebar();
        if tick % 5 == 0 {
            app.catalog.load_lockfile();
        }

        terminal.draw(|f| render(f, &mut app))?;
        if app.quit {
            let shell = if app.msg.starts_with("SHELL:") {
                Some(app.msg.trim_start_matches("SHELL:").to_string())
            } else {
                None
            };
            disable_raw_mode()?;
            terminal.backend_mut().execute(LeaveAlternateScreen)?;
            return Ok(shell);
        }
        if !crossterm::event::poll(std::time::Duration::from_millis(100))? {
            continue;
        }
        if let Event::Key(key) = crossterm::event::read()? {
            if key.kind == KeyEventKind::Release {
                continue;
            }
            // Debounce rapid key repeats (scrolling mice, held keys)
            let now = std::time::Instant::now();
            if now.duration_since(app.last_key_time) < std::time::Duration::from_millis(30) {
                continue;
            }
            app.last_key_time = now;
            handle(&mut app, key.code, &source);
        }
    }
}

fn handle(app: &mut App, code: KeyCode, source: &CatalogSource) {
    // Confirm overlay intercepts everything until resolved.
    if matches!(app.overlay, Some(Overlay::Confirm(_, _))) {
        match code {
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                if let Some(Overlay::Confirm(action, _)) = app.overlay.take() {
                    apply_action(app, action, source);
                }
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                app.overlay = None;
            }
            _ => {}
        }
        return;
    }

    if code == KeyCode::Char('q') && app.overlay.is_none() {
        app.quit = true;
        return;
    }
    if matches!(app.overlay, Some(Overlay::Help)) {
        app.overlay = None;
        return;
    }
    if code == KeyCode::Char('?') {
        app.overlay = Some(Overlay::Help);
        return;
    }

    let prev_idx = app.catalog.sidebar_idx;
    let action = app.catalog.handle(code);
    if app.catalog.sidebar_idx != prev_idx {
        if let Some(env) = app.catalog.selected_env_name() {
            let names = resolve_tool_names(&app.resolver, &env);
            app.catalog.refresh_tools(names);
        }
    }

    if let Some(action) = action {
        // Skip confirm for tools that aren't installed
        if let CatalogAction::RemoveTool(ref name) = action {
            let installed = app.catalog.tools.iter().any(|p| p.name == *name)
                || crate::paths::envs_dir().join(format!("_{}", name)).exists();
            if !installed {
                app.msg = format!("'{}' is not installed", name);
                app.msg_ticks = 40;
                return;
            }
        }
        if let Some(msg) = confirm_message(&app.resolver, &action) {
            app.overlay = Some(Overlay::Confirm(action, msg));
        } else {
            apply_action(app, action, source);
        }
    }
}

fn apply_action(app: &mut App, action: CatalogAction, source: &CatalogSource) {
    match action {
        CatalogAction::InstallEnv(name) => {
            app.msg = format!("Installing {}...", name);
            app.msg_ticks = 80;
            let tool_names = resolve_tool_names(&app.resolver, &name);
            for t in &tool_names {
                app.downloads
                    .lock()
                    .unwrap()
                    .push(DownloadItem { name: t.clone(), progress: 0, stage: "queued".into(), done_ticks: 0 });
            }
            app.catalog.downloads = app.downloads.lock().unwrap().clone();
            app.catalog.rebuild_sidebar();
            spawn_install(app, &name, source);
        }
        CatalogAction::InstallTool(name) => {
            // Skip if already installed or already queued
            if app.catalog.tools.iter().any(|p| p.name == name) {
                app.msg = format!("'{}' is already installed", name);
                app.msg_ticks = 40;
                return;
            }
            if app.downloads.lock().unwrap().iter().any(|d| d.name == name && d.progress < 100) {
                app.msg = format!("'{}' is already in queue", name);
                app.msg_ticks = 40;
                return;
            }
            app.msg = format!("Installing {}...", name);
            app.msg_ticks = 80;
            app.downloads
                .lock()
                .unwrap()
                .push(DownloadItem { name: name.clone(), progress: 0, stage: "queued".into(), done_ticks: 0 });
            app.catalog.downloads = app.downloads.lock().unwrap().clone();
            spawn_install(app, &name, source);
        }
        CatalogAction::InstallPdk(name) => {
            app.msg = format!("Installing PDK {}...", name);
            app.msg_ticks = 80;
            spawn_pdk_install(app, &name, source);
        }
        CatalogAction::RemoveEnv(name) => {
            let tool_names = resolve_tool_names(&app.resolver, &name);
            let shared_check = |t: &str| -> bool {
                !other_envs_using(&app.resolver, t, &name).is_empty()
            };
            match crate::actions::remove_env(&name, &tool_names, &shared_check) {
                Ok((removed, skipped)) => {
                    app.catalog.load_lockfile();
                    if skipped > 0 {
                        app.msg = format!("Removed {} tools from {} ({} shared tools kept)", removed, name, skipped);
                    } else {
                        app.msg = format!("Removed {} ({} tools)", name, removed);
                    }
                    app.msg_ticks = 40;
                }
                Err(e) => {
                    app.msg = format!("Error: {}", e);
                    app.msg_ticks = 40;
                }
            }
        }
        CatalogAction::RemoveTool(name) => {
            match crate::actions::remove_tool(&name) {
                Ok(true) => {
                    app.catalog.load_lockfile();
                    app.msg = format!("Removed {}", name);
                }
                Ok(false) => {
                    app.msg = format!("'{}' not found", name);
                }
                Err(e) => {
                    app.msg = format!("Error: {}", e);
                }
            }
            app.msg_ticks = 40;
        }
        CatalogAction::RemoveAllPdks => {
            match crate::actions::remove_all_pdks() {
                Ok(count) => {
                    app.catalog.load_lockfile();
                    app.msg = format!("Removed {} PDKs", count);
                    app.msg_ticks = 40;
                }
                Err(e) => {
                    app.msg = format!("Error: {}", e);
                    app.msg_ticks = 40;
                }
            }
        }
        CatalogAction::RemovePdk(name) => {
            match crate::actions::remove_pdk(&name) {
                Ok(true) => {
                    app.catalog.load_lockfile();
                    app.msg = format!("Removed PDK {}", name);
                }
                Ok(false) => {
                    app.msg = format!("PDK '{}' not found", name);
                }
                Err(e) => {
                    app.msg = format!("Error: {}", e);
                }
            }
            app.msg_ticks = 40;
        }
        CatalogAction::Doctor(name) | CatalogAction::DoctorTool(name) => {
            spawn_doctor(app, &name, source);
        }
        CatalogAction::Shell(name) => {
            if name.is_empty() {
                app.msg = "select a valid environment first".into();
                app.msg_ticks = 40;
            } else {
                app.quit = true;
                app.msg = format!("SHELL:{}", name);
            }
        }
    }
}

fn spawn_pdk_install(app: &mut App, name: &str, _source: &CatalogSource) {
    let name = name.to_string();
    let tx = app.progress_tx.clone();
    let dl = Arc::clone(&app.downloads);

    // Check idempotency
    let lock_path = crate::paths::lockfile_path();
    if lock_path.exists() {
        if let Ok(lf) = crate::lockfile::writer::read_lockfile(&lock_path) {
            if lf.pdk.contains_key(&name) {
                return;
            }
        }
    }

    dl.lock().unwrap().push(DownloadItem { name: name.clone(), progress: 0, stage: "fetching...".into(), done_ticks: 0 });

    thread::spawn(move || {
        let _ = tx.send(ProgressEvent { tool: name.clone(), stage: "fetching PDK...".into(), done: false, error: None });
        match crate::actions::install_pdk(&name) {
            Ok(_) => {
                let _ = tx.send(ProgressEvent { tool: name.clone(), stage: "done".into(), done: true, error: None });
            }
            Err(e) => {
                let _ = tx.send(ProgressEvent { tool: name.clone(), stage: e.to_string(), done: true, error: Some(e.to_string()) });
            }
        }
    });
}

fn spawn_doctor(app: &mut App, name: &str, source: &CatalogSource) {
    let name = name.to_string();
    let src = source.clone();
    let tx = app.progress_tx.clone();

    app.catalog.doctor_running = true;
    app.catalog.doctor_results.clear();

    thread::spawn(move || {
        let resolver = match crate::catalog::resolver::Resolver::load_from(&src) {
            Ok(r) => r,
            Err(e) => {
                let _ = tx.send(ProgressEvent { tool: name.clone(), stage: e.to_string(), done: true, error: Some(e.to_string()) });
                return;
            }
        };
        let items = match resolver.resolve(&name) {
            Ok(i) => i,
            Err(e) => {
                let _ = tx.send(ProgressEvent { tool: name.clone(), stage: e.to_string(), done: true, error: Some(e.to_string()) });
                return;
            }
        };
        let envs_dir = crate::paths::envs_dir();

        for item in &items {
            let req = match item {
                crate::catalog::index::ResolvedItem::Tool(r) => r,
                _ => continue,
            };
            let bin_dir = match req.backend {
                crate::catalog::index::BackendKind::OssCadSuite => envs_dir.join("oss-cad-suite").join("bin"),
                _ => envs_dir.join(format!("_{}", req.name)).join("bin"),
            };
            let bin_name = match req.name.as_str() {
                "xyce" => "Xyce",
                "nextpnr" => "nextpnr-ecp5",
                "icestorm" => "icepack",
                "prjtrellis" => "ecppack",
                "openfpgaloader" => "openFPGALoader",
                _ => &req.name,
            };
            let bin_path = bin_dir.join(bin_name);

            if !bin_path.exists() {
                let _ = tx.send(ProgressEvent { tool: req.name.clone(), stage: "PNF".into(), done: false, error: None });
                continue;
            }

            let result = crate::doctor::checks::run_check(&req.name, &bin_path.to_string_lossy());
            let _ = tx.send(ProgressEvent {
                tool: req.name.clone(),
                stage: format!("{}|{:.1}|{}", if result.passed { "PASS" } else { "FAIL" }, result.duration_ms as f64 / 1000.0, result.detail),
                done: false,
                error: if result.passed { None } else { Some(result.detail.clone()) },
            });
        }
        let _ = tx.send(ProgressEvent { tool: name.clone(), stage: "DONE".into(), done: true, error: None });
    });
}

fn spawn_install(app: &mut App, name: &str, source: &CatalogSource) {
    let name = name.to_string();
    let src = source.clone();
    let tx = app.progress_tx.clone();
    let dl = Arc::clone(&app.downloads);

    thread::spawn(move || {
        let resolver = match Resolver::load_from(&src) {
            Ok(r) => r,
            Err(e) => {
                let _ = tx.send(ProgressEvent { tool: name.clone(), stage: e.to_string(), done: true, error: Some(e.to_string()) });
                return;
            }
        };
        let items = match resolver.resolve(&name) {
            Ok(i) => i,
            Err(e) => {
                let _ = tx.send(ProgressEvent { tool: name.clone(), stage: e.to_string(), done: true, error: Some(e.to_string()) });
                return;
            }
        };

        for item in &items {
            let req = match item {
                crate::catalog::index::ResolvedItem::Tool(r) => r,
                _ => continue,
            };

            // Check idempotency
            let lock_path = crate::paths::lockfile_path();
            if lock_path.exists() {
                if let Ok(lf) = crate::lockfile::writer::read_lockfile(&lock_path) {
                    if lf.package.iter().any(|p| p.name == req.name)
                        && crate::paths::envs_dir().join(format!("_{}", req.name)).exists()
                    {
                        let _ = tx.send(ProgressEvent { tool: req.name.clone(), stage: "already installed".into(), done: true, error: None });
                        continue;
                    }
                }
            }

            let _ = dl.lock().map(|mut d| {
                if !d.iter().any(|x| x.name == req.name) {
                    d.push(DownloadItem { name: req.name.clone(), progress: 0, stage: "queued".into(), done_ticks: 0 });
                }
            });
            let _ = tx.send(ProgressEvent { tool: req.name.clone(), stage: "installing...".into(), done: false, error: None });

            match crate::actions::install_tool(req) {
                Ok(_pkg) => {
                    let _ = tx.send(ProgressEvent { tool: req.name.clone(), stage: "done".into(), done: true, error: None });
                }
                Err(e) => {
                    let _ = tx.send(ProgressEvent { tool: req.name.clone(), stage: e.to_string(), done: true, error: Some(e.to_string()) });
                }
            }
        }
    });
}

fn render(f: &mut Frame, app: &mut App) {
    let area = f.area();
    let version = env!("CARGO_PKG_VERSION");

    let footer_text = if app.msg_ticks > 0 {
        app.msg_ticks -= 1;
        app.msg.clone()
    } else {
        app.catalog.footer()
    };

    let block = Block::new().borders(Borders::ALL);
    let inner = block.inner(area);
    f.render_widget(block, area);

    // Footer with dim action names
    let footer_line = ratatui::text::Line::from(
        footer_text
            .split_whitespace()
            .enumerate()
            .map(|(i, word)| {
                let is_key = i % 2 == 0 || word == "i" || word == "r" || word == "/" || word == "?" || word == "q";
                if is_key || word.starts_with('←') || word.starts_with('↑') || word.starts_with('→') || word.starts_with('↓') || word == "i" || word == "r" || word == "/" || word == "?" || word == "q" || word == "v" || word.starts_with("↵") {
                    Span::raw(format!("{} ", word))
                } else {
                    Span::styled(format!("{} ", word), Style::new().fg(Color::Rgb(120, 120, 120)))
                }
            })
            .collect::<Vec<Span>>(),
    );
    let footer_area = Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(1)])
        .split(inner)[1];
    f.render_widget(Paragraph::new(footer_line), footer_area);

    let content_area = Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(1)])
        .split(inner)[0];

    // Title (6 lines) then main content
    let v = Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .constraints([Constraint::Length(6), Constraint::Min(1)])
        .split(content_area);

    let ver = format!("{:>67}", format!("v{}", version));
    let sep = "─".repeat(130);
    let title = format!(
        "────┐        ┌──────┐   ┌─┐     ┌────────┐      ┌───────┐ {ver}\n    └────────┘      └───┘ └─────┘        └──────┘       └─────────────────────────────────────────────────────────────────────────\n \u{2007}░█▀▀░█▀▄░█▀█░█▀▀░█░█\n \u{2007}░█▀▀░█░█░█▀█░▀▀█░█▀█                       Unified EDA Package Manager Built on Rust.\n \u{2007}░▀▀▀░▀▀░░▀░▀░▀▀▀░▀░▀\n{sep}"
    );
    f.render_widget(Paragraph::new(title), v[0]);

    app.catalog.draw(f, v[1]);

    match &app.overlay {
        Some(Overlay::Help) => overlays::help::draw(f, area),
        Some(Overlay::Confirm(_, msg)) => overlays::confirm::draw(f, area, msg),
        None => {}
    }
}
