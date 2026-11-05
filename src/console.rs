//! Consola integrada: ejecuta `git` y unos pocos comandos propios, sin invocar una shell.

use std::fs::{self, OpenOptions};
use std::io::{ErrorKind, Read, Write};
use std::path::{Component, PathBuf};
use std::process::{ChildStdin, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::thread;
use std::time::Duration;

use anyhow::{Context, Result, anyhow, bail};

use crate::sandbox::{Sandbox, remove_dir_all_force};

/// Lo que los hilos de fondo le informan a la interfaz.
pub enum Event {
    Output(String),
    /// Código de salida (`None` si el proceso se mató o no tuvo código).
    Finished(Option<i32>),
}

pub enum Outcome {
    /// Terminó en el acto (comandos propios).
    Done,
    Clear,
    Edit(PathBuf),
    /// Hay un git corriendo en segundo plano.
    Running(Running),
}

/// Mango de un proceso git en ejecución.
pub struct Running {
    stdin: Option<ChildStdin>,
    kill: Arc<AtomicBool>,
}

impl Running {
    pub fn send_line(&mut self, line: &str) -> Result<()> {
        let stdin = self.stdin.as_mut().context("stdin is already closed")?;
        stdin.write_all(line.as_bytes())?;
        stdin.write_all(b"\n")?;
        stdin.flush()?;
        Ok(())
    }

    pub fn close_stdin(&mut self) {
        self.stdin = None;
    }

    pub fn kill(&self) {
        self.kill.store(true, Ordering::SeqCst);
    }
}

const OUTSIDE: &str = "path is outside the sandbox";

pub struct Console {
    sandbox: Sandbox,
    cwd: PathBuf,
    /// Líneas ejecutadas desde el último reset (para checks de tipo `ran`).
    history: Vec<String>,
}

impl Console {
    pub fn new(sandbox: Sandbox) -> Self {
        let cwd = sandbox.work_dir();
        Self {
            sandbox,
            cwd,
            history: Vec::new(),
        }
    }

    pub fn history(&self) -> &[String] {
        &self.history
    }

    /// Vuelve al snapshot del ejercicio y a la raíz del repo.
    pub fn restore_snapshot(&mut self) -> Result<()> {
        self.sandbox.restore()?;
        self.cwd = self.sandbox.work_dir();
        Ok(())
    }

    pub fn sandbox(&self) -> &Sandbox {
        &self.sandbox
    }

    pub fn reset(&mut self) -> Result<()> {
        self.sandbox.reset()?;
        self.cwd = self.sandbox.work_dir();
        self.history.clear();
        Ok(())
    }

    /// Directorio actual visto desde adentro del sandbox: `/` es la raíz del repo de práctica.
    pub fn display_cwd(&self) -> String {
        let rel = self
            .cwd
            .strip_prefix(self.sandbox.work_dir())
            .unwrap_or(&self.cwd);
        let parts: Vec<_> = rel
            .components()
            .map(|c| c.as_os_str().to_string_lossy())
            .collect();
        format!("/{}", parts.join("/"))
    }

    pub fn execute(&mut self, line: &str, tx: &Sender<Event>) -> Result<Outcome> {
        let tokens =
            shell_words::split(line).map_err(|e| anyhow!("cannot parse the command line: {e}"))?;
        let Some((cmd, args)) = tokens.split_first() else {
            return Ok(Outcome::Done);
        };
        self.history.push(line.trim().to_string());
        let text = match cmd.as_str() {
            "git" => return self.run_git(args, tx),
            "clear" => return Ok(Outcome::Clear),
            "edit" => return self.edit(args),
            "pwd" => format!("{}\n", self.display_cwd()),
            "cd" => self.cd(args)?,
            "ls" => self.ls(args)?,
            "cat" => self.cat(args)?,
            "mkdir" => self.mkdir(args)?,
            "touch" => self.touch(args)?,
            "rm" => self.rm(args)?,
            "echo" => self.echo(args)?,
            other => bail!(
                "unknown command '{other}'. Available: git, ls, cat, cd, pwd, mkdir, touch, rm, echo, edit, clear"
            ),
        };
        if !text.is_empty() {
            let _ = tx.send(Event::Output(text));
        }
        Ok(Outcome::Done)
    }

    // ---------- rutas ----------

    /// Resuelve una ruta del usuario. Una `/` inicial apunta a la raíz del sandbox.
    /// Falla si el resultado se sale del repo de práctica.
    fn resolve(&self, raw: &str) -> Result<PathBuf> {
        let root = self.sandbox.work_dir();
        let (mut out, rest) = match raw.strip_prefix('/') {
            Some(rest) => (root.clone(), rest),
            None => (self.cwd.clone(), raw),
        };
        for comp in std::path::Path::new(rest).components() {
            match comp {
                Component::CurDir => {}
                Component::ParentDir => {
                    if out == root {
                        bail!(OUTSIDE);
                    }
                    out.pop();
                }
                Component::Normal(name) => out.push(name),
                // Prefijos de unidad (C:\) o raíces absolutas del sistema.
                _ => bail!(OUTSIDE),
            }
        }
        // Un symlink que existe no puede sacarnos del sandbox.
        let mut probe = out.as_path();
        while !probe.exists() {
            match probe.parent() {
                Some(parent) => probe = parent,
                None => break,
            }
        }
        if let (Ok(real), Ok(real_root)) = (probe.canonicalize(), root.canonicalize())
            && !real.starts_with(real_root)
        {
            bail!(OUTSIDE);
        }
        Ok(out)
    }

    // ---------- comandos propios ----------

    fn cd(&mut self, args: &[String]) -> Result<String> {
        let target = match args {
            [] => self.sandbox.work_dir(),
            [dir] => self.resolve(dir)?,
            _ => bail!("usage: cd [dir]"),
        };
        if !target.is_dir() {
            bail!("cd: not a directory");
        }
        self.cwd = target;
        Ok(String::new())
    }

    fn ls(&self, args: &[String]) -> Result<String> {
        let mut all = false;
        let mut paths = Vec::new();
        for arg in args {
            match arg.as_str() {
                "-a" => all = true,
                opt if opt.starts_with('-') => bail!("ls: unsupported option '{opt}' (only -a)"),
                path => paths.push(path),
            }
        }
        let dir = match paths.as_slice() {
            [] => self.cwd.clone(),
            [path] => self.resolve(path)?,
            _ => bail!("usage: ls [-a] [path]"),
        };
        if dir.is_file() {
            return Ok(format!(
                "{}\n",
                dir.file_name().unwrap_or_default().to_string_lossy()
            ));
        }
        let mut names: Vec<String> = fs::read_dir(&dir)
            .with_context(|| "ls: cannot read directory".to_string())?
            .flatten()
            .filter_map(|entry| {
                let name = entry.file_name().to_string_lossy().into_owned();
                if !all && name.starts_with('.') {
                    return None;
                }
                let is_dir = entry.file_type().is_ok_and(|t| t.is_dir());
                Some(if is_dir { format!("{name}/") } else { name })
            })
            .collect();
        names.sort();
        Ok(names.iter().map(|n| format!("{n}\n")).collect())
    }

    fn cat(&self, args: &[String]) -> Result<String> {
        if args.is_empty() {
            bail!("usage: cat file...");
        }
        let mut out = String::new();
        for arg in args {
            let bytes = fs::read(self.resolve(arg)?).with_context(|| format!("cat: {arg}"))?;
            out.push_str(&String::from_utf8_lossy(&bytes));
        }
        Ok(out)
    }

    fn mkdir(&self, args: &[String]) -> Result<String> {
        let parents = args.iter().any(|a| a == "-p");
        let dirs: Vec<_> = args.iter().filter(|a| *a != "-p").collect();
        if dirs.is_empty() {
            bail!("usage: mkdir [-p] dir...");
        }
        for dir in dirs {
            let path = self.resolve(dir)?;
            let result = if parents {
                fs::create_dir_all(&path)
            } else {
                fs::create_dir(&path)
            };
            result.with_context(|| format!("mkdir: {dir}"))?;
        }
        Ok(String::new())
    }

    fn touch(&self, args: &[String]) -> Result<String> {
        if args.is_empty() {
            bail!("usage: touch file...");
        }
        for arg in args {
            OpenOptions::new()
                .create(true)
                .append(true)
                .open(self.resolve(arg)?)
                .with_context(|| format!("touch: {arg}"))?;
        }
        Ok(String::new())
    }

    fn rm(&mut self, args: &[String]) -> Result<String> {
        let (mut recursive, mut force) = (false, false);
        let mut targets = Vec::new();
        for arg in args {
            match arg.as_str() {
                opt if opt.starts_with('-') && opt.len() > 1 => {
                    for flag in opt[1..].chars() {
                        match flag {
                            'r' | 'R' => recursive = true,
                            'f' => force = true,
                            _ => bail!("rm: unsupported option '-{flag}' (only -r and -f)"),
                        }
                    }
                }
                path => targets.push(path),
            }
        }
        if targets.is_empty() {
            bail!("usage: rm [-r] [-f] path...");
        }
        let root = self.sandbox.work_dir();
        for target in targets {
            let path = self.resolve(target)?;
            if path == root {
                bail!("rm: cannot remove the sandbox root (use Ctrl+R to reset it)");
            }
            let Ok(meta) = fs::symlink_metadata(&path) else {
                if force {
                    continue;
                }
                bail!("rm: {target}: no such file or directory");
            };
            if meta.is_dir() {
                if !recursive {
                    bail!("rm: {target}: is a directory (use -r)");
                }
                remove_dir_all_force(&path)?;
            } else {
                let mut perms = meta.permissions();
                #[allow(clippy::permissions_set_readonly_false)]
                perms.set_readonly(false);
                let _ = fs::set_permissions(&path, perms);
                fs::remove_file(&path).with_context(|| format!("rm: {target}"))?;
            }
        }
        // Si borraron el directorio donde estamos, subimos al más cercano que exista.
        while !self.cwd.exists() && self.cwd != root {
            self.cwd.pop();
        }
        Ok(String::new())
    }

    fn echo(&self, args: &[String]) -> Result<String> {
        let Some(pos) = args.iter().position(|a| a == ">" || a == ">>") else {
            return Ok(format!("{}\n", args.join(" ")));
        };
        let [target] = &args[pos + 1..] else {
            bail!("usage: echo text > file   (or >> to append)");
        };
        let path = self.resolve(target)?;
        let mut options = OpenOptions::new();
        options.create(true);
        if args[pos] == ">>" {
            options.append(true);
        } else {
            options.write(true).truncate(true);
        }
        let mut file = options
            .open(path)
            .with_context(|| format!("echo: {target}"))?;
        writeln!(file, "{}", args[..pos].join(" "))?;
        Ok(String::new())
    }

    fn edit(&self, args: &[String]) -> Result<Outcome> {
        let [file] = args else {
            bail!("usage: edit file");
        };
        let path = self.resolve(file)?;
        if path.is_dir() {
            bail!("edit: {file} is a directory");
        }
        if !path.parent().is_some_and(|p| p.is_dir()) {
            bail!("edit: the directory does not exist");
        }
        Ok(Outcome::Edit(path))
    }

    // ---------- git ----------

    fn run_git(&self, args: &[String], tx: &Sender<Event>) -> Result<Outcome> {
        let args = self.guard_git_args(args)?;
        let mut child = self
            .sandbox
            .git(&self.cwd)
            .args(&args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| match e.kind() {
                ErrorKind::NotFound => anyhow!("git was not found in PATH"),
                _ => anyhow!("could not start git: {e}"),
            })?;
        let stdin = child.stdin.take();
        let stdout = child.stdout.take().context("git has no stdout")?;
        let stderr = child.stderr.take().context("git has no stderr")?;
        let kill = Arc::new(AtomicBool::new(false));

        let tx = tx.clone();
        let kill_flag = Arc::clone(&kill);
        thread::spawn(move || {
            let readers = [
                spawn_pump(Box::new(stdout), tx.clone()),
                spawn_pump(Box::new(stderr), tx.clone()),
            ];
            let mut killed = false;
            let code = loop {
                match child.try_wait() {
                    Ok(Some(status)) => break status.code(),
                    Ok(None) => {}
                    Err(_) => break None,
                }
                if kill_flag.load(Ordering::SeqCst) {
                    let _ = child.kill();
                    let _ = child.wait();
                    killed = true;
                    break None;
                }
                thread::sleep(Duration::from_millis(20));
            };
            // Damos un margen para vaciar los pipes; si un nieto los retiene no nos colgamos.
            let patience = if killed { 5 } else { 100 };
            for _ in 0..patience {
                if readers.iter().all(|r| r.is_finished()) {
                    break;
                }
                thread::sleep(Duration::from_millis(10));
            }
            let _ = tx.send(Event::Finished(code));
        });

        Ok(Outcome::Running(Running { stdin, kill }))
    }

    /// Bloquea que `git` salga del sandbox con `-C`, `--git-dir`, `--work-tree` o
    /// `--separate-git-dir`, y reescribe esas rutas a rutas reales ya validadas.
    fn guard_git_args(&self, args: &[String]) -> Result<Vec<String>> {
        let real = |value: &str| -> Result<String> {
            Ok(self.resolve(value)?.to_string_lossy().into_owned())
        };
        let mut out = Vec::with_capacity(args.len());
        let mut i = 0;
        // Opciones globales: van antes del subcomando (`git commit -C <commit>` no es una ruta).
        while i < args.len() && args[i].starts_with('-') {
            let arg = &args[i];
            if matches!(arg.as_str(), "-C" | "--git-dir" | "--work-tree") {
                let value = args
                    .get(i + 1)
                    .ok_or_else(|| anyhow!("git: option {arg} requires a value"))?;
                out.push(arg.clone());
                out.push(real(value)?);
                i += 2;
            } else if arg == "-c" {
                out.extend(args[i..args.len().min(i + 2)].iter().cloned());
                i += 2;
            } else if let Some((key, value)) = arg.split_once('=')
                && matches!(key, "--git-dir" | "--work-tree")
            {
                out.push(format!("{key}={}", real(value)?));
                i += 1;
            } else {
                out.push(arg.clone());
                i += 1;
            }
        }
        while i < args.len() {
            let arg = &args[i];
            if arg == "--separate-git-dir" {
                let value = args
                    .get(i + 1)
                    .ok_or_else(|| anyhow!("git: option {arg} requires a value"))?;
                out.push(arg.clone());
                out.push(real(value)?);
                i += 2;
            } else if let Some(value) = arg.strip_prefix("--separate-git-dir=") {
                out.push(format!("--separate-git-dir={}", real(value)?));
                i += 1;
            } else {
                out.push(arg.clone());
                i += 1;
            }
        }
        Ok(out)
    }
}

fn spawn_pump(mut reader: Box<dyn Read + Send>, tx: Sender<Event>) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        let mut buf = [0u8; 4096];
        // Bytes de un carácter UTF-8 cortado entre dos lecturas.
        let mut pending: Vec<u8> = Vec::new();
        while let Ok(n) = reader.read(&mut buf) {
            if n == 0 {
                break;
            }
            pending.extend_from_slice(&buf[..n]);
            let text = match std::str::from_utf8(&pending) {
                Ok(valid) => {
                    let text = valid.to_string();
                    pending.clear();
                    text
                }
                Err(e) if e.error_len().is_none() => {
                    let text = String::from_utf8_lossy(&pending[..e.valid_up_to()]).into_owned();
                    pending.drain(..e.valid_up_to());
                    text
                }
                Err(_) => {
                    let text = String::from_utf8_lossy(&pending).into_owned();
                    pending.clear();
                    text
                }
            };
            if !text.is_empty() && tx.send(Event::Output(text)).is_err() {
                break;
            }
        }
    })
}

/// Ejecuta una línea y espera a que termine (sin editor ni stdin). Lo usan el motor de
/// lecciones y los tests de contenido. Devuelve el código de salida y la salida combinada.
pub fn run_blocking(console: &mut Console, line: &str) -> Result<(Option<i32>, String)> {
    let (tx, rx) = std::sync::mpsc::channel();
    let mut running = match console.execute(line, &tx)? {
        Outcome::Running(running) => running,
        Outcome::Edit(_) => bail!("`{line}` needs the editor, which is not available here"),
        Outcome::Done | Outcome::Clear => {
            let output = rx
                .try_iter()
                .filter_map(|e| {
                    if let Event::Output(t) = e {
                        Some(t)
                    } else {
                        None
                    }
                })
                .collect();
            return Ok((None, output));
        }
    };
    running.close_stdin();
    let deadline = std::time::Instant::now() + Duration::from_secs(30);
    let mut output = String::new();
    loop {
        if std::time::Instant::now() > deadline {
            running.kill();
            bail!("`{line}` timed out (does it wait for an editor or for input?)");
        }
        match rx.recv_timeout(Duration::from_millis(50)) {
            Ok(Event::Output(text)) => output.push_str(&text),
            Ok(Event::Finished(code)) => return Ok((code, output)),
            Err(_) => {}
        }
    }
}
