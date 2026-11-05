//! Estado de la aplicación: navegación entre pantallas, consola, editor y progreso.

use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender};

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::console::{Console, Event, Outcome, Running};
use crate::content::Content;
use crate::editor::{EditorAction, EditorState, IpcServer};
use crate::engine::{self, Verdict};
use crate::i18n::{Lang, Text, tr};
use crate::progress::Progress;
use crate::sandbox::Sandbox;

const MAX_LINES: usize = 5000;

/// Salida de la consola. Siempre hay al menos una línea: la que está en curso.
pub struct Output {
    pub lines: Vec<String>,
}

impl Output {
    fn new() -> Self {
        Self {
            lines: vec![String::new()],
        }
    }

    pub fn push_text(&mut self, text: &str) {
        for ch in text.chars() {
            match ch {
                '\n' => self.lines.push(String::new()),
                '\r' => {}
                '\t' => self.current().push_str("    "),
                c => self.current().push(c),
            }
        }
        if self.lines.len() > MAX_LINES {
            let excess = self.lines.len() - MAX_LINES;
            self.lines.drain(..excess);
        }
    }

    /// Escribe una línea completa, cortando antes la que estuviera a medias.
    pub fn push_line(&mut self, line: &str) {
        if !self.current().is_empty() {
            self.lines.push(String::new());
        }
        self.push_text(line);
        self.lines.push(String::new());
    }

    fn current(&mut self) -> &mut String {
        self.lines.last_mut().expect("output always has a line")
    }

    fn clear(&mut self) {
        self.lines = vec![String::new()];
    }
}

pub enum Screen {
    LangSelect,
    ExamOffer { selected: usize },
    Exam(ExamState),
    Map,
    Level(u8),
    Theory { lesson: usize, scroll: u16 },
    Exercise(ExerciseState),
    Free,
    Achievements,
}

pub struct ExamState {
    pub question: usize,
    pub selected: usize,
    /// Primer nivel (el más bajo) donde se falló alguna pregunta.
    pub first_wrong: Option<u8>,
    /// Nivel de arranque, cuando el examen terminó.
    pub result: Option<u8>,
}

pub struct ExerciseState {
    pub lesson: usize,
    pub index: usize,
    pub hints_shown: usize,
    pub failures: u32,
    pub status: ExStatus,
}

pub enum ExStatus {
    Doing,
    Failed(Text),
    Passed { points: u32 },
}

/// Una fila seleccionable del mapa de niveles.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum MapItem {
    Level(u8),
    Free,
    Achievements,
}

pub struct App {
    pub content: Content,
    pub progress: Progress,
    progress_path: Option<PathBuf>,
    pub lang: Lang,
    pub screen: Screen,
    pub map_selected: usize,
    pub level_selected: usize,
    pub console: Console,
    ipc: IpcServer,
    pub output: Output,
    pub input: String,
    /// Posición del cursor en caracteres.
    pub cursor: usize,
    history: Vec<String>,
    history_pos: Option<usize>,
    pub running: Option<Running>,
    pub editor: Option<EditorState>,
    pub confirm_reset: bool,
    pub scroll: usize,
    pub quit: bool,
    tx: Sender<Event>,
    rx: Receiver<Event>,
}

impl App {
    /// `progress_path`: dónde guardar el progreso (`None` en los tests).
    pub fn new(
        sandbox: Sandbox,
        content: Content,
        progress: Progress,
        progress_path: Option<PathBuf>,
    ) -> Self {
        let (tx, rx) = mpsc::channel();
        let lang = progress.lang.unwrap_or_default();
        let screen = match (progress.lang, progress.exam_done) {
            (None, _) => Screen::LangSelect,
            (Some(_), false) => Screen::ExamOffer { selected: 0 },
            (Some(_), true) => Screen::Map,
        };
        let mut app = Self {
            content,
            progress,
            progress_path,
            lang,
            screen,
            map_selected: 0,
            level_selected: 0,
            ipc: IpcServer::new(sandbox.ipc_dir()),
            console: Console::new(sandbox),
            output: Output::new(),
            input: String::new(),
            cursor: 0,
            history: Vec::new(),
            history_pos: None,
            running: None,
            editor: None,
            confirm_reset: false,
            scroll: 0,
            quit: false,
            tx,
            rx,
        };
        app.select_start_level();
        app
    }

    pub fn prompt(&self) -> String {
        if self.running.is_some() {
            "> ".to_string()
        } else {
            format!("{} $ ", self.console.display_cwd())
        }
    }

    pub fn map_items(&self) -> Vec<MapItem> {
        let mut items: Vec<MapItem> = self
            .content
            .levels
            .iter()
            .map(|l| MapItem::Level(l.id))
            .collect();
        items.push(MapItem::Free);
        items.push(MapItem::Achievements);
        items
    }

    /// Deja el cursor del mapa en el nivel recomendado por el examen.
    fn select_start_level(&mut self) {
        if let Some(level) = self.progress.placement {
            let items = self.map_items();
            if let Some(pos) = items.iter().position(|i| *i == MapItem::Level(level)) {
                self.map_selected = pos;
            }
        }
    }

    fn save_progress(&mut self) {
        let Some(path) = &self.progress_path else {
            return;
        };
        if let Err(e) = self.progress.save(path) {
            self.output
                .push_line(&format!("warning: progress not saved: {e:#}"));
        }
    }

    /// Procesa lo que llegó de los hilos de fondo y los pedidos de editor de git.
    pub fn tick(&mut self) {
        while let Ok(event) = self.rx.try_recv() {
            match event {
                Event::Output(text) => self.output.push_text(&text),
                Event::Finished(code) => {
                    self.running = None;
                    if self.editor.as_ref().is_some_and(|e| e.from_git) {
                        self.editor = None;
                    }
                    match code {
                        Some(0) => {}
                        Some(code) => self.output.push_line(&format!("[exit code {code}]")),
                        None => self.output.push_line("[terminated]"),
                    }
                }
            }
        }
        if self.editor.is_none()
            && let Some(path) = self.ipc.poll_request()
        {
            match EditorState::open(path, true) {
                Ok(editor) => self.editor = Some(editor),
                Err(e) => {
                    self.output.push_line(&format!("error: {e:#}"));
                    let _ = self.ipc.respond(false);
                }
            }
        }
    }

    pub fn shutdown(&mut self) {
        if let Some(running) = &self.running {
            running.kill();
        }
    }

    pub fn on_key(&mut self, key: KeyEvent) {
        if self.editor.is_some() {
            self.on_editor_key(key);
            return;
        }
        match self.screen {
            Screen::LangSelect => self.on_lang_key(key),
            Screen::ExamOffer { .. } => self.on_exam_offer_key(key),
            Screen::Exam(_) => self.on_exam_key(key),
            Screen::Map => self.on_map_key(key),
            Screen::Level(_) => self.on_level_key(key),
            Screen::Theory { .. } => self.on_theory_key(key),
            Screen::Achievements => {
                if matches!(key.code, KeyCode::Esc | KeyCode::Char('q') | KeyCode::Enter) {
                    self.screen = Screen::Map;
                }
            }
            Screen::Exercise(_) | Screen::Free => self.on_console_key(key),
        }
    }

    // ---------- idioma y examen ----------

    fn on_lang_key(&mut self, key: KeyEvent) {
        let chosen = match key.code {
            KeyCode::Char('1') | KeyCode::Char('e') => Some(Lang::En),
            KeyCode::Char('2') | KeyCode::Char('s') => Some(Lang::Es),
            KeyCode::Up | KeyCode::Down => {
                self.lang = self.lang.toggle();
                None
            }
            KeyCode::Enter => Some(self.lang),
            KeyCode::Char('q') | KeyCode::Esc => {
                self.quit = true;
                None
            }
            _ => None,
        };
        if let Some(lang) = chosen {
            self.lang = lang;
            self.progress.lang = Some(lang);
            self.save_progress();
            self.screen = if self.progress.exam_done {
                Screen::Map
            } else {
                Screen::ExamOffer { selected: 0 }
            };
        }
    }

    fn on_exam_offer_key(&mut self, key: KeyEvent) {
        let Screen::ExamOffer { selected } = &mut self.screen else {
            return;
        };
        match key.code {
            KeyCode::Up | KeyCode::Down => *selected = 1 - *selected,
            KeyCode::Char('l') => self.lang = self.lang.toggle(),
            KeyCode::Char('q') => self.quit = true,
            KeyCode::Enter => {
                if *selected == 0 && !self.content.exam.is_empty() {
                    self.screen = Screen::Exam(ExamState {
                        question: 0,
                        selected: 0,
                        first_wrong: None,
                        result: None,
                    });
                } else {
                    // Sin examen se arranca en el nivel 1.
                    self.progress.exam_done = true;
                    self.progress.placement = None;
                    self.save_progress();
                    self.map_selected = 0;
                    self.screen = Screen::Map;
                }
            }
            _ => {}
        }
    }

    fn on_exam_key(&mut self, key: KeyEvent) {
        let Screen::Exam(exam) = &mut self.screen else {
            return;
        };
        if let Some(level) = exam.result {
            if key.code == KeyCode::Enter {
                self.progress.exam_done = true;
                self.progress.placement = Some(level);
                self.save_progress();
                self.select_start_level();
                self.screen = Screen::Map;
            }
            return;
        }
        let options = self.content.exam[exam.question].options.len();
        match key.code {
            KeyCode::Up => exam.selected = exam.selected.saturating_sub(1),
            KeyCode::Down => exam.selected = (exam.selected + 1).min(options - 1),
            KeyCode::Char(c) if c.is_ascii_digit() => {
                let n = c.to_digit(10).unwrap_or(0) as usize;
                if (1..=options).contains(&n) {
                    exam.selected = n - 1;
                }
            }
            KeyCode::Enter => {
                let question = &self.content.exam[exam.question];
                if exam.selected != question.answer {
                    exam.first_wrong = Some(
                        exam.first_wrong
                            .map_or(question.level, |l| l.min(question.level)),
                    );
                }
                exam.question += 1;
                exam.selected = 0;
                if exam.question >= self.content.exam.len() {
                    // Todo bien arranca después del último nivel evaluado (el 6).
                    let tested_max = self.content.exam.iter().map(|q| q.level).max().unwrap_or(0);
                    let start = exam.first_wrong.unwrap_or(tested_max + 1);
                    exam.result = Some(start.clamp(1, self.content.max_level()));
                }
            }
            _ => {}
        }
    }

    // ---------- mapa, nivel y teoría ----------

    fn on_map_key(&mut self, key: KeyEvent) {
        let items = self.map_items();
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => self.quit = true,
            KeyCode::Char('c') if ctrl => self.quit = true,
            KeyCode::Char('l') => {
                self.lang = self.lang.toggle();
                self.progress.lang = Some(self.lang);
                self.save_progress();
            }
            KeyCode::Up => self.map_selected = self.map_selected.saturating_sub(1),
            KeyCode::Down => self.map_selected = (self.map_selected + 1).min(items.len() - 1),
            KeyCode::Enter => match items[self.map_selected] {
                MapItem::Level(level) => {
                    self.level_selected = 0;
                    self.screen = Screen::Level(level);
                }
                MapItem::Free => self.enter_free_mode(),
                MapItem::Achievements => self.screen = Screen::Achievements,
            },
            _ => {}
        }
    }

    fn on_level_key(&mut self, key: KeyEvent) {
        let Screen::Level(level) = self.screen else {
            return;
        };
        let lessons: Vec<usize> = self.content.lessons_of(level).map(|(i, _)| i).collect();
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => self.screen = Screen::Map,
            KeyCode::Char('l') => self.lang = self.lang.toggle(),
            KeyCode::Up => self.level_selected = self.level_selected.saturating_sub(1),
            KeyCode::Down if !lessons.is_empty() => {
                self.level_selected = (self.level_selected + 1).min(lessons.len() - 1);
            }
            KeyCode::Enter if !lessons.is_empty() => {
                self.screen = Screen::Theory {
                    lesson: lessons[self.level_selected],
                    scroll: 0,
                };
            }
            _ => {}
        }
    }

    fn on_theory_key(&mut self, key: KeyEvent) {
        let Screen::Theory { lesson, scroll } = &mut self.screen else {
            return;
        };
        let lesson = *lesson;
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => {
                self.screen = Screen::Level(self.content.lessons[lesson].level);
            }
            KeyCode::Char('l') => self.lang = self.lang.toggle(),
            KeyCode::Up => *scroll = scroll.saturating_sub(1),
            KeyCode::Down => *scroll += 1,
            KeyCode::PageUp => *scroll = scroll.saturating_sub(10),
            KeyCode::PageDown => *scroll += 10,
            KeyCode::Enter if !self.content.lessons[lesson].exercises.is_empty() => {
                self.start_exercise(lesson, 0);
            }
            _ => {}
        }
    }

    // ---------- ejercicios y modo libre ----------

    fn enter_free_mode(&mut self) {
        self.screen = Screen::Free;
        self.reset();
    }

    fn leave_console(&mut self) {
        if let Some(running) = self.running.take() {
            running.kill();
        }
        self.clear_input();
        self.confirm_reset = false;
        self.screen = match &self.screen {
            Screen::Exercise(state) => Screen::Level(self.content.lessons[state.lesson].level),
            _ => Screen::Map,
        };
    }

    fn start_exercise(&mut self, lesson: usize, index: usize) {
        if let Some(running) = self.running.take() {
            running.kill();
        }
        self.clear_input();
        self.scroll = 0;
        self.output.clear();
        let exercise = &self.content.lessons[lesson].exercises[index];
        match engine::start(&mut self.console, exercise) {
            Ok(()) => self.output.push_line(tr(
                self.lang,
                "Sandbox ready. Work in the console, then press F5 to check.",
                "Sandbox listo. Trabajá en la consola y apretá F5 para verificar.",
            )),
            Err(e) => self.output.push_line(&format!("error: {e:#}")),
        }
        // Reiniciar el mismo ejercicio conserva pistas y fallos: no se pueden "borrar" reiniciando.
        let (hints_shown, failures) = match &self.screen {
            Screen::Exercise(s) if s.lesson == lesson && s.index == index => {
                (s.hints_shown, s.failures)
            }
            _ => (0, 0),
        };
        self.screen = Screen::Exercise(ExerciseState {
            lesson,
            index,
            hints_shown,
            failures,
            status: ExStatus::Doing,
        });
    }

    fn verify_exercise(&mut self) {
        let Screen::Exercise(state) = &self.screen else {
            return;
        };
        if self.running.is_some() || matches!(state.status, ExStatus::Passed { .. }) {
            return;
        }
        let (lesson, index, hints, failures) =
            (state.lesson, state.index, state.hints_shown, state.failures);
        let exercise = &self.content.lessons[lesson].exercises[index];
        let verdict = engine::verify(&mut self.console, exercise);
        self.scroll = 0;

        let status = match verdict {
            Ok(Verdict::Pass) => {
                let id = exercise.id.clone();
                let points = self.progress.record_pass(&id, hints as u32, failures);
                self.output.push_line(&format!(
                    "{} +{points} {}",
                    tr(
                        self.lang,
                        "✓ Correct! Press F6 to continue.",
                        "✓ ¡Correcto! Apretá F6 para continuar."
                    ),
                    tr(self.lang, "points", "puntos")
                ));
                self.award_achievements();
                self.save_progress();
                ExStatus::Passed { points }
            }
            Ok(Verdict::Fail(why)) => {
                self.output.push_line(&format!(
                    "✗ {} {}",
                    why.get(self.lang),
                    tr(
                        self.lang,
                        "(sandbox restored to the start of the exercise)",
                        "(el sandbox volvió al inicio del ejercicio)"
                    )
                ));
                ExStatus::Failed(why)
            }
            Err(e) => {
                self.output.push_line(&format!("error: {e:#}"));
                return;
            }
        };
        let failed = matches!(status, ExStatus::Failed(_));
        if let Screen::Exercise(state) = &mut self.screen {
            state.status = status;
            if failed {
                state.failures += 1;
            }
        }
    }

    fn award_achievements(&mut self) {
        let unlocked: Vec<(String, Text)> = self
            .content
            .newly_unlocked(&self.progress)
            .iter()
            .map(|a| (a.id.clone(), a.title.clone()))
            .collect();
        for (id, title) in unlocked {
            self.output.push_line(&format!(
                "★ {}: {}",
                tr(self.lang, "Achievement unlocked", "Logro desbloqueado"),
                title.get(self.lang)
            ));
            self.progress.achievements.insert(id);
        }
    }

    fn next_exercise(&mut self) {
        let Screen::Exercise(state) = &self.screen else {
            return;
        };
        if !matches!(state.status, ExStatus::Passed { .. }) {
            return;
        }
        let (lesson, next) = (state.lesson, state.index + 1);
        if next < self.content.lessons[lesson].exercises.len() {
            self.start_exercise(lesson, next);
        } else {
            self.leave_console();
        }
    }

    fn on_console_key(&mut self, key: KeyEvent) {
        if self.confirm_reset {
            self.confirm_reset = false;
            if key.code == KeyCode::Char('y') {
                self.reset();
            }
            return;
        }
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        match key.code {
            KeyCode::Char('q') if ctrl => self.quit = true,
            KeyCode::Char('c') if ctrl => match &self.running {
                Some(running) => running.kill(),
                None => self.clear_input(),
            },
            KeyCode::Char('d') if ctrl => {
                if let Some(running) = &mut self.running {
                    running.close_stdin();
                }
            }
            KeyCode::Char('r') if ctrl => match &self.screen {
                Screen::Exercise(state) => self.start_exercise(state.lesson, state.index),
                _ => self.confirm_reset = true,
            },
            KeyCode::Esc => self.leave_console(),
            KeyCode::F(1) => {
                if let Screen::Exercise(state) = &mut self.screen {
                    let total = self.content.lessons[state.lesson].exercises[state.index]
                        .hints
                        .len();
                    state.hints_shown = (state.hints_shown + 1).min(total);
                }
            }
            KeyCode::F(5) => self.verify_exercise(),
            KeyCode::F(6) => self.next_exercise(),
            KeyCode::Char(c) if !ctrl && !key.modifiers.contains(KeyModifiers::ALT) => {
                self.insert(c)
            }
            KeyCode::Enter => self.submit(),
            KeyCode::Backspace => self.backspace(),
            KeyCode::Delete => self.delete(),
            KeyCode::Left => self.cursor = self.cursor.saturating_sub(1),
            KeyCode::Right => self.cursor = (self.cursor + 1).min(self.input.chars().count()),
            KeyCode::Home => self.cursor = 0,
            KeyCode::End => self.cursor = self.input.chars().count(),
            KeyCode::Up => self.history_prev(),
            KeyCode::Down => self.history_next(),
            KeyCode::PageUp => self.scroll = (self.scroll + 10).min(self.output.lines.len()),
            KeyCode::PageDown => self.scroll = self.scroll.saturating_sub(10),
            _ => {}
        }
    }

    fn on_editor_key(&mut self, key: KeyEvent) {
        let Some(editor) = &mut self.editor else {
            return;
        };
        let accepted = match editor.handle_key(key) {
            Ok(EditorAction::Continue) => return,
            Ok(EditorAction::Saved) => true,
            Ok(EditorAction::Cancelled) => false,
            Err(e) => {
                self.output.push_line(&format!("error: {e:#}"));
                return;
            }
        };
        if let Some(editor) = self.editor.take() {
            if editor.from_git {
                let _ = self.ipc.respond(accepted);
            } else if !accepted {
                self.output.push_line("edit: cancelled, nothing saved");
            }
        }
    }

    fn reset(&mut self) {
        if let Some(running) = self.running.take() {
            running.kill();
        }
        self.editor = None;
        match self.console.reset() {
            Ok(()) => {
                self.output.clear();
                self.output.push_line(tr(
                    self.lang,
                    "Free mode: a real git sandbox. Your own git config is never touched.",
                    "Modo libre: un sandbox de git real. Tu configuración de git nunca se toca.",
                ));
                self.output
                    .push_line("git, ls, cat, cd, pwd, mkdir, touch, rm, echo > / >>, edit, clear");
            }
            Err(e) => self.output.push_line(&format!("error: {e:#}")),
        }
        self.scroll = 0;
    }

    fn submit(&mut self) {
        let line = std::mem::take(&mut self.input);
        self.cursor = 0;
        self.history_pos = None;
        self.scroll = 0;

        if let Some(running) = &mut self.running {
            // Respuesta para un git en curso (por ejemplo `add -p`): se reenvía a su stdin.
            self.output.push_line(&line);
            if let Err(e) = running.send_line(&line) {
                self.output.push_line(&format!("error: {e:#}"));
            }
            return;
        }

        let echo = format!("{}{line}", self.prompt());
        self.output.push_line(&echo);
        if line.trim().is_empty() {
            return;
        }
        if self.history.last() != Some(&line) {
            self.history.push(line.clone());
        }
        match self.console.execute(&line, &self.tx) {
            Ok(Outcome::Done) => {}
            Ok(Outcome::Clear) => self.output.clear(),
            Ok(Outcome::Edit(path)) => match EditorState::open(path, false) {
                Ok(editor) => self.editor = Some(editor),
                Err(e) => self.output.push_line(&format!("error: {e:#}")),
            },
            Ok(Outcome::Running(running)) => self.running = Some(running),
            Err(e) => self.output.push_line(&format!("error: {e:#}")),
        }
    }

    // ---------- edición de la línea de entrada ----------

    fn byte_index(&self, chars: usize) -> usize {
        self.input
            .char_indices()
            .nth(chars)
            .map_or(self.input.len(), |(i, _)| i)
    }

    fn insert(&mut self, c: char) {
        let at = self.byte_index(self.cursor);
        self.input.insert(at, c);
        self.cursor += 1;
    }

    fn backspace(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
            let at = self.byte_index(self.cursor);
            self.input.remove(at);
        }
    }

    fn delete(&mut self) {
        if self.cursor < self.input.chars().count() {
            let at = self.byte_index(self.cursor);
            self.input.remove(at);
        }
    }

    fn clear_input(&mut self) {
        self.input.clear();
        self.cursor = 0;
        self.history_pos = None;
    }

    fn history_prev(&mut self) {
        if self.history.is_empty() || self.running.is_some() {
            return;
        }
        let pos = match self.history_pos {
            None => self.history.len() - 1,
            Some(p) => p.saturating_sub(1),
        };
        self.history_pos = Some(pos);
        self.input = self.history[pos].clone();
        self.cursor = self.input.chars().count();
    }

    fn history_next(&mut self) {
        let Some(pos) = self.history_pos else { return };
        if pos + 1 < self.history.len() {
            self.history_pos = Some(pos + 1);
            self.input = self.history[pos + 1].clone();
            self.cursor = self.input.chars().count();
        } else {
            self.clear_input();
        }
    }
}
