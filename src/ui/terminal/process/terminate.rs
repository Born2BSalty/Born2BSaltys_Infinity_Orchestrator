// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::process::{Command, Stdio};

use super::super::EmbeddedTerminal;

impl EmbeddedTerminal {
    pub fn force_terminate(&mut self) {
        self.terminate_process_tree("Force terminate requested");
    }

    pub fn graceful_terminate(&mut self) {
        self.terminate_process_tree("Graceful terminate requested");
    }

    fn terminate_process_tree(&mut self, marker: &str) {
        self.log_bio_debug(&format!("terminate_process_tree marker=\"{}\"", marker));
        let pid = self.child.as_ref().map(|c| c.id());
        if let Some(child) = self.child.as_mut() {
            let _ = child.kill();
        }
        #[cfg(target_os = "windows")]
        if let Some(pid) = pid {
            // Non-blocking tree kill: do not wait on UI thread.
            let _ = Command::new("taskkill")
                .args(["/PID", &pid.to_string(), "/T", "/F"])
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn();
        }
        #[cfg(not(target_os = "windows"))]
        if let Some(pid) = pid {
            // Best-effort child tree termination on Unix-like systems.
            let pid_s = pid.to_string();
            let _ = Command::new("pkill")
                .args(["-TERM", "-P", &pid_s])
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn();
            let _ = Command::new("kill")
                .args(["-TERM", &pid_s])
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn();
        }
        self.child = None;
        self.stdin = None;
        self.output_rx = None;
        self.last_exit_code = Some(1);
        self.saw_exit_event = true;
        self.append_marker(marker);
        self.has_new_data = true;
    }
}
