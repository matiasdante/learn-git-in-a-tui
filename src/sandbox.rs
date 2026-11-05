//! Sandbox aislado: un directorio propio con su HOME, su gitconfig y su repo de trabajo.
//! Cada proceso de git se lanza con un entorno limpio para no tocar nunca la config real.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use anyhow::{Context, Result};

/// Fecha fija para que los hashes sean estables entre corridas.
const FIXED_DATE: &str = "2024-01-01 12:00:00 +0000";

const GITCONFIG: &str = "\
[user]
	name = Learner
	email = learner@example.com
[init]
	defaultBranch = main
[core]
	autocrlf = false
[color]
	ui = never
[commit]
	gpgsign = false
[tag]
	gpgsign = false
[protocol \"file\"]
	allow = always
";

/// Variables del entorno real que git necesita para funcionar (sobre todo en Windows).
const PASSTHROUGH_ENV: &[&str] = &[
    "PATH",
    "SYSTEMROOT",
    "SystemRoot",
    "COMSPEC",
    "PATHEXT",
    "TEMP",
    "TMP",
    "TMPDIR",
    "TERM",
];

#[derive(Clone, Debug)]
pub struct Sandbox {
    root: PathBuf,
    exe: PathBuf,
}

impl Sandbox {
    /// `<datos del usuario>/learn-git-tui/sandbox`
    pub fn default_root() -> Result<PathBuf> {
        let base = dirs::data_dir().context("could not find the user data directory")?;
        Ok(base.join("learn-git-tui").join("sandbox"))
    }

    /// `exe` es el binario que git invoca como editor (`__editor`).
    pub fn new(root: PathBuf, exe: PathBuf) -> Result<Self> {
        let sandbox = Self { root, exe };
        sandbox.prepare()?;
        Ok(sandbox)
    }

    pub fn git_available() -> bool {
        Command::new("git")
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok_and(|s| s.success())
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn work_dir(&self) -> PathBuf {
        self.root.join("work")
    }

    pub fn ipc_dir(&self) -> PathBuf {
        self.root.join("ipc")
    }

    fn home_dir(&self) -> PathBuf {
        self.root.join("home")
    }

    fn snapshot_dir(&self) -> PathBuf {
        self.root.join("snapshot")
    }

    /// Guarda una copia del repo de práctica para poder volver a este punto.
    pub fn snapshot(&self) -> Result<()> {
        remove_dir_all_force(&self.snapshot_dir())?;
        copy_dir_all(&self.work_dir(), &self.snapshot_dir())
    }

    /// Vuelve el repo de práctica al último snapshot.
    pub fn restore(&self) -> Result<()> {
        remove_dir_all_force(&self.work_dir())?;
        copy_dir_all(&self.snapshot_dir(), &self.work_dir())
    }

    pub fn gitconfig_path(&self) -> PathBuf {
        self.root.join("gitconfig")
    }

    /// Vuelve el sandbox al estado inicial: repo vacío y config por defecto.
    pub fn reset(&self) -> Result<()> {
        remove_dir_all_force(&self.work_dir())?;
        remove_dir_all_force(&self.home_dir())?;
        remove_dir_all_force(&self.snapshot_dir())?;
        self.prepare()
    }

    fn prepare(&self) -> Result<()> {
        for dir in [self.work_dir(), self.home_dir(), self.ipc_dir()] {
            fs::create_dir_all(&dir)
                .with_context(|| format!("could not create {}", dir.display()))?;
        }
        fs::write(self.gitconfig_path(), GITCONFIG)
            .context("could not write the sandbox gitconfig")?;
        // Señales del editor que pudieron quedar de una corrida anterior.
        for entry in fs::read_dir(self.ipc_dir())?.flatten() {
            let _ = fs::remove_file(entry.path());
        }
        Ok(())
    }

    /// Comando `git` con entorno aislado, parado en `cwd`.
    pub fn git(&self, cwd: &Path) -> Command {
        let home = self.home_dir();
        let mut cmd = Command::new("git");
        cmd.current_dir(cwd).env_clear();
        for key in PASSTHROUGH_ENV {
            if let Some(value) = env::var_os(key) {
                cmd.env(key, value);
            }
        }
        let editor = self.editor_command();
        cmd.env("HOME", &home)
            .env("USERPROFILE", &home)
            .env("XDG_CONFIG_HOME", home.join(".config"))
            .env("GIT_CONFIG_GLOBAL", self.gitconfig_path())
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CEILING_DIRECTORIES", &self.root)
            .env("GIT_PAGER", "cat")
            .env("PAGER", "cat")
            .env("GIT_TERMINAL_PROMPT", "0")
            .env("GIT_AUTHOR_DATE", FIXED_DATE)
            .env("GIT_COMMITTER_DATE", FIXED_DATE)
            .env("LC_ALL", "C")
            .env("GIT_EDITOR", &editor)
            .env("GIT_SEQUENCE_EDITOR", &editor)
            .env("LEARN_GIT_IPC", self.ipc_dir());
        cmd
    }

    /// Git interpreta el editor con sh: comillas simples y barras `/` (también en Windows).
    fn editor_command(&self) -> String {
        let mut exe = self.exe.to_string_lossy().into_owned();
        if cfg!(windows) {
            exe = exe.replace('\\', "/");
        }
        format!("'{}' __editor", exe.replace('\'', r"'\''"))
    }
}

/// Borra un directorio aunque tenga archivos de solo lectura (los objetos de git en Windows).
pub fn remove_dir_all_force(path: &Path) -> Result<()> {
    if !path.exists() {
        return Ok(());
    }
    if fs::remove_dir_all(path).is_ok() {
        return Ok(());
    }
    make_writable(path);
    fs::remove_dir_all(path).with_context(|| format!("could not remove {}", path.display()))
}

#[allow(clippy::permissions_set_readonly_false)]
fn make_writable(path: &Path) {
    let Ok(meta) = fs::symlink_metadata(path) else {
        return;
    };
    let mut perms = meta.permissions();
    perms.set_readonly(false);
    let _ = fs::set_permissions(path, perms);
    if meta.is_dir()
        && let Ok(entries) = fs::read_dir(path)
    {
        for entry in entries.flatten() {
            make_writable(&entry.path());
        }
    }
}

/// Copia recursiva; ignora enlaces simbólicos (el sandbox no los usa).
fn copy_dir_all(from: &Path, to: &Path) -> Result<()> {
    fs::create_dir_all(to)?;
    for entry in fs::read_dir(from)?.flatten() {
        let target = to.join(entry.file_name());
        let kind = entry.file_type()?;
        if kind.is_dir() {
            copy_dir_all(&entry.path(), &target)?;
        } else if kind.is_file() {
            fs::copy(entry.path(), &target)
                .with_context(|| format!("could not copy {}", entry.path().display()))?;
        }
    }
    Ok(())
}
