//! Requisitos de las lecciones: versión mínima de git y herramientas opcionales.
//! Si falta algo, la lección se muestra como explicación (sin ejercicios).

use std::process::{Command, Stdio};

use crate::content::{Content, Lesson};
use crate::sandbox::Sandbox;

/// Versión instalada de git como (mayor, menor).
pub fn git_version() -> Option<(u32, u32)> {
    let out = Command::new("git").arg("--version").output().ok()?;
    parse_version(&String::from_utf8_lossy(&out.stdout))
}

fn parse_version(text: &str) -> Option<(u32, u32)> {
    // "git version 2.43.0" o "git version 2.43.0.windows.1"
    let version = text.split_whitespace().nth(2)?;
    let mut parts = version.split('.');
    Some((parts.next()?.parse().ok()?, parts.next()?.parse().ok()?))
}

/// Qué le falta a cada lección (vacío = disponible), en el orden de `content.lessons`.
pub fn missing_for_all(content: &Content, sandbox: &Sandbox) -> Vec<Vec<String>> {
    let version = git_version();
    let subcommands = Command::new("git")
        .args(["--list-cmds=main,others"])
        .current_dir(sandbox.root())
        .stdin(Stdio::null())
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
        .unwrap_or_default();
    content
        .lessons
        .iter()
        .map(|lesson| missing(lesson, version, &subcommands))
        .collect()
}

fn missing(lesson: &Lesson, version: Option<(u32, u32)>, subcommands: &str) -> Vec<String> {
    let mut missing = Vec::new();
    if let Some(min) = &lesson.min_git
        && let Some(wanted) = parse_version(&format!("git version {min}"))
        && version.is_none_or(|v| v < wanted)
    {
        missing.push(format!("git {min}+"));
    }
    for requirement in &lesson.requires {
        let found = match requirement.split_once(':') {
            Some(("git", name)) => subcommands.lines().any(|l| l.trim() == name),
            Some(("bin", name)) => Command::new(name)
                .arg("-?")
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .is_ok(),
            _ => false,
        };
        if !found {
            missing.push(
                requirement
                    .split_once(':')
                    .map_or(requirement.as_str(), |(_, n)| n)
                    .to_string(),
            );
        }
    }
    missing
}

#[cfg(test)]
mod tests {
    use super::parse_version;

    #[test]
    fn parses_git_versions() {
        assert_eq!(parse_version("git version 2.43.0\n"), Some((2, 43)));
        assert_eq!(parse_version("git version 2.45.1.windows.1"), Some((2, 45)));
        assert_eq!(parse_version("nonsense"), None);
    }
}
