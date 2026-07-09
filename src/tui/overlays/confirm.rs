use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};
use ratatui::Frame;

const CYAN: Color = Color::Rgb(80, 180, 200);

pub fn draw(f: &mut Frame, area: Rect, msg: &str) {
    let r = centered(40, 30, area);
    f.render_widget(Clear, r);

    let text = Paragraph::new(vec![
        Line::from(""),
        Line::from(msg),
        Line::from(""),
        Line::from(vec![
            Span::styled("  y", Style::new().fg(CYAN)),
            Span::raw(" confirm    "),
            Span::styled("n / esc", Style::new().fg(CYAN)),
            Span::raw(" cancel"),
        ]),
    ])
    .block(Block::new().borders(Borders::ALL).title(" confirm "))
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
