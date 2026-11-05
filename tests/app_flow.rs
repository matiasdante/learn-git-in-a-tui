use std::path::PathBuf;
use std::time::{Duration, Instant};

use learn_git_tui::app::{App, ExStatus, Screen};
use learn_git_tui::content::Content;
use learn_git_tui::i18n::Lang;
use learn_git_tui::progress::Progress;
use learn_git_tui::sandbox::{Sandbox, remove_dir_all_force};
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

struct Driver {
    app: App,
    root: PathBuf,
}

impl Driver {
    fn new(name: &str, progress: Progress) -> Self {
        let root =
            std::env::temp_dir().join(format!("learn-git-tui-flow-{}-{name}", std::process::id()));
        remove_dir_all_force(&root).unwrap();
        let exe = PathBuf::from(env!("CARGO_BIN_EXE_learn-git-tui"));
        let sandbox = Sandbox::new(root.clone(), exe).unwrap();
        Self {
            app: App::new(sandbox, Content::load().unwrap(), progress, None),
            root,
        }
    }

    fn press(&mut self, code: KeyCode) {
        self.app.on_key(key(code));
    }

    fn type_line(&mut self, text: &str) {
        for c in text.chars() {
            self.press(KeyCode::Char(c));
        }
        self.press(KeyCode::Enter);
        // Espera a que el comando termine.
        let deadline = Instant::now() + Duration::from_secs(20);
        loop {
            self.app.tick();
            if self.app.running.is_none() {
                return;
            }
            assert!(Instant::now() < deadline, "command did not finish");
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    /// Abre la lección `id` desde el mapa (nivel 1) y entra a su primer ejercicio.
    fn open_exercise(&mut self, lesson_id: &str) {
        self.app.screen = Screen::Map;
        self.app.map_selected = 0;
        self.press(KeyCode::Enter);
        let position = self
            .app
            .content
            .lessons_of(1)
            .position(|(_, l)| l.id == lesson_id)
            .unwrap();
        for _ in 0..position {
            self.press(KeyCode::Down);
        }
        self.press(KeyCode::Enter); // teoría
        self.press(KeyCode::Enter); // primer ejercicio
        assert!(matches!(self.app.screen, Screen::Exercise(_)));
    }
}

impl Drop for Driver {
    fn drop(&mut self) {
        let _ = remove_dir_all_force(&self.root);
    }
}

#[test]
fn first_run_goes_language_then_exam_offer_then_map() {
    let mut d = Driver::new("first-run", Progress::default());
    assert!(matches!(d.app.screen, Screen::LangSelect));
    d.press(KeyCode::Char('2'));
    assert_eq!(d.app.lang, Lang::Es);
    assert_eq!(d.app.progress.lang, Some(Lang::Es));
    assert!(matches!(d.app.screen, Screen::ExamOffer { .. }));
    d.press(KeyCode::Down); // omitir
    d.press(KeyCode::Enter);
    assert!(matches!(d.app.screen, Screen::Map));
    assert!(d.app.progress.exam_done);
    assert_eq!(d.app.progress.placement, None);
}

#[test]
fn perfect_exam_places_at_the_highest_available_level_and_a_mistake_places_lower() {
    let content = Content::load().unwrap();
    for mistake_at in [None, Some(0usize)] {
        let mut d = Driver::new("exam", Progress::default());
        d.press(KeyCode::Char('1'));
        d.press(KeyCode::Enter); // rendir el examen
        for (i, q) in content.exam.iter().enumerate() {
            let pick = if Some(i) == mistake_at {
                (q.answer + 1) % q.options.len()
            } else {
                q.answer
            };
            d.press(KeyCode::Char(
                char::from_digit(pick as u32 + 1, 10).unwrap(),
            ));
            d.press(KeyCode::Enter);
        }
        d.press(KeyCode::Enter); // resultado
        assert!(matches!(d.app.screen, Screen::Map));
        let expected = match mistake_at {
            None => content.max_level(), // 6 pedido, recortado al contenido disponible
            Some(i) => content.exam[i].level,
        };
        assert_eq!(d.app.progress.placement, Some(expected));
    }
}

#[test]
fn hints_and_failures_reduce_points_and_achievements_unlock() {
    let progress = Progress {
        lang: Some(Lang::En),
        exam_done: true,
        ..Progress::default()
    };
    let mut d = Driver::new("points", progress);
    assert!(matches!(d.app.screen, Screen::Map));

    d.open_exercise("init");
    d.press(KeyCode::F(1)); // una pista: -25
    d.press(KeyCode::F(5)); // sin hacer nada: falla, -10
    match &d.app.screen {
        Screen::Exercise(s) => assert!(matches!(s.status, ExStatus::Failed(_)) && s.failures == 1),
        _ => panic!("expected the exercise screen"),
    }
    d.type_line("git init");
    d.press(KeyCode::F(5));
    match &d.app.screen {
        Screen::Exercise(s) => assert!(
            matches!(s.status, ExStatus::Passed { points: 65 }),
            "100 - 25 - 10"
        ),
        _ => panic!("expected the exercise screen"),
    }
    assert_eq!(d.app.progress.total_points(), 65);
    assert!(d.app.progress.achievements.contains("first-repo"));
}

#[test]
fn a_failed_check_restores_the_exercise_snapshot() {
    let progress = Progress {
        lang: Some(Lang::En),
        exam_done: true,
        ..Progress::default()
    };
    let mut d = Driver::new("restore", progress);
    d.open_exercise("add"); // add-1: repo con notes.txt sin seguimiento
    d.type_line("echo extra > other.txt");
    d.press(KeyCode::F(5)); // no hizo git add: falla y restaura
    assert!(
        !d.root.join("work").join("other.txt").exists(),
        "snapshot restored"
    );
    assert!(d.root.join("work").join("notes.txt").exists());
}
