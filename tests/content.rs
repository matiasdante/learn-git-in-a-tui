use std::path::PathBuf;

use learn_git_tui::console::Console;
use learn_git_tui::content::Content;
use learn_git_tui::engine::{self, Verdict};
use learn_git_tui::requirements;
use learn_git_tui::sandbox::{Sandbox, remove_dir_all_force};

#[test]
fn every_text_exists_in_both_languages() {
    let content = Content::load().unwrap();
    let missing: Vec<String> = content
        .lessons
        .iter()
        .flat_map(|l| l.texts())
        .filter(|(_, text)| !text.is_complete())
        .map(|(name, _)| name)
        .collect();
    assert!(missing.is_empty(), "texts missing en/es: {missing:?}");
    for level in &content.levels {
        assert!(level.title.is_complete(), "level {} title", level.id);
    }
    for lesson in &content.lessons {
        assert!(
            content.level(lesson.level).is_some(),
            "lesson {} has an unknown level",
            lesson.id
        );
    }
}

/// Para cada ejercicio: el estado inicial sin tocar falla los checks y la solución de
/// referencia los pasa. Así no puede haber ejercicios rotos.
#[test]
fn every_exercise_fails_untouched_and_passes_with_its_solution() {
    let content = Content::load().unwrap();
    let root = std::env::temp_dir().join(format!("learn-git-tui-content-{}", std::process::id()));
    remove_dir_all_force(&root).unwrap();
    let exe = PathBuf::from(env!("CARGO_BIN_EXE_learn-git-tui"));
    let mut console = Console::new(Sandbox::new(root.clone(), exe).unwrap());

    let missing = requirements::missing_for_all(&content, console.sandbox());
    let mut count = 0;
    for (lesson, missing) in content.lessons.iter().zip(&missing) {
        if !missing.is_empty() {
            eprintln!("skipping lesson '{}': missing {missing:?}", lesson.id);
            continue;
        }
        for ex in &lesson.exercises {
            let ctx = format!("exercise '{}'", ex.id);

            engine::start(&mut console, ex).unwrap_or_else(|e| panic!("{ctx}: {e:#}"));
            assert!(
                matches!(engine::verify(&mut console, ex).unwrap(), Verdict::Fail(_)),
                "{ctx}: the untouched initial state must fail the checks"
            );

            engine::start(&mut console, ex).unwrap();
            for step in &ex.solution {
                let (code, output) =
                    engine::run_step(&mut console, step).unwrap_or_else(|e| panic!("{ctx}: {e:#}"));
                assert!(
                    code.is_none_or(|c| c == 0),
                    "{ctx}: {step:?} failed: {output}"
                );
            }
            if let Verdict::Fail(why) = engine::verify(&mut console, ex).unwrap() {
                panic!("{ctx}: the reference solution does not pass: {}", why.en);
            }
            count += 1;
        }
    }
    assert!(count > 0);
    let _ = remove_dir_all_force(&root);
}

#[test]
fn exam_has_two_questions_for_each_of_levels_1_to_5() {
    let content = Content::load().unwrap();
    for level in 1..=5u8 {
        let n = content.exam.iter().filter(|q| q.level == level).count();
        assert_eq!(n, 2, "exam level {level}");
    }
    assert_eq!(content.exam.len(), 10);
}

#[test]
fn the_course_covers_the_eleven_levels_of_the_plan() {
    let content = Content::load().unwrap();
    assert_eq!(content.levels.len(), 11);
    for level in 1..=11u8 {
        assert!(
            content.lessons.iter().any(|l| l.level == level),
            "level {level} has no lessons"
        );
    }
    assert!(
        content.lessons.len() > 100,
        "the plan promises 100+ lessons"
    );
}
