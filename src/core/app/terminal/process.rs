// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

mod config {
    use std::path::PathBuf;

    use crate::app::state::Step1State;
    use crate::app::step5::log_files::run_dir_from_id;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::super::EmbeddedTerminal;

    impl EmbeddedTerminal {
        pub fn configure_from_step1(&mut self, step1: &Step1State, run_id: Option<&str>) {
            self.child_env.clear();
            let rust_log = if step1.rust_log_trace {
                Some("trace")
            } else if step1.rust_log_debug {
                Some("debug")
            } else {
                None
            };
            if let Some(level) = rust_log {
                self.child_env
                    .push(("RUST_LOG".to_string(), level.to_string()));
            }
            if step1.bio_full_debug {
                self.child_env
                    .push(("BIO_FULL_DEBUG".to_string(), "1".to_string()));
            }
            let diagnostics_dir =
                run_id.map_or_else(|| PathBuf::from("diagnostics"), run_dir_from_id);
            let logs_dir = diagnostics_dir.join("logs");
            self.raw_log_path = if step1.log_raw_output_dev {
                let ts = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map_or(0, |d| d.as_secs());
                Some(logs_dir.join(format!("raw_output_{ts}.log")))
            } else {
                None
            };
            self.bio_debug_log_path = if step1.bio_full_debug {
                let ts = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map_or(0, |d| d.as_secs());
                Some(logs_dir.join(format!("bio_full_debug_{ts}.log")))
            } else {
                None
            };
        }
    }
}
mod lifecycle {
    use std::fs::{self, File};
    use std::io::Write;

    use anyhow::Result;

    use super::super::{EmbeddedTerminal, backend, scripted_inputs};

    impl EmbeddedTerminal {
        pub fn start_process(&mut self, program: &str, args: &[String]) -> Result<()> {
            self.last_runtime_error = None;
            let raw_log_path = self.raw_log_path.clone();
            if let Some(path) = raw_log_path.as_ref() {
                if let Some(parent) = path.parent()
                    && let Err(err) = fs::create_dir_all(parent)
                {
                    self.record_runtime_error(format!(
                        "raw log directory create failed for {}: {err}",
                        parent.display()
                    ));
                }
                match File::create(path) {
                    Ok(file) => self.raw_log_file = Some(file),
                    Err(err) => {
                        self.record_runtime_error(format!(
                            "raw log file create failed for {}: {err}",
                            path.display()
                        ));
                        self.raw_log_file = None;
                    }
                }
            } else {
                self.raw_log_file = None;
            }
            let bio_debug_log_path = self.bio_debug_log_path.clone();
            if let Some(path) = bio_debug_log_path.as_ref() {
                if let Some(parent) = path.parent()
                    && let Err(err) = fs::create_dir_all(parent)
                {
                    self.record_runtime_error(format!(
                        "debug log directory create failed for {}: {err}",
                        parent.display()
                    ));
                }
                match File::create(path) {
                    Ok(file) => self.bio_debug_log_file = Some(file),
                    Err(err) => {
                        self.record_runtime_error(format!(
                            "debug log file create failed for {}: {err}",
                            path.display()
                        ));
                        self.bio_debug_log_file = None;
                    }
                }
            } else {
                self.bio_debug_log_file = None;
            }

            let spawned = backend::spawn_process(program, args, &self.child_env)?;
            self.child = Some(spawned.child);
            self.stdin = spawned.stdin;
            self.output_rx = Some(spawned.rx);
            self.saw_exit_event = false;
            self.current_component_key = None;
            self.current_component_tp2 = None;
            self.current_component_id = None;
            self.current_component_name = None;

            self.log_bio_debug(&format!(
                "start_process program=\"{}\" args_count={} env={:?}",
                program,
                args.len(),
                self.child_env
            ));

            let command_line = format!(
                "$ {} {}\n",
                program,
                args.iter()
                    .map(|a| format!("\"{a}\""))
                    .collect::<Vec<_>>()
                    .join(" ")
            );
            self.append_output(&command_line);
            self.important_buffer.push_str(&command_line);
            self.installed_buffer.push_str(&command_line);
            Ok(())
        }

        pub fn poll_output(&mut self) {
            self.has_new_data = false;
            if let Some(rx) = &self.output_rx {
                let mut chunks: Vec<String> = Vec::new();
                while let Ok(event) = rx.try_recv() {
                    match event {
                        backend::OutputEvent::Data(chunk) => chunks.push(chunk),
                    }
                }
                if !chunks.is_empty() {
                    let joined = chunks.join("");
                    self.update_boundary_events(&joined);
                    scripted_inputs::update_current_component_from_output(self, &joined);
                    self.update_important_lines(&joined);
                    let mut raw_log_error = None::<String>;
                    if let Some(raw) = self.raw_log_file.as_mut() {
                        if let Err(err) = raw.write_all(joined.as_bytes()) {
                            raw_log_error = Some(format!("raw log write failed: {err}"));
                        } else if let Err(err) = raw.flush() {
                            raw_log_error = Some(format!("raw log flush failed: {err}"));
                        }
                    }
                    if let Some(message) = raw_log_error {
                        self.record_runtime_error(message);
                        self.raw_log_file = None;
                    }
                    for chunk in chunks {
                        self.append_output(&chunk);
                    }
                    self.log_bio_debug(&format!(
                        "poll_output chunk_count={} total_bytes={}",
                        joined.len(),
                        joined.len()
                    ));
                    self.has_new_data = true;
                }
            }
            if let Some(child) = self.child.as_mut() {
                match child.try_wait() {
                    Ok(Some(status)) => {
                        self.last_exit_code = status.code();
                        self.child = None;
                        self.stdin = None;
                        self.output_rx = None;
                        self.raw_log_file = None;
                        self.bio_debug_log_file = None;
                        self.current_component_key = None;
                        self.current_component_tp2 = None;
                        self.current_component_id = None;
                        self.current_component_name = None;
                        self.saw_exit_event = true;
                        self.has_new_data = true;
                    }
                    Ok(None) => {}
                    Err(err) => {
                        self.record_runtime_error(format!("process state error: {err}"));
                        self.last_exit_code = Some(1);
                        self.child = None;
                        self.stdin = None;
                        self.output_rx = None;
                        self.raw_log_file = None;
                        self.bio_debug_log_file = None;
                        self.current_component_key = None;
                        self.current_component_tp2 = None;
                        self.current_component_id = None;
                        self.current_component_name = None;
                        self.saw_exit_event = true;
                        self.has_new_data = true;
                    }
                }
            }
        }

        #[must_use]
        pub fn process_id(&self) -> Option<u32> {
            self.child.as_ref().map(std::process::Child::id)
        }

        pub(in crate::app::terminal) fn write_bytes(&mut self, data: &[u8]) {
            self.log_bio_debug(&format!("write_bytes len={}", data.len()));
            let mut stdin_error = None::<String>;
            if let Some(stdin) = self.stdin.as_mut() {
                if let Err(err) = stdin.write_all(data) {
                    stdin_error = Some(format!("stdin write failed: {err}"));
                } else if let Err(err) = stdin.flush() {
                    stdin_error = Some(format!("stdin flush failed: {err}"));
                }
            }
            if let Some(message) = stdin_error {
                self.record_runtime_error(message);
            }
        }

        pub(in crate::app::terminal) fn record_runtime_error(
            &mut self,
            message: impl Into<String>,
        ) {
            let message = message.into();
            self.last_runtime_error = Some(message.clone());
            self.append_output(&format!("\n[terminal] {message}\n"));
            self.important_buffer.push_str("[terminal] ");
            self.important_buffer.push_str(&message);
            self.important_buffer.push('\n');
            self.has_new_data = true;
        }
    }
}
mod terminate {
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
            self.log_bio_debug(&format!("terminate_process_tree marker=\"{marker}\""));
            let pid = self.child.as_ref().map(std::process::Child::id);
            let kill_error = if let Some(child) = self.child.as_mut()
                && let Err(err) = child.kill()
            {
                Some(format!("process kill failed: {err}"))
            } else {
                None
            };
            if let Some(message) = kill_error {
                self.record_runtime_error(message);
            }
            #[cfg(target_os = "windows")]
            if let Some(pid) = pid {
                // Non-blocking tree kill: do not wait on UI thread.
                if let Err(err) = Command::new("taskkill")
                    .args(["/PID", &pid.to_string(), "/T", "/F"])
                    .stdin(Stdio::null())
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .spawn()
                {
                    self.record_runtime_error(format!("taskkill spawn failed: {err}"));
                }
            }
            #[cfg(not(target_os = "windows"))]
            if let Some(pid) = pid {
                // Best-effort child tree termination on Unix-like systems.
                let pid_s = pid.to_string();
                if let Err(err) = Command::new("pkill")
                    .args(["-TERM", "-P", &pid_s])
                    .stdin(Stdio::null())
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .spawn()
                {
                    self.record_runtime_error(format!("pkill spawn failed: {err}"));
                }
                if let Err(err) = Command::new("kill")
                    .args(["-TERM", &pid_s])
                    .stdin(Stdio::null())
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .spawn()
                {
                    self.record_runtime_error(format!("kill spawn failed: {err}"));
                }
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
}
