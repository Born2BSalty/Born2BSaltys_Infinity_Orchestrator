// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::path::Path;

use tracing::warn;

use crate::registry::model::ModlistEntry;

pub const IMPORT_CODE_FILENAME: &str = "modlist-import-code.txt";

impl ModlistEntry {
    pub fn set_latest_share_code(&mut self, code: String) {
        write_import_code_file(&self.id, &self.destination_folder, &code);
        self.latest_share_code = Some(code);
    }
}

fn write_import_code_file(id: &str, destination_folder: &str, code: &str) {
    let destination = destination_folder.trim();
    if destination.is_empty() {
        return;
    }
    let folder = Path::new(destination);
    if !folder.is_dir() {
        return;
    }
    if let Err(err) = std::fs::write(folder.join(IMPORT_CODE_FILENAME), code.as_bytes()) {
        warn!(
            target = "orchestrator",
            "writing {IMPORT_CODE_FILENAME} for {id} to {destination} failed: {err}"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEMP_SEQ: AtomicU64 = AtomicU64::new(0);

    struct TempRoot(PathBuf);

    impl TempRoot {
        fn new(tag: &str) -> Self {
            let root = std::env::temp_dir().join(format!(
                "bio_share_code_file_{tag}_{}_{}",
                std::process::id(),
                TEMP_SEQ.fetch_add(1, Ordering::Relaxed)
            ));
            std::fs::create_dir_all(&root).expect("create the temp root");
            Self(root)
        }
    }

    impl Drop for TempRoot {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn entry_at(destination: &Path) -> ModlistEntry {
        ModlistEntry {
            id: "CODEFILE0001".to_string(),
            destination_folder: destination.to_string_lossy().into_owned(),
            ..Default::default()
        }
    }

    #[test]
    fn setting_the_code_writes_the_file() {
        let root = TempRoot::new("writes");
        let mut entry = entry_at(&root.0);

        entry.set_latest_share_code("BIO-MODLIST-V1:first".to_string());

        assert_eq!(
            entry.latest_share_code.as_deref(),
            Some("BIO-MODLIST-V1:first")
        );
        assert_eq!(
            std::fs::read_to_string(root.0.join(IMPORT_CODE_FILENAME)).expect("file written"),
            "BIO-MODLIST-V1:first"
        );
    }

    #[test]
    fn setting_the_code_rewrites_the_file_with_the_new_code() {
        let root = TempRoot::new("rewrites");
        let mut entry = entry_at(&root.0);

        entry.set_latest_share_code("BIO-MODLIST-V1:first".to_string());
        entry.set_latest_share_code("BIO-MODLIST-V1:second".to_string());

        assert_eq!(
            entry.latest_share_code.as_deref(),
            Some("BIO-MODLIST-V1:second")
        );
        assert_eq!(
            std::fs::read_to_string(root.0.join(IMPORT_CODE_FILENAME)).expect("file written"),
            "BIO-MODLIST-V1:second"
        );
    }

    #[test]
    fn blank_destination_writes_no_file() {
        let mut entry = ModlistEntry {
            destination_folder: "   ".to_string(),
            ..Default::default()
        };

        entry.set_latest_share_code("BIO-MODLIST-V1:blank".to_string());

        assert_eq!(
            entry.latest_share_code.as_deref(),
            Some("BIO-MODLIST-V1:blank")
        );
        assert!(!Path::new(IMPORT_CODE_FILENAME).exists());
    }

    #[test]
    fn missing_destination_writes_nothing_and_creates_no_folder() {
        let root = TempRoot::new("missing");
        let missing = root.0.join("not here");
        let mut entry = entry_at(&missing);

        entry.set_latest_share_code("BIO-MODLIST-V1:missing".to_string());

        assert_eq!(
            entry.latest_share_code.as_deref(),
            Some("BIO-MODLIST-V1:missing")
        );
        assert!(!missing.exists());
    }
}
