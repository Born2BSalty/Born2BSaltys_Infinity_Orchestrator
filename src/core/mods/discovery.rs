// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use walkdir::WalkDir;

use crate::mods::component::Component;

#[derive(Debug, Clone)]
pub struct DiscoveryIndex {
    by_component: HashMap<(String, String), PathBuf>,
}

impl DiscoveryIndex {
    pub fn build(mod_root: &Path, depth: usize) -> Self {
        let mut by_component = HashMap::new();
        for entry in WalkDir::new(mod_root)
            .follow_links(true)
            .max_depth(depth)
            .into_iter()
            .flatten()
        {
            if !entry.file_type().is_file() {
                continue;
            }
            let file_name = entry.file_name().to_string_lossy().to_string();
            if !file_name.to_ascii_lowercase().ends_with(".tp2") {
                continue;
            }
            let Some(parent) = entry.path().parent() else {
                continue;
            };
            let Some(mod_name) = parent.file_name().map(|n| n.to_string_lossy().to_string()) else {
                continue;
            };
            by_component
                .entry((normalize(&mod_name), normalize(&file_name)))
                .or_insert_with(|| parent.to_path_buf());
        }
        Self { by_component }
    }

    pub fn find_folder(&self, component: &Component) -> Option<&Path> {
        self.by_component
            .get(&(normalize(&component.name), normalize(&component.tp_file)))
            .map(PathBuf::as_path)
    }
}

fn normalize(value: &str) -> String {
    value.to_ascii_lowercase()
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    use crate::mods::component::Component;
    use crate::mods::discovery::DiscoveryIndex;

    fn temp_dir() -> PathBuf {
        let mut dir = std::env::temp_dir();
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be valid")
            .as_nanos();
        dir.push(format!("bio_discovery_test_{ts}"));
        dir
    }

    #[test]
    fn finds_tp2_folder_by_name_and_file() {
        let root = temp_dir();
        let mod_dir = root.join("bg1ub");
        fs::create_dir_all(&mod_dir).expect("should create mod dir");
        fs::write(mod_dir.join("BG1UB.TP2"), b"// tp2").expect("should create tp2");

        let index = DiscoveryIndex::build(&root, 5);
        let component = Component {
            tp_file: "BG1UB.TP2".to_string(),
            name: "BG1UB".to_string(),
            lang: "0".to_string(),
            component: "3".to_string(),
            component_name: String::new(),
            sub_component: String::new(),
            version: String::new(),
            wlb_inputs: None,
        };
        let found = index.find_folder(&component);
        assert!(found.is_some(), "folder should be found");
        assert_eq!(
            found
                .and_then(|p| p.file_name())
                .and_then(|n| n.to_str())
                .unwrap_or_default()
                .to_ascii_lowercase(),
            "bg1ub"
        );

        let _ = fs::remove_dir_all(root);
    }
}
