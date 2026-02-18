// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::io::Read;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;

use anyhow::{anyhow, Result};

pub(super) enum OutputEvent {
    Data(String),
}

pub(super) struct SpawnedProcess {
    pub child: Child,
    pub stdin: Option<ChildStdin>,
    pub rx: Receiver<OutputEvent>,
}

pub(super) fn spawn_process(
    program: &str,
    args: &[String],
    extra_env: &[(String, String)],
) -> Result<SpawnedProcess> {
    let mut cmd = Command::new(program);
    cmd.args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .env("PAGER", "cat")
        .env("LESS", "FRX")
        .env("MORE", "");
    for (k, v) in extra_env {
        cmd.env(k, v);
    }

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }

    let mut child = cmd
        .spawn()
        .map_err(|e| anyhow!("failed to spawn process `{program}`: {e}"))?;

    let stdin = child.stdin.take();
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    let (tx, rx) = mpsc::channel::<OutputEvent>();

    if let Some(stdout) = stdout {
        spawn_reader(stdout, tx.clone());
    }
    if let Some(stderr) = stderr {
        spawn_reader(stderr, tx);
    }

    Ok(SpawnedProcess { child, stdin, rx })
}

fn spawn_reader<R: Read + Send + 'static>(mut reader: R, tx: Sender<OutputEvent>) {
    thread::spawn(move || {
        let mut buf = [0_u8; 4096];
        loop {
            match reader.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    let chunk = String::from_utf8_lossy(&buf[..n]).to_string();
                    if tx.send(OutputEvent::Data(chunk)).is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    });
}
