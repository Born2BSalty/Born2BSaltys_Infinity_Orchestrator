// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::fs::File;
use std::process::{Child, ChildStdin};
use std::sync::mpsc::Receiver;
use std::{collections::{HashMap, VecDeque}};

use anyhow::Result;
use eframe::egui;

mod analyze;
mod api;
mod backend;
mod input;
mod output;
mod process;
mod scripted_inputs;
mod view;

pub struct EmbeddedTerminal {
    pub(super) child: Option<Child>,
    pub(super) stdin: Option<ChildStdin>,
    output_rx: Option<Receiver<backend::OutputEvent>>,
    pub(super) output_buffer: String,
    important_buffer: String,
    installed_buffer: String,
    important_scan_tail: String,
    prompt_capture_active: bool,
    prompt_capture_lines: usize,
    prompt_capture_after_send: bool,
    warning_capture_active: bool,
    warning_capture_lines: usize,
    pub(super) max_buffer_chars: usize,
    boundary_event_count: u64,
    boundary_scan_tail: String,
    child_env: Vec<(String, String)>,
    scripted_inputs_by_component: HashMap<String, VecDeque<String>>,
    current_component_key: Option<String>,
    current_component_tp2: Option<String>,
    current_component_id: Option<String>,
    current_component_name: Option<String>,
    scripted_inputs_loaded_count: usize,
    raw_log_path: Option<std::path::PathBuf>,
    raw_log_file: Option<File>,
    bio_debug_log_path: Option<std::path::PathBuf>,
    bio_debug_log_file: Option<File>,
    pub(super) show_important_only: bool,
    pub(super) show_installed_only: bool,
    pub(super) stick_to_bottom: bool,
    pub(super) active: bool,
    pub(super) request_focus: bool,
    has_new_data: bool,
    saw_exit_event: bool,
    last_exit_code: Option<i32>,
}

impl EmbeddedTerminal {
    pub fn new(_ctx: &egui::Context) -> Result<Self> {
        Ok(Self {
            child: None,
            stdin: None,
            output_rx: None,
            output_buffer: String::new(),
            important_buffer: String::new(),
            installed_buffer: String::new(),
            important_scan_tail: String::new(),
            prompt_capture_active: false,
            prompt_capture_lines: 0,
            prompt_capture_after_send: false,
            warning_capture_active: false,
            warning_capture_lines: 0,
            max_buffer_chars: 250_000,
            boundary_event_count: 0,
            boundary_scan_tail: String::new(),
            child_env: Vec::new(),
            scripted_inputs_by_component: HashMap::new(),
            current_component_key: None,
            current_component_tp2: None,
            current_component_id: None,
            current_component_name: None,
            scripted_inputs_loaded_count: 0,
            raw_log_path: None,
            raw_log_file: None,
            bio_debug_log_path: None,
            bio_debug_log_file: None,
            show_important_only: false,
            show_installed_only: false,
            stick_to_bottom: true,
            active: false,
            request_focus: true,
            has_new_data: false,
            saw_exit_event: false,
            last_exit_code: None,
        })
    }
}
