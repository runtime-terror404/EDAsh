mod widgets;

use crate::catalog::resolver::Resolver;
use crossterm::event::{Event, KeyCode, KeyEventKind};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use crossterm::ExecutableCommand;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap};
use ratatui::{Frame, Terminal};
use std::io::stdout;
use std::path::PathBuf;

// ── palette ──────────────────────────────────────────────────────────────────
const CYAN: Color = Color::Rgb(80, 180, 200);
const GRAY: Color = Color::Rgb(100, 100, 100);

// ── app state ────────────────────────────────────────────────────────────────
struct App {
    resolver: Resolver,
    envs: Vec<String>,
    tools: Vec<(String, String)>,  // name, version
    sel_env: usize,
    sel_tool: usize,
    focus: Focus,                  // which area has keyboard focus
    mode: Mode,                    // what we're doing
    search_query: String,
    search_results: Vec<(String, String)>,  // name, kind
    search_cursor: usize,
    toast: String,
}

#[derive(Clone, Copy, PartialEq)]
enum Focus { Sidebar, Content }

#[derive(PartialEq)]
enum Mode {
    Browse,           // normal dashboard
    SearchEditing,    // search bar has focus, typing
    SearchBrowsing,   // search results visible, can navigate
    Help,
    Quit,
}

impl App {
    fn new(catalog_dir: PathBuf) -> Result<Self, Box<dyn std::error::Error>> {
        let resolver = Resolver::load(&catalog_dir)?;
        let envs = resolver.list_environments();
        let mut app = Self {
            resolver,
            envs,
            tools: Vec::new(),
            sel_env: 0,
            sel_tool: 0,
            focus: Focus::Sidebar,
            mode: Mode::Browse,
            search_query: String::new(),
            search_results: Vec::new(),
            search_cursor: 0,
            toast: String::new(),
        };
        app.refresh_tools();
        Ok(app)
    }

    fn refresh_tools(&mut self) {
        self.tools.clear();
        if self.envs.is_empty() { return; }
        let env = &self.envs[self.sel_env].clone();
        if let Ok(items) = self.resolver.resolve(env) {
            self.tools = items
                .iter()
                .filter_map(|item| match item {
                    crate::catalog::index::ResolvedItem::Tool(r) => {
                        Some((r.name.clone(), "—".into()))
                    }
                    _ => None,
                })
                .collect();
        }
    }

    fn selected_env_name(&self) -> &str {
        self.envs.get(self.sel_env).map(|s| s.as_str()).unwrap_or("")
    }
}

// ── entry ────────────────────────────────────────────────────────────────────
pub fn run(catalog_dir: PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let mut app = App::new(catalog_dir)?;
    enable_raw_mode()?;
    let mut out = stdout();
    out.execute(EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(out);
    let mut terminal = Terminal::new(backend)?;

    loop {
        terminal.draw(|f| render(f, &app))?;
        if app.mode == Mode::Quit { break; }
        if !crossterm::event::poll(std::time::Duration::from_millis(100))? { continue; }
        if let Event::Key(key) = crossterm::event::read()? {
            if key.kind == KeyEventKind::Release { continue; }
            handle(&mut app, key.code);
        }
    }

    disable_raw_mode()?;
    terminal.backend_mut().execute(LeaveAlternateScreen)?;
    Ok(())
}

// ── keyboard ─────────────────────────────────────────────────────────────────
fn handle(app: &mut App, code: KeyCode) {
    app.toast.clear();

    // q always quits from any mode
    if code == KeyCode::Char('q') {
        app.mode = Mode::Quit;
        return;
    }

    // ? always toggles help
    if app.mode != Mode::Help && app.mode != Mode::Quit && code == KeyCode::Char('?') {
        app.mode = Mode::Help;
        return;
    }

    match app.mode {
        Mode::Browse => handle_browse(app, code),
        Mode::SearchEditing => handle_search_edit(app, code),
        Mode::SearchBrowsing => handle_search_browse(app, code),
        Mode::Help => app.mode = Mode::Browse,
        Mode::Quit => {}
    }
}

// ── browse mode (dashboard) ──────────────────────────────────────────────────
fn handle_browse(app: &mut App, code: KeyCode) {
    match code {
        // Navigation
        KeyCode::Down | KeyCode::Char('j') => {
            match app.focus {
                Focus::Sidebar => {
                    if !app.envs.is_empty() {
                        app.sel_env = (app.sel_env + 1) % app.envs.len();
                        app.refresh_tools();
                        app.sel_tool = 0;
                    }
                }
                Focus::Content => {
                    if !app.tools.is_empty() {
                        app.sel_tool = (app.sel_tool + 1) % app.tools.len();
                    }
                }
            }
        }
        KeyCode::Up | KeyCode::Char('k') => {
            match app.focus {
                Focus::Sidebar => {
                    if !app.envs.is_empty() {
                        app.sel_env = app.sel_env.checked_sub(1).unwrap_or(app.envs.len() - 1);
                        app.refresh_tools();
                        app.sel_tool = 0;
                    }
                }
                Focus::Content => {
                    if !app.tools.is_empty() {
                        app.sel_tool = app.sel_tool.checked_sub(1).unwrap_or(app.tools.len() - 1);
                    }
                }
            }
        }

        // Switch pane
        KeyCode::Right | KeyCode::Char('l') => app.focus = Focus::Content,
        KeyCode::Left | KeyCode::Char('h') => app.focus = Focus::Sidebar,
        KeyCode::Tab => {
            app.focus = match app.focus {
                Focus::Sidebar => Focus::Content,
                Focus::Content => Focus::Sidebar,
            };
        }

        // Open search
        KeyCode::Char('/') => {
            app.search_query.clear();
            app.search_results.clear();
            app.search_cursor = 0;
            app.mode = Mode::SearchEditing;
        }

        // Actions on selection
        KeyCode::Enter | KeyCode::Char('i') => {
            match app.focus {
                Focus::Sidebar => {
                    // Open env → move focus to content
                    app.focus = Focus::Content;
                    app.sel_tool = 0;
                }
                Focus::Content => {
                    // Install selected tool
                    if let Some((name, _)) = app.tools.get(app.sel_tool) {
                        app.toast = format!("Run: edash install {}", name);
                    }
                }
            }
        }

        KeyCode::Char('d') => {
            if app.focus == Focus::Sidebar && !app.envs.is_empty() {
                let env = app.envs[app.sel_env].clone();
                app.toast = format!("Run: edash doctor {}", env);
            }
        }

        KeyCode::Char('v') => {
            app.toast = "Run: edash verify".into();
        }

        // Esc from content → sidebar
        KeyCode::Esc => {
            if app.focus == Focus::Content {
                app.focus = Focus::Sidebar;
            }
        }

        _ => {}
    }
}

// ── search editing ───────────────────────────────────────────────────────────
fn handle_search_edit(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Esc => {
            if app.search_query.is_empty() {
                app.mode = Mode::Browse;
            } else {
                // Clear query, stay to browse results or go back
                app.search_query.clear();
                app.search_results.clear();
                app.search_cursor = 0;
                app.mode = Mode::Browse;
            }
        }
        KeyCode::Enter => {
            if !app.search_results.is_empty() {
                // Install first result
                let (name, _) = &app.search_results[app.search_cursor];
                app.toast = format!("Run: edash install {}", name);
            }
            app.mode = Mode::Browse;
        }
        KeyCode::Down => {
            // Move into results browsing
            app.mode = Mode::SearchBrowsing;
        }
        KeyCode::Backspace => {
            app.search_query.pop();
            app.search_cursor = 0;
            refresh_search(app);
        }
        KeyCode::Char(c) => {
            app.search_query.push(c);
            app.search_cursor = 0;
            refresh_search(app);
        }
        _ => {}
    }
}

fn handle_search_browse(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Esc => {
            app.mode = Mode::SearchEditing;
        }
        KeyCode::Enter => {
            if let Some((name, _)) = app.search_results.get(app.search_cursor) {
                app.toast = format!("Run: edash install {}", name);
            }
            app.mode = Mode::Browse;
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if !app.search_results.is_empty() {
                app.search_cursor = (app.search_cursor + 1) % app.search_results.len();
            }
        }
        KeyCode::Up | KeyCode::Char('k') => {
            if !app.search_results.is_empty() {
                app.search_cursor = app.search_cursor.checked_sub(1).unwrap_or(app.search_results.len() - 1);
            }
        }
        _ => {
            // Any other key → back to editing
            handle_search_edit(app, code);
        }
    }
}

fn refresh_search(app: &mut App) {
    let results = app.resolver.search(&app.search_query);
    app.search_results = results.into_iter().map(|e| (e.name, e.kind)).collect();
}

// ── render ───────────────────────────────────────────────────────────────────
fn render(f: &mut Frame, app: &App) {
    let area = f.area();

    // Footer — contextual
    let footer = match app.mode {
        Mode::Help => " any key to dismiss ",
        Mode::SearchEditing | Mode::SearchBrowsing => {
            " esc back  ↵ install  j/k results  type to filter "
        }
        Mode::Browse => match app.focus {
            Focus::Sidebar => " ↑↓/jk move  ↵/i open  tab content  / search  ? help  q quit ",
            Focus::Content => " ↑↓/jk move  ↵/i install  d doctor  v verify  tab sidebar  esc sidebar  / search  ? help  q quit ",
        },
        Mode::Quit => "",
    };

    let main = Block::new()
        .borders(Borders::ALL)
        .title_top(" edash ")
        .title_bottom(footer);
    let inner = main.inner(area);
    f.render_widget(main, area);

    match app.mode {
        Mode::SearchEditing | Mode::SearchBrowsing => render_search(f, inner, app),
        _ => render_dashboard(f, inner, app),
    }

    // Help overlay
    if app.mode == Mode::Help {
        let r = centered(60, 60, area);
        f.render_widget(Clear, r);
        render_help(f, r);
    }

    // Toast
    if !app.toast.is_empty() {
        let r = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0), Constraint::Length(1)])
            .split(area)[1];
        f.render_widget(
            Paragraph::new(app.toast.as_str()).style(Style::new().fg(GRAY)),
            r,
        );
    }
}

// ── dashboard ────────────────────────────────────────────────────────────────
fn render_dashboard(f: &mut Frame, area: Rect, app: &App) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Ratio(1, 4), Constraint::Ratio(3, 4)])
        .split(area);

    // Sidebar — environments
    let envs: Vec<ListItem> = app
        .envs
        .iter()
        .enumerate()
        .map(|(i, name)| {
            let here = i == app.sel_env && app.focus == Focus::Sidebar;
            let txt = if here {
                format!("▸ {}", name)
            } else {
                format!("  {}", name)
            };
            if here {
                ListItem::new(txt).style(Style::new().fg(CYAN))
            } else {
                ListItem::new(txt)
            }
        })
        .collect();
    f.render_widget(
        List::new(envs).block(Block::new().borders(Borders::RIGHT).title("Environments")),
        cols[0],
    );

    // Content — tools
    if !app.envs.is_empty() {
        let tools: Vec<ListItem> = app
            .tools
            .iter()
            .enumerate()
            .map(|(i, (name, ver))| {
                let here = i == app.sel_tool && app.focus == Focus::Content;
                let txt = if here {
                    format!("▸ {:<30} {}", name, ver)
                } else {
                    format!("  {:<30} {}", name, ver)
                };
                if here {
                    ListItem::new(txt).style(Style::new().fg(CYAN))
                } else {
                    ListItem::new(txt)
                }
            })
            .collect();
        f.render_widget(
            List::new(tools).block(
                Block::new()
                    .borders(Borders::NONE)
                    .title(app.selected_env_name()),
            ),
            cols[1],
        );
    }
}

// ── search ───────────────────────────────────────────────────────────────────
fn render_search(f: &mut Frame, area: Rect, app: &App) {
    let v = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(4), Constraint::Min(0)])
        .split(area);

    // Search input bar
    let cursor_char = if app.mode == Mode::SearchEditing {
        "█"
    } else {
        ""
    };
    let editing = app.mode == Mode::SearchEditing;
    let input_block = if editing {
        Block::new()
            .borders(Borders::ALL)
            .title(" search ")
            .style(Style::new().fg(CYAN))
    } else {
        Block::new().borders(Borders::ALL).title(" search ")
    };
    let input = Paragraph::new(format!("> {}{}", app.search_query, cursor_char)).block(input_block);
    f.render_widget(input, v[0]);

    // Results
    let items: Vec<ListItem> = app
        .search_results
        .iter()
        .enumerate()
        .map(|(i, (name, kind))| {
            let here = app.mode == Mode::SearchBrowsing && i == app.search_cursor;
            let txt = if here {
                format!("▸ {:<30} [{}]", name, kind)
            } else {
                format!("  {:<30} [{}]", name, kind)
            };
            if here {
                ListItem::new(txt).style(Style::new().fg(CYAN))
            } else {
                ListItem::new(txt)
            }
        })
        .collect();
    let n = app.search_results.len();
    f.render_widget(
        List::new(items).block(Block::new().borders(Borders::ALL).title(format!(
            " results ({}) ",
            n
        ))),
        v[1],
    );
}

// ── help overlay ─────────────────────────────────────────────────────────────
fn render_help(f: &mut Frame, area: Rect) {
    let k = Style::new().fg(CYAN);
    let text = Paragraph::new(vec![
        Line::from(""),
        Line::from(vec![Span::raw("  "), Span::styled("Navigation", k), Span::raw("")]),
        Line::from(vec![Span::raw("    "), Span::styled("↑↓ / j k", k), Span::raw("    move selection")]),
        Line::from(vec![Span::raw("    "), Span::styled("←→ / h l / tab", k), Span::raw("  switch pane")]),
        Line::from(vec![Span::raw("    "), Span::styled("esc", k), Span::raw("          back one level")]),
        Line::from(vec![Span::raw("")]),
        Line::from(vec![Span::raw("  "), Span::styled("Actions", k), Span::raw("")]),
        Line::from(vec![Span::raw("    "), Span::styled("↵ / i", k), Span::raw("        install selected")]),
        Line::from(vec![Span::raw("    "), Span::styled("d", k), Span::raw("            run doctor on env")]),
        Line::from(vec![Span::raw("    "), Span::styled("v", k), Span::raw("            verify")]),
        Line::from(vec![Span::raw("")]),
        Line::from(vec![Span::raw("  "), Span::styled("Search", k), Span::raw("")]),
        Line::from(vec![Span::raw("    "), Span::styled("/", k), Span::raw("            open search")]),
        Line::from(vec![Span::raw("    "), Span::styled("type", k), Span::raw("         filter results")]),
        Line::from(vec![Span::raw("    "), Span::styled("↵", k), Span::raw("            install result")]),
        Line::from(vec![Span::raw("    "), Span::styled("esc", k), Span::raw("          close search")]),
        Line::from(vec![Span::raw("")]),
        Line::from(vec![Span::raw("    "), Span::styled("?", k), Span::raw("            this help")]),
        Line::from(vec![Span::raw("    "), Span::styled("q", k), Span::raw("            quit")]),
    ])
    .block(Block::new().borders(Borders::ALL).title(" help "))
    .wrap(Wrap { trim: true });
    f.render_widget(text, area);
}

fn centered(px: u16, py: u16, r: Rect) -> Rect {
    let v = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - py) / 2),
            Constraint::Percentage(py),
            Constraint::Percentage((100 - py) / 2),
        ])
        .split(r);
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - px) / 2),
            Constraint::Percentage(px),
            Constraint::Percentage((100 - px) / 2),
        ])
        .split(v[1])[1]
}
