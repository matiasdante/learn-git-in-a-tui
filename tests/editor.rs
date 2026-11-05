mod common;

use std::fs;

use common::Harness;

#[test]
fn commit_without_m_uses_the_embedded_editor() {
    let mut h = Harness::new("editor-commit");
    h.run("git init");
    h.run("echo hola > a.txt");
    h.run("git add a.txt");

    let mut opened = 0;
    let mut handler = |path: &std::path::Path| {
        opened += 1;
        assert!(
            path.ends_with("COMMIT_EDITMSG"),
            "unexpected file: {}",
            path.display()
        );
        fs::write(path, "feat: primer commit\n").unwrap();
        true
    };
    h.try_run("git commit", None, Some(&mut handler)).unwrap();
    assert_eq!(opened, 1);
    assert_eq!(h.run("git log --format=%s"), "feat: primer commit\n");
}

#[test]
fn cancelling_the_editor_aborts_the_commit() {
    let mut h = Harness::new("editor-abort");
    h.run("git init");
    h.run("echo hola > a.txt");
    h.run("git add a.txt");

    let mut handler = |_: &std::path::Path| false;
    let (code, _) = h.run_raw("git commit", None, Some(&mut handler)).unwrap();
    assert_ne!(
        code,
        Some(0),
        "git commit should fail when the editor is cancelled"
    );
    let (head, _) = h
        .run_raw("git rev-parse --verify HEAD", None, None)
        .unwrap();
    assert_ne!(head, Some(0), "no commit should have been created");
}

#[test]
fn rebase_interactive_goes_through_the_sequence_editor() {
    let mut h = Harness::new("editor-rebase");
    h.run("git init");
    for n in ["c1", "c2", "c3"] {
        h.run(&format!("echo {n} > {n}.txt"));
        h.run("git add .");
        h.run(&format!("git commit -m {n}"));
    }

    // Se descarta el commit c2 editando el todo de rebase.
    let mut handler = |path: &std::path::Path| {
        assert!(
            path.ends_with("git-rebase-todo"),
            "unexpected file: {}",
            path.display()
        );
        let todo = fs::read_to_string(path).unwrap();
        let edited: String = todo
            .lines()
            .map(|l| {
                if l.starts_with("pick ") && l.ends_with(" c2") {
                    l.replacen("pick", "drop", 1)
                } else {
                    l.to_string()
                }
            })
            .collect::<Vec<_>>()
            .join("\n");
        fs::write(path, edited + "\n").unwrap();
        true
    };
    h.try_run("git rebase -i HEAD~2", None, Some(&mut handler))
        .unwrap();
    assert_eq!(h.run("git log --format=%s"), "c3\nc1\n");
}
