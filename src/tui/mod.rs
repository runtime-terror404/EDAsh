mod screens;
mod overlays;

use crate::catalog::resolver::Resolver;
use crossterm::event::{Event, KeyCode, KeyEventKind};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::ExecutableCommand;
use ratatui::backend::CrosstermBackend;
use ratatui::widgets::{Block, Borders};
use ratatui::{Frame, Terminal};
use screens::catalog::{CatalogAction, CatalogScreen, DownloadItem};
use std::collections::HashSet;
use std::io::stdout;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;

enum Overlay { Help }

struct App {
    resolver: Resolver,
    catalog: CatalogScreen,
    overlay: Option<Overlay>,
    quit: bool,
    msg: String,
    msg_ticks: u8,
    progress_rx: Option<std::sync::mpsc::Receiver<ProgressEvent>>,
    downloads: Arc<Mutex<Vec<DownloadItem>>>,
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

impl App {
    fn new(catalog_dir: PathBuf) -> Result<Self, Box<dyn std::error::Error>> {
        let resolver = Resolver::load(&catalog_dir)?;
        let envs = resolver.list_environments();
        let mut catalog = CatalogScreen::new(envs);
        catalog.rebuild_sidebar();
        let names = resolve_tool_names(&resolver, &resolver.list_environments()[0]);
        catalog.refresh_tools(names);
        catalog.load_lockfile();
        Ok(Self {
            resolver,
            catalog,
            overlay: None,
            quit: false,
            msg: String::new(),
            msg_ticks: 0,
            progress_rx: None,
            downloads: Arc::new(Mutex::new(Vec::new())),
        })
    }
}

pub fn run(catalog_dir: PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let mut app = App::new(catalog_dir.clone())?;
    enable_raw_mode()?;
    let mut out = stdout();
    out.execute(EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(out);
    let mut terminal = Terminal::new(backend)?;

    let mut tick: u64 = 0;
    loop {
        tick += 1;

        // Drain progress events from background threads
        if let Some(ref rx) = app.progress_rx {
            let mut has_active = false;
            while let Ok(ev) = rx.try_recv() {
                if let Ok(mut dls) = app.downloads.lock() {
                    if let Some(dl) = dls.iter_mut().find(|d| d.name == ev.tool) {
                        dl.stage = ev.stage.clone();
                        dl.progress = if ev.done { 100 } else { dl.progress.saturating_add(3).min(95) };
                        dl.done_ticks = if ev.done { 60 } else { 0 };
                    } else if !ev.done {
                        dls.push(DownloadItem { name: ev.tool.clone(), progress: 5, stage: ev.stage, done_ticks: 0 });
                    }
                }
                if !ev.done { has_active = true; }
                if ev.done && ev.error.is_none() {
                    // Reload lockfile after successful install
                    app.catalog.load_lockfile();
                }
            }
        }

        // Update downloads: count down done_ticks, remove expired
        if let Ok(mut dls) = app.downloads.lock() {
            dls.retain(|dl| dl.done_ticks > 0 || dl.progress < 100);
            for dl in dls.iter_mut() {
                if dl.done_ticks > 0 { dl.done_ticks -= 1; }
            }
        }

        // Sync to catalog
        if let Ok(dls) = app.downloads.lock() {
            app.catalog.downloads = dls.clone();
        }
        app.catalog.rebuild_sidebar();
        // Reload lockfile periodically to pick up changes
        if tick % 5 == 0 {
            app.catalog.load_lockfile();
        }

        terminal.draw(|f| render(f, &mut app))?;
        if app.quit { break; }
        if !crossterm::event::poll(std::time::Duration::from_millis(100))? { continue; }
        if let Event::Key(key) = crossterm::event::read()? {
            if key.kind == KeyEventKind::Release { continue; }
            handle(&mut app, key.code, &catalog_dir);
        }
    }

    disable_raw_mode()?;
    terminal.backend_mut().execute(LeaveAlternateScreen)?;
    Ok(())
}

fn handle(app: &mut App, code: KeyCode, catalog_dir: &PathBuf) {
    if code == KeyCode::Char('q') && app.overlay.is_none() { app.quit = true; return; }
    if let Some(Overlay::Help) = app.overlay { app.overlay = None; return; }
    if code == KeyCode::Char('?') { app.overlay = Some(Overlay::Help); return; }

    let prev_idx = app.catalog.sidebar_idx;
    let action = app.catalog.handle(code);
    if app.catalog.sidebar_idx != prev_idx {
        if let Some(env) = app.catalog.selected_env_name() {
            let names = resolve_tool_names(&app.resolver, &env);
            app.catalog.refresh_tools(names);
        }
    }

    if let Some(action) = action {
        match action {
            CatalogAction::InstallEnv(name) => {
                app.msg = format!("Installing {}...", name);
                app.msg_ticks = 80;
                // Add all child tools to downloads instead of env name
                let tool_names = resolve_tool_names(&app.resolver, &name);
                for t in &tool_names {
                    app.downloads.lock().unwrap().push(DownloadItem {
                        name: t.clone(), progress: 0, stage: "queued".into(), done_ticks: 0
                    });
                }
                app.catalog.downloads = app.downloads.lock().unwrap().clone();
                app.catalog.rebuild_sidebar();
                spawn_install(app, &name, catalog_dir);
            }
            CatalogAction::InstallTool(name) => {
                app.msg = format!("Installing {}...", name);
                app.msg_ticks = 80;
                spawn_install(app, &name, catalog_dir);
            }
            CatalogAction::InstallPdk(name) => {
                app.msg = format!("Installing PDK {}...", name);
                app.msg_ticks = 80;
                spawn_pdk_install(app, &name, catalog_dir);
            }
            CatalogAction::RemoveEnv(name) => {
                let lock_path = crate::paths::lockfile_path();
                if lock_path.exists() {
                    if let Ok(mut lf) = crate::lockfile::writer::read_lockfile(&lock_path) {
                        let tool_names = resolve_tool_names(&app.resolver, &name);
                        let mut has_oss = false;
                        for t in &tool_names {
                            let pkg_dir = crate::paths::envs_dir().join(format!("_{}", t));
                            if pkg_dir.exists() {
                                let _ = std::process::Command::new("chmod").args(["-R", "u+w", &pkg_dir.to_string_lossy()]).status();
                                let _ = std::fs::remove_dir_all(&pkg_dir);
                            }
                            // Check if this tool is oss-cad-suite
                            if lf.package.iter().any(|p| p.name == *t && p.backend == "oss-cad-suite") {
                                has_oss = true;
                            }
                        }
                        if has_oss {
                            let oss_dir = crate::paths::envs_dir().join("oss-cad-suite");
                            if oss_dir.exists() {
                                let _ = std::process::Command::new("chmod").args(["-R", "u+w", &oss_dir.to_string_lossy()]).status();
                                let _ = std::fs::remove_dir_all(&oss_dir);
                            }
                        }
                        lf.package.retain(|p| !tool_names.contains(&p.name));
                        let _ = crate::lockfile::writer::write_lockfile(&lf, &lock_path);
                        app.catalog.load_lockfile();
                        app.msg = format!("Removed {} ({} tools)", name, tool_names.len());
                        app.msg_ticks = 40;
                    }
                }
            }
            CatalogAction::RemoveTool(name) => {
                let lock_path = crate::paths::lockfile_path();
                if lock_path.exists() {
                    if let Ok(mut lf) = crate::lockfile::writer::read_lockfile(&lock_path) {
                        lf.package.retain(|p| p.name != name);
                        let _ = crate::lockfile::writer::write_lockfile(&lf, &lock_path);
                        app.catalog.load_lockfile();
                    }
                }
                let pkg_dir = crate::paths::envs_dir().join(format!("_{}", name));
                if pkg_dir.exists() {
                    let _ = std::process::Command::new("chmod").args(["-R", "u+w", &pkg_dir.to_string_lossy()]).status();
                    let _ = std::fs::remove_dir_all(&pkg_dir);
                }
                app.msg = format!("Removed {}", name);
                app.msg_ticks = 40;
            }
            CatalogAction::RemovePdk(name) => {
                let lock_path = crate::paths::lockfile_path();
                if lock_path.exists() {
                    if let Ok(mut lf) = crate::lockfile::writer::read_lockfile(&lock_path) {
                        lf.pdk.remove(&name);
                        let _ = crate::lockfile::writer::write_lockfile(&lf, &lock_path);
                        app.catalog.load_lockfile();
                    }
                }
                // Also remove PDK directory
                let pdk_dir = crate::paths::pdks_dir();
                if pdk_dir.exists() {
                    let _ = std::process::Command::new("chmod").args(["-R", "u+w", &pdk_dir.to_string_lossy()]).status();
                    let _ = std::fs::remove_dir_all(&pdk_dir);
                }
                app.msg = format!("Removed PDK {}", name);
                app.msg_ticks = 40;
            }
            _ => {
                app.msg = format!("{:?}", action);
                app.msg_ticks = 40;
            }
        }
    }
}

fn spawn_pdk_install(app: &mut App, name: &str, _catalog_dir: &PathBuf) {
    let name = name.to_string();
    let (tx, rx) = std::sync::mpsc::channel();
    let dl = Arc::clone(&app.downloads);
    let lock_path = crate::paths::lockfile_path();

    // Check if already installed
    if lock_path.exists() {
        if let Ok(lf) = crate::lockfile::writer::read_lockfile(&lock_path) {
            if lf.pdk.contains_key(&name) {
                return; // Already installed, skip
            }
        }
    }

    dl.lock().unwrap().push(DownloadItem { name: name.clone(), progress: 0, stage: "fetching...".into(), done_ticks: 0 });
    app.progress_rx = Some(rx);

    thread::spawn(move || {
        let _ = tx.send(ProgressEvent { tool: name.clone(), stage: "fetching PDK...".into(), done: false, error: None });
        match crate::pdk::ciel::resolve_and_install(&name, &None) {
            Ok(pdk) => {
                let _ = tx.send(ProgressEvent { tool: name.clone(), stage: "done".into(), done: true, error: None });
                let mut lf = if lock_path.exists() {
                    crate::lockfile::writer::read_lockfile(&lock_path).unwrap_or_else(|_| crate::lockfile::schema::Lockfile::new())
                } else {
                    crate::lockfile::schema::Lockfile::new()
                };
                lf.pdk.insert(name.clone(), pdk);
                let _ = crate::lockfile::writer::write_lockfile(&lf, &lock_path);
            }
            Err(e) => {
                let _ = tx.send(ProgressEvent { tool: name.clone(), stage: e.to_string(), done: true, error: Some(e.to_string()) });
            }
        }
    });
}

fn spawn_install(app: &mut App, name: &str, catalog_dir: &PathBuf) {
    let name = name.to_string();
    let cd = catalog_dir.clone();
    let (tx, rx) = std::sync::mpsc::channel();
    let dl = Arc::clone(&app.downloads);
    let lock_path = crate::paths::lockfile_path();

    dl.lock().unwrap().push(DownloadItem { name: name.clone(), progress: 0, stage: "queued".into(), done_ticks: 0 });

    app.progress_rx = Some(rx);

    thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let resolver = match Resolver::load(&cd) {
                Ok(r) => r,
                Err(e) => { let _ = tx.send(ProgressEvent { tool: name.clone(), stage: e.to_string(), done: true, error: Some(e.to_string()) }); return; }
            };
            let items = match resolver.resolve(&name) {
                Ok(i) => i,
                Err(e) => { let _ = tx.send(ProgressEvent { tool: name.clone(), stage: e.to_string(), done: true, error: Some(e.to_string()) }); return; }
            };

            // Load existing lockfile
            let mut lockfile = if lock_path.exists() {
                crate::lockfile::writer::read_lockfile(&lock_path).unwrap_or_else(|_| crate::lockfile::schema::Lockfile::new())
            } else {
                crate::lockfile::schema::Lockfile::new()
            };

            for item in &items {
                let req = match item {
                    crate::catalog::index::ResolvedItem::Tool(r) => r,
                    _ => continue,
                };

                // Skip if already installed
                if lockfile.package.iter().any(|p| p.name == req.name) {
                    let _ = tx.send(ProgressEvent { tool: req.name.clone(), stage: "already installed".into(), done: true, error: None });
                    continue;
                }

                let _ = tx.send(ProgressEvent { tool: req.name.clone(), stage: "installing...".into(), done: false, error: None });

                let (ptx, _prx) = tokio::sync::mpsc::unbounded_channel();

                match req.backend {
                    crate::catalog::index::BackendKind::OssCadSuite => {
                        let backend = crate::backend::oss_cad_suite::OssCadSuiteBackend::new();
                        match backend.install_package(req, ptx) {
                            Ok(pkg) => {
                                lockfile.package.retain(|p| p.name != pkg.name);
                                lockfile.package.push(pkg);
                                let _ = crate::lockfile::writer::write_lockfile(&lockfile, &lock_path);
                                let _ = tx.send(ProgressEvent { tool: req.name.clone(), stage: "done".into(), done: true, error: None });
                            }
                            Err(e) => {
                                let _ = tx.send(ProgressEvent { tool: req.name.clone(), stage: e.to_string(), done: true, error: Some(e.to_string()) });
                            }
                        }
                    }
                    _ => {
                        let backend = crate::backend::micromamba::MicromambaBackend::new();
                        match backend.install_package(req, ptx) {
                            Ok(pkg) => {
                                lockfile.package.retain(|p| p.name != pkg.name);
                                lockfile.package.push(pkg);
                                let _ = crate::lockfile::writer::write_lockfile(&lockfile, &lock_path);
                                let _ = tx.send(ProgressEvent { tool: req.name.clone(), stage: "done".into(), done: true, error: None });
                            }
                            Err(e) => {
                                let _ = tx.send(ProgressEvent { tool: req.name.clone(), stage: e.to_string(), done: true, error: Some(e.to_string()) });
                            }
                        }
                    }
                }
            }
        });
    });
}

fn render(f: &mut Frame, app: &mut App) {
    let area = f.area();
    let footer = if app.msg_ticks > 0 {
        app.msg_ticks -= 1;
        app.msg.clone()
    } else {
        app.catalog.footer()
    };
    let block = Block::new().borders(Borders::ALL).title_top(" edash ").title_bottom(footer);
    let inner = block.inner(area);
    f.render_widget(block, area);
    app.catalog.draw(f, inner);

    if let Some(Overlay::Help) = app.overlay {
        overlays::help::draw(f, area);
    }
}
