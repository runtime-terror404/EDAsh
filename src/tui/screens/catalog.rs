use crate::lockfile::schema::LockedPackage;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph, Row, Table};
use ratatui::Frame;

const CYAN: Color = Color::Rgb(80, 180, 200);
const DIM: Color = Color::Rgb(100, 100, 100);

#[derive(Clone, Copy, PartialEq)]
pub enum CatalogFocus { Sidebar, Results }

pub struct CatalogScreen {
    pub envs: Vec<String>,
    pub pdks: Vec<(String, String, String)>, // name, variant, status
    pub downloads: Vec<DownloadItem>,

    pub sidebar_idx: usize,  // 0=analog, 1=digital, 2=PDKs, then downloads
    pub tool_idx: usize,
    pub pdk_idx: usize,

    pub search_query: String,

    pub tools: Vec<LockedPackage>,
    pub resolved_tools: Vec<String>,

    pub focus: CatalogFocus,
    pub show_pdks: bool, // true when PDKs selected in sidebar
}

#[derive(Clone)]
pub struct DownloadItem {
    pub name: String,
    pub progress: u8,
    pub stage: String,
    pub done_ticks: u16,
}

impl CatalogScreen {
    pub fn new(envs: Vec<String>) -> Self {
        let pdks = Self::load_pdks();
        Self {
            envs,
            pdks,
            downloads: Vec::new(),
            sidebar_idx: 0,
            tool_idx: 0,
            pdk_idx: 0,
            search_query: String::new(),
            tools: Vec::new(),
            resolved_tools: Vec::new(),
            focus: CatalogFocus::Sidebar,
            show_pdks: false,
        }
    }

    fn load_pdks() -> Vec<(String, String, String)> {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("catalog/index.yaml");
        if let Ok(data) = std::fs::read_to_string(&path) {
            if let Ok(idx) = serde_yaml::from_str::<serde_yaml::Value>(&data) {
                if let Some(pdks) = idx.get("pdks").and_then(|p| p.as_mapping()) {
                    return pdks.iter().map(|(k, v)| {
                        let name = k.as_str().unwrap_or("?").to_string();
                        let variant = v.get("variant").and_then(|x| x.as_str()).unwrap_or("?").to_string();
                        (name, variant, "not installed".to_string())
                    }).collect();
                }
            }
        }
        Vec::new()
    }

    /// Number of selectable sidebar rows
    fn sidebar_len(&self) -> usize {
        self.envs.len() + 2 // envs + PDKs + Downloads
    }

    fn in_downloads(&self, idx: usize) -> bool {
        idx > self.envs.len() // after envs and PDKs
    }

    pub fn rebuild_sidebar(&mut self) {}

    pub fn refresh_tools(&mut self, tool_names: Vec<String>) {
        self.resolved_tools = tool_names;
        self.tool_idx = 0;
        self.show_pdks = self.sidebar_idx == self.envs.len();
    }

    pub fn selected_env_name(&self) -> Option<String> {
        if self.sidebar_idx < self.envs.len() {
            Some(self.envs[self.sidebar_idx].clone())
        } else { None }
    }

    pub fn load_lockfile(&mut self) {
        let lp = crate::paths::lockfile_path();
        if lp.exists() {
            if let Ok(lf) = crate::lockfile::writer::read_lockfile(&lp) {
                self.tools = lf.package;
                // Update PDK statuses
                for (name, _, status) in &mut self.pdks {
                    *status = if lf.pdk.contains_key(name) { "installed".into() } else { "not installed".into() };
                }
            }
        }
    }

    // ── draw ──
    pub fn draw(&self, f: &mut Frame, area: Rect) {
        let cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Ratio(1, 5), Constraint::Ratio(4, 5)])
            .split(area);

        self.draw_sidebar(f, cols[0]);

        let right = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(0)])
            .split(cols[1]);

        self.draw_search(f, right[0]);
        self.draw_content(f, right[1]);
    }

    // ── sidebar ──
    fn draw_sidebar(&self, f: &mut Frame, area: Rect) {
        let mut items: Vec<ListItem> = Vec::new();

        // Environments header
        items.push(ListItem::new("Environments").style(Style::new().fg(DIM)));

        // Envs
        for (i, name) in self.envs.iter().enumerate() {
            let sel = i == self.sidebar_idx && self.focus == CatalogFocus::Sidebar;
            let installed = self.resolved_tools.iter().filter(|t| {
                self.tools.iter().any(|lp| &lp.name == *t)
            }).count();
            let total = self.resolved_tools.len();
            let txt = if sel {
                format!("▸   {}", name)
            } else {
                format!("    {}", name)
            };
            let style = if sel { Style::new().fg(CYAN) } else { Style::new() };
            items.push(ListItem::new(txt).style(style));
        }

        // PDKs row
        let pdk_idx = self.envs.len();
        let pdk_sel = self.sidebar_idx == pdk_idx && self.focus == CatalogFocus::Sidebar;
        let pdk_txt = if pdk_sel {
            format!("▸ PDKs ({})", self.pdks.len())
        } else {
            format!("  PDKs ({})", self.pdks.len())
        };
        items.push(ListItem::new(pdk_txt).style(if pdk_sel { Style::new().fg(CYAN) } else { Style::new() }));

        // Downloads section — always visible
        items.push(ListItem::new(""));
        let dl_idx = pdk_idx + 1;
        let dl_sel = self.sidebar_idx == dl_idx && self.focus == CatalogFocus::Sidebar;
        let dl_text = if dl_sel {
            format!("▸ Downloads({})", self.downloads.len())
        } else {
            format!("  Downloads({})", self.downloads.len())
        };
        items.push(ListItem::new(dl_text).style(if dl_sel { Style::new().fg(CYAN) } else { Style::new() }));

        f.render_widget(
            List::new(items).block(Block::new().borders(Borders::RIGHT)),
            area,
        );
    }

    // ── search ──
    fn draw_search(&self, f: &mut Frame, area: Rect) {
        let display = if self.search_query.is_empty() {
            "> / to search".to_string()
        } else {
            format!("> {}", self.search_query)
        };
        f.render_widget(
            Paragraph::new(display).block(Block::new().borders(Borders::ALL).title(" Search ")),
            area,
        );
    }

    // ── content ──
    fn draw_content(&self, f: &mut Frame, area: Rect) {
        let is_pdks = self.sidebar_idx == self.envs.len();
        if is_pdks {
            self.draw_pdk_table(f, area);
        } else if self.in_downloads(self.sidebar_idx) {
            self.draw_download_queue(f, area);
        } else if self.sidebar_idx < self.envs.len() {
            self.draw_tool_table(f, area);
        }
    }

    // ── tool table ──
    fn draw_tool_table(&self, f: &mut Frame, area: Rect) {
        let filtered: Vec<&String> = if self.search_query.is_empty() {
            self.resolved_tools.iter().collect()
        } else {
            let q = self.search_query.to_lowercase();
            self.resolved_tools.iter().filter(|n| n.to_lowercase().contains(&q)).collect()
        };

        if filtered.is_empty() {
            let env_name = self.envs.get(self.sidebar_idx).map(|s| s.as_str()).unwrap_or("?");
            let block = Block::new().borders(Borders::ALL).title(format!(" {} ", env_name));
            f.render_widget(Paragraph::new("No tools found").block(block), area);
            return;
        }

        let cursor = self.tool_idx.min(filtered.len().saturating_sub(1));
        let env_name = self.envs.get(self.sidebar_idx).map(|s| s.as_str()).unwrap_or("?");

        let header = Row::new(vec!["", "Tool", "Version", "Backend", "Status"]).style(Style::new().fg(DIM));

        let rows: Vec<Row> = filtered.iter().enumerate().map(|(i, name)| {
            let lp = self.tools.iter().find(|lp| &lp.name == *name);
            let (version, backend, status_text) = if let Some(pkg) = lp {
                (pkg.version.clone(), pkg.backend.clone(),
                 if pkg.sha256.is_empty() { "installed".to_string() } else { "verified".to_string() })
            } else {
                ("—".to_string(), "—".to_string(), "not installed".to_string())
            };
            let prefix = if i == cursor && self.focus == CatalogFocus::Results { "▸" } else { " " };
            let style = if i == cursor && self.focus == CatalogFocus::Results { Style::new().fg(CYAN) } else { Style::new() };
            Row::new(vec![prefix.to_string(), name.to_string(), version, backend, status_text]).style(style)
        }).collect();

        let widths = [
            Constraint::Length(2),
            Constraint::Percentage(30), Constraint::Percentage(22), Constraint::Percentage(22), Constraint::Percentage(24),
        ];

        f.render_widget(
            Table::new(rows, widths).header(header)
                .block(Block::new().borders(Borders::ALL).title(format!(" {} ({}) ", env_name, filtered.len())))
                .column_spacing(1),
            area,
        );
    }

    // ── PDK table ──
    fn draw_pdk_table(&self, f: &mut Frame, area: Rect) {
        let cursor = self.pdk_idx.min(self.pdks.len().saturating_sub(1));

        let header = Row::new(vec!["", "PDK", "Version", "Backend", "Status"]).style(Style::new().fg(DIM));

        let rows: Vec<Row> = self.pdks.iter().enumerate().map(|(i, (name, variant, status))| {
            let prefix = if i == cursor && self.focus == CatalogFocus::Results { "▸" } else { " " };
            let style = if i == cursor && self.focus == CatalogFocus::Results { Style::new().fg(CYAN) } else { Style::new() };
            Row::new(vec![prefix.to_string(), name.clone(), variant.clone(), "ciel".to_string(), status.clone()]).style(style)
        }).collect();

        let widths = [
            Constraint::Length(2),
            Constraint::Percentage(30), Constraint::Percentage(22), Constraint::Percentage(22), Constraint::Percentage(24),
        ];

        f.render_widget(
            Table::new(rows, widths).header(header)
                .block(Block::new().borders(Borders::ALL).title(format!(" PDKs ({}) ", self.pdks.len())))
                .column_spacing(1),
            area,
        );
    }

    // ── download queue ──
    fn draw_download_queue(&self, f: &mut Frame, area: Rect) {
        if self.downloads.is_empty() {
            f.render_widget(
                Paragraph::new("No active downloads").block(Block::new().borders(Borders::ALL).title(" Downloads ")),
                area,
            );
            return;
        }

        let width = area.width.saturating_sub(4) as usize;
        let lines: Vec<String> = self.downloads.iter().enumerate().map(|(i, dl)| {
            let bar = progress_bar(dl.progress, (width as f32 * 0.4) as usize);
            format!("  {}. {:<20} {:>12}\n     {}\n", i + 1, dl.name, dl.stage, bar)
        }).collect();

        f.render_widget(
            Paragraph::new(lines.join("")).block(Block::new().borders(Borders::ALL)
                .title(format!(" Downloads ({}) ", self.downloads.len()))),
            area,
        );
    }

    // ── handle ──
    pub fn handle(&mut self, code: ratatui::crossterm::event::KeyCode) -> Option<CatalogAction> {
        use ratatui::crossterm::event::KeyCode;

        if code == KeyCode::Esc {
            if !self.search_query.is_empty() {
                self.search_query.clear();
                self.tool_idx = 0;
                self.pdk_idx = 0;
            } else if self.focus == CatalogFocus::Results {
                self.focus = CatalogFocus::Sidebar;
            }
            return None;
        }

        // / — focus search
        if code == KeyCode::Char('/') {
            self.search_query.clear();
            self.tool_idx = 0;
            self.pdk_idx = 0;
            return None;
        }

        match self.focus {
            CatalogFocus::Sidebar => {
                let n = self.sidebar_len();
                match code {
                    KeyCode::Char('j') | KeyCode::Down => {
                        if n > 0 { self.sidebar_idx = (self.sidebar_idx + 1) % n; }
                        self.tool_idx = 0; self.pdk_idx = 0;
                        None
                    }
                    KeyCode::Char('k') | KeyCode::Up => {
                        if n > 0 {
                            self.sidebar_idx = self.sidebar_idx.checked_sub(1).unwrap_or(n - 1);
                        }
                        self.tool_idx = 0; self.pdk_idx = 0;
                        None
                    }
                    KeyCode::Right | KeyCode::Char('l') | KeyCode::Tab | KeyCode::Enter => {
                        if self.sidebar_idx < self.envs.len() || self.sidebar_idx == self.envs.len() {
                            self.focus = CatalogFocus::Results;
                        }
                        None
                    }
                    KeyCode::Char('i') => {
                        let env_count = self.envs.len();
                        if self.sidebar_idx < env_count {
                            Some(CatalogAction::InstallEnv(self.envs[self.sidebar_idx].clone()))
                        } else if self.sidebar_idx == env_count {
                            if let Some((name, _, _)) = self.pdks.get(self.pdk_idx) {
                                Some(CatalogAction::InstallPdk(name.clone()))
                            } else { None }
                        } else { None }
                    }
                    KeyCode::Char('r') => {
                        let env_count = self.envs.len();
                        if self.sidebar_idx < env_count {
                            Some(CatalogAction::RemoveEnv(self.envs[self.sidebar_idx].clone()))
                        } else if self.sidebar_idx == env_count {
                            if let Some((name, _, _)) = self.pdks.get(self.pdk_idx) {
                                Some(CatalogAction::RemovePdk(name.clone()))
                            } else { None }
                        } else { None }
                    }
                    KeyCode::Char('d') => {
                        let env_count = self.envs.len();
                        if self.sidebar_idx < env_count {
                            Some(CatalogAction::Doctor(self.envs[self.sidebar_idx].clone()))
                        } else { None }
                    }
                    _ => None
                }
            }
            CatalogFocus::Results => {
                let is_pdks = self.sidebar_idx == self.envs.len();
                if is_pdks {
                    let n = self.pdks.len();
                    match code {
                        KeyCode::Char('j') | KeyCode::Down => {
                            if n > 0 { self.pdk_idx = (self.pdk_idx + 1) % n; } None
                        }
                        KeyCode::Char('k') | KeyCode::Up => {
                            if n > 0 { self.pdk_idx = self.pdk_idx.checked_sub(1).unwrap_or(n - 1); } None
                        }
                        KeyCode::Left | KeyCode::Char('h') => { self.focus = CatalogFocus::Sidebar; None }
                        KeyCode::Char('i') | KeyCode::Enter => {
                            self.pdks.get(self.pdk_idx).map(|(n, _, _)| CatalogAction::InstallPdk(n.clone()))
                        }
                        KeyCode::Char('r') => {
                            self.pdks.get(self.pdk_idx).map(|(n, _, _)| CatalogAction::RemovePdk(n.clone()))
                        }
                        _ => None
                    }
                } else {
                    let filtered: Vec<&String> = if self.search_query.is_empty() {
                        self.resolved_tools.iter().collect()
                    } else {
                        let q = self.search_query.to_lowercase();
                        self.resolved_tools.iter().filter(|n| n.to_lowercase().contains(&q)).collect()
                    };
                    let n = filtered.len();
                    match code {
                        KeyCode::Char('j') | KeyCode::Down => {
                            if n > 0 { self.tool_idx = (self.tool_idx + 1) % n; } None
                        }
                        KeyCode::Char('k') | KeyCode::Up => {
                            if n > 0 { self.tool_idx = self.tool_idx.checked_sub(1).unwrap_or(n - 1); } None
                        }
                        KeyCode::Left | KeyCode::Char('h') => { self.focus = CatalogFocus::Sidebar; None }
                        KeyCode::Backspace => { self.search_query.pop(); self.tool_idx = 0; None }
                        KeyCode::Char(c) if c != 'j' && c != 'k' && c != 'h' && c != 'l' && c != 'i' && c != 'r' && c != 'd' && c != 'v' => {
                            self.search_query.push(c); self.tool_idx = 0; None
                        }
                        KeyCode::Char('i') | KeyCode::Enter => {
                            filtered.get(self.tool_idx).map(|s| CatalogAction::InstallTool(s.to_string()))
                        }
                        KeyCode::Char('r') => {
                            filtered.get(self.tool_idx).map(|s| CatalogAction::RemoveTool(s.to_string()))
                        }
                        KeyCode::Char('d') => {
                            filtered.get(self.tool_idx).map(|s| CatalogAction::DoctorTool(s.to_string()))
                        }
                        KeyCode::Char('v') => Some(CatalogAction::Verify),
                        _ => None
                    }
                }
            }
        }
    }

    pub fn footer(&self) -> String {
        match self.focus {
            CatalogFocus::Sidebar => " ←→/tab switch  ↑↓/jk move  i install  r remove  d doctor  / search  ? help  q quit ".into(),
            CatalogFocus::Results => " ↑↓/jk move  i install  r remove  d doctor  v verify  esc back  / search  ? help  q quit ".into(),
        }
    }
}

fn progress_bar(pct: u8, width: usize) -> String {
    let filled = (pct as usize * width / 100).min(width);
    format!("{}{}", "█".repeat(filled), "░".repeat(width - filled))
}

#[derive(Debug, Clone)]
pub enum CatalogAction {
    InstallEnv(String),
    InstallTool(String),
    InstallPdk(String),
    RemoveEnv(String),
    RemoveTool(String),
    RemovePdk(String),
    Doctor(String),
    DoctorTool(String),
    Verify,
}
