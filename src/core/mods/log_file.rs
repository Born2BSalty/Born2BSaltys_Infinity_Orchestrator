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
        Self::from_text(&content)
    }

    pub fn from_text(content: &str) -> Result<Self> {
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

    #[must_use]
    pub fn components(&self) -> &[Component] {
        &self.components
    }

    #[must_use]
    pub const fn len(&self) -> usize {
        self.components.len()
    }

    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.components.is_empty()
    }

    #[cfg(test)]
    pub(crate) const fn from_components(components: Vec<Component>) -> Self {
        Self { components }
    }
}

#[cfg(test)]
mod tests {
    use super::LogFile;

    #[test]
    fn from_text_keeps_root_level_lines_and_skips_blank_and_comment_lines() {
        let text = "\n// comment\n~SETUP-D0QUESTPACK.TP2~ #0 #5 // Additional Shadow Thieves Content: v3.5\n";
        let log = LogFile::from_text(text).expect("log should parse");
        assert_eq!(log.len(), 1);
        assert_eq!(log.components()[0].name, "");
        assert_eq!(log.components()[0].tp_file, "SETUP-D0QUESTPACK.TP2");
    }
}
