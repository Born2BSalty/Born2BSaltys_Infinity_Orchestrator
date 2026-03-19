// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

mod config {
    use std::path::PathBuf;

    use crate::ui::step5::service_diagnostics_run_step5::run_dir_from_id;
    use std::time::{SystemTime, UNIX_EPOCH};

    use crate::ui::state::Step1State;

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
            let diagnostics_dir = run_id
                .map(run_dir_from_id)
                .unwrap_or_else(|| PathBuf::from("diagnostics"));
            self.raw_log_path = if step1.log_raw_output_dev {
                let ts = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0);
                Some(diagnostics_dir.join(format!("raw_output_{ts}.log")))
            } else {
                None
            };
            self.bio_debug_log_path = if step1.bio_full_debug {
                let ts = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0);
                Some(diagnostics_dir.join(format!("bio_full_debug_{ts}.log")))
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
            if let Some(path) = &self.raw_log_path {
                if let Some(parent) = path.parent() {
                    let _ = fs::create_dir_all(parent);
                }
                self.raw_log_file = File::create(path).ok();
            } else {
                self.raw_log_file = None;
            }
            if let Some(path) = &self.bio_debug_log_path {
                if let Some(parent) = path.parent() {
                    let _ = fs::create_dir_all(parent);
                }
                self.bio_debug_log_file = File::create(path).ok();
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
                    if let Some(raw) = self.raw_log_file.as_mut() {
                        let _ = raw.write_all(joined.as_bytes());
                        let _ = raw.flush();
                    }
                    for chunk in chunks {
                        self.append_output(&chunk);
                    }
                    self.log_bio_debug(&format!(
                        "poll_output chunk_count={} total_bytes={}",
                        joined.len(),
                        joined.as_bytes().len()
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
                        self.append_output(&format!("\n[terminal] process state error: {err}\n"));
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

        pub fn process_id(&self) -> Option<u32> {
            self.child.as_ref().map(|c| c.id())
        }

        pub(in crate::ui::terminal) fn write_bytes(&mut self, data: &[u8]) {
            self.log_bio_debug(&format!("write_bytes len={}", data.len()));
            if let Some(stdin) = self.stdin.as_mut() {
                if stdin.write_all(data).is_ok() {
                    let _ = stdin.flush();
                }
            }
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
}
