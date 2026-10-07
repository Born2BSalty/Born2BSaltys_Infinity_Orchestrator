// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use chrono::Utc;
use tracing::warn;

use crate::app::state::WizardState;
use crate::install_runtime::flag_policies::{self, InstallWorkflow};
use crate::install_runtime::registry_transition;
use crate::registry::model::ModlistRegistry;
use crate::registry::share_export::{self, ShareMeta};
use crate::registry::store::RegistryStore;
use crate::registry::workspace_model::ModsSource;
use crate::settings::model::Step1Settings;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstallButtonVariant {
    Install,

    Restart,

    Resume,

    Reinstall,
}

impl InstallButtonVariant {
    #[must_use]
    pub const fn from_step5(state: &WizardState, reinstall: bool) -> Self {
        if reinstall {
            Self::Reinstall
        } else if state.step5.resume_available {
            Self::Resume
        } else if state.step5.has_run_once {
            Self::Restart
        } else {
            Self::Install
        }
    }

    #[must_use]
    pub fn from_step5_and_reinstall(
        state: &WizardState,
        modlist_id: &str,
        pending_reinstall_id: Option<&str>,
    ) -> Self {
        Self::from_step5(state, pending_reinstall_id == Some(modlist_id))
    }
}

fn ensure_destination_exists(modlist_id: &str, destination: &str) {
    if destination.is_empty() {
        return;
    }
    if let Err(err) = std::fs::create_dir_all(destination) {
        warn!(
            target = "orchestrator",
            "creating the destination {destination} for {modlist_id} at install start failed: {err}"
        );
    }
}

pub fn write_install_start_artifacts(
    modlist_id: &str,
    wizard_state: &WizardState,
    registry: &mut ModlistRegistry,
    store: &RegistryStore,
) -> Result<(), String> {
    crate::install_runtime::active_modlist_source_path::set_ambient_for_modlist(modlist_id);

    let entry = registry
        .find(modlist_id)
        .ok_or_else(|| format!("modlist {modlist_id} not in registry at install start"))?;
    let destination = entry.destination_folder.trim().to_string();
    let meta = ShareMeta::from_entry(entry, false)
        .with_archive_meta(share_export::archive_meta_for_draft(wizard_state));

    let share_code = share_export::pack_meta(wizard_state, &meta)?;

    let entry_mut = registry
        .find_mut(modlist_id)
        .ok_or_else(|| format!("modlist {modlist_id} vanished from registry mid-hook"))?;
    ensure_destination_exists(modlist_id, &destination);
    entry_mut.set_latest_share_code(share_code);

    entry_mut.install_started_at = Some(Utc::now());

    store
        .save(registry)
        .map_err(|err| format!("registry write at install start failed: {err}"))?;

    Ok(())
}

pub fn write_install_start_artifacts_with_code(
    modlist_id: &str,
    code_source: &str,
    registry: &mut ModlistRegistry,
    store: &RegistryStore,
) -> Result<(), String> {
    let entry = registry
        .find(modlist_id)
        .ok_or_else(|| format!("modlist {modlist_id} not in registry at install start"))?;
    let destination = entry.destination_folder.trim().to_string();
    let trimmed_source = code_source.trim();
    let identity = ShareMeta::from_entry(entry, false);
    let share_code = match share_export::set_allow_auto_install(trimmed_source, false)
        .and_then(|code| share_export::set_packed_identity(&code, &identity))
    {
        Ok(code) => code,
        Err(err) => {
            warn!(
                target = "orchestrator",
                "install-start: could not decode the held code for {modlist_id} \
                 to set allow_auto_install=false ({err}) — persisting it \
                 VERBATIM (the real code is the priority; SPEC §13.13)"
            );
            trimmed_source.to_string()
        }
    };

    let entry_mut = registry
        .find_mut(modlist_id)
        .ok_or_else(|| format!("modlist {modlist_id} vanished from registry mid-hook"))?;
    ensure_destination_exists(modlist_id, &destination);
    entry_mut.set_latest_share_code(share_code);
    entry_mut.install_started_at = Some(Utc::now());

    store
        .save(registry)
        .map_err(|err| format!("registry write at install start failed: {err}"))?;

    Ok(())
}

pub struct InstallStartCtx<'a> {
    pub settings: &'a Step1Settings,
    pub mods_source: ModsSource,
}

fn global_mods_folder_override(ctx: &InstallStartCtx<'_>) -> Option<String> {
    if ctx.mods_source != ModsSource::GlobalModsFolder {
        return None;
    }
    let folder = ctx.settings.effective_global_mods_folder().trim();
    (!folder.is_empty()).then(|| folder.to_string())
}

pub fn on_install_start(
    modlist_id: &str,
    variant: InstallButtonVariant,
    workflow: InstallWorkflow,
    wizard_state: &mut WizardState,
    registry: &mut ModlistRegistry,
    store: &RegistryStore,
    ctx: &InstallStartCtx<'_>,
) -> Result<(), String> {
    flag_policies::apply_flags(&mut wizard_state.step1, workflow, ctx.settings);

    write_install_start_artifacts(modlist_id, wizard_state, registry, store)?;

    let destination = registry
        .find(modlist_id)
        .map(|e| e.destination_folder.trim().to_string())
        .ok_or_else(|| {
            format!("modlist {modlist_id} vanished from registry after §13.13 artifacts")
        })?;

    if variant == InstallButtonVariant::Reinstall {
        tracing::debug!(
            target = "orchestrator",
            "on_install_start saw InstallButtonVariant::Reinstall for \
             {modlist_id}; the Installed→InProgress flip is performed at \
             the Install-Modlist Install-click via \
             reinstall_flip_at_install_click (the Reinstall route does not \
             pass through on_install_start — see start_hooks module note)"
        );
    }

    let game = registry
        .find(modlist_id)
        .map(|e| e.game)
        .ok_or_else(|| format!("modlist {modlist_id} vanished from registry before dir derive"))?;
    crate::install_runtime::per_install_dirs::derive_per_install_dirs(
        &mut wizard_state.step1,
        &destination,
        game,
    )
    .map_err(|err| format!("per-install directory derivation failed for {modlist_id}: {err}"))?;

    if let Some(folder) = global_mods_folder_override(ctx) {
        wizard_state.step1.mods_folder = folder;
    }

    Ok(())
}

pub fn reinstall_flip_at_install_click(
    modlist_id: &str,
    wizard_state: &WizardState,
    registry: &mut ModlistRegistry,
    store: &RegistryStore,
    pending_reinstall_id: &mut Option<String>,
) -> bool {
    let variant = InstallButtonVariant::from_step5_and_reinstall(
        wizard_state,
        modlist_id,
        pending_reinstall_id.as_deref(),
    );
    if variant != InstallButtonVariant::Reinstall {
        return false;
    }

    let flipped = registry_transition::flip_to_in_progress(modlist_id, registry, store);

    *pending_reinstall_id = None;

    if flipped {
        tracing::info!(
            target = "orchestrator",
            "Reinstall: {modlist_id} flipped Installed → InProgress at \
             Install-click (SPEC §3.1); pending_reinstall_id cleared"
        );
    } else {
        warn!(
            target = "orchestrator",
            "Reinstall: flip_to_in_progress for {modlist_id} did not persist \
             (see prior log); pending_reinstall_id cleared anyway — the \
             install proceeds, the entry stays Installed (returns to \
             Installed on clean exit via flip_to_installed). SPEC §13.14"
        );
    }
    flipped
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::state::WizardState;

    #[test]
    fn variant_from_step5_mirrors_bio_label_logic() {
        let mut s = WizardState::default();
        assert_eq!(
            InstallButtonVariant::from_step5(&s, false),
            InstallButtonVariant::Install
        );

        s.step5.has_run_once = true;
        assert_eq!(
            InstallButtonVariant::from_step5(&s, false),
            InstallButtonVariant::Restart
        );

        s.step5.resume_available = true;
        assert_eq!(
            InstallButtonVariant::from_step5(&s, false),
            InstallButtonVariant::Resume
        );

        assert_eq!(
            InstallButtonVariant::from_step5(&s, true),
            InstallButtonVariant::Reinstall
        );
    }

    #[test]
    fn from_step5_and_reinstall_wires_pending_reinstall_id() {
        let s = WizardState::default();

        assert_eq!(
            InstallButtonVariant::from_step5_and_reinstall(&s, "MOD-A", None),
            InstallButtonVariant::Install
        );

        assert_eq!(
            InstallButtonVariant::from_step5_and_reinstall(&s, "MOD-A", Some("MOD-B")),
            InstallButtonVariant::Install,
            "a pending reinstall for a different modlist must not tag this one"
        );

        assert_eq!(
            InstallButtonVariant::from_step5_and_reinstall(&s, "MOD-A", Some("MOD-A")),
            InstallButtonVariant::Reinstall,
            "pending_reinstall_id == Some(this id) ⇒ Reinstall (SPEC §3.1)"
        );
    }

    use crate::registry::model::{Game, ModlistEntry, ModlistRegistry, ModlistState};
    use std::sync::atomic::{AtomicU64, Ordering};

    static TMP_COUNTER: AtomicU64 = AtomicU64::new(0);

    fn temp_registry_store(label: &str) -> (RegistryStore, std::path::PathBuf) {
        let n = TMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir()
            .join(format!(
                "bio_start_hooks_test_{}_{}_{}",
                std::process::id(),
                n,
                label
            ))
            .with_extension("json");
        (RegistryStore::new_with_path(&path), path)
    }

    fn installed_entry(id: &str) -> ModlistEntry {
        ModlistEntry {
            id: id.to_string(),
            name: "Polished EET".to_string(),
            game: Game::EET,
            state: ModlistState::Installed,
            latest_share_code: Some("BIO-MODLIST-V1:VERIFIED".to_string()),
            ..Default::default()
        }
    }

    #[test]
    fn reinstall_flip_at_install_click_flips_and_clears_when_reinstall() {
        let (store, path) = temp_registry_store("flip_happy");
        let mut registry = ModlistRegistry::default();
        registry.entries.push(installed_entry("REINSTALL0001"));
        let s = WizardState::default();
        let mut pending = Some("REINSTALL0001".to_string());

        let flipped = reinstall_flip_at_install_click(
            "REINSTALL0001",
            &s,
            &mut registry,
            &store,
            &mut pending,
        );

        assert!(flipped, "a real Reinstall route flips + persists");
        assert_eq!(
            registry.find("REINSTALL0001").unwrap().state,
            ModlistState::InProgress,
            "Installed → InProgress at Install-click (SPEC §3.1)"
        );
        assert_eq!(
            pending, None,
            "pending_reinstall_id cleared so a later frame cannot re-flip"
        );

        let again = reinstall_flip_at_install_click(
            "REINSTALL0001",
            &s,
            &mut registry,
            &store,
            &mut pending,
        );
        assert!(!again, "marker cleared ⇒ no-op (not a Reinstall anymore)");
        assert_eq!(
            registry.find("REINSTALL0001").unwrap().state,
            ModlistState::InProgress
        );
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn reinstall_flip_at_install_click_is_noop_without_pending() {
        let (store, path) = temp_registry_store("flip_nopending");
        let mut registry = ModlistRegistry::default();
        registry.entries.push(installed_entry("SOME-MODLIST"));
        let s = WizardState::default();
        let mut pending: Option<String> = None;

        let flipped = reinstall_flip_at_install_click(
            "SOME-MODLIST",
            &s,
            &mut registry,
            &store,
            &mut pending,
        );
        assert!(
            !flipped,
            "no pending_reinstall_id ⇒ not a Reinstall ⇒ no-op"
        );
        assert_eq!(
            registry.find("SOME-MODLIST").unwrap().state,
            ModlistState::Installed,
            "a non-Reinstall Install-Modlist paste must NOT flip state"
        );
        assert_eq!(pending, None);

        assert!(!path.exists());
    }

    #[test]
    fn reinstall_flip_at_install_click_pending_for_other_modlist_is_noop() {
        let (store, _path) = temp_registry_store("flip_other");
        let mut registry = ModlistRegistry::default();
        registry.entries.push(installed_entry("MOD-A"));
        let s = WizardState::default();
        let mut pending = Some("MOD-B".to_string());

        let flipped =
            reinstall_flip_at_install_click("MOD-A", &s, &mut registry, &store, &mut pending);
        assert!(
            !flipped,
            "pending is for MOD-B, not MOD-A ⇒ no-op for MOD-A"
        );
        assert_eq!(
            registry.find("MOD-A").unwrap().state,
            ModlistState::Installed
        );
        assert_eq!(
            pending,
            Some("MOD-B".to_string()),
            "MOD-B's pending marker is untouched (only MOD-B's own \
             Install-click consumes it)"
        );
    }

    #[test]
    fn write_install_start_artifacts_errs_cleanly_when_share_code_ungeneratable() {
        let (store, store_path) = temp_registry_store("artifacts_pack_err");
        let mut registry = ModlistRegistry::default();
        registry.entries.push(installed_entry("MODLIST-ART-1"));
        let before = registry.find("MODLIST-ART-1").unwrap().clone();
        let s = WizardState::default();

        let r = write_install_start_artifacts("MODLIST-ART-1", &s, &mut registry, &store);

        assert!(
            r.is_err(),
            "ungeneratable share code ⇒ Err (SPEC §13.14 — caller must not \
             proceed-as-clean)"
        );

        let after = registry.find("MODLIST-ART-1").unwrap();
        assert_eq!(
            after.latest_share_code, before.latest_share_code,
            "Err before any entry mutation — latest_share_code untouched"
        );
        assert_eq!(
            after.install_started_at, before.install_started_at,
            "Err before any entry mutation — install_started_at untouched"
        );

        assert!(
            !store_path.exists(),
            "the §13.13 helper must not save the registry when it Errs early"
        );
    }

    #[test]
    fn write_install_start_artifacts_errs_when_modlist_not_in_registry() {
        let (store, store_path) = temp_registry_store("artifacts_no_entry");
        let mut registry = ModlistRegistry::default();
        let s = WizardState::default();

        let r = write_install_start_artifacts("GHOST-MODLIST", &s, &mut registry, &store);

        let msg = r.expect_err("a missing registry entry must Err");
        assert!(
            msg.contains("GHOST-MODLIST") && msg.contains("not in registry"),
            "documented Err message naming the missing modlist; got: {msg}"
        );
        assert!(
            !store_path.exists(),
            "no registry save on the missing-entry early Err"
        );
    }

    #[test]
    fn global_mods_folder_override_returns_some_for_global_source_with_path() {
        let settings = Step1Settings {
            global_mods_folder: r"D:\GlobalMods".to_string(),
            ..Step1Settings::default()
        };
        let ctx = InstallStartCtx {
            settings: &settings,
            mods_source: ModsSource::GlobalModsFolder,
        };
        let result = global_mods_folder_override(&ctx);
        assert_eq!(result, Some(r"D:\GlobalMods".to_string()));
    }

    #[test]
    fn global_mods_folder_override_returns_none_when_the_global_folder_is_blank() {
        let settings = Step1Settings {
            global_mods_folder: "   ".to_string(),
            ..Step1Settings::default()
        };
        let ctx = InstallStartCtx {
            settings: &settings,
            mods_source: ModsSource::GlobalModsFolder,
        };
        let result = global_mods_folder_override(&ctx);
        assert_eq!(result, None);
    }

    #[test]
    fn global_mods_folder_override_returns_none_for_installation_folder_source() {
        let settings = Step1Settings {
            global_mods_folder: r"D:\GlobalMods".to_string(),
            ..Step1Settings::default()
        };
        let ctx = InstallStartCtx {
            settings: &settings,
            mods_source: ModsSource::InstallationFolder,
        };
        let result = global_mods_folder_override(&ctx);
        assert_eq!(result, None);
    }

    #[test]
    fn write_install_start_artifacts_with_code_stamps_the_entry_identity_onto_a_carried_code() {
        use crate::app::modlist_share::{ForkAncestor, preview_modlist_share_code};

        struct StoreFileGuard(std::path::PathBuf);
        impl Drop for StoreFileGuard {
            fn drop(&mut self) {
                let _ = std::fs::remove_file(&self.0);
            }
        }

        let (store, store_path) = temp_registry_store("carried_identity");
        let _guard = StoreFileGuard(store_path);
        let mut registry = ModlistRegistry::default();
        registry.entries.push(ModlistEntry {
            id: "FORK-1".to_string(),
            name: "EET Essentials +++".to_string(),
            game: Game::EET,
            state: ModlistState::InProgress,
            author: Some("Xgatt".to_string()),
            forked_from: vec![ForkAncestor {
                name: "EET Essentials".to_string(),
                author: "BIO Team".to_string(),
            }],
            ..Default::default()
        });
        let parent_code = crate::app::modlist_share::encode_share_payload_text(
            r#"{
                "format_version": 1,
                "game_install": "EET",
                "install_mode": "start_from_scratch",
                "allow_auto_install": true,
                "name": "EET Essentials",
                "author": "BIO Team",
                "weidu_logs": { "bgee": "~MOD/MOD.TP2~ #0 #0 // A component" }
            }"#,
        )
        .expect("mint parent code");

        write_install_start_artifacts_with_code("FORK-1", &parent_code, &mut registry, &store)
            .expect("hook ok");

        let stored = registry
            .find("FORK-1")
            .unwrap()
            .latest_share_code
            .clone()
            .expect("code stored");
        let preview = preview_modlist_share_code(&stored).expect("preview");
        assert_eq!(preview.name.as_deref(), Some("EET Essentials +++"));
        assert_eq!(preview.author.as_deref(), Some("Xgatt"));
        assert_eq!(
            preview.forked_from,
            vec![ForkAncestor {
                name: "EET Essentials".to_string(),
                author: "BIO Team".to_string(),
            }]
        );
        assert!(!preview.allow_auto_install);
        assert_eq!(preview.bgee_entries, 1);
    }

    #[test]
    fn install_start_creates_the_destination_and_writes_the_code_file() {
        use crate::registry::share_code_file::IMPORT_CODE_FILENAME;

        struct TempRoot(std::path::PathBuf);
        impl Drop for TempRoot {
            fn drop(&mut self) {
                let _ = std::fs::remove_dir_all(&self.0);
            }
        }

        let root = TempRoot(std::env::temp_dir().join(format!(
            "bio_start_hooks_code_file_{}_{}",
            std::process::id(),
            TMP_COUNTER.fetch_add(1, Ordering::Relaxed)
        )));
        let destination = root.0.join("install here");
        let store = RegistryStore::new_with_path(root.0.join("registry.json"));
        let mut registry = ModlistRegistry::default();
        registry.entries.push(ModlistEntry {
            id: "RESUME000001".to_string(),
            name: "Resumed run".to_string(),
            game: Game::BGEE,
            state: ModlistState::InProgress,
            destination_folder: destination.to_string_lossy().into_owned(),
            ..Default::default()
        });
        let carried = crate::app::modlist_share::encode_share_payload_text(
            r#"{
                "format_version": 1,
                "game_install": "BGEE",
                "install_mode": "start_from_scratch",
                "name": "Resumed run",
                "weidu_logs": { "bgee": "~MOD/MOD.TP2~ #0 #0 // A component" }
            }"#,
        )
        .expect("mint code");

        write_install_start_artifacts_with_code("RESUME000001", &carried, &mut registry, &store)
            .expect("hook ok");

        let stored = registry
            .find("RESUME000001")
            .unwrap()
            .latest_share_code
            .clone()
            .expect("code stored");
        assert_eq!(
            std::fs::read_to_string(destination.join(IMPORT_CODE_FILENAME))
                .expect("the file is written into the created destination"),
            stored
        );
    }
}
