//! Editor integrado. Git lo abre vía `GIT_EDITOR` / `GIT_SEQUENCE_EDITOR`, que apuntan a
//! este mismo binario con el subcomando oculto `__editor <archivo>`.
//!
//! Protocolo por archivos en el directorio IPC del sandbox:
//!   cliente (proceso del editor)  escribe `request` con la ruta absoluta y espera `response`;
//!   servidor (la TUI)             consume `request`, muestra el editor y escribe `response`
//!                                 con `ok` (guardado) o `abort` (cancelado).

use std::fs;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::widgets::Block;
use tui_textarea::TextArea;

const REQUEST: &str = "request";
const RESPONSE: &str = "response";
/// Si la TUI desaparece, el proceso del editor no queda colgado para siempre.
const CLIENT_TIMEOUT: Duration = Duration::from_secs(10 * 60);

/// Lado de la TUI.
pub struct IpcServer {
    dir: PathBuf,
}

impl IpcServer {
    pub fn new(dir: PathBuf) -> Self {
        Self { dir }
    }

    /// Si git pidió abrir un archivo, devuelve su ruta (y consume el pedido).
    pub fn poll_request(&self) -> Option<PathBuf> {
        let request = self.dir.join(REQUEST);
        let content = fs::read_to_string(&request).ok()?;
        fs::remove_file(&request).ok()?;
        Some(PathBuf::from(content.trim_end_matches(['\r', '\n'])))
    }

    /// Libera al proceso del editor: `true` = git sigue, `false` = git aborta.
    pub fn respond(&self, accepted: bool) -> Result<()> {
        let tmp = self.dir.join("response.tmp");
        fs::write(&tmp, if accepted { "ok" } else { "abort" })?;
        fs::rename(&tmp, self.dir.join(RESPONSE)).context("could not answer the editor request")
    }
}

/// Lado del proceso `__editor`: devuelve el código de salida que git espera.
pub fn run_client(file: &Path) -> Result<i32> {
    let dir = PathBuf::from(
        std::env::var_os("LEARN_GIT_IPC")
            .context("LEARN_GIT_IPC is not set (run this through the app)")?,
    );
    let absolute = std::path::absolute(file)?;
    // Se escribe a un temporal y se renombra, así la TUI nunca lee un pedido a medias.
    let tmp = dir.join("request.tmp");
    fs::write(&tmp, absolute.to_string_lossy().as_bytes())?;
    fs::rename(&tmp, dir.join(REQUEST))?;

    let response = dir.join(RESPONSE);
    let deadline = Instant::now() + CLIENT_TIMEOUT;
    while Instant::now() < deadline {
        if let Ok(answer) = fs::read_to_string(&response) {
            let _ = fs::remove_file(&response);
            return Ok(if answer.trim() == "ok" { 0 } else { 1 });
        }
        thread::sleep(Duration::from_millis(30));
    }
    Ok(1)
}

pub enum EditorAction {
    Continue,
    Saved,
    Cancelled,
}

pub struct EditorState {
    pub path: PathBuf,
    pub textarea: TextArea<'static>,
    /// `true` si lo abrió git (hay un proceso esperando respuesta); `false` si fue el comando `edit`.
    pub from_git: bool,
}

impl EditorState {
    pub fn open(path: PathBuf, from_git: bool) -> Result<Self> {
        let content = match fs::read(&path) {
            Ok(bytes) => String::from_utf8_lossy(&bytes).into_owned(),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
            Err(e) => return Err(e).with_context(|| format!("could not read {}", path.display())),
        };
        let lines: Vec<String> = content.lines().map(str::to_string).collect();
        let mut textarea = TextArea::new(if lines.is_empty() {
            vec![String::new()]
        } else {
            lines
        });
        let name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        textarea.set_block(
            Block::bordered().title(format!(" {name}  —  Ctrl+S save & close · Esc cancel ")),
        );
        Ok(Self {
            path,
            textarea,
            from_git,
        })
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> Result<EditorAction> {
        match (key.code, key.modifiers) {
            (KeyCode::Char('s'), m) if m.contains(KeyModifiers::CONTROL) => {
                self.save()?;
                Ok(EditorAction::Saved)
            }
            (KeyCode::Esc, _) => Ok(EditorAction::Cancelled),
            _ => {
                self.textarea.input(key);
                Ok(EditorAction::Continue)
            }
        }
    }

    fn save(&self) -> Result<()> {
        let mut text = self.textarea.lines().join("\n");
        text.push('\n');
        fs::write(&self.path, text)
            .with_context(|| format!("could not save {}", self.path.display()))
    }
}
