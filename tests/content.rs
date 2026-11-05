use std::path::PathBuf;

use learn_git_tui::console::{Console, run_blocking};
use learn_git_tui::content::Content;
use learn_git_tui::engine::{self, Verdict};
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

    let mut count = 0;
    for lesson in &content.lessons {
        for ex in &lesson.exercises {
            let ctx = format!("exercise '{}'", ex.id);

            engine::start(&mut console, ex).unwrap_or_else(|e| panic!("{ctx}: {e:#}"));
            assert!(
                matches!(engine::verify(&mut console, ex).unwrap(), Verdict::Fail(_)),
                "{ctx}: the untouched initial state must fail the checks"
            );

            engine::start(&mut console, ex).unwrap();
            for line in &ex.solution {
                let line = engine::expand_solution(&console, line)
                    .unwrap_or_else(|e| panic!("{ctx}: {e:#}"));
                let (code, output) =
                    run_blocking(&mut console, &line).unwrap_or_else(|e| panic!("{ctx}: {e:#}"));
                assert!(
                    code.is_none_or(|c| c == 0),
                    "{ctx}: `{line}` failed: {output}"
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
