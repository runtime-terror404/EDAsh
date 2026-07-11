use crate::lockfile::schema::LockedPackage;
use crate::tui::widgets::{self, CYAN, DIM, GREEN, YELLOW};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Cell, List, ListItem, Paragraph, Row, Table};
use ratatui::Frame;
use std::collections::HashMap;

#[derive(Clone, Copy, PartialEq)]
pub enum CatalogFocus {
    Sidebar,
    Search,
    Results,
}

pub struct CatalogScreen {
    pub envs: Vec<String>,
    pub pdks: Vec<(String, String, String)>, // name, variant, status
    pub downloads: Vec<DownloadItem>,
    pub tick: u64,

    pub sidebar_idx: usize, // 0..envs.len()=envs, envs.len()=PDKs, envs.len()+1=Downloads
    pub tool_idx: usize,
    pub pdk_idx: usize,

    pub search_query: String,

    pub tools: Vec<LockedPackage>,
    pub resolved_tools: Vec<String>,
    pub env_tools: HashMap<String, Vec<String>>,

    pub focus: CatalogFocus,
    pub show_pdks: bool,
    pub tool_scroll: usize,
    pub pdk_scroll: usize,
    last_visible_rows: usize,
    pub doctor_results: Vec<DoctorLine>,
    pub doctor_running: bool,
}

#[derive(Clone)]
pub struct DoctorLine {
    pub name: String,
    pub passed: bool,
    pub detail: String,
    pub elapsed: f64,
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
            tick: 0,
            sidebar_idx: 0,
            tool_idx: 0,
            pdk_idx: 0,
            search_query: String::new(),
            tools: Vec::new(),
            resolved_tools: Vec::new(),
            env_tools: HashMap::new(),
            focus: CatalogFocus::Sidebar,
            show_pdks: false,
            tool_scroll: 0,
            pdk_scroll: 0,
            last_visible_rows: 8,
            doctor_results: Vec::new(),
            doctor_running: false,
        }
    }

    fn load_pdks() -> Vec<(String, String, String)> {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("catalog/index.yaml");
        if let Ok(data) = std::fs::read_to_string(&path) {
            if let Ok(idx) = serde_yaml::from_str::<serde_yaml::Value>(&data) {
                if let Some(pdks) = idx.get("pdks").and_then(|p| p.as_mapping()) {
                    return pdks
                        .iter()
                        .map(|(k, v)| {
                            let name = k.as_str().unwrap_or("?").to_string();
                            let variant = v.get("variant").and_then(|x| x.as_str()).unwrap_or("?").to_string();
                            (name, variant, "✗ not installed".to_string())
                        })
                        .collect();
                }
            }
        }
        Vec::new()
    }

    /// Number of selectable sidebar rows: envs + PDKs + Downloads
    fn sidebar_len(&self) -> usize {
        self.envs.len() + 2 // envs + PDKs + Downloads
    }

    fn downloads_idx(&self) -> usize {
        self.envs.len() + 1 // envs + PDKs
    }

    fn in_downloads(&self, idx: usize) -> bool {
        idx == self.downloads_idx()
    }

    pub fn rebuild_sidebar(&mut self) {}

    fn adjust_scroll(idx: usize, scroll: &mut usize, total: usize, visible: usize) {
        if visible == 0 { return; }
        if idx >= *scroll + visible {
            *scroll = idx - visible + 1;
        }
        if idx < *scroll {
            *scroll = idx;
        }
        let max_scroll = total.saturating_sub(visible);
        *scroll = (*scroll).min(max_scroll);
    }

    pub fn refresh_tools(&mut self, tool_names: Vec<String>) {
        if let Some(env) = self.selected_env_name() {
            self.env_tools.insert(env, tool_names.clone());
        }
        self.resolved_tools = tool_names;
        self.tool_idx = 0;
        self.show_pdks = self.sidebar_idx == self.envs.len();
    }

    pub fn refresh_tools_for(&mut self, env: &str, tool_names: Vec<String>) {
        self.env_tools.insert(env.to_string(), tool_names.clone());
        // Also set resolved_tools if this is the currently selected env
        if Some(env.to_string()) == self.selected_env_name() {
            self.resolved_tools = tool_names;
        }
    }

    pub fn selected_env_name(&self) -> Option<String> {
        if self.sidebar_idx < self.envs.len() {
            Some(self.envs[self.sidebar_idx].clone())
        } else {
            None
        }
    }

    pub fn load_lockfile(&mut self) {
        let lp = crate::paths::lockfile_path();
        if lp.exists() {
            if let Ok(lf) = crate::lockfile::writer::read_lockfile(&lp) {
                self.tools = lf.package;
                for (name, _, status) in &mut self.pdks {
                    *status = if lf.pdk.contains_key(name) {
                        "✓ installed".into()
                    } else {
                        "✗ not installed".into()
                    };
                }
            }
        }
    }

    // ── draw ──
    pub fn draw(&mut self, f: &mut Frame, area: Rect) {
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

        items.push(ListItem::new("  Environments"));

        for (i, name) in self.envs.iter().enumerate() {
            let sel = i == self.sidebar_idx && self.focus == CatalogFocus::Sidebar;
            let env_tools = self.env_tools.get(name);
            let (installed, total) = if let Some(et) = env_tools {
                let inst = et.iter().filter(|t| self.tools.iter().any(|lp| &lp.name == *t)).count();
                (inst, et.len())
            } else {
                (0, 0)
            };
            let dot = if total == 0 {
                "○"
            } else if installed == total {
                "●"
            } else if installed == 0 {
                "○"
            } else {
                "◐"
            };
            let txt = if sel {
                format!("▸   {:<7} {}", name, dot)
            } else {
                format!("    {:<7} {}", name, dot)
            };
            let style = if sel { Style::new().fg(CYAN) } else { Style::new() };
            items.push(ListItem::new(txt).style(style));
        }

        let pdk_idx = self.envs.len();
        let pdk_sel = self.sidebar_idx == pdk_idx && self.focus == CatalogFocus::Sidebar;
        let pdk_installed = self.pdks.iter().filter(|(_, _, s)| s.starts_with('✓')).count();
        let pdk_total = self.pdks.len();
        let pdk_dot = if pdk_total == 0 { "○" } else if pdk_installed == pdk_total { "●" } else if pdk_installed == 0 { "○" } else { "◐" };
        let pdk_txt = if pdk_sel {
            format!("▸ PDKs {}", pdk_dot)
        } else {
            format!("  PDKs {}", pdk_dot)
        };
        items.push(ListItem::new(pdk_txt).style(if pdk_sel { Style::new().fg(CYAN) } else { Style::new() }));

        // Separator before Downloads
        items.push(ListItem::new("──────────────────────────────────────────────────────").style(Style::new().fg(DIM)));
        let dl_idx = self.downloads_idx();
        let dl_sel = self.sidebar_idx == dl_idx && self.focus == CatalogFocus::Sidebar;
        let active = self.downloads.iter().filter(|d| d.done_ticks == 0 && d.progress < 100).count();
        let dl_count = self.downloads.len();
        let dl_label = if dl_count > 0 {
            format!("Downloads ({})", dl_count)
        } else {
            "Downloads".to_string()
        };
        let dl_text = if dl_sel {
            format!("▸ {}", dl_label)
        } else if active > 0 {
            format!("  {} ⠿", dl_label)
        } else {
            format!("  {}", dl_label)
        };
        let dl_style = if dl_sel {
            Style::new().fg(CYAN)
        } else if active > 0 {
            Style::new().fg(YELLOW)
        } else {
            Style::new()
        };
        items.push(ListItem::new(dl_text).style(dl_style));

        f.render_widget(List::new(items).block(Block::new().borders(Borders::RIGHT)), area);
    }

    // ── search ──
    fn draw_search(&self, f: &mut Frame, area: Rect) {
        let focused = self.focus == CatalogFocus::Search;
        let display = if self.search_query.is_empty() && !focused {
            "  / to search".to_string()
        } else if focused {
            format!("  {}█", self.search_query)
        } else {
            format!("  {}", self.search_query)
        };
        let border_style = if focused { Style::new().fg(CYAN) } else { Style::new().fg(DIM) };
        f.render_widget(
            Paragraph::new(display).block(
                Block::new()
                    .borders(Borders::ALL)
                    .border_style(border_style)
                    .title(" Search "),
            ),
            area,
        );
    }

    // ── content ──
    fn draw_content(&mut self, f: &mut Frame, area: Rect) {
        if self.doctor_running || !self.doctor_results.is_empty() {
            self.draw_doctor_results(f, area);
            return;
        }
        let is_pdks = self.sidebar_idx == self.envs.len();
        if is_pdks {
            self.draw_pdk_table(f, area);
        } else if self.in_downloads(self.sidebar_idx) {
            self.draw_download_queue(f, area);
        } else if self.sidebar_idx < self.envs.len() {
            self.draw_tool_table(f, area);
        }
    }

    fn draw_doctor_results(&self, f: &mut Frame, area: Rect) {
        let title = if self.doctor_running { " doctor: running... " } else { " doctor: complete " };
        let items: Vec<ListItem> = self.doctor_results.iter().map(|r| {
            let glyph = if r.passed { "✓" } else { "✗" };
            let line = format!("  {} {:<20} {:<40} ({:.1}s)", glyph, r.name, r.detail, r.elapsed);
            ListItem::new(line)
        }).collect();

        if self.doctor_running && !self.doctor_results.is_empty() {
            let passed = self.doctor_results.iter().filter(|r| r.passed).count();
            let total = self.doctor_results.len();
            let summary = format!("  {} passed, {} failed", passed, total - passed);
            let mut all = vec![ListItem::new(summary), ListItem::new("")];
            all.extend(items);
            f.render_widget(List::new(all).block(Block::new().borders(Borders::ALL).title(title)), area);
        } else {
            f.render_widget(List::new(items).block(Block::new().borders(Borders::ALL).title(title)), area);
        }
    }

    fn filtered_tools(&self) -> Vec<&String> {
        if self.search_query.is_empty() {
            self.resolved_tools.iter().collect()
        } else {
            let q = self.search_query.to_lowercase();
            self.resolved_tools.iter().filter(|n| n.to_lowercase().contains(&q)).collect()
        }
    }

    // ── tool table ──
    fn draw_tool_table(&mut self, f: &mut Frame, area: Rect) {
        let filtered: Vec<String> = self.filtered_tools().iter().map(|s| s.to_string()).collect();

        if filtered.is_empty() {
            let env_name = self.envs.get(self.sidebar_idx).map(|s| s.as_str()).unwrap_or("?");
            let block = Block::new().borders(Borders::ALL).title(format!(" {} ", env_name));
            f.render_widget(Paragraph::new("No tools match").block(block), area);
            return;
        }

        // Viewport scrolling — keep cursor in view
        let total = filtered.len();
        let visible = (area.height.saturating_sub(4) as usize).max(1).min(total);
        self.last_visible_rows = visible;

        // Always keep cursor visible
        if self.tool_idx >= self.tool_scroll + visible {
            self.tool_scroll = self.tool_idx - visible + 1;
        } else if self.tool_idx < self.tool_scroll {
            self.tool_scroll = self.tool_idx;
        }
        self.tool_scroll = self.tool_scroll.min(total.saturating_sub(visible));

        let end = (self.tool_scroll + visible).min(total);
        let window = &filtered[self.tool_scroll..end];
        let cursor = self.tool_idx;
        let cursor_visible = self.tool_idx >= self.tool_scroll && self.tool_idx < end;
        let env_name = self.envs.get(self.sidebar_idx).map(|s| s.as_str()).unwrap_or("?");

        let header = Row::new(vec!["", "Tool", "Version", "Backend", "Status"]).style(Style::new().fg(DIM));

        let rows: Vec<Row> = window
            .iter()
            .enumerate()
            .map(|(wi, name)| {
                let i = self.tool_scroll + wi;
                // Check if this tool is currently downloading
                let downloading = self.downloads.iter().any(|d| d.name.as_str() == name.as_str() && d.progress < 100);

                let pkg = self.tools.iter().find(|lp| lp.name == *name);
                let (version, backend, status_text) = if downloading {
                    ("—".to_string(), "—".to_string(), "◐ installing".to_string())
                } else if let Some(pkg) = pkg {
                    let status = if pkg.sha256.is_empty() { "✓ installed" } else { "✓ verified" };
                    (pkg.version.clone(), pkg.backend.clone(), status.to_string())
                } else {
                    ("—".to_string(), "—".to_string(), "✗ not installed".to_string())
                };
                let row_sel = cursor_visible && i == cursor && self.focus == CatalogFocus::Results;
                let prefix = if row_sel { "▸" } else { " " };
                let row_style = if row_sel { Style::new().fg(CYAN) } else { Style::new() };
                Row::new(vec![
                    Cell::from(prefix.to_string()),
                    Cell::from(name.to_string()),
                    Cell::from(version),
                    Cell::from(backend),
                    Cell::from(widgets::status_span(status_text)),
                ])
                .style(row_style)
            })
            .collect();

        let widths = [
            Constraint::Length(2),
            Constraint::Percentage(30),
            Constraint::Percentage(22),
            Constraint::Percentage(22),
            Constraint::Percentage(24),
        ];

        f.render_widget(
            Table::new(rows, widths)
                .header(header)
                .block(Block::new().borders(Borders::ALL).title(format!(" {} ({}) ", env_name, filtered.len())))
                .column_spacing(1),
            area,
        );
    }

    // ── PDK table ──
    fn draw_pdk_table(&mut self, f: &mut Frame, area: Rect) {
        // Filter PDKs by search query (clone to avoid borrow issues with scroll)
        let filtered: Vec<(usize, String, String, String)> = self
            .pdks
            .iter()
            .enumerate()
            .filter(|(_, (name, _, _))| {
                self.search_query.is_empty()
                    || name.to_lowercase().contains(&self.search_query.to_lowercase())
            })
            .map(|(i, (n, v, s))| (i, n.clone(), v.clone(), s.clone()))
            .collect();

        if filtered.is_empty() && !self.search_query.is_empty() {
            let block = Block::new().borders(Borders::ALL).title(" PDKs ");
            f.render_widget(Paragraph::new("No PDKs match").block(block), area);
            return;
        }

        // Viewport scrolling — keep cursor in view
        let total = filtered.len();
        let visible = (area.height.saturating_sub(4) as usize).max(1).min(total);
        self.last_visible_rows = visible;

        if self.pdk_idx >= self.pdk_scroll + visible {
            self.pdk_scroll = self.pdk_idx - visible + 1;
        } else if self.pdk_idx < self.pdk_scroll {
            self.pdk_scroll = self.pdk_idx;
        }
        self.pdk_scroll = self.pdk_scroll.min(total.saturating_sub(visible));

        let end = (self.pdk_scroll + visible).min(total);
        let window = &filtered[self.pdk_scroll..end];
        let cursor = self.pdk_idx;
        let cursor_visible = self.pdk_idx >= self.pdk_scroll && self.pdk_idx < end;

        let header = Row::new(vec!["", "PDK", "Version", "Backend", "Status"]).style(Style::new().fg(DIM));

        let rows: Vec<Row> = window.iter().enumerate().map(|(_wi, (_, name, variant, status))| {
            let original_idx = _wi + self.pdk_scroll;
            let is_cursor = cursor_visible && original_idx == cursor && self.focus == CatalogFocus::Results;
            let prefix = if is_cursor { "▸" } else { " " };
            let row_style = if is_cursor { Style::new().fg(CYAN) } else { Style::new() };
            Row::new(vec![
                    Cell::from(prefix.to_string()),
                    Cell::from(name.clone()),
                    Cell::from(variant.clone()),
                    Cell::from("ciel".to_string()),
                    Cell::from(widgets::status_span(status.as_str())),
                ])
                .style(row_style)
            })
            .collect();

        let widths = [
            Constraint::Length(2),
            Constraint::Percentage(30),
            Constraint::Percentage(22),
            Constraint::Percentage(22),
            Constraint::Percentage(24),
        ];

        f.render_widget(
            Table::new(rows, widths)
                .header(header)
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

        let width = ((area.width.saturating_sub(6)) as usize * 2 / 5).max(10);
        let lines: Vec<Line> = self
            .downloads
            .iter()
            .flat_map(|dl| {
                let done = dl.progress >= 100;
                let failed = dl.stage.to_lowercase().contains("error")
                    || dl.stage.to_lowercase().contains("fail")
                    || (done && dl.stage != "done");
                let label_color = if failed { widgets::RED } else if done { GREEN } else { YELLOW };
                let name_line = Line::from(vec![
                    Span::raw(format!("  {:<22}", dl.name)),
                    Span::styled(format!("{:>14}", dl.stage), Style::new().fg(label_color)),
                ]);
                let bar_str = if dl.progress == 0 {
                    format!("     {} waiting…", widgets::spinner(self.tick))
                } else {
                    format!("     {}", widgets::glyph_gauge_width(dl.progress, width))
                };
                let bar_color = if failed { widgets::RED } else if done { GREEN } else { CYAN };
                vec![name_line, Line::from(Span::styled(bar_str, Style::new().fg(bar_color))), Line::from("")]
            })
            .collect();

        f.render_widget(
            Paragraph::new(lines).block(Block::new().borders(Borders::ALL).title(format!(" Downloads ({}) ", self.downloads.len()))),
            area,
        );
    }

    // ── handle ──
    pub fn handle(&mut self, code: ratatui::crossterm::event::KeyCode) -> Option<CatalogAction> {
        use ratatui::crossterm::event::KeyCode;

        // Esc or any nav clears completed doctor results
        if !self.doctor_running && !self.doctor_results.is_empty() {
            self.doctor_results.clear();
            return None;
        }

        // Global: enter search from anywhere in the catalog screen
        if code == KeyCode::Char('/') && self.focus != CatalogFocus::Search {
            self.focus = CatalogFocus::Search;
            return None;
        }

        match self.focus {
            CatalogFocus::Search => match code {
                KeyCode::Esc => {
                    self.search_query.clear();
                    self.tool_idx = 0;
                    self.tool_scroll = 0;
                    self.focus = CatalogFocus::Results;
                    None
                }
                KeyCode::Enter | KeyCode::Down => {
                    self.focus = CatalogFocus::Results;
                    None
                }
                KeyCode::Left => {
                    self.focus = CatalogFocus::Sidebar;
                    None
                }
                KeyCode::Backspace => {
                    self.search_query.pop();
                    self.tool_idx = 0;
                    self.tool_scroll = 0;
                    None
                }
                KeyCode::Char(c) => {
                    self.search_query.push(c);
                    self.tool_idx = 0;
                    self.tool_scroll = 0;
                    None
                }
                _ => None,
            },
            CatalogFocus::Sidebar => {
                if code == KeyCode::Esc {
                    return None;
                }
                let n = self.sidebar_len();
                match code {
                    KeyCode::Char('j') | KeyCode::Down => {
                        if n > 0 {
                            self.sidebar_idx = (self.sidebar_idx + 1) % n;
                        }
                        self.tool_idx = 0;
                        self.pdk_idx = 0;
                        self.tool_scroll = 0;
                        self.pdk_scroll = 0;
                        None
                    }
                    KeyCode::Char('k') | KeyCode::Up => {
                        if n > 0 {
                            self.sidebar_idx = self.sidebar_idx.checked_sub(1).unwrap_or(n - 1);
                        }
                        self.tool_idx = 0;
                        self.pdk_idx = 0;
                        self.tool_scroll = 0;
                        self.pdk_scroll = 0;
                        None
                    }
                    KeyCode::Right | KeyCode::Char('l') | KeyCode::Tab | KeyCode::Enter => {
                        if self.in_downloads(self.sidebar_idx) {
                            None
                        } else {
                            // Envs and PDKs: go directly to Results
                            self.focus = CatalogFocus::Results;
                            self.tool_idx = 0;
                            self.pdk_idx = 0;
                            None
                        }
                    }
                    KeyCode::Char('i') => {
                        let env_count = self.envs.len();
                        if self.sidebar_idx < env_count {
                            Some(CatalogAction::InstallEnv(self.envs[self.sidebar_idx].clone()))
                        } else if self.sidebar_idx == env_count {
                            self.pdks.get(self.pdk_idx).map(|(name, _, _)| CatalogAction::InstallPdk(name.clone()))
                        } else {
                            None
                        }
                    }
                    KeyCode::Char('r') => {
                        let env_count = self.envs.len();
                        if self.sidebar_idx < env_count {
                            Some(CatalogAction::RemoveEnv(self.envs[self.sidebar_idx].clone()))
                        } else if self.sidebar_idx == env_count {
                            Some(CatalogAction::RemoveAllPdks)
                        } else {
                            None
                        }
                    }
                    KeyCode::Char('d') => {
                        let env_count = self.envs.len();
                        if self.sidebar_idx < env_count {
                            Some(CatalogAction::Doctor(self.envs[self.sidebar_idx].clone()))
                        } else {
                            None
                        }
                    }
                    KeyCode::Char('E') => {
                        let env_count = self.envs.len();
                        if self.sidebar_idx < env_count {
                            Some(CatalogAction::Shell(self.envs[self.sidebar_idx].clone()))
                        } else {
                            Some(CatalogAction::Shell(String::new()))
                        }
                    }
                    _ => None,
                }
            }
            CatalogFocus::Results => {
                let is_pdks = self.sidebar_idx == self.envs.len();
                if is_pdks {
                    let filtered_pdks: Vec<&(String, String, String)> = self.pdks.iter()
                        .filter(|(name, _, _)| self.search_query.is_empty() || name.to_lowercase().contains(&self.search_query.to_lowercase()))
                        .collect();
                    let n = filtered_pdks.len();
                    match code {
                        KeyCode::Esc => {
                            self.focus = CatalogFocus::Sidebar;
                            None
                        }
                        KeyCode::Char('j') | KeyCode::Down => {
                            if n > 0 {
                                self.pdk_idx = (self.pdk_idx + 1) % n;
                                if self.pdk_idx == 0 { self.pdk_scroll = 0; }
                                Self::adjust_scroll(self.pdk_idx, &mut self.pdk_scroll, n, self.last_visible_rows);
                            }
                            None
                        }
                        KeyCode::Char('k') | KeyCode::Up => {
                            if self.pdk_idx == 0 {
                                self.focus = CatalogFocus::Search;
                            } else {
                                self.pdk_idx -= 1;
                                Self::adjust_scroll(self.pdk_idx, &mut self.pdk_scroll, n, self.last_visible_rows);
                            }
                            None
                        }
                        KeyCode::Left | KeyCode::Char('h') => {
                            self.focus = CatalogFocus::Sidebar;
                            None
                        }
                        KeyCode::Char('i') | KeyCode::Enter => self.pdks.get(self.pdk_idx).map(|(n, _, _)| CatalogAction::InstallPdk(n.clone())),
                        KeyCode::Char('r') => self.pdks.get(self.pdk_idx).map(|(n, _, _)| CatalogAction::RemovePdk(n.clone())),
                        _ => None,
                    }
                } else {
                    let filtered = self.filtered_tools();
                    let n = filtered.len();
                    match code {
                        KeyCode::Esc => {
                            self.focus = CatalogFocus::Sidebar;
                            None
                        }
                        KeyCode::Char('j') | KeyCode::Down => {
                            if n > 0 {
                                self.tool_idx = (self.tool_idx + 1) % n;
                                if self.tool_idx == 0 { self.tool_scroll = 0; }
                                Self::adjust_scroll(self.tool_idx, &mut self.tool_scroll, n, self.last_visible_rows);
                            }
                            None
                        }
                        KeyCode::Char('k') | KeyCode::Up => {
                            if self.tool_idx == 0 {
                                self.focus = CatalogFocus::Search;
                            } else {
                                self.tool_idx -= 1;
                                Self::adjust_scroll(self.tool_idx, &mut self.tool_scroll, n, self.last_visible_rows);
                            }
                            None
                        }
                        KeyCode::Left | KeyCode::Char('h') => {
                            self.focus = CatalogFocus::Sidebar;
                            None
                        }
                        KeyCode::Char('/') => {
                            self.focus = CatalogFocus::Search;
                            self.tool_scroll = 0;
                            None
                        }
                        KeyCode::Char('i') | KeyCode::Enter => filtered.get(self.tool_idx).map(|s| CatalogAction::InstallTool(s.to_string())),
                        KeyCode::Char('r') => filtered.get(self.tool_idx).map(|s| CatalogAction::RemoveTool(s.to_string())),
                        KeyCode::Char('d') => filtered.get(self.tool_idx).map(|s| CatalogAction::DoctorTool(s.to_string())),
                        _ => None,
                    }
                }
            }
        }
    }

    pub fn footer(&self) -> String {
        match self.focus {
            CatalogFocus::Sidebar => " ←→ switch  ↑↓ move  i install  r remove  E shell  / search  ? help  q quit ".into(),
            CatalogFocus::Search => " type to filter  ↵/↓ to results  ← sidebar  esc clear  ? help ".into(),
            CatalogFocus::Results => " ↑↓ move  i install  r remove  esc back  / search  ? help  q quit ".into(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum CatalogAction {
    InstallEnv(String),
    InstallTool(String),
    InstallPdk(String),
    RemoveEnv(String),
    RemoveTool(String),
    RemovePdk(String),
    RemoveAllPdks,
    Doctor(String),
    DoctorTool(String),
    Shell(String),
}
