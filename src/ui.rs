//! Dibujo de la pantalla del modo libre.

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Position, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Block, Paragraph};

use crate::app::App;

pub fn draw(frame: &mut Frame, app: &App) {
    if let Some(editor) = &app.editor {
        frame.render_widget(&editor.textarea, frame.area());
        return;
    }

    let [output_area, input_area, help_area] = Layout::vertical([
        Constraint::Min(3),
        Constraint::Length(3),
        Constraint::Length(1),
    ])
    .areas(frame.area());

    draw_output(frame, app, output_area);
    draw_input(frame, app, input_area);

    let help = if app.confirm_reset {
        "Reset the sandbox? Everything you created will be lost.  y: yes · any other key: no"
    } else if app.running.is_some() {
        "Enter: send to git · Ctrl+D: close stdin · Ctrl+C: stop git · PgUp/PgDn: scroll"
    } else {
        "Enter: run · Ctrl+R: reset sandbox · Ctrl+C: clear line · PgUp/PgDn: scroll · Ctrl+Q: quit"
    };
    frame.render_widget(
        Paragraph::new(help).style(Style::new().add_modifier(Modifier::DIM)),
        help_area,
    );
}

fn draw_output(frame: &mut Frame, app: &App, area: Rect) {
    let block = Block::bordered().title(" Free mode ");
    let inner = block.inner(area);
    let height = inner.height as usize;
    let rows = tail_rows(
        &app.output.lines,
        inner.width.max(1) as usize,
        height + app.scroll,
    );
    let end = rows.len().saturating_sub(app.scroll);
    let start = end.saturating_sub(height);
    let text = rows[start..end].join("\n");
    frame.render_widget(Paragraph::new(text).block(block), area);
}

fn draw_input(frame: &mut Frame, app: &App, area: Rect) {
    let block = Block::bordered();
    let inner = block.inner(area);
    let prompt = app.prompt();
    let prompt_len = prompt.chars().count();
    let width = inner.width.max(1) as usize;
    // Si la línea es más larga que el panel, se muestra la parte que contiene el cursor.
    let offset = (prompt_len + app.cursor).saturating_sub(width - 1);
    let visible: String = format!("{prompt}{}", app.input)
        .chars()
        .skip(offset)
        .take(width)
        .collect();
    frame.render_widget(Paragraph::new(visible).block(block), area);
    frame.set_cursor_position(Position::new(
        inner.x + (prompt_len + app.cursor - offset) as u16,
        inner.y,
    ));
}

/// Las últimas `need` filas de pantalla, con las líneas largas cortadas al ancho del panel.
fn tail_rows(lines: &[String], width: usize, need: usize) -> Vec<String> {
    let mut rows: Vec<String> = Vec::new();
    for line in lines.iter().rev() {
        let chars: Vec<char> = line.chars().collect();
        let chunks: Vec<String> = if chars.is_empty() {
            vec![String::new()]
        } else {
            chars.chunks(width).map(|c| c.iter().collect()).collect()
        };
        rows.extend(chunks.into_iter().rev());
        if rows.len() >= need {
            break;
        }
    }
    rows.reverse();
    rows
}
