// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::path::Path;
use std::sync::mpsc::{self, Receiver, Sender};

use chrono::Utc;
use tracing::warn;

use crate::app::state::WizardState;
use crate::install_runtime::import_code_writer;
use crate::registry::model::{ModlistEntry, ModlistRegistry, ModlistState};
use crate::registry::share_export::{self, ArchiveMeta, ShareMeta};
use crate::registry::store::RegistryStore;
use crate::ui::workspace::step4::workspace_step4;

pub type SizeWorkerResult = (String, u64);

pub type SizeWorkerReceiver = Receiver<SizeWorkerResult>;

#[must_use]
pub fn count_mods_and_components(state: &WizardState) -> (u32, u32) {
    let tabs: &[&[crate::app::state::Step3ItemState]] = if workspace_step4::is_dual_game(state) {
        &[&state.step3.bgee_items, &state.step3.bg2ee_items]
    } else {
        &[&state.step3.bgee_items]
    };

    let mut component_count: usize = 0;
    let mut seen_tp: Vec<String> = Vec::new();
    for items in tabs {
        for it in *items {
            if it.is_parent {
                continue;
            }
            component_count += 1;

            if !seen_tp.iter().any(|t| t.eq_ignore_ascii_case(&it.tp_file)) {
                seen_tp.push(it.tp_file.clone());
            }
        }
    }

    (
        u32::try_from(seen_tp.len()).unwrap_or(u32::MAX),
        u32::try_from(component_count).unwrap_or(u32::MAX),
    )
}

pub fn flip_to_installed(
    id: &str,
    registry: &mut ModlistRegistry,
    store: &RegistryStore,
    wizard_state: &WizardState,
    share_code_override: Option<&str>,
) -> Option<SizeWorkerReceiver> {
    let (mod_count, component_count) = count_mods_and_components(wizard_state);

    let Some(entry_ref) = registry.find(id) else {
        warn!(
            target = "orchestrator",
            "flip_to_installed: modlist {id} not in registry on clean exit \
             (install completed; registry flip skipped — SPEC §13.14)"
        );
        return None;
    };
    let destination = entry_ref.destination_folder.trim().to_string();

    let archive_dir =
        std::path::PathBuf::from(wizard_state.step1.mods_archive_folder.trim().to_string());
    let archive_meta =
        share_export::build_archive_meta_from_install_lock(&destination, &archive_dir);

    let new_code = build_verified_code(
        id,
        entry_ref,
        wizard_state,
        share_code_override,
        archive_meta,
    )?;

    let Some(entry) = registry.find_mut(id) else {
        warn!(
            target = "orchestrator",
            "flip_to_installed: modlist {id} vanished mid-flip; skipping"
        );
        return None;
    };
    entry.state = ModlistState::Installed;
    entry.install_date = Some(Utc::now());
    entry.mod_count = mod_count;
    entry.component_count = component_count;

    let verified_code = new_code.clone();
    entry.latest_share_code = Some(new_code);

    entry.total_size_bytes = None;

    if let Err(err) = store.save(registry) {
        warn!(
            target = "orchestrator",
            "flip_to_installed: atomic registry write for {id} failed: {err} \
             (install completed; in-progress→installed flip not persisted — \
             SPEC §13.14)"
        );
        return None;
    }

    if destination.is_empty() {
        warn!(
            target = "orchestrator",
            "flip_to_installed: modlist {id} has no destination_folder on \
             clean exit — skipping the modlist-import-code.txt rewrite \
             (nothing to write it next to; registry latest_share_code is \
             canonical — SPEC §13.13)"
        );
    } else if let Err(err) =
        import_code_writer::write_modlist_import_code_txt(Path::new(&destination), &verified_code)
    {
        warn!(
            target = "orchestrator",
            "flip_to_installed: rewriting modlist-import-code.txt to \
             {destination} on clean exit failed: {err} (non-fatal — the \
             registry holds the verified allow_auto_install=true code; the \
             on-disk file stays the install-start draft — SPEC §13.13/§13.14)"
        );
    }

    spawn_size_worker(id, destination)
}

fn build_verified_code(
    id: &str,
    entry_ref: &ModlistEntry,
    wizard_state: &WizardState,
    share_code_override: Option<&str>,
    archive_meta: Vec<ArchiveMeta>,
) -> Option<String> {
    if let Some(src) = share_code_override {
        let bit_flipped = match share_export::set_allow_auto_install(src.trim(), true) {
            Ok(code) => code,
            Err(err) => {
                warn!(
                    target = "orchestrator",
                    "flip_to_installed: could not decode the held code for \
                     {id} to set allow_auto_install=true ({err}) — persisting \
                     it VERBATIM (the real code is the priority; SPEC §13.13)"
                );
                src.trim().to_string()
            }
        };

        match share_export::bake_archive_meta_into_code(&bit_flipped, &archive_meta) {
            Ok(code) => code,
            Err(err) => {
                warn!(
                    target = "orchestrator",
                    "flip_to_installed: could not bake archive_meta into the \
                     held code for {id} ({err}) — keeping it without the \
                     per-archive {{size,hash}} (recipients fall back to \
                     always-download; the real code is preserved)"
                );
                bit_flipped
            }
        }
    } else {
        let meta = ShareMeta::from_entry(entry_ref, true).with_archive_meta(archive_meta);
        match share_export::pack_meta(wizard_state, &meta) {
            Ok(code) => code,
            Err(err) => {
                warn!(
                    target = "orchestrator",
                    "flip_to_installed: share-code regeneration for {id} \
                     failed: {err} (registry NOT flipped — install already \
                     completed)"
                );
                return None;
            }
        }
    }
    .into()
}

fn spawn_size_worker(id: &str, destination: String) -> Option<SizeWorkerReceiver> {
    let (tx, rx): (Sender<SizeWorkerResult>, Receiver<SizeWorkerResult>) = mpsc::channel();
    let id_for_worker = id.to_string();

    let spawn = std::thread::Builder::new()
        .name(format!("io-size-{id}"))
        .spawn(move || {
            let bytes = directory_size_bytes(Path::new(&destination));

            let _ = tx.send((id_for_worker, bytes));
        });
    if let Err(err) = spawn {
        warn!(
            target = "orchestrator",
            "flip_to_installed: failed to spawn size worker for {id}: {err} \
             (size will render as — ; install + state flip already persisted)"
        );
        return None;
    }

    Some(rx)
}

fn directory_size_bytes(root: &Path) -> u64 {
    if !root.is_dir() {
        return 0;
    }
    let mut total: u64 = 0;
    let mut stack: Vec<std::path::PathBuf> = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(read_dir) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in read_dir.flatten() {
            let Ok(file_type) = entry.file_type() else {
                continue;
            };
            if file_type.is_symlink() {
                continue;
            }
            if file_type.is_dir() {
                stack.push(entry.path());
            } else {
                match entry.metadata() {
                    Ok(meta) if file_type.is_file() => total = total.saturating_add(meta.len()),
                    _ => {}
                }
            }
        }
    }
    total
}

pub fn flip_to_in_progress(
    id: &str,
    registry: &mut ModlistRegistry,
    store: &RegistryStore,
) -> bool {
    let Some(entry) = registry.find_mut(id) else {
        warn!(
            target = "orchestrator",
            "flip_to_in_progress: modlist {id} not in registry at Reinstall \
             install-start (Reinstall aborted — nothing to flip)"
        );
        return false;
    };

    if entry.state != ModlistState::Installed {
        warn!(
            target = "orchestrator",
            "flip_to_in_progress: modlist {id} is not Installed \
             (state = {:?}); Reinstall flip skipped (only Installed → \
             InProgress is valid — SPEC §3.1)",
            entry.state
        );
        return false;
    }

    entry.state = ModlistState::InProgress;

    if let Err(err) = store.save(registry) {
        warn!(
            target = "orchestrator",
            "flip_to_in_progress: atomic registry write for {id} failed: \
             {err} (Reinstall NOT started; entry reverted to Installed — \
             SPEC §13.14)"
        );
        if let Some(entry) = registry.find_mut(id) {
            entry.state = ModlistState::Installed;
        }
        return false;
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::state::{Step2ComponentState, Step2ModState, Step3ItemState};
    use crate::registry::model::{Game, ModlistEntry};
    use std::sync::atomic::{AtomicU64, Ordering};

    static TMP_COUNTER: AtomicU64 = AtomicU64::new(0);

    fn temp_registry_store(label: &str) -> (RegistryStore, std::path::PathBuf) {
        let n = TMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir()
            .join(format!(
                "bio_flip_to_installed_test_{}_{}_{}",
                std::process::id(),
                n,
                label
            ))
            .with_extension("json");
        (RegistryStore::new_with_path(&path), path)
    }

    fn leaf(tp: &str, id: &str) -> Step3ItemState {
        Step3ItemState {
            tp_file: tp.to_string(),
            component_id: id.to_string(),
            mod_name: tp.to_string(),
            component_label: format!("comp {id}"),
            raw_line: String::new(),
            prompt_summary: None,
            prompt_events: Vec::new(),
            selected_order: 1,
            block_id: String::new(),
            is_parent: false,
            parent_placeholder: false,
        }
    }
    fn parent(tp: &str) -> Step3ItemState {
        let mut p = leaf(tp, "__PARENT__");
        p.is_parent = true;
        p
    }

    #[test]
    fn counts_skip_parents_and_dedupe_tp_single_game() {
        let mut s = WizardState::default();
        s.step1.game_install = "BGEE".to_string();
        s.step3.bgee_items = vec![
            parent("EEFIXPACK.TP2"),
            leaf("EEFIXPACK.TP2", "0"),
            leaf("eefixpack.tp2", "2"),
            parent("BG1UB.TP2"),
            leaf("BG1UB.TP2", "0"),
        ];

        s.step3.bg2ee_items = vec![leaf("SHOULD_NOT_COUNT.TP2", "0")];
        let (mods, comps) = count_mods_and_components(&s);
        assert_eq!(comps, 3, "3 installable leaves, parents excluded");
        assert_eq!(mods, 2, "2 distinct tp_files (case-insensitive)");
    }

    #[test]
    fn counts_sum_both_tabs_for_eet() {
        let mut s = WizardState::default();
        s.step1.game_install = "EET".to_string();
        s.step3.bgee_items = vec![parent("A.TP2"), leaf("A.TP2", "0"), leaf("A.TP2", "1")];
        s.step3.bg2ee_items = vec![leaf("B.TP2", "0"), leaf("C.TP2", "0")];
        let (mods, comps) = count_mods_and_components(&s);
        assert_eq!(comps, 4, "2 BGEE leaves + 2 BG2EE leaves");
        assert_eq!(mods, 3, "A + B + C distinct across both tabs");
    }

    #[test]
    fn flip_sets_installed_state_date_counts_and_true_bit_code() {
        let (store, path) = temp_registry_store("happy");
        let mut registry = ModlistRegistry::default();
        registry.entries.push(ModlistEntry {
            id: "FLIPME000001".to_string(),
            name: "Polished EET".to_string(),
            game: Game::EET,

            destination_folder: String::new(),
            state: ModlistState::InProgress,

            latest_share_code: Some("BIO-MODLIST-V1:STALE".to_string()),
            ..Default::default()
        });

        let mut s = WizardState::default();

        s.step1.game_install = "EET".to_string();
        s.step3.bgee_items = vec![leaf("A.TP2", "0"), leaf("A.TP2", "1")];
        s.step3.bg2ee_items = vec![leaf("B.TP2", "0")];

        let rx = flip_to_installed("FLIPME000001", &mut registry, &store, &s, None);

        let entry = registry.find("FLIPME000001").expect("entry present");
        assert_eq!(entry.state, ModlistState::Installed, "state flipped");
        assert!(entry.install_date.is_some(), "install_date stamped");
        assert_eq!(entry.mod_count, 2, "A + B distinct across both EET tabs");
        assert_eq!(entry.component_count, 3, "2 BGEE leaves + 1 BG2EE leaf");
        assert_eq!(
            entry.total_size_bytes, None,
            "size is None until the async worker reports"
        );
        let code = entry.latest_share_code.as_deref().expect("code");
        assert!(
            code.starts_with("BIO-MODLIST-V1:"),
            "regenerated a BIO-MODLIST-V1 code, not the stale snapshot"
        );
        assert_ne!(
            code, "BIO-MODLIST-V1:STALE",
            "the install-start snapshot was overwritten"
        );

        assert!(
            decoded_allow_auto_install(code),
            "flip_to_installed must regenerate with allow_auto_install = true"
        );

        let rx = rx.expect("size worker spawned");
        let (got_id, bytes) = rx
            .recv_timeout(std::time::Duration::from_secs(5))
            .expect("worker reports");
        assert_eq!(got_id, "FLIPME000001");
        assert_eq!(bytes, 0, "no destination on disk ⇒ 0 bytes");

        assert!(path.exists(), "registry written to the temp path");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn flip_on_missing_entry_is_a_logged_noop() {
        let (store, path) = temp_registry_store("missing");
        let mut registry = ModlistRegistry::default();
        let s = WizardState::default();
        let rx = flip_to_installed("DOES_NOT_EXIST", &mut registry, &store, &s, None);
        assert!(
            rx.is_none(),
            "no entry ⇒ None (no worker, no write) — SPEC §13.14 non-fatal"
        );

        assert!(!path.exists());
    }

    #[test]
    fn flip_to_in_progress_installed_to_inprogress_and_persists() {
        let (store, path) = temp_registry_store("reinstall_happy");
        let mut registry = ModlistRegistry::default();
        registry.entries.push(ModlistEntry {
            id: "REINSTALL0001".to_string(),
            name: "Polished EET".to_string(),
            game: Game::EET,
            destination_folder: String::new(),
            state: ModlistState::Installed,

            latest_share_code: Some("BIO-MODLIST-V1:VERIFIED".to_string()),
            mod_count: 9,
            component_count: 136,
            ..Default::default()
        });

        let ok = flip_to_in_progress("REINSTALL0001", &mut registry, &store);
        assert!(ok, "Installed → InProgress flip succeeds + persists");

        let entry = registry.find("REINSTALL0001").expect("entry present");
        assert_eq!(
            entry.state,
            ModlistState::InProgress,
            "state flipped Installed → InProgress (SPEC §3.1)"
        );

        assert_eq!(entry.mod_count, 9, "counts untouched (state-only flip)");
        assert_eq!(entry.component_count, 136);
        assert_eq!(
            entry.latest_share_code.as_deref(),
            Some("BIO-MODLIST-V1:VERIFIED"),
            "the verified code is NOT rewritten by flip_to_in_progress \
             (install-start path owns the code per SPEC §13.13)"
        );
        assert!(
            path.exists(),
            "registry written to the temp path (NOT %APPDATA%)"
        );
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn flip_to_in_progress_missing_entry_is_a_logged_noop() {
        let (store, path) = temp_registry_store("reinstall_missing");
        let mut registry = ModlistRegistry::default();
        let ok = flip_to_in_progress("DOES_NOT_EXIST", &mut registry, &store);
        assert!(
            !ok,
            "no entry ⇒ false (no write) — Reinstall aborts, nothing to flip"
        );
        assert!(!path.exists(), "the early return precedes the save");
    }

    #[test]
    fn flip_to_in_progress_non_installed_is_skipped() {
        let (store, path) = temp_registry_store("reinstall_notinstalled");
        let mut registry = ModlistRegistry::default();
        registry.entries.push(ModlistEntry {
            id: "NOTINSTALLED1".to_string(),
            name: "Draft".to_string(),
            game: Game::BGEE,
            state: ModlistState::InProgress,
            ..Default::default()
        });
        let ok = flip_to_in_progress("NOTINSTALLED1", &mut registry, &store);
        assert!(
            !ok,
            "non-Installed ⇒ false (skipped, only Installed→InProgress)"
        );
        assert_eq!(
            registry.find("NOTINSTALLED1").unwrap().state,
            ModlistState::InProgress,
            "state left untouched"
        );
        assert!(!path.exists(), "a skipped flip does not write the registry");
    }

    #[test]
    fn directory_size_sums_files_recursively() {
        let dir = std::env::temp_dir().join(format!(
            "bio_dirsize_test_{}_{}",
            std::process::id(),
            TMP_COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(dir.join("sub")).unwrap();
        std::fs::write(dir.join("a.bin"), [0u8; 100]).unwrap();
        std::fs::write(dir.join("sub").join("b.bin"), [0u8; 50]).unwrap();
        assert_eq!(directory_size_bytes(&dir), 150, "100 + 50 recursively");
        assert_eq!(
            directory_size_bytes(&dir.join("does_not_exist")),
            0,
            "non-existent path ⇒ 0 (honest 'nothing measurable')"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    fn temp_destination(label: &str) -> std::path::PathBuf {
        let n = TMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!(
            "bio_flip_dest_test_{}_{}_{}",
            std::process::id(),
            n,
            label
        ))
    }

    fn eet_state_with_leaves() -> WizardState {
        let mut s = WizardState::default();
        s.step1.game_install = "EET".to_string();
        s.step3.bgee_items = vec![leaf("A.TP2", "0"), leaf("A.TP2", "1")];
        s.step3.bg2ee_items = vec![leaf("B.TP2", "0")];
        s
    }

    #[test]
    fn flip_to_installed_rewrites_ondisk_import_code_with_true_bit() {
        let (store, store_path) = temp_registry_store("ondisk_rewrite");
        let dest = temp_destination("rewrite");
        std::fs::create_dir_all(&dest).unwrap();

        let draft_path = dest.join(import_code_writer::IMPORT_CODE_FILENAME);
        std::fs::write(&draft_path, "BIO-MODLIST-V1:INSTALL-START-DRAFT").unwrap();

        let mut registry = ModlistRegistry::default();
        registry.entries.push(ModlistEntry {
            id: "ONDISK000001".to_string(),
            name: "Polished EET".to_string(),
            game: Game::EET,
            destination_folder: dest.to_string_lossy().into_owned(),
            state: ModlistState::InProgress,
            latest_share_code: Some("BIO-MODLIST-V1:STALE".to_string()),
            ..Default::default()
        });
        let s = eet_state_with_leaves();

        let rx = flip_to_installed("ONDISK000001", &mut registry, &store, &s, None);
        assert!(rx.is_some(), "clean-exit flip succeeded");

        let on_disk = std::fs::read_to_string(&draft_path).expect("file still present");
        assert_ne!(
            on_disk, "BIO-MODLIST-V1:INSTALL-START-DRAFT",
            "the install-start draft on disk must be overwritten on clean exit"
        );
        assert!(
            on_disk.starts_with("BIO-MODLIST-V1:"),
            "rewritten with a BIO-MODLIST-V1 code"
        );

        let entry = registry.find("ONDISK000001").unwrap();
        assert_eq!(
            Some(on_disk.as_str()),
            entry.latest_share_code.as_deref(),
            "on-disk file == registry latest_share_code (the verified code)"
        );

        assert!(
            decoded_allow_auto_install(&on_disk),
            "on-disk modlist-import-code.txt carries allow_auto_install=true \
             on clean exit (SPEC §13.13 FIX 2)"
        );

        let _ = rx.unwrap().recv_timeout(std::time::Duration::from_secs(5));
        let _ = std::fs::remove_dir_all(&dest);
        let _ = std::fs::remove_file(&store_path);
    }

    #[test]
    fn flip_to_installed_install_modlist_override_uses_held_code_not_pack_meta() {
        let (store, store_path) = temp_registry_store("im_override");
        let dest = temp_destination("im_override");
        std::fs::create_dir_all(&dest).unwrap();
        let draft_path = dest.join(import_code_writer::IMPORT_CODE_FILENAME);

        let provenance = ShareMeta {
            allow_auto_install: false,
            name: Some("Tactical EET 2026".to_string()),
            author: Some("@sharer".to_string()),
            forked_from: vec![crate::app::modlist_share::ForkAncestor {
                name: "Root".to_string(),
                author: "@root".to_string(),
            }],
            archive_meta: vec![],
        };
        let generated = share_export::pack_meta(&eet_state_with_leaves(), &provenance)
            .expect("fixture: pack_meta over a populated WizardState");

        let held_false = share_export::set_allow_auto_install(&generated, false).unwrap();
        std::fs::write(&draft_path, &held_false).unwrap();

        let mut registry = ModlistRegistry::default();
        registry.entries.push(ModlistEntry {
            id: "IMOVR0000001".to_string(),
            name: "Tactical EET 2026".to_string(),
            game: Game::EET,
            destination_folder: dest.to_string_lossy().into_owned(),
            state: ModlistState::InProgress,

            latest_share_code: Some(held_false.clone()),
            ..Default::default()
        });

        let empty_step3 = WizardState::default();

        let rx = flip_to_installed(
            "IMOVR0000001",
            &mut registry,
            &store,
            &empty_step3,
            Some(held_false.as_str()),
        );
        assert!(
            rx.is_some(),
            "the Install-Modlist override path flips even with EMPTY step3 \
             (it does NOT regenerate via pack_meta — the pinned symptom fix)"
        );

        let entry = registry.find("IMOVR0000001").unwrap();
        assert_eq!(entry.state, ModlistState::Installed, "state flipped");
        let code = entry.latest_share_code.as_deref().expect("code");
        assert!(decoded_allow_auto_install(code), "true-bit on clean exit");

        let on_disk = std::fs::read_to_string(&draft_path).unwrap();
        assert_eq!(Some(on_disk.as_str()), entry.latest_share_code.as_deref());
        assert!(decoded_allow_auto_install(&on_disk));

        let encoded = on_disk.strip_prefix("BIO-MODLIST-V1:").unwrap();
        let mut vals: Vec<u8> = Vec::new();
        for ch in encoded.chars().filter(|c| !c.is_whitespace()) {
            vals.push(match ch {
                'A'..='Z' => ch as u8 - b'A',
                'a'..='z' => ch as u8 - b'a' + 26,
                '0'..='9' => ch as u8 - b'0' + 52,
                '-' => 62,
                '_' => 63,
                _ => panic!("bad b64url"),
            });
        }
        let mut bytes = Vec::new();
        for chunk in vals.chunks(4) {
            let c0 = chunk[0];
            let c1 = *chunk.get(1).unwrap_or(&0);
            let c2 = *chunk.get(2).unwrap_or(&0);
            let c3 = *chunk.get(3).unwrap_or(&0);
            bytes.push((c0 << 2) | (c1 >> 4));
            if chunk.len() > 2 {
                bytes.push(((c1 & 0x0F) << 4) | (c2 >> 2));
            }
            if chunk.len() > 3 {
                bytes.push(((c2 & 0x03) << 6) | c3);
            }
        }
        let mut json = String::new();
        {
            use flate2::read::ZlibDecoder;
            use std::io::Read;
            ZlibDecoder::new(&bytes[..])
                .read_to_string(&mut json)
                .unwrap();
        }
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(
            v["name"],
            serde_json::json!("Tactical EET 2026"),
            "the pasted code's packed name rode through verbatim"
        );
        assert_eq!(v["author"], serde_json::json!("@sharer"));
        assert_eq!(
            v["forked_from"],
            serde_json::json!([{ "name": "Root", "author": "@root" }]),
            "the pasted code's lineage rode through verbatim (every \
             ancestor stays credited — SPEC §13.3)"
        );

        let _ = rx.unwrap().recv_timeout(std::time::Duration::from_secs(5));
        let _ = std::fs::remove_dir_all(&dest);
        let _ = std::fs::remove_file(&store_path);
    }

    #[test]
    fn flip_to_installed_override_verbatim_fallback_on_undecodable_held_code() {
        let (store, store_path) = temp_registry_store("im_verbatim");
        let dest = temp_destination("im_verbatim");
        std::fs::create_dir_all(&dest).unwrap();
        let draft_path = dest.join(import_code_writer::IMPORT_CODE_FILENAME);

        let mut registry = ModlistRegistry::default();
        registry.entries.push(ModlistEntry {
            id: "IMVERB000001".to_string(),
            name: "Shared modlist".to_string(),
            game: Game::BGEE,
            destination_folder: dest.to_string_lossy().into_owned(),
            state: ModlistState::InProgress,
            latest_share_code: Some("BIO-MODLIST-V1:undecodable!!!".to_string()),
            ..Default::default()
        });
        let empty_step3 = WizardState::default();

        let rx = flip_to_installed(
            "IMVERB000001",
            &mut registry,
            &store,
            &empty_step3,
            Some("BIO-MODLIST-V1:undecodable!!!"),
        );
        assert!(
            rx.is_some(),
            "an undecodable held code still flips (verbatim fallback — the \
             real code is the priority; never a None/failed flip)"
        );
        let entry = registry.find("IMVERB000001").unwrap();
        assert_eq!(entry.state, ModlistState::Installed);
        assert_eq!(
            entry.latest_share_code.as_deref(),
            Some("BIO-MODLIST-V1:undecodable!!!"),
            "the held code persisted VERBATIM (the user's priority)"
        );
        assert_eq!(
            std::fs::read_to_string(&draft_path).unwrap(),
            "BIO-MODLIST-V1:undecodable!!!",
            "on-disk file == the verbatim held code"
        );
        let _ = rx.unwrap().recv_timeout(std::time::Duration::from_secs(5));
        let _ = std::fs::remove_dir_all(&dest);
        let _ = std::fs::remove_file(&store_path);
    }

    #[test]
    fn ondisk_import_code_unchanged_on_non_clean_exit() {
        let dest = temp_destination("noclean");
        std::fs::create_dir_all(&dest).unwrap();
        let draft_path = dest.join(import_code_writer::IMPORT_CODE_FILENAME);
        std::fs::write(&draft_path, "BIO-MODLIST-V1:INSTALL-START-DRAFT").unwrap();

        assert_eq!(
            std::fs::read_to_string(&draft_path).unwrap(),
            "BIO-MODLIST-V1:INSTALL-START-DRAFT",
            "no flip call ⇒ on-disk draft untouched (rewrite is only inside \
             flip_to_installed, gated by the caller's C3 triple)"
        );

        let (store, store_path) = temp_registry_store("noclean_regenfail");
        let mut registry = ModlistRegistry::default();
        registry.entries.push(ModlistEntry {
            id: "NOCLEAN00001".to_string(),
            name: "Draft".to_string(),
            game: Game::EET,
            destination_folder: dest.to_string_lossy().into_owned(),
            state: ModlistState::InProgress,
            ..Default::default()
        });
        let empty = WizardState::default();
        let rx = flip_to_installed("NOCLEAN00001", &mut registry, &store, &empty, None);
        assert!(
            rx.is_none(),
            "share-code regen failure ⇒ None (no flip, no step 5b)"
        );
        assert_eq!(
            std::fs::read_to_string(&draft_path).unwrap(),
            "BIO-MODLIST-V1:INSTALL-START-DRAFT",
            "an early-None flip_to_installed must NOT rewrite the on-disk \
             draft (step 5b is reached only after a successful regen + save)"
        );
        assert_eq!(
            registry.find("NOCLEAN00001").unwrap().state,
            ModlistState::InProgress,
            "entry not flipped on the regen-failure path"
        );

        let _ = std::fs::remove_dir_all(&dest);
        let _ = std::fs::remove_file(&store_path);
    }

    #[test]
    fn flip_to_installed_empty_destination_skips_ondisk_write_no_panic() {
        let (store, store_path) = temp_registry_store("empty_dest");
        let mut registry = ModlistRegistry::default();
        registry.entries.push(ModlistEntry {
            id: "EMPTYDEST001".to_string(),
            name: "Polished EET".to_string(),
            game: Game::EET,
            destination_folder: String::new(),
            state: ModlistState::InProgress,
            ..Default::default()
        });
        let s = eet_state_with_leaves();

        let rx = flip_to_installed("EMPTYDEST001", &mut registry, &store, &s, None);
        assert!(
            rx.is_some(),
            "the flip still succeeds with an empty destination (5b skipped, \
             not fatal)"
        );
        let entry = registry.find("EMPTYDEST001").unwrap();
        assert_eq!(entry.state, ModlistState::Installed, "state still flipped");
        assert!(
            entry
                .latest_share_code
                .as_deref()
                .is_some_and(|c| c.starts_with("BIO-MODLIST-V1:")),
            "latest_share_code still regenerated (5b skip is independent)"
        );
        let _ = rx.unwrap().recv_timeout(std::time::Duration::from_secs(5));
        let _ = std::fs::remove_file(&store_path);
    }

    fn unchecked_component(id: &str, label: &str, raw: &str) -> Step2ComponentState {
        Step2ComponentState {
            component_id: id.to_string(),
            label: label.to_string(),
            weidu_group: None,
            collapsible_group: None,
            collapsible_group_is_umbrella: false,
            raw_line: raw.to_string(),
            prompt_summary: None,
            prompt_events: Vec::new(),
            is_meta_mode_component: false,
            disabled: false,
            compat_kind: None,
            compat_source: None,
            compat_related_mod: None,
            compat_related_component: None,
            compat_graph: None,
            compat_evidence: None,
            disabled_reason: None,
            checked: false,
            selected_order: None,
        }
    }

    fn scanned_mod(name: &str, tp: &str, components: Vec<Step2ComponentState>) -> Step2ModState {
        Step2ModState {
            name: name.to_string(),
            tp_file: tp.to_string(),
            tp2_path: String::new(),
            readme_path: None,
            ini_path: None,
            web_url: None,
            package_marker: None,
            latest_checked_version: None,
            update_locked: false,
            mod_prompt_summary: None,
            mod_prompt_events: Vec::new(),
            checked: false,
            hidden_components: Vec::new(),
            components,
        }
    }

    #[test]
    fn fix_1d_paste_path_re_derives_step3_so_counts_are_non_zero() {
        let (store, store_path) = temp_registry_store("fix_1d_paste");
        let dest = temp_destination("fix_1d_paste");
        std::fs::create_dir_all(&dest).unwrap();

        let bgee_log_dir = dest.join("BGEE-logs");
        std::fs::create_dir_all(&bgee_log_dir).unwrap();
        std::fs::write(
            bgee_log_dir.join("weidu.log"),
            "~EEFIXPACK/EEFIXPACK.TP2~ #0 #0 // Fix Pack: v1\n\
             ~EEFIXPACK/EEFIXPACK.TP2~ #0 #2 // Fix Pack extra: v1\n\
             ~BG1UB/SETUP-BG1UB.TP2~ #0 #0 // BG1 UB: v15\n",
        )
        .unwrap();

        let mut s = WizardState::default();
        s.step1.game_install = "BGEE".to_string();
        s.step1.bgee_log_folder = bgee_log_dir.to_string_lossy().into_owned();

        s.step2.bgee_mods = vec![
            scanned_mod(
                "EEFIXPACK",
                "EEFIXPACK/EEFIXPACK.TP2",
                vec![
                    unchecked_component("0", "Fix Pack", "0 // Fix Pack: v1"),
                    unchecked_component("2", "Fix Pack extra", "2 // Fix Pack extra: v1"),
                ],
            ),
            scanned_mod(
                "BG1UB",
                "BG1UB/SETUP-BG1UB.TP2",
                vec![unchecked_component("0", "BG1 UB", "0 // BG1 UB: v15")],
            ),
        ];

        let (mods_before, comps_before) = count_mods_and_components(&s);
        assert_eq!(
            (mods_before, comps_before),
            (0, 0),
            "BASELINE — without the re-derivation, the paste path leaves \
             step3 empty and `count_mods_and_components` reads (0, 0)"
        );

        crate::app::app_step2_log::apply_saved_weidu_log_selection(&mut s);
        crate::app::app_step3_sync_flow::sync_step3_from_step2(&mut s);

        let (mods_after, comps_after) = count_mods_and_components(&s);
        assert!(
            mods_after > 0 && comps_after > 0,
            "Fix 1d — after the pre-flip re-derivation, step3 is populated \
             and counts are NON-ZERO: got ({mods_after}, {comps_after})"
        );

        assert_eq!(mods_after, 2, "2 distinct tp_files");
        assert_eq!(comps_after, 3, "3 component leaves in the synthetic log");

        let mut registry = ModlistRegistry::default();
        registry.entries.push(ModlistEntry {
            id: "FIX1DPASTE01".to_string(),
            name: "Polished BGEE".to_string(),
            game: Game::BGEE,
            destination_folder: dest.to_string_lossy().into_owned(),
            state: ModlistState::InProgress,
            ..Default::default()
        });
        let rx = flip_to_installed("FIX1DPASTE01", &mut registry, &store, &s, None);
        let entry = registry.find("FIX1DPASTE01").unwrap();
        assert_eq!(entry.state, ModlistState::Installed);
        assert_eq!(
            entry.mod_count, 2,
            "registry entry's mod_count reflects the re-derived step3"
        );
        assert_eq!(
            entry.component_count, 3,
            "registry entry's component_count reflects the re-derived step3"
        );
        if let Some(rx) = rx {
            let _ = rx.recv_timeout(std::time::Duration::from_secs(5));
        }
        let _ = std::fs::remove_dir_all(&dest);
        let _ = std::fs::remove_file(&store_path);
    }

    fn decoded_allow_auto_install(code: &str) -> bool {
        use flate2::read::ZlibDecoder;
        use std::io::Read;

        let encoded = code
            .strip_prefix("BIO-MODLIST-V1:")
            .expect("BIO-MODLIST-V1 prefix");

        let mut vals: Vec<u8> = Vec::new();
        for ch in encoded.chars().filter(|c| !c.is_whitespace()) {
            vals.push(match ch {
                'A'..='Z' => ch as u8 - b'A',
                'a'..='z' => ch as u8 - b'a' + 26,
                '0'..='9' => ch as u8 - b'0' + 52,
                '-' => 62,
                '_' => 63,
                _ => panic!("invalid base64url char"),
            });
        }
        let mut bytes = Vec::new();
        for chunk in vals.chunks(4) {
            let c0 = chunk[0];
            let c1 = *chunk.get(1).unwrap_or(&0);
            let c2 = *chunk.get(2).unwrap_or(&0);
            let c3 = *chunk.get(3).unwrap_or(&0);
            bytes.push((c0 << 2) | (c1 >> 4));
            if chunk.len() > 2 {
                bytes.push(((c1 & 0x0F) << 4) | (c2 >> 2));
            }
            if chunk.len() > 3 {
                bytes.push(((c2 & 0x03) << 6) | c3);
            }
        }
        let mut json = String::new();
        ZlibDecoder::new(&bytes[..])
            .read_to_string(&mut json)
            .expect("zlib inflate");
        let v: serde_json::Value = serde_json::from_str(&json).expect("json");
        v["allow_auto_install"].as_bool().expect("bit present")
    }
}
