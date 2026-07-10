use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};
use ratatui::Frame;

const CYAN: Color = Color::Rgb(80, 180, 200);
const DIM: Color = Color::Rgb(120, 120, 120);

pub fn draw(f: &mut Frame, area: Rect, msg: &str) {
    let r = centered(55, 40, area);
    f.render_widget(Clear, r);

    let block = Block::new().borders(Borders::ALL).title(" confirm ");
    let inner = block.inner(r);
    f.render_widget(block, r);

    // Split inner area: message on top, centered hint at bottom
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(1)])
        .split(inner);

    let mut lines: Vec<Line> = vec![Line::from("")];
    for line in msg.lines() {
        lines.push(Line::from(format!("  {}", line)));
    }

    f.render_widget(
        Paragraph::new(lines).wrap(Wrap { trim: true }),
        chunks[0],
    );

    let hint = Line::from(vec![
        Span::styled("y", Style::new().fg(CYAN)),
        Span::styled(" confirm    ", Style::new().fg(DIM)),
        Span::styled("n/esc", Style::new().fg(CYAN)),
        Span::styled(" cancel", Style::new().fg(DIM)),
    ]);

    f.render_widget(
        Paragraph::new(hint).alignment(Alignment::Center),
        chunks[1],
    );
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
