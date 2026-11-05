//! Banner de inicio: "LEARN GIT" en letras grandes, en blanco y negro.

use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

const L: [&str; 6] = [
    "██╗     ",
    "██║     ",
    "██║     ",
    "██║     ",
    "███████╗",
    "╚══════╝",
];
const E: [&str; 6] = [
    "███████╗",
    "██╔════╝",
    "█████╗  ",
    "██╔══╝  ",
    "███████╗",
    "╚══════╝",
];
const A: [&str; 6] = [
    " █████╗ ",
    "██╔══██╗",
    "███████║",
    "██╔══██║",
    "██║  ██║",
    "╚═╝  ╚═╝",
];
const R: [&str; 6] = [
    "██████╗ ",
    "██╔══██╗",
    "██████╔╝",
    "██╔══██╗",
    "██║  ██║",
    "╚═╝  ╚═╝",
];
const N: [&str; 6] = [
    "███╗   ██╗",
    "████╗  ██║",
    "██╔██╗ ██║",
    "██║╚██╗██║",
    "██║ ╚████║",
    "╚═╝  ╚═══╝",
];
const G: [&str; 6] = [
    " ██████╗ ",
    "██╔════╝ ",
    "██║  ███╗",
    "██║   ██║",
    "╚██████╔╝",
    " ╚═════╝ ",
];
const I: [&str; 6] = ["██╗", "██║", "██║", "██║", "██║", "╚═╝"];
const T: [&str; 6] = [
    "████████╗",
    "╚══██╔══╝",
    "   ██║   ",
    "   ██║   ",
    "   ██║   ",
    "   ╚═╝   ",
];

const GAP: &str = "   ";

/// Ancho en columnas del banner completo.
pub const WIDTH: u16 = 66;
pub const HEIGHT: u16 = 6;

fn rows() -> [String; 6] {
    let word_1 = [&L, &E, &A, &R, &N];
    let word_2 = [&G, &I, &T];
    std::array::from_fn(|row| {
        let mut line = String::new();
        for glyph in word_1 {
            line.push_str(glyph[row]);
        }
        line.push_str(GAP);
        for glyph in word_2 {
            line.push_str(glyph[row]);
        }
        line
    })
}

/// El bloque (`█`) va en blanco y la sombra (`╗╔║═╝╚`) en gris.
fn styled(row: &str) -> Line<'static> {
    let block = Style::new().fg(Color::White).add_modifier(Modifier::BOLD);
    let shadow = Style::new().fg(Color::DarkGray);
    let mut spans: Vec<Span<'static>> = Vec::new();
    let mut current = String::new();
    let mut current_is_block = true;
    for ch in row.chars() {
        let is_block = ch == '█' || ch == ' ';
        if is_block != current_is_block && !current.is_empty() {
            let style = if current_is_block { block } else { shadow };
            spans.push(Span::styled(std::mem::take(&mut current), style));
        }
        current_is_block = is_block;
        current.push(ch);
    }
    if !current.is_empty() {
        spans.push(Span::styled(
            current,
            if current_is_block { block } else { shadow },
        ));
    }
    Line::from(spans)
}

/// El banner completo, o una línea simple si no entra.
pub fn lines(width: u16) -> Vec<Line<'static>> {
    if width >= WIDTH {
        rows().iter().map(|r| styled(r)).collect()
    } else {
        vec![Line::styled(
            "LEARN GIT",
            Style::new().fg(Color::White).add_modifier(Modifier::BOLD),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rows_have_the_declared_width() {
        for row in rows() {
            assert_eq!(row.chars().count() as u16, WIDTH, "{row}");
        }
    }
}
