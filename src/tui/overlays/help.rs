use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};
use ratatui::Frame;

const CYAN: Color = Color::Rgb(80, 180, 200);

pub fn draw(f: &mut Frame, area: Rect) {
    let r = centered(60, 60, area);
    f.render_widget(Clear, r);

    let k = Style::new().fg(CYAN);
    let text = Paragraph::new(vec![
        Line::from(""),
        Line::from(vec![Span::styled("  Navigation", k), Span::raw("")]),
        Line::from(vec![Span::styled("    ↑↓ / j k     ", k), Span::raw(" move selection")]),
        Line::from(vec![Span::styled("    ←→ / tab     ", k), Span::raw(" switch pane")]),
        Line::from(vec![Span::styled("    /            ", k), Span::raw(" focus search bar")]),
        Line::from(vec![Span::styled("    esc          ", k), Span::raw(" back")]),
        Line::from(vec![Span::raw("")]),
        Line::from(vec![Span::styled("  Actions", k), Span::raw("")]),
        Line::from(vec![Span::styled("    i            ", k), Span::raw(" install tool")]),
        Line::from(vec![Span::styled("    u            ", k), Span::raw(" update tool")]),
        Line::from(vec![Span::styled("    d            ", k), Span::raw(" run doctor on tool")]),
        Line::from(vec![Span::styled("    E            ", k), Span::raw(" open shell with env")]),
        Line::from(vec![Span::styled("    ↵            ", k), Span::raw(" install / open")]),
        Line::from(vec![Span::raw("")]),
        Line::from(vec![Span::styled("  Search", k), Span::raw("")]),
        Line::from(vec![Span::styled("    type          ", k), Span::raw(" filter tools")]),
        Line::from(vec![Span::styled("    persist       ", k), Span::raw(" across env switches")]),
        Line::from(vec![Span::styled("    esc           ", k), Span::raw(" clear search")]),
        Line::from(vec![Span::raw("")]),
        Line::from(vec![Span::styled("    ?             ", k), Span::raw(" this help (any key dismisses)")]),
        Line::from(vec![Span::styled("    q             ", k), Span::raw(" quit")]),
    ])
    .block(Block::new().borders(Borders::ALL).title(" help "))
    .wrap(Wrap { trim: true });

    f.render_widget(text, r);
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
