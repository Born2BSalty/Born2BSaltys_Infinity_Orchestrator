// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

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
