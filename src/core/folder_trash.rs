// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::path::Path;

pub fn move_folder_to_trash(path: &Path) -> Result<(), String> {
    remove_folder(path).map_err(|err| {
        format!(
            "couldn't move {} to the Recycle Bin: {err}; the folder was kept",
            path.display()
        )
    })
}

#[cfg(not(test))]
fn remove_folder(path: &Path) -> Result<(), trash::Error> {
    trash::delete(path)
}

#[cfg(test)]
fn remove_folder(path: &Path) -> std::io::Result<()> {
    std::fs::remove_dir_all(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    static COUNTER: AtomicU64 = AtomicU64::new(0);

    struct TempRoot(PathBuf);

    impl TempRoot {
        fn new(label: &str) -> Self {
            let root = std::env::temp_dir().join(format!(
                "bio_folder_trash_test_{}_{}_{label}",
                std::process::id(),
                COUNTER.fetch_add(1, Ordering::Relaxed)
            ));
            Self(root)
        }
    }

    impl Drop for TempRoot {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn removes_populated_folder() {
        let root = TempRoot::new("populated");
        let target = root.0.join("target");
        std::fs::create_dir_all(target.join("sub")).unwrap();
        std::fs::write(target.join("sub").join("f.txt"), b"x").unwrap();
        move_folder_to_trash(&target).expect("removal succeeds");
        assert!(!target.exists());
    }

    #[test]
    fn missing_folder_reports_kept_error() {
        let root = TempRoot::new("missing");
        let target = root.0.join("absent");
        let err = move_folder_to_trash(&target).expect_err("missing folder errors");
        let prefix = format!("couldn't move {} to the Recycle Bin: ", target.display());
        assert!(err.starts_with(&prefix), "{err}");
        assert!(err.ends_with("; the folder was kept"), "{err}");
    }
}
