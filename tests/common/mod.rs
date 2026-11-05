#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::time::{Duration, Instant};

use anyhow::Result;
use learn_git_tui::console::{Console, Event, Outcome};
use learn_git_tui::editor::IpcServer;
use learn_git_tui::sandbox::{Sandbox, remove_dir_all_force};

/// Qué hace el "usuario" cuando git abre el editor: recibe la ruta y devuelve si acepta.
pub type EditorHandler<'a> = &'a mut dyn FnMut(&Path) -> bool;

pub struct Harness {
    pub console: Console,
    pub root: PathBuf,
    ipc: IpcServer,
    tx: Sender<Event>,
    rx: Receiver<Event>,
}

impl Harness {
    pub fn new(name: &str) -> Self {
        let root =
            std::env::temp_dir().join(format!("learn-git-tui-test-{}-{name}", std::process::id()));
        remove_dir_all_force(&root).unwrap();
        let exe = PathBuf::from(env!("CARGO_BIN_EXE_learn-git-tui"));
        let sandbox = Sandbox::new(root.clone(), exe).unwrap();
        let (tx, rx) = mpsc::channel();
        Self {
            ipc: IpcServer::new(sandbox.ipc_dir()),
            console: Console::new(sandbox),
            root,
            tx,
            rx,
        }
    }

    /// Corre un comando y devuelve toda su salida. Falla si la consola lo rechaza.
    pub fn try_run(
        &mut self,
        line: &str,
        stdin: Option<&str>,
        editor: Option<EditorHandler>,
    ) -> Result<String> {
        let (code, output) = self.run_raw(line, stdin, editor)?;
        assert!(
            code.is_none_or(|c| c == 0),
            "`{line}` failed ({code:?}): {output}"
        );
        Ok(output)
    }

    /// Como `try_run`, pero devuelve el código de salida en vez de exigir que sea 0.
    pub fn run_raw(
        &mut self,
        line: &str,
        stdin: Option<&str>,
        mut editor: Option<EditorHandler>,
    ) -> Result<(Option<i32>, String)> {
        let outcome = self.console.execute(line, &self.tx)?;
        let mut running = match outcome {
            Outcome::Running(r) => r,
            _ => return Ok((None, self.drain())),
        };
        if let Some(input) = stdin {
            running.send_line(input)?;
            running.close_stdin();
        }
        let mut output = String::new();
        let deadline = Instant::now() + Duration::from_secs(30);
        loop {
            assert!(
                Instant::now() < deadline,
                "timeout running `{line}`; output so far: {output}"
            );
            if let Some(handler) = editor.as_mut()
                && let Some(path) = self.ipc.poll_request()
            {
                let accepted = handler(&path);
                self.ipc.respond(accepted).unwrap();
            }
            match self.rx.recv_timeout(Duration::from_millis(20)) {
                Ok(Event::Output(text)) => output.push_str(&text),
                Ok(Event::Finished(code)) => {
                    output.push_str(&self.drain());
                    return Ok((code, output));
                }
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => panic!("channel closed"),
            }
        }
    }

    pub fn run(&mut self, line: &str) -> String {
        self.try_run(line, None, None).unwrap()
    }

    fn drain(&self) -> String {
        self.rx
            .try_iter()
            .filter_map(|e| {
                if let Event::Output(t) = e {
                    Some(t)
                } else {
                    None
                }
            })
            .collect()
    }
}

impl Drop for Harness {
    fn drop(&mut self) {
        let _ = remove_dir_all_force(&self.root);
    }
}
