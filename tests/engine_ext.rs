mod common;

use std::fs;

use common::Harness;

#[test]
fn git_output_and_input_can_be_redirected() {
    let mut h = Harness::new("redirects");
    h.run("git init");
    h.run("git rev-parse --is-inside-work-tree > inside.txt");
    assert_eq!(h.run("cat inside.txt"), "true\n");
    h.run("git rev-parse --is-bare-repository >> inside.txt");
    assert_eq!(h.run("cat inside.txt"), "true\nfalse\n");

    h.run("echo hola > f.txt");
    let id = h.run("git hash-object --stdin < f.txt");
    assert_eq!(id.trim().len(), 40);
    // Las redirecciones tampoco pueden escapar del sandbox.
    assert!(h.try_run("git status > /../../x", None, None).is_err());
    assert!(
        h.try_run("git hash-object --stdin < /../../etc/passwd", None, None)
            .is_err()
    );
}

#[test]
fn chmod_makes_a_hook_runnable() {
    let mut h = Harness::new("hooks");
    h.run("git init");
    h.run("echo #!/bin/sh > .git/hooks/pre-commit");
    h.run("echo echo hook-ran >> .git/hooks/pre-commit");
    h.run("chmod +x .git/hooks/pre-commit");
    assert!(h.run("git hook run pre-commit").contains("hook-ran"));
    assert!(h.try_run("chmod 755 f", None, None).is_err());
}

#[test]
fn mergetool_opens_the_embedded_editor_on_the_conflicted_file() {
    let mut h = Harness::new("mergetool");
    h.run("git init");
    h.run("echo base > f.txt");
    h.run("git add f.txt");
    h.run("git commit -m base");
    h.run("git switch -c other");
    h.run("echo other > f.txt");
    h.run("git commit -am other");
    h.run("git switch main");
    h.run("echo mine > f.txt");
    h.run("git commit -am mine");
    let (code, _) = h.run_raw("git merge other", None, None).unwrap();
    assert_eq!(code, Some(1), "the merge must stop with a conflict");

    let mut opened = Vec::new();
    let mut handler = |path: &std::path::Path| {
        opened.push(path.file_name().unwrap().to_string_lossy().into_owned());
        fs::write(path, "resolved\n").unwrap();
        true
    };
    h.try_run("git mergetool", None, Some(&mut handler))
        .unwrap();
    assert_eq!(opened, vec!["f.txt".to_string()]);
    h.run("git commit -m merged");
    assert_eq!(h.run("cat f.txt"), "resolved\n");
}

#[test]
fn difftool_prints_a_unified_diff() {
    let mut h = Harness::new("difftool");
    h.run("git init");
    h.run("echo one > f.txt");
    h.run("git add f.txt");
    h.run("git commit -m one");
    h.run("echo two > f.txt");
    assert!(h.run("git difftool").contains("+two"));
}
