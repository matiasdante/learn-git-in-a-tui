//! Lecciones en TOML embebidas en el binario.

use std::collections::HashSet;

use anyhow::{Context, Result, bail};
use include_dir::{Dir, include_dir};
use serde::Deserialize;

use crate::checks::Check;
use crate::i18n::Text;
use crate::progress::Progress;

static CONTENT_DIR: Dir = include_dir!("$CARGO_MANIFEST_DIR/content");

#[derive(Debug, Deserialize)]
pub struct Level {
    pub id: u8,
    pub title: Text,
}

#[derive(Debug, Deserialize)]
struct LevelsFile {
    level: Vec<Level>,
}

#[derive(Debug, Deserialize)]
pub struct Achievement {
    pub id: String,
    pub title: Text,
    pub description: Text,
    pub condition: Condition,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Condition {
    ExerciseDone {
        exercise: String,
    },
    LevelDone {
        level: u8,
    },
    /// Todos los ejercicios del nivel hechos sin usar ninguna pista.
    LevelNoHints {
        level: u8,
    },
}

#[derive(Debug, Deserialize)]
struct AchievementsFile {
    achievement: Vec<Achievement>,
}

#[derive(Debug, Deserialize)]
pub struct Question {
    pub id: String,
    pub level: u8,
    pub text: Text,
    pub options: Vec<Text>,
    /// Índice (desde 0) de la opción correcta.
    pub answer: usize,
}

#[derive(Debug, Deserialize)]
struct ExamFile {
    question: Vec<Question>,
}

#[derive(Debug, Deserialize)]
pub struct Lesson {
    pub id: String,
    pub level: u8,
    pub order: u32,
    /// Versión mínima de git (`"2.38"`); si no alcanza, la lección es solo explicación.
    pub min_git: Option<String>,
    /// Herramientas que la lección necesita: `"git:subtree"` (subcomando de git) o
    /// `"bin:ssh-keygen"` (programa en el PATH). Si falta alguna, pasa a ser explicación.
    #[serde(default)]
    pub requires: Vec<String>,
    pub title: Text,
    pub theory: Text,
    #[serde(default, rename = "exercise")]
    pub exercises: Vec<Exercise>,
}

#[derive(Debug, Deserialize)]
pub struct Exercise {
    pub id: String,
    pub prompt: Text,
    #[serde(default)]
    pub setup: Vec<Action>,
    #[serde(default)]
    pub hints: Vec<Text>,
    pub checks: Vec<Check>,
    /// Carpeta (relativa a la raíz del sandbox) donde arranca la consola y donde se verifican
    /// los checks. Vacía = la raíz.
    #[serde(default)]
    pub cwd: String,
    /// Pasos que resuelven el ejercicio (los usa el test de contenido).
    pub solution: Vec<SolutionStep>,
}

/// Un paso de la solución de referencia: una línea de consola, o una línea con stdin
/// y/o respuestas para el editor de git.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum SolutionStep {
    Line(String),
    Full {
        run: String,
        #[serde(default)]
        stdin: Option<String>,
        #[serde(default)]
        editor: Vec<EditRule>,
        /// El paso termina con error a propósito (por ejemplo un merge con conflicto).
        #[serde(default)]
        allow_fail: bool,
    },
}

/// Qué hace la solución cuando git abre el editor sobre un archivo.
#[derive(Debug, Deserialize)]
pub struct EditRule {
    /// Si está, la regla solo se aplica al archivo con este nombre (`git-rebase-todo`...).
    pub file: Option<String>,
    /// Reemplaza todo el contenido del archivo.
    pub write: Option<String>,
    /// En las líneas que contienen este texto, cambia `from` por `to`.
    pub line_containing: Option<String>,
    pub from: Option<String>,
    pub to: Option<String>,
}

impl EditRule {
    pub fn apply(&self, file_name: &str, content: &str) -> String {
        if self.file.as_ref().is_some_and(|f| f != file_name) {
            return content.to_string();
        }
        if let Some(text) = &self.write {
            return text.clone();
        }
        let (Some(needle), Some(from), Some(to)) = (&self.line_containing, &self.from, &self.to)
        else {
            return content.to_string();
        };
        let mut out: Vec<String> = content
            .lines()
            .map(|line| {
                if line.contains(needle.as_str()) {
                    line.replacen(from.as_str(), to, 1)
                } else {
                    line.to_string()
                }
            })
            .collect();
        out.push(String::new());
        out.join("\n")
    }
}

/// Preparación del estado inicial: acciones del motor, nunca una shell.
#[derive(Debug, Deserialize)]
#[serde(tag = "do", rename_all = "snake_case")]
pub enum Action {
    Write {
        path: String,
        content: String,
        #[serde(default)]
        executable: bool,
    },
    Mkdir {
        path: String,
    },
    /// Borra un archivo o carpeta (por ejemplo un clon auxiliar usado para sembrar un remoto).
    Rm {
        path: String,
    },
    /// `dir` es relativo a la raíz del sandbox. Con `allow_fail` el setup sigue aunque git
    /// termine con error (por ejemplo un merge que deja un conflicto a propósito).
    Git {
        args: String,
        #[serde(default)]
        dir: String,
        #[serde(default)]
        allow_fail: bool,
    },
    /// Programas auxiliares permitidos (solo `ssh-keygen`).
    Exec {
        program: String,
        args: String,
        #[serde(default)]
        dir: String,
    },
}

impl Lesson {
    /// Todos los textos de la lección, para verificar que existan en los dos idiomas.
    pub fn texts(&self) -> Vec<(String, &Text)> {
        let mut out = vec![
            (format!("{}.title", self.id), &self.title),
            (format!("{}.theory", self.id), &self.theory),
        ];
        for ex in &self.exercises {
            out.push((format!("{}.prompt", ex.id), &ex.prompt));
            for (i, hint) in ex.hints.iter().enumerate() {
                out.push((format!("{}.hint{}", ex.id, i + 1), hint));
            }
            for (i, check) in ex.checks.iter().enumerate() {
                out.push((format!("{}.check{}.fail", ex.id, i + 1), &check.fail));
            }
        }
        out
    }
}

pub struct Content {
    pub levels: Vec<Level>,
    pub lessons: Vec<Lesson>,
    pub achievements: Vec<Achievement>,
    pub exam: Vec<Question>,
}

impl Content {
    pub fn load() -> Result<Self> {
        let levels_file = CONTENT_DIR
            .get_file("levels.toml")
            .context("content/levels.toml is missing")?;
        let levels: LevelsFile = toml::from_str(
            levels_file
                .contents_utf8()
                .context("levels.toml is not UTF-8")?,
        )
        .context("invalid levels.toml")?;

        let mut lessons = Vec::new();
        let mut files = Vec::new();
        collect_toml(&CONTENT_DIR, &mut files);
        for file in files {
            let path = file.path();
            if ["levels.toml", "achievements.toml", "exam.toml"]
                .iter()
                .any(|name| path.ends_with(name))
            {
                continue;
            }
            let text = file
                .contents_utf8()
                .with_context(|| format!("{} is not UTF-8", path.display()))?;
            let lesson: Lesson = toml::from_str(text)
                .with_context(|| format!("invalid lesson {}", path.display()))?;
            lessons.push(lesson);
        }
        lessons.sort_by_key(|l| (l.level, l.order));

        let mut seen = HashSet::new();
        for lesson in &lessons {
            if !seen.insert(lesson.id.clone()) {
                bail!("duplicate lesson id '{}'", lesson.id);
            }
            for ex in &lesson.exercises {
                if !seen.insert(ex.id.clone()) {
                    bail!("duplicate exercise id '{}'", ex.id);
                }
            }
        }
        let achievements: AchievementsFile = read_toml("achievements.toml")?;
        let exam: ExamFile = read_toml("exam.toml")?;
        let content = Self {
            levels: levels.level,
            lessons,
            achievements: achievements.achievement,
            exam: exam.question,
        };
        content.validate_references()?;
        Ok(content)
    }

    /// Los logros y el examen no pueden apuntar a niveles o ejercicios que no existen.
    fn validate_references(&self) -> Result<()> {
        for a in &self.achievements {
            match &a.condition {
                Condition::ExerciseDone { exercise } => {
                    if self.exercise(exercise).is_none() {
                        bail!(
                            "achievement '{}' refers to unknown exercise '{exercise}'",
                            a.id
                        );
                    }
                }
                Condition::LevelDone { level } | Condition::LevelNoHints { level } => {
                    if self.level(*level).is_none() {
                        bail!("achievement '{}' refers to unknown level {level}", a.id);
                    }
                }
            }
        }
        for q in &self.exam {
            if q.options.len() < 2 || q.answer >= q.options.len() {
                bail!("question '{}' has invalid options or answer", q.id);
            }
        }
        Ok(())
    }

    pub fn exercise(&self, id: &str) -> Option<&Exercise> {
        self.lessons
            .iter()
            .flat_map(|l| &l.exercises)
            .find(|e| e.id == id)
    }

    pub fn lessons_of(&self, level: u8) -> impl Iterator<Item = (usize, &Lesson)> {
        self.lessons
            .iter()
            .enumerate()
            .filter(move |(_, l)| l.level == level)
    }

    pub fn max_level(&self) -> u8 {
        self.levels.iter().map(|l| l.id).max().unwrap_or(1)
    }

    /// Logros cuya condición se cumple con este progreso y que todavía no se otorgaron.
    pub fn newly_unlocked(&self, progress: &Progress) -> Vec<&Achievement> {
        self.achievements
            .iter()
            .filter(|a| !progress.achievements.contains(&a.id))
            .filter(|a| match &a.condition {
                Condition::ExerciseDone { exercise } => progress.is_done(exercise),
                Condition::LevelDone { level } => self
                    .level_exercises(*level)
                    .all(|e| progress.is_done(&e.id)),
                Condition::LevelNoHints { level } => self
                    .level_exercises(*level)
                    .all(|e| progress.exercises.get(&e.id).is_some_and(|r| r.hints == 0)),
            })
            .collect()
    }

    fn level_exercises(&self, level: u8) -> impl Iterator<Item = &Exercise> {
        self.lessons
            .iter()
            .filter(move |l| l.level == level)
            .flat_map(|l| &l.exercises)
    }

    /// (hechos, total) de ejercicios de un nivel.
    pub fn level_progress(&self, level: u8, progress: &Progress) -> (usize, usize) {
        let total = self.level_exercises(level).count();
        let done = self
            .level_exercises(level)
            .filter(|e| progress.is_done(&e.id))
            .count();
        (done, total)
    }

    pub fn level(&self, id: u8) -> Option<&Level> {
        self.levels.iter().find(|l| l.id == id)
    }
}

fn collect_toml<'a>(dir: &'a Dir<'static>, out: &mut Vec<&'a include_dir::File<'static>>) {
    for file in dir.files() {
        if file.path().extension().is_some_and(|e| e == "toml") {
            out.push(file);
        }
    }
    for sub in dir.dirs() {
        collect_toml(sub, out);
    }
}

fn read_toml<T: serde::de::DeserializeOwned>(name: &str) -> Result<T> {
    let file = CONTENT_DIR
        .get_file(name)
        .with_context(|| format!("content/{name} is missing"))?;
    toml::from_str(file.contents_utf8().context("not UTF-8")?)
        .with_context(|| format!("invalid {name}"))
}
