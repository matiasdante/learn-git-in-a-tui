//! Estado de la aplicación (modo libre): salida de la consola, línea de entrada, editor.

use std::sync::mpsc::{self, Receiver, Sender};

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::console::{Console, Event, Outcome, Running};
use crate::editor::{EditorAction, EditorState, IpcServer};
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

pub struct App {
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
    pub fn new(sandbox: Sandbox) -> Self {
        let (tx, rx) = mpsc::channel();
        let mut output = Output::new();
        output.push_line("Free mode: a real git sandbox. Your own git config is never touched.");
        output.push_line(
            "Commands: git, ls, cat, cd, pwd, mkdir, touch, rm, echo > / >>, edit, clear",
        );
        Self {
            ipc: IpcServer::new(sandbox.ipc_dir()),
            console: Console::new(sandbox),
            output,
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
        }
    }

    pub fn prompt(&self) -> String {
        if self.running.is_some() {
            "> ".to_string()
        } else {
            format!("{} $ ", self.console.display_cwd())
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
            KeyCode::Char('r') if ctrl => self.confirm_reset = true,
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
                self.output
                    .push_line("Sandbox reset: empty repo and default git config.");
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
