// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::path::Path;

use anyhow::{Context, Result};

use crate::mods::component::Component;

#[derive(Debug, Clone)]
pub struct LogFile {
    components: Vec<Component>,
}

impl LogFile {
    pub fn from_path(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("failed to read log file: {}", path.display()))?;
        let mut components = Vec::new();
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with("//") || !trimmed.starts_with('~') {
                continue;
            }
            components.push(Component::parse_weidu_line(trimmed)?);
        }
        Ok(Self { components })
    }

    pub fn components(&self) -> &[Component] {
        &self.components
    }

    pub fn len(&self) -> usize {
        self.components.len()
    }

    #[cfg(test)]
    pub(crate) fn from_components(components: Vec<Component>) -> Self {
        Self { components }
    }
}
