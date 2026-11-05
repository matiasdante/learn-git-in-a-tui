//! Motor de ejercicios: prepara el estado inicial, toma el snapshot y verifica.

use std::path::{Component, Path, PathBuf};
use std::process::Stdio;

use anyhow::{Context, Result, bail};

use crate::checks::Ctx;
use crate::console::Console;
use crate::content::{Action, Exercise};
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
    console.sandbox().snapshot()
}

/// Corre los checks en orden. Si alguno falla, devuelve su explicación y
/// restaura el snapshot del inicio del ejercicio.
pub fn verify(console: &mut Console, exercise: &Exercise) -> Result<Verdict> {
    let ctx = Ctx {
        sandbox: console.sandbox(),
        history: console.history(),
    };
    let failed = exercise.checks.iter().find(|check| !check.passes(&ctx));
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
        Action::Write { path, content } => {
            let target = safe_join(work, path)?;
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(&target, content)?;
        }
        Action::Mkdir { path } => std::fs::create_dir_all(safe_join(work, path)?)?,
        Action::Git { args } => {
            let args = shell_words::split(args)?;
            let out = console
                .sandbox()
                .git(work)
                .args(&args)
                .stdin(Stdio::null())
                .output()?;
            if !out.status.success() {
                bail!(
                    "git {} failed: {}",
                    args.join(" "),
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
            .git(&console.sandbox().work_dir())
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
