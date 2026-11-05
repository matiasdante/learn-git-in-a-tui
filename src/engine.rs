//! Motor de ejercicios: prepara el estado inicial, toma el snapshot y verifica.

use std::path::{Component, Path, PathBuf};
use std::process::Stdio;

use anyhow::{Context, Result, bail};

use crate::checks::Ctx;
use crate::console::Console;
use crate::content::{Action, Exercise, SolutionStep};
use crate::i18n::Text;

pub enum Verdict {
    Pass,
    /// Explicación del primer check que falló.
    Fail(Text),
}

/// Deja el sandbox listo para el ejercicio y guarda el snapshot de ese estado.
pub fn start(console: &mut Console, exercise: &Exercise) -> Result<()> {
    console.reset()?;
    let work = console.sandbox().work_dir();
    for action in &exercise.setup {
        apply(console, &work, action)
            .with_context(|| format!("setup of exercise '{}' failed", exercise.id))?;
    }
    console.sandbox().snapshot()?;
    if !exercise.cwd.is_empty() {
        console.set_cwd(&exercise.cwd)?;
    }
    Ok(())
}

/// Corre los checks en orden. Si alguno falla, devuelve su explicación y
/// restaura el snapshot del inicio del ejercicio.
pub fn verify(console: &mut Console, exercise: &Exercise) -> Result<Verdict> {
    let work = console.sandbox().work_dir();
    let failed = exercise.checks.iter().find(|check| {
        let rel = check.repo.as_deref().unwrap_or(&exercise.cwd);
        let ctx = Ctx {
            sandbox: console.sandbox(),
            dir: safe_join(&work, rel).unwrap_or_else(|_| work.clone()),
            history: console.history(),
        };
        !check.passes(&ctx)
    });
    match failed {
        None => Ok(Verdict::Pass),
        Some(check) => {
            let explanation = check.fail.clone();
            console.restore_snapshot()?;
            Ok(Verdict::Fail(explanation))
        }
    }
}

fn apply(console: &Console, work: &Path, action: &Action) -> Result<()> {
    match action {
        Action::Write {
            path,
            content,
            executable,
        } => {
            let target = safe_join(work, path)?;
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(&target, content)?;
            if *executable {
                crate::console::set_executable(&target, true)?;
            }
        }
        Action::Mkdir { path } => std::fs::create_dir_all(safe_join(work, path)?)?,
        Action::Rm { path } => crate::sandbox::remove_dir_all_force(&safe_join(work, path)?)?,
        Action::Git {
            args,
            dir,
            allow_fail,
        } => {
            let args = shell_words::split(args)?;
            let cwd = safe_join(work, dir)?;
            std::fs::create_dir_all(&cwd)?;
            let out = console
                .sandbox()
                .git(&cwd)
                .args(&args)
                .stdin(Stdio::null())
                .output()?;
            if !out.status.success() && !allow_fail {
                bail!(
                    "git {} failed: {}",
                    args.join(" "),
                    String::from_utf8_lossy(&out.stderr).trim()
                );
            }
        }
        Action::Exec { program, args, dir } => {
            if program != "ssh-keygen" {
                bail!("setup cannot run '{program}'");
            }
            let cwd = safe_join(work, dir)?;
            std::fs::create_dir_all(&cwd)?;
            let out = std::process::Command::new(program)
                .args(shell_words::split(args)?)
                .current_dir(&cwd)
                .stdin(Stdio::null())
                .output()
                .with_context(|| format!("could not run {program}"))?;
            if !out.status.success() {
                bail!(
                    "{program} failed: {}",
                    String::from_utf8_lossy(&out.stderr).trim()
                );
            }
        }
    }
    Ok(())
}

/// Une una ruta relativa del contenido con el repo de práctica, sin dejar que se escape.
fn safe_join(base: &Path, rel: &str) -> Result<PathBuf> {
    let rel = Path::new(rel);
    if rel.components().any(|c| !matches!(c, Component::Normal(_))) {
        bail!("invalid path in lesson content: {}", rel.display());
    }
    Ok(base.join(rel))
}

/// Expande `{{args de git}}` en una línea de solución con la salida de ese git
/// (por ejemplo `{{rev-parse --short HEAD~1}}`), para soluciones que dependen de hashes.
pub fn expand_solution(console: &Console, line: &str) -> Result<String> {
    let mut out = String::new();
    let mut rest = line;
    while let Some(start) = rest.find("{{") {
        let end = rest[start..]
            .find("}}")
            .context("unclosed {{ in solution")?
            + start;
        out.push_str(&rest[..start]);
        let args = shell_words::split(&rest[start + 2..end])?;
        let result = console
            .sandbox()
            .git(console.cwd())
            .args(&args)
            .stdin(Stdio::null())
            .output()?;
        if !result.status.success() {
            bail!("solution placeholder failed: git {}", args.join(" "));
        }
        out.push_str(String::from_utf8_lossy(&result.stdout).trim());
        rest = &rest[end + 2..];
    }
    out.push_str(rest);
    Ok(out)
}

/// Ejecuta un paso de la solución de referencia, contestando el editor de git si hace falta.
pub fn run_step(console: &mut Console, step: &SolutionStep) -> Result<(Option<i32>, String)> {
    match step {
        SolutionStep::Line(line) => {
            let line = expand_solution(console, line)?;
            crate::console::run_blocking(console, &line)
        }
        SolutionStep::Full {
            run,
            stdin,
            editor,
            allow_fail,
        } => {
            let line = expand_solution(console, run)?;
            let stdin = match stdin {
                Some(text) => Some(expand_solution(console, text)?),
                None => None,
            };
            let mut handler = |path: &Path| -> bool {
                let mut content = std::fs::read_to_string(path).unwrap_or_default();
                for rule in editor {
                    content = rule.apply(&content);
                }
                std::fs::write(path, content).is_ok()
            };
            let (code, output) =
                crate::console::run_with(console, &line, stdin.as_deref(), &mut handler)?;
            Ok((if *allow_fail { Some(0) } else { code }, output))
        }
    }
}
