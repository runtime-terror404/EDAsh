use ratatui::style::{Color, Style};
use ratatui::text::Span;

pub const GREEN: Color = Color::Rgb(100, 200, 100);
pub const YELLOW: Color = Color::Rgb(220, 180, 60);
pub const RED: Color = Color::Rgb(220, 80, 80);
pub const CYAN: Color = Color::Rgb(80, 180, 200);
pub const DIM: Color = Color::Rgb(100, 100, 100);

pub fn status_span(status: &str) -> Span<'_> {
    match status {
        "✓" | "✓ ok" | "✓ verified" => Span::styled(status, Style::new().fg(GREEN)),
        "✗" | "✗ missing" => Span::styled(status, Style::new().fg(RED)),
        "◐" | "⠋" | "⠙" | "⠹" | "⠸" | "⠼" | "⠴" | "⠦" | "⠧" | "⠇" | "⠏" => {
            Span::styled(status, Style::new().fg(YELLOW))
        }
        _ => Span::styled(status, Style::new().fg(DIM)),
    }
}

pub fn glyph_gauge(progress: u8) -> String {
    let filled = (progress as usize * 40 / 100).min(40);
    let empty = 40 - filled;
    format!("{}{}", "█".repeat(filled), "░".repeat(empty))
}

const BRAILLE: &[char] = &['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];

pub fn spinner(tick: u64) -> char {
    BRAILLE[(tick as usize / 12) % BRAILLE.len()]
}
