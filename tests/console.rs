mod common;

use std::fs;

use common::Harness;

#[test]
fn global_git_config_stays_inside_the_sandbox() {
    let real = dirs::home_dir().map(|h| h.join(".gitconfig"));
    let before = real.as_ref().map(|p| fs::read(p).ok());

    let mut h = Harness::new("isolation");
    h.run("git config --global user.name Intruder");

    let sandbox_config = fs::read_to_string(h.console.sandbox().gitconfig_path()).unwrap();
    assert!(sandbox_config.contains("Intruder"));
    let after = real.as_ref().map(|p| fs::read(p).ok());
    assert_eq!(before, after, "the real ~/.gitconfig must not change");
}

#[test]
fn builtins_work_and_stay_confined() {
    let mut h = Harness::new("builtins");
    h.run("mkdir -p a/b");
    h.run("echo hola > a/f.txt");
    h.run("echo chau >> a/f.txt");
    assert_eq!(h.run("cat a/f.txt"), "hola\nchau\n");
    h.run("touch a/b/x");
    assert_eq!(h.run("ls a"), "b/\nf.txt\n");
    h.run("cd a/b");
    assert_eq!(h.run("pwd"), "/a/b\n");

    // No se puede salir del sandbox ni con `..` ni con rutas absolutas del sistema.
    assert!(h.try_run("cd ../../..", None, None).is_err());
    assert!(h.try_run("cat /../../etc/passwd", None, None).is_err());
    assert!(
        h.try_run("cat /etc/passwd", None, None).is_err(),
        "'/' is the sandbox root, not the system root"
    );

    h.run("rm -rf /a");
    assert_eq!(h.run("pwd"), "/\n", "removing the current dir moves us up");
    assert!(h.try_run("rm -r /", None, None).is_err());
    assert!(h.try_run("ls -l", None, None).is_err());
    assert!(h.try_run("curl example.com", None, None).is_err());
}

#[test]
fn git_cannot_escape_the_sandbox() {
    let mut h = Harness::new("guard");
    assert!(h.try_run("git -C /../.. status", None, None).is_err());
    assert!(
        h.try_run("git --git-dir=../../x status", None, None)
            .is_err()
    );
    assert!(
        h.try_run("git --work-tree ../../x status", None, None)
            .is_err()
    );
    assert!(
        h.try_run("git init --separate-git-dir=../../x", None, None)
            .is_err()
    );

    // `-C` dentro del sandbox funciona, y `commit -C` no es una ruta.
    h.run("mkdir sub");
    h.run("git -C /sub init");
    assert!(h.run("git -C sub status").contains("On branch main"));
}

#[test]
fn reset_wipes_the_repo_and_restores_the_config() {
    let mut h = Harness::new("reset");
    h.run("git init");
    h.run("echo x > f");
    h.run("git config --global user.name Changed");
    h.console.reset().unwrap();
    assert_eq!(h.run("ls -a"), "");
    let config = fs::read_to_string(h.console.sandbox().gitconfig_path()).unwrap();
    assert!(config.contains("Learner") && !config.contains("Changed"));
}

#[test]
fn add_patch_reads_answers_from_a_pipe() {
    let mut h = Harness::new("addp");
    h.run("git init");
    h.run("echo one > f.txt");
    h.run("git add f.txt");
    h.run("git commit -m init");
    h.run("echo two >> f.txt");
    h.try_run("git add -p", Some("y"), None).unwrap();
    assert_eq!(h.run("git diff --cached --name-only"), "f.txt\n");
}

#[test]
fn hashes_are_stable_thanks_to_fixed_dates() {
    let mut h = Harness::new("hashes");
    h.run("git init");
    h.run("echo hola > a.txt");
    h.run("git add a.txt");
    h.run("git commit -m first");
    let first = h.run("git rev-parse HEAD");

    let mut h2 = Harness::new("hashes2");
    h2.run("git init");
    h2.run("echo hola > a.txt");
    h2.run("git add a.txt");
    h2.run("git commit -m first");
    assert_eq!(first, h2.run("git rev-parse HEAD"));
}
