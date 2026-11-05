//! Dibujo de las pantallas: menú, teoría, ejercicio y modo libre.

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Position, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph, Wrap};

use crate::app::{App, ExStatus, ExamState, ExerciseState, MapItem, Screen};
use crate::i18n::Lang;
use crate::i18n::tr;

pub fn draw(frame: &mut Frame, app: &App) {
    if let Some(editor) = &app.editor {
        frame.render_widget(&editor.textarea, frame.area());
        return;
    }
    match &app.screen {
        Screen::LangSelect => draw_lang(frame, app),
        Screen::ExamOffer { selected } => draw_exam_offer(frame, app, *selected),
        Screen::Exam(exam) => draw_exam(frame, app, exam),
        Screen::Map => draw_map(frame, app),
        Screen::Level(level) => draw_level(frame, app, *level),
        Screen::Theory { lesson, scroll } => draw_theory(frame, app, *lesson, *scroll),
        Screen::Exercise(state) => draw_exercise(frame, app, state),
        Screen::Free => draw_free(frame, app),
        Screen::Achievements => draw_achievements(frame, app),
    }
}

fn dim() -> Style {
    Style::new().add_modifier(Modifier::DIM)
}

fn draw_help(frame: &mut Frame, area: Rect, text: &str) {
    frame.render_widget(Paragraph::new(text.to_string()).style(dim()), area);
}

fn centered(frame: &Frame, width: u16, height: u16) -> Rect {
    let area = frame.area();
    let w = width.min(area.width);
    let h = height.min(area.height);
    Rect::new(
        area.x + (area.width - w) / 2,
        area.y + (area.height - h) / 2,
        w,
        h,
    )
}

fn option_line(label: String, selected: bool) -> Line<'static> {
    if selected {
        Line::from(Span::styled(
            format!("  ▶ {label}"),
            Style::new().add_modifier(Modifier::REVERSED),
        ))
    } else {
        Line::raw(format!("    {label}"))
    }
}

fn draw_lang(frame: &mut Frame, app: &App) {
    let area = centered(frame, 44, 8);
    let lines = vec![
        Line::raw("Choose your language / Elegí tu idioma"),
        Line::raw(""),
        option_line("1  English".into(), app.lang == Lang::En),
        option_line("2  Español".into(), app.lang == Lang::Es),
        Line::raw(""),
        Line::styled("↑/↓ · Enter", dim()),
    ];
    frame.render_widget(
        Paragraph::new(lines).block(Block::bordered().title(" learn-git-tui ")),
        area,
    );
}

fn draw_exam_offer(frame: &mut Frame, app: &App, selected: usize) {
    let area = centered(frame, 70, 11);
    let lines = vec![
        Line::raw(tr(
            app.lang,
            "Do you already know some git? A 10-question placement exam picks your starting level.",
            "¿Ya sabés algo de git? Un examen de ubicación de 10 preguntas elige tu nivel de arranque.",
        )),
        Line::raw(""),
        option_line(
            tr(
                app.lang,
                "Take the placement exam",
                "Rendir el examen de ubicación",
            )
            .into(),
            selected == 0,
        ),
        option_line(
            tr(
                app.lang,
                "Skip it and start at level 1",
                "Omitirlo y empezar en el nivel 1",
            )
            .into(),
            selected == 1,
        ),
        Line::raw(""),
        Line::styled(
            tr(
                app.lang,
                "Earlier levels always stay available.  ↑/↓ · Enter · L language",
                "Los niveles anteriores siempre quedan disponibles.  ↑/↓ · Enter · L idioma",
            ),
            dim(),
        ),
    ];
    frame.render_widget(
        Paragraph::new(lines)
            .block(Block::bordered().title(tr(
                app.lang,
                " Placement exam ",
                " Examen de ubicación ",
            )))
            .wrap(Wrap { trim: false }),
        area,
    );
}

fn draw_exam(frame: &mut Frame, app: &App, exam: &ExamState) {
    let area = centered(frame, 90, 16);
    let title = tr(app.lang, " Placement exam ", " Examen de ubicación ");
    if let Some(level) = exam.result {
        let name = app
            .content
            .level(level)
            .map_or("", |l| l.title.get(app.lang));
        let lines = vec![
            Line::raw(tr(app.lang, "Exam finished.", "Examen terminado.")),
            Line::raw(""),
            Line::styled(
                format!(
                    "{} {level} — {name}",
                    tr(app.lang, "You start at level", "Arrancás en el nivel")
                ),
                Style::new().add_modifier(Modifier::BOLD),
            ),
            Line::raw(""),
            Line::styled(tr(app.lang, "Enter: continue", "Enter: continuar"), dim()),
        ];
        frame.render_widget(
            Paragraph::new(lines).block(Block::bordered().title(title)),
            area,
        );
        return;
    }
    let question = &app.content.exam[exam.question];
    let mut lines = vec![
        Line::styled(
            format!(
                "{} {}/{}",
                tr(app.lang, "Question", "Pregunta"),
                exam.question + 1,
                app.content.exam.len()
            ),
            dim(),
        ),
        Line::raw(question.text.get(app.lang).to_string()),
        Line::raw(""),
    ];
    for (i, option) in question.options.iter().enumerate() {
        lines.push(option_line(
            format!("{}  {}", i + 1, option.get(app.lang)),
            i == exam.selected,
        ));
    }
    lines.push(Line::raw(""));
    lines.push(Line::styled(
        tr(
            app.lang,
            "↑/↓ or 1-4 to choose · Enter to answer",
            "↑/↓ o 1-4 para elegir · Enter para responder",
        ),
        dim(),
    ));
    frame.render_widget(
        Paragraph::new(lines)
            .block(Block::bordered().title(title))
            .wrap(Wrap { trim: false }),
        area,
    );
}

fn draw_map(frame: &mut Frame, app: &App) {
    let [body, help] =
        Layout::vertical([Constraint::Min(3), Constraint::Length(1)]).areas(frame.area());
    let title = format!(
        " learn-git-tui — {} {} ",
        app.progress.total_points(),
        tr(app.lang, "points", "puntos")
    );
    let mut lines: Vec<Line> = Vec::new();
    for (i, item) in app.map_items().into_iter().enumerate() {
        let label = match item {
            MapItem::Level(id) => {
                let name = app.content.level(id).map_or("", |l| l.title.get(app.lang));
                let (done, total) = app.content.level_progress(id, &app.progress);
                let mark = if total > 0 && done == total {
                    "✓"
                } else {
                    " "
                };
                let start = if app.progress.placement == Some(id) {
                    tr(app.lang, "  ← your start", "  ← tu inicio")
                } else {
                    ""
                };
                format!(
                    "{mark} {} {id:>2} — {name}  [{done}/{total}]{start}",
                    tr(app.lang, "Level", "Nivel")
                )
            }
            MapItem::Free => format!("  {}", tr(app.lang, "Free mode", "Modo libre")),
            MapItem::Achievements => format!(
                "  {} ({}/{})",
                tr(app.lang, "Achievements", "Logros"),
                app.progress.achievements.len(),
                app.content.achievements.len()
            ),
        };
        if matches!(item, MapItem::Free) {
            lines.push(Line::raw(""));
        }
        lines.push(option_line(label, i == app.map_selected));
    }
    frame.render_widget(
        Paragraph::new(lines).block(Block::bordered().title(title)),
        body,
    );
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

fn draw_level(frame: &mut Frame, app: &App, level: u8) {
    let [body, help] =
        Layout::vertical([Constraint::Min(3), Constraint::Length(1)]).areas(frame.area());
    let name = app
        .content
        .level(level)
        .map_or("", |l| l.title.get(app.lang));
    let mut lines: Vec<Line> = Vec::new();
    for (pos, (_, lesson)) in app.content.lessons_of(level).enumerate() {
        let done = !lesson.exercises.is_empty()
            && lesson.exercises.iter().all(|e| app.progress.is_done(&e.id));
        let mark = if done { "✓" } else { " " };
        lines.push(option_line(
            format!("{mark} {}", lesson.title.get(app.lang)),
            pos == app.level_selected,
        ));
    }
    if lines.is_empty() {
        lines.push(Line::raw(tr(app.lang, "Coming soon.", "Próximamente.")));
    }
    let block = Block::bordered().title(format!(
        " {} {level} — {name} ",
        tr(app.lang, "Level", "Nivel")
    ));
    frame.render_widget(Paragraph::new(lines).block(block), body);
    draw_help(
        frame,
        help,
        tr(
            app.lang,
            "↑/↓ move · Enter open lesson · L language · Esc back",
            "↑/↓ mover · Enter abrir lección · L idioma · Esc volver",
        ),
    );
}

fn draw_achievements(frame: &mut Frame, app: &App) {
    let [body, help] =
        Layout::vertical([Constraint::Min(3), Constraint::Length(1)]).areas(frame.area());
    let mut lines: Vec<Line> = Vec::new();
    for a in &app.content.achievements {
        let unlocked = app.progress.achievements.contains(&a.id);
        let (mark, style) = if unlocked {
            ("★", Style::new().add_modifier(Modifier::BOLD))
        } else {
            ("☆", dim())
        };
        lines.push(Line::styled(
            format!("{mark} {}", a.title.get(app.lang)),
            style,
        ));
        lines.push(Line::styled(
            format!("    {}", a.description.get(app.lang)),
            dim(),
        ));
    }
    let title = format!(
        " {} — {} {} ",
        tr(app.lang, "Achievements", "Logros"),
        app.progress.total_points(),
        tr(app.lang, "points", "puntos")
    );
    frame.render_widget(
        Paragraph::new(lines).block(Block::bordered().title(title)),
        body,
    );
    draw_help(frame, help, tr(app.lang, "Esc: back", "Esc: volver"));
}

fn draw_theory(frame: &mut Frame, app: &App, lesson_index: usize, scroll: u16) {
    let lesson = &app.content.lessons[lesson_index];
    let [body, help] =
        Layout::vertical([Constraint::Min(3), Constraint::Length(1)]).areas(frame.area());
    let block = Block::bordered().title(format!(" {} ", lesson.title.get(app.lang)));
    let mut lines: Vec<Line> = lesson
        .theory
        .get(app.lang)
        .lines()
        .map(|l| Line::raw(l.to_string()))
        .collect();
    let missing = &app.unavailable[lesson_index];
    if !missing.is_empty() {
        lines.push(Line::raw(""));
        lines.push(Line::styled(
            format!(
                "{} {}",
                tr(
                    app.lang,
                    "Explanation only: this machine is missing",
                    "Solo explicación: a esta máquina le falta"
                ),
                missing.join(", ")
            ),
            Style::new().add_modifier(Modifier::BOLD),
        ));
    } else if lesson.exercises.is_empty() {
        lines.push(Line::raw(""));
        lines.push(Line::styled(
            tr(
                app.lang,
                "This topic is explained without an exercise.",
                "Este tema se explica sin ejercicio.",
            ),
            dim(),
        ));
    }
    frame.render_widget(
        Paragraph::new(lines)
            .block(block)
            .wrap(Wrap { trim: false })
            .scroll((scroll, 0)),
        body,
    );
    let can_practice = !lesson.exercises.is_empty() && missing.is_empty();
    draw_help(
        frame,
        help,
        match (can_practice, app.lang) {
            (true, Lang::En) => "↑/↓ scroll · Enter start exercises · L language · Esc back",
            (true, Lang::Es) => "↑/↓ desplazar · Enter empezar ejercicios · L idioma · Esc volver",
            (false, Lang::En) => "↑/↓ scroll · L language · Esc back",
            (false, Lang::Es) => "↑/↓ desplazar · L idioma · Esc volver",
        },
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
        ExStatus::Passed { points } => {
            lines.push(Line::raw(""));
            lines.push(Line::raw(format!(
                "{} +{points} {}",
                tr(app.lang, "✓ Correct!", "✓ ¡Correcto!"),
                tr(app.lang, "points", "puntos")
            )));
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
    } else if matches!(state.status, ExStatus::Passed { .. }) {
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
