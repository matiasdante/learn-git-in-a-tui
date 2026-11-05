//! Dibujo de las pantallas: menú, teoría, ejercicio y modo libre.

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Position, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph, Wrap};

use crate::app::{App, ExStatus, ExerciseState, MenuItem, Screen};
use crate::i18n::tr;

pub fn draw(frame: &mut Frame, app: &App) {
    if let Some(editor) = &app.editor {
        frame.render_widget(&editor.textarea, frame.area());
        return;
    }
    match &app.screen {
        Screen::Menu => draw_menu(frame, app),
        Screen::Theory { lesson, scroll } => draw_theory(frame, app, *lesson, *scroll),
        Screen::Exercise(state) => draw_exercise(frame, app, state),
        Screen::Free => draw_free(frame, app),
    }
}

fn dim() -> Style {
    Style::new().add_modifier(Modifier::DIM)
}

fn draw_help(frame: &mut Frame, area: Rect, text: &str) {
    frame.render_widget(Paragraph::new(text.to_string()).style(dim()), area);
}

fn draw_menu(frame: &mut Frame, app: &App) {
    let [body, help] =
        Layout::vertical([Constraint::Min(3), Constraint::Length(1)]).areas(frame.area());
    let block = Block::bordered().title(" learn-git-tui ");
    let height = block.inner(body).height as usize;

    let mut lines: Vec<Line> = Vec::new();
    let mut selected_line = 0;
    let mut last_level = None;
    for (i, item) in app.menu_items().into_iter().enumerate() {
        let (level, label) = match item {
            MenuItem::Lesson(l) => {
                let lesson = &app.content.lessons[l];
                (Some(lesson.level), lesson.title.get(app.lang).to_string())
            }
            MenuItem::Free => (None, tr(app.lang, "Free mode", "Modo libre").to_string()),
        };
        if level != last_level {
            let header = match level {
                Some(id) => {
                    let title = app
                        .content
                        .level(id)
                        .map_or("", |lvl| lvl.title.get(app.lang));
                    format!("{} {id} — {title}", tr(app.lang, "Level", "Nivel"))
                }
                None => tr(app.lang, "Practice", "Práctica").to_string(),
            };
            if !lines.is_empty() {
                lines.push(Line::raw(""));
            }
            lines.push(Line::styled(
                header,
                Style::new().add_modifier(Modifier::BOLD),
            ));
            last_level = level;
        }
        if i == app.menu_selected {
            selected_line = lines.len();
            lines.push(Line::from(Span::styled(
                format!("  ▶ {label}"),
                Style::new().add_modifier(Modifier::REVERSED),
            )));
        } else {
            lines.push(Line::raw(format!("    {label}")));
        }
    }
    let offset = selected_line.saturating_sub(height.saturating_sub(2)) as u16;
    frame.render_widget(Paragraph::new(lines).block(block).scroll((offset, 0)), body);
    draw_help(
        frame,
        help,
        tr(
            app.lang,
            "↑/↓ move · Enter open · L: English/Español · Q quit",
            "↑/↓ mover · Enter abrir · L: English/Español · Q salir",
        ),
    );
}

fn draw_theory(frame: &mut Frame, app: &App, lesson: usize, scroll: u16) {
    let lesson = &app.content.lessons[lesson];
    let [body, help] =
        Layout::vertical([Constraint::Min(3), Constraint::Length(1)]).areas(frame.area());
    let block = Block::bordered().title(format!(" {} ", lesson.title.get(app.lang)));
    frame.render_widget(
        Paragraph::new(lesson.theory.get(app.lang).to_string())
            .block(block)
            .wrap(Wrap { trim: false })
            .scroll((scroll, 0)),
        body,
    );
    draw_help(
        frame,
        help,
        tr(
            app.lang,
            "↑/↓ scroll · Enter start exercises · L language · Esc back",
            "↑/↓ desplazar · Enter empezar ejercicios · L idioma · Esc volver",
        ),
    );
}

fn draw_exercise(frame: &mut Frame, app: &App, state: &ExerciseState) {
    let lesson = &app.content.lessons[state.lesson];
    let exercise = &lesson.exercises[state.index];
    let prompt_height = (frame.area().height / 3).clamp(6, 13);
    let [prompt_area, output_area, input_area, help_area] = Layout::vertical([
        Constraint::Length(prompt_height),
        Constraint::Min(3),
        Constraint::Length(3),
        Constraint::Length(1),
    ])
    .areas(frame.area());

    let mut lines = vec![Line::styled(
        exercise.prompt.get(app.lang).to_string(),
        Style::new().add_modifier(Modifier::BOLD),
    )];
    for (i, hint) in exercise.hints.iter().take(state.hints_shown).enumerate() {
        lines.push(Line::raw(format!(
            "{} {}: {}",
            tr(app.lang, "Hint", "Pista"),
            i + 1,
            hint.get(app.lang)
        )));
    }
    match &state.status {
        ExStatus::Doing => {}
        ExStatus::Failed(why) => {
            lines.push(Line::raw(""));
            lines.push(Line::raw(format!("✗ {}", why.get(app.lang))));
        }
        ExStatus::Passed => {
            lines.push(Line::raw(""));
            lines.push(Line::raw(tr(app.lang, "✓ Correct!", "✓ ¡Correcto!")));
        }
    }
    let title = format!(
        " {} — {} {}/{} ",
        lesson.title.get(app.lang),
        tr(app.lang, "exercise", "ejercicio"),
        state.index + 1,
        lesson.exercises.len()
    );
    frame.render_widget(
        Paragraph::new(lines)
            .block(Block::bordered().title(title))
            .wrap(Wrap { trim: false }),
        prompt_area,
    );

    draw_console(
        frame,
        app,
        output_area,
        input_area,
        tr(app.lang, " Console ", " Consola "),
    );
    let help = if app.running.is_some() {
        tr(
            app.lang,
            "Enter: send to git · Ctrl+D: close stdin · Ctrl+C: stop git",
            "Enter: enviar a git · Ctrl+D: cerrar stdin · Ctrl+C: detener git",
        )
    } else if matches!(state.status, ExStatus::Passed) {
        tr(
            app.lang,
            "F6: continue · Esc: back to the menu",
            "F6: continuar · Esc: volver al menú",
        )
    } else {
        tr(
            app.lang,
            "F5: check · F1: hint · Ctrl+R: restart · Esc: menu · Ctrl+Q: quit",
            "F5: verificar · F1: pista · Ctrl+R: reiniciar · Esc: menú · Ctrl+Q: salir",
        )
    };
    draw_help(frame, help_area, help);
}

fn draw_free(frame: &mut Frame, app: &App) {
    let [output_area, input_area, help_area] = Layout::vertical([
        Constraint::Min(3),
        Constraint::Length(3),
        Constraint::Length(1),
    ])
    .areas(frame.area());
    draw_console(
        frame,
        app,
        output_area,
        input_area,
        tr(app.lang, " Free mode ", " Modo libre "),
    );
    let help = if app.confirm_reset {
        tr(
            app.lang,
            "Reset the sandbox? Everything you created will be lost.  y: yes · any other key: no",
            "¿Reiniciar el sandbox? Se pierde todo lo que creaste.  y: sí · otra tecla: no",
        )
    } else if app.running.is_some() {
        tr(
            app.lang,
            "Enter: send to git · Ctrl+D: close stdin · Ctrl+C: stop git",
            "Enter: enviar a git · Ctrl+D: cerrar stdin · Ctrl+C: detener git",
        )
    } else {
        tr(
            app.lang,
            "Enter: run · Ctrl+R: reset sandbox · Esc: menu · PgUp/PgDn: scroll · Ctrl+Q: quit",
            "Enter: ejecutar · Ctrl+R: reiniciar sandbox · Esc: menú · PgUp/PgDn: desplazar · Ctrl+Q: salir",
        )
    };
    draw_help(frame, help_area, help);
}

fn draw_console(frame: &mut Frame, app: &App, output_area: Rect, input_area: Rect, title: &str) {
    draw_output(frame, app, output_area, title);
    draw_input(frame, app, input_area);
}

fn draw_output(frame: &mut Frame, app: &App, area: Rect, title: &str) {
    let block = Block::bordered().title(title.to_string());
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
