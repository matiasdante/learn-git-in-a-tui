//! Checks declarativos. Verifican el estado del repo con plumbing y formatos de máquina
//! (`rev-parse`, `status --porcelain=v2`, ...), nunca con salida pensada para humanos.

use std::fs;
use std::path::PathBuf;
use std::process::Stdio;

use serde::Deserialize;

use crate::i18n::Text;
use crate::sandbox::Sandbox;

#[derive(Debug, Clone, Deserialize)]
pub struct Check {
    #[serde(flatten)]
    pub kind: CheckKind,
    /// Qué explicarle al usuario si este check falla.
    pub fail: Text,
}

fn head() -> String {
    "HEAD".to_string()
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CheckKind {
    RepoExists,
    /// La rama actual (HEAD) se llama así.
    Branch {
        name: String,
    },
    /// Cantidad exacta de commits alcanzables desde HEAD.
    CommitCount {
        count: u32,
    },
    CommitSubject {
        #[serde(default = "head")]
        rev: String,
        contains: String,
    },
    FileExists {
        path: String,
    },
    FileMissing {
        path: String,
    },
    /// Contenido exacto (ignorando espacios en los bordes).
    FileContent {
        path: String,
        equals: String,
    },
    /// El archivo contiene un hash (abreviado o completo) del commit `rev`.
    FileMatchesRev {
        path: String,
        rev: String,
    },
    Tracked {
        path: String,
    },
    /// Entrada de `status --porcelain=v2` para `path`: `xy` es "A.", ".M", "??", "R."...
    /// Con `".."` se exige que `path` no figure (limpio).
    Status {
        path: String,
        xy: String,
    },
    Ignored {
        path: String,
    },
    NotIgnored {
        path: String,
    },
    Config {
        key: String,
        value: String,
    },
    /// Para temas sin estado que verificar (help, version): se mira lo que se tipeó.
    Ran {
        any: Vec<String>,
    },
}

pub struct Ctx<'a> {
    pub sandbox: &'a Sandbox,
    /// Líneas que el usuario ejecutó en la consola desde que empezó el ejercicio.
    pub history: &'a [String],
}

impl Ctx<'_> {
    fn work(&self) -> PathBuf {
        self.sandbox.work_dir()
    }

    fn git_ok(&self, args: &[&str]) -> Option<std::process::Output> {
        self.sandbox
            .git(&self.work())
            .args(args)
            .stdin(Stdio::null())
            .output()
            .ok()
    }

    /// Salida estándar (sin espacios en los bordes) si git terminó bien.
    fn git_out(&self, args: &[&str]) -> Option<String> {
        let out = self.git_ok(args)?;
        out.status
            .success()
            .then(|| String::from_utf8_lossy(&out.stdout).trim().to_string())
    }

    fn git_success(&self, args: &[&str]) -> bool {
        self.git_ok(args).is_some_and(|o| o.status.success())
    }

    fn read(&self, path: &str) -> Option<String> {
        fs::read_to_string(self.work().join(path)).ok()
    }
}

impl Check {
    pub fn passes(&self, ctx: &Ctx) -> bool {
        match &self.kind {
            CheckKind::RepoExists => ctx.git_out(&["rev-parse", "--git-dir"]).is_some(),
            CheckKind::Branch { name } => {
                ctx.git_out(&["symbolic-ref", "--short", "HEAD"]).as_deref() == Some(name)
            }
            CheckKind::CommitCount { count } => {
                let actual = ctx
                    .git_out(&["rev-list", "--count", "HEAD"])
                    .and_then(|s| s.parse::<u32>().ok())
                    .unwrap_or(0);
                actual == *count
            }
            CheckKind::CommitSubject { rev, contains } => ctx
                .git_out(&["log", "-1", "--format=%s", rev])
                .is_some_and(|s| s.contains(contains.as_str())),
            CheckKind::FileExists { path } => ctx.work().join(path).exists(),
            CheckKind::FileMissing { path } => !ctx.work().join(path).exists(),
            CheckKind::FileContent { path, equals } => {
                ctx.read(path).is_some_and(|c| c.trim() == equals.trim())
            }
            CheckKind::FileMatchesRev { path, rev } => {
                let Some(content) = ctx.read(path) else {
                    return false;
                };
                let wanted = content.trim().to_lowercase();
                let full = ctx.git_out(&["rev-parse", rev]);
                wanted.len() >= 4 && full.is_some_and(|f| f.starts_with(&wanted))
            }
            CheckKind::Tracked { path } => {
                ctx.git_success(&["ls-files", "--error-unmatch", "--", path])
            }
            CheckKind::Status { path, xy } => status_of(ctx, path) == *xy,
            CheckKind::Ignored { path } => ctx.git_success(&["check-ignore", "-q", "--", path]),
            CheckKind::NotIgnored { path } => ctx
                .git_ok(&["check-ignore", "-q", "--", path])
                .is_some_and(|o| o.status.code() == Some(1)),
            CheckKind::Config { key, value } => {
                ctx.git_out(&["config", "--get", key]).as_deref() == Some(value)
            }
            CheckKind::Ran { any } => ctx
                .history
                .iter()
                .any(|line| any.iter().any(|needle| line.contains(needle.as_str()))),
        }
    }
}

/// XY de `path` en `status --porcelain=v2` (`..` si no aparece).
fn status_of(ctx: &Ctx, path: &str) -> String {
    let Some(out) = ctx.git_out(&["status", "--porcelain=v2", "--untracked-files=all"]) else {
        return String::new();
    };
    for line in out.lines() {
        match line.split_once(' ') {
            Some(("?", p)) if p == path => return "??".to_string(),
            Some(("1", rest)) => {
                // 1 XY sub mH mI mW hH hI path
                let mut f = rest.splitn(8, ' ');
                let xy = f.next().unwrap_or("");
                if f.nth(6) == Some(path) {
                    return xy.to_string();
                }
            }
            Some(("2", rest)) => {
                // 2 XY sub mH mI mW hH hI Xscore path\torigPath
                let mut f = rest.splitn(9, ' ');
                let xy = f.next().unwrap_or("");
                let new_path = f.nth(7).and_then(|p| p.split('\t').next());
                if new_path == Some(path) {
                    return xy.to_string();
                }
            }
            _ => {}
        }
    }
    "..".to_string()
}
