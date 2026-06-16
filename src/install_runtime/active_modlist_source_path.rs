// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::app::mod_downloads::set_active_modlist_dir;
use crate::registry::store_workspace::modlist_data_dir;

pub fn set_ambient_for_modlist(modlist_id: &str) {
    set_active_modlist_dir(Some(modlist_data_dir(modlist_id)));
}

pub fn clear_ambient() {
    set_active_modlist_dir(None);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::mod_downloads::{
        AMBIENT_TEST_LOCK, active_modlist_dir, active_modlist_downloads_path,
    };

    struct AmbientGuard(Option<std::path::PathBuf>);
    impl AmbientGuard {
        fn acquire() -> Self {
            Self(crate::app::mod_downloads::active_modlist_dir())
        }
    }
    impl Drop for AmbientGuard {
        fn drop(&mut self) {
            crate::app::mod_downloads::set_active_modlist_dir(self.0.take());
        }
    }

    #[test]
    fn set_ambient_for_modlist_sets_the_dir() {
        let _lock = AMBIENT_TEST_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let _guard = AmbientGuard::acquire();

        set_ambient_for_modlist("MODLIST-A");
        let dir = active_modlist_dir();
        assert!(
            dir.is_some(),
            "dir should be set after set_ambient_for_modlist"
        );
        let path = active_modlist_downloads_path();
        assert!(path.is_some(), "downloads path should be derived from dir");
        let p = path.unwrap();
        assert!(p.ends_with("mod_downloads_user.toml"));
        assert!(p.to_string_lossy().contains("MODLIST-A"));
    }

    #[test]
    fn clear_ambient_unsets_the_dir() {
        let _lock = AMBIENT_TEST_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let _guard = AmbientGuard::acquire();

        set_ambient_for_modlist("MODLIST-A");
        clear_ambient();
        assert!(
            active_modlist_dir().is_none(),
            "dir cleared after clear_ambient"
        );
        assert!(
            active_modlist_downloads_path().is_none(),
            "downloads path is None when dir is cleared"
        );
    }

    #[test]
    fn workspace_switch_refreshes_ambient() {
        let _lock = AMBIENT_TEST_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let _guard = AmbientGuard::acquire();

        set_ambient_for_modlist("MODLIST-A");
        let path_a = active_modlist_downloads_path().unwrap();

        set_ambient_for_modlist("MODLIST-B");
        let path_b = active_modlist_downloads_path().unwrap();

        assert_ne!(
            path_a, path_b,
            "switching modlists refreshes the ambient path"
        );
        assert!(
            path_b.to_string_lossy().contains("MODLIST-B"),
            "B's path contains B's id"
        );
    }

    #[test]
    fn editor_destination_defaults_global() {
        use crate::app::step2_action::ModSourceEditDestination;
        assert_eq!(
            ModSourceEditDestination::default(),
            ModSourceEditDestination::GlobalDefault,
            "default destination is GlobalDefault"
        );
        let d = ModSourceEditDestination::ThisModlist;
        let d2 = d;
        assert_eq!(d, d2);
    }
}
