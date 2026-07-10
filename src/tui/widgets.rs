use ratatui::style::{Color, Style};
use ratatui::text::Span;

pub const GREEN: Color = Color::Rgb(100, 200, 100);
pub const YELLOW: Color = Color::Rgb(220, 180, 60);
pub const RED: Color = Color::Rgb(220, 80, 80);
pub const CYAN: Color = Color::Rgb(80, 180, 200);
pub const DIM: Color = Color::Rgb(100, 100, 100);

/// Colors a status string by its leading glyph (✓ / ✗ / spinner frames),
/// so callers just need to prefix text with the right glyph to get color for free.
pub fn status_span(status: impl Into<String>) -> Span<'static> {
    let status = status.into();
    let color = match status.chars().next() {
        Some('✓') => GREEN,
        Some('✗') => RED,
        Some('◐') | Some('⠋') | Some('⠙') | Some('⠹') | Some('⠸') | Some('⠼') | Some('⠴') | Some('⠦') | Some('⠧') | Some('⠇') | Some('⠏') => YELLOW,
        _ => DIM,
    };
    Span::styled(status, Style::new().fg(color))
}

/// Fixed 40-column glyph gauge (legacy default width).
pub fn glyph_gauge(progress: u8) -> String {
    glyph_gauge_width(progress, 40)
}

/// Glyph gauge at an arbitrary width, for panes of varying size.
pub fn glyph_gauge_width(progress: u8, width: usize) -> String {
    let width = width.max(1);
    let filled = (progress as usize * width / 100).min(width);
    let empty = width - filled;
    format!("{}{}", "█".repeat(filled), "░".repeat(empty))
}

const BRAILLE: &[char] = &['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];

pub fn spinner(tick: u64) -> char {
    BRAILLE[(tick as usize / 12) % BRAILLE.len()]
}
