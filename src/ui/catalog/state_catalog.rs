use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use crate::app::github_forks_list::{ForksListState, ForksStatus};
use crate::app::github_release_list::{CachedRelease, ReleaseListStatus};
use crate::app::mod_downloads::{self, ModDownloadSource, SourceTiers};
use crate::app::source_form::{self, SourceFormIdentity};
use crate::app::state::{Step2DiscoveredFork, Step2State, VersionsSheet};
use crate::app::step2_action::ModSourceEditDestination;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum CatalogKind {
    #[default]
    All,
    GitHub,
    WeaselMods,
    MorpheusMart,
    DirectArchive,
}

impl CatalogKind {
    pub(crate) fn of(source: &ModDownloadSource) -> Self {
        if source.github.is_some() {
            Self::GitHub
        } else if mod_downloads::source_is_weaselmods_page_url(&source.url) {
            Self::WeaselMods
        } else if mod_downloads::source_is_morpheus_mart_page_url(&source.url) {
            Self::MorpheusMart
        } else {
            Self::DirectArchive
        }
    }

    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::All => "All",
            Self::GitHub => "GitHub",
            Self::WeaselMods => "Weaselmods",
            Self::MorpheusMart => "Morpheus Mart",
            Self::DirectArchive => "Direct archives",
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) enum LocalPresence {
    #[default]
    Unknown,
    Scanning,
    Found(PathBuf),
    NotFound,
    Error(String),
}

impl LocalPresence {
    pub(crate) const fn label(&self) -> &'static str {
        match self {
            Self::Unknown => "Folder not configured",
            Self::Scanning => "Checking\u{2026}",
            Self::Found(_) => "Found locally",
            Self::NotFound => "Not found",
            Self::Error(_) => "Could not check",
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct CatalogRow {
    pub(crate) tp2: String,
    pub(crate) name: String,
    pub(crate) sources: Vec<ModDownloadSource>,
    pub(crate) category_keys: BTreeSet<String>,
    search: String,
}

impl CatalogRow {
    fn collect_category_keys(
        sources: &[ModDownloadSource],
        options: &mut BTreeMap<String, String>,
    ) -> BTreeSet<String> {
        sources
            .iter()
            .flat_map(|source| &source.categories)
            .filter_map(|category| {
                let label = category.trim();
                let key = label.to_lowercase();
                if key.is_empty()
                    || matches!(
                        key.as_str(),
                        "bg1"
                            | "bg2"
                            | "bgee"
                            | "sod"
                            | "bg2ee"
                            | "eet"
                            | "tutu"
                            | "bgt"
                            | "iwd"
                            | "iwdee"
                            | "iwd2"
                            | "pst"
                            | "pstee"
                    )
                {
                    return None;
                }
                options
                    .entry(key.clone())
                    .and_modify(|current| {
                        if label < current.as_str() {
                            label.clone_into(current);
                        }
                    })
                    .or_insert_with(|| label.to_string());
                Some(key)
            })
            .collect()
    }

    fn matches_kind(&self, kind: CatalogKind) -> bool {
        kind == CatalogKind::All
            || self
                .sources
                .iter()
                .any(|source| CatalogKind::of(source) == kind)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct CatalogDestination {
    pub(crate) mods_root: PathBuf,
    pub(crate) archives: PathBuf,
    pub(crate) backup_root: PathBuf,
}

impl CatalogDestination {
    pub(crate) fn validate(&self) -> Result<(), String> {
        for (label, path) in [
            ("Global Mods", &self.mods_root),
            ("Mods Archive", &self.archives),
            ("Mods Backup", &self.backup_root),
        ] {
            if path.as_os_str().is_empty() {
                return Err(format!("Set the {label} folder in Settings / Paths"));
            }
            if path.exists() && !path.is_dir() {
                return Err(format!("{label} must be a folder"));
            }
        }
        let roots = [&self.mods_root, &self.archives, &self.backup_root]
            .map(|root| crate::app::app_step2_update_download::destination_key(root));
        if roots[0] == roots[1] || roots[0] == roots[2] || roots[1] == roots[2] {
            return Err(
                "Choose separate Mods, Archive and Backup folders in Settings / Paths".to_string(),
            );
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub(crate) struct CatalogTransferSnapshot {
    pub(crate) generation: u64,
    pub(crate) sources: Vec<ModDownloadSource>,
    pub(crate) destination: CatalogDestination,
}

#[derive(Default)]
pub(crate) struct CatalogScreenState {
    pub(crate) rows: Vec<CatalogRow>,
    pub(crate) query: String,
    pub(crate) kind: CatalogKind,
    pub(crate) selected_types: BTreeSet<String>,
    pub(crate) category_options: BTreeMap<String, String>,
    pub(crate) focused_tp2: Option<String>,
    pub(crate) selected_tp2: BTreeSet<String>,
    pub(crate) source_picks: BTreeMap<String, String>,
    pub(crate) local_presence: BTreeMap<String, LocalPresence>,
    pub(crate) editor_step2: Step2State,
    pub(crate) tiers: SourceTiers,
    pub(crate) loaded: bool,
    pub(crate) error: Option<String>,
    pub(crate) generation: u64,
    pub(crate) scan_destination: PathBuf,
    pub(crate) status: String,
}

impl CatalogScreenState {
    pub(crate) fn replace_from_texts(&mut self, defaults: &str, user: &str) {
        let load = mod_downloads::load_mod_download_sources_from_texts(defaults, user, "");
        self.tiers = mod_downloads::source_tiers_from_texts(defaults, user, "");
        self.replace_sources(load.sources, load.error);
    }

    pub(crate) fn replace_sources(
        &mut self,
        sources: Vec<ModDownloadSource>,
        error: Option<String>,
    ) {
        let mut groups = BTreeMap::<String, Vec<ModDownloadSource>>::new();
        for source in sources {
            groups
                .entry(mod_downloads::normalize_mod_download_tp2(&source.tp2))
                .or_default()
                .push(source);
        }
        let mut category_options = BTreeMap::<String, String>::new();
        self.rows = groups
            .into_iter()
            .map(|(tp2, mut sources)| {
                mod_downloads::sort_sources(&mut sources);
                let name = sources
                    .iter()
                    .find(|s| !s.name.trim().is_empty())
                    .map_or_else(|| tp2.clone(), |s| s.name.clone());
                let search = sources
                    .iter()
                    .map(|source| {
                        format!(
                            "{} {} {} {} {} {} {}",
                            source.name,
                            source.tp2,
                            source.aliases.join(" "),
                            source.source_id,
                            source.source_label,
                            source.github.as_deref().unwrap_or(&source.url),
                            source.categories.join(" ")
                        )
                    })
                    .collect::<Vec<_>>()
                    .join(" ")
                    .to_lowercase();
                let category_keys =
                    CatalogRow::collect_category_keys(&sources, &mut category_options);
                CatalogRow {
                    tp2,
                    name,
                    sources,
                    category_keys,
                    search,
                }
            })
            .collect();
        self.category_options = category_options;
        self.rows
            .sort_by_cached_key(|r| (r.name.to_lowercase(), r.tp2.clone()));
        let keys: BTreeSet<_> = self.rows.iter().map(|r| r.tp2.clone()).collect();
        self.selected_tp2.retain(|key| keys.contains(key));
        self.local_presence.retain(|key, _| keys.contains(key));
        if self
            .focused_tp2
            .as_ref()
            .is_some_and(|key| !keys.contains(key))
        {
            self.focused_tp2 = None;
        }
        self.source_picks.retain(|key, id| {
            self.rows.iter().any(|row| {
                &row.tp2 == key
                    && row
                        .sources
                        .iter()
                        .any(|s| mod_downloads::normalize_source_id(&s.source_id) == *id)
            })
        });
        self.error = error;
        self.loaded = true;
        self.generation = self.generation.wrapping_add(1);
    }

    pub(crate) fn kind_count(&self, kind: CatalogKind) -> usize {
        self.rows
            .iter()
            .filter(|row| row.matches_kind(kind))
            .count()
    }

    pub(crate) fn visible_rows(&self) -> Vec<usize> {
        let query = self.query.trim().to_lowercase();
        self.rows
            .iter()
            .enumerate()
            .filter(|(_, row)| {
                (query.is_empty() || row.search.contains(&query))
                    && row.matches_kind(self.kind)
                    && (self.selected_types.is_empty()
                        || !self.selected_types.is_disjoint(&row.category_keys))
            })
            .map(|(index, _)| index)
            .collect()
    }

    pub(crate) fn focus(&mut self, tp2: &str) {
        self.focused_tp2 = Some(mod_downloads::normalize_mod_download_tp2(tp2));
    }

    pub(crate) fn toggle(&mut self, tp2: &str) {
        let key = mod_downloads::normalize_mod_download_tp2(tp2);
        if self.rows.iter().any(|row| row.tp2 == key) && !self.selected_tp2.remove(&key) {
            self.selected_tp2.insert(key);
        }
    }

    pub(crate) fn toggle_visible(&mut self) {
        let keys: Vec<_> = self
            .visible_rows()
            .iter()
            .map(|&i| self.rows[i].tp2.clone())
            .collect();
        let clear = keys.iter().all(|key| self.selected_tp2.contains(key));
        for key in keys {
            if clear {
                self.selected_tp2.remove(&key);
            } else {
                self.selected_tp2.insert(key);
            }
        }
    }

    pub(crate) fn choose_source(&mut self, tp2: &str, id: &str) {
        let key = mod_downloads::normalize_mod_download_tp2(tp2);
        let id = mod_downloads::normalize_source_id(id);
        if self.rows.iter().any(|row| {
            row.tp2 == key
                && row
                    .sources
                    .iter()
                    .any(|s| mod_downloads::normalize_source_id(&s.source_id) == id)
        }) {
            self.source_picks.insert(key, id);
            self.editor_step2.forks_list = ForksListState::default();
            self.editor_step2.mod_download_forks.clear();
        }
    }

    pub(crate) fn source(&self, tp2: &str) -> Option<&ModDownloadSource> {
        let key = mod_downloads::normalize_mod_download_tp2(tp2);
        let row = self.rows.iter().find(|row| row.tp2 == key)?;
        self.source_picks
            .get(&key)
            .and_then(|id| {
                row.sources
                    .iter()
                    .find(|s| mod_downloads::normalize_source_id(&s.source_id) == *id)
            })
            .or_else(|| row.sources.first())
    }

    pub(crate) fn snapshot(
        &self,
        keys: &BTreeSet<String>,
        destination: CatalogDestination,
        wizard_busy: bool,
    ) -> Result<CatalogTransferSnapshot, String> {
        if wizard_busy {
            return Err("Wait for the current download or extraction to finish".to_string());
        }
        destination.validate()?;
        let sources = keys
            .iter()
            .filter_map(|key| self.source(key).cloned())
            .collect::<Vec<_>>();
        if sources.is_empty() {
            return Err("Select at least one mod".to_string());
        }
        Ok(CatalogTransferSnapshot {
            generation: self.generation,
            sources,
            destination,
        })
    }

    pub(crate) fn open_editor(&mut self, tp2: &str) {
        let Some(source) = self.source(tp2).cloned() else {
            return;
        };
        let mut form = source_form::from_source(
            &source,
            SourceFormIdentity::default(),
            ModSourceEditDestination::GlobalDefault,
            &format!("catalog:{}", mod_downloads::normalize_mod_download_tp2(tp2)),
        );
        let history = crate::app::mod_source_history::load_store();
        let signature = crate::app::mod_source_history::rule_signature(&source);
        let note_key = crate::app::mod_source_history::note_key(&source.tp2, &signature);
        if let Some(note) = history.notes.get(&note_key) {
            form.note.clone_from(&note.text);
        }
        form.note_seed.clone_from(&form.note);
        form.note_who = "My default".to_string();
        self.editor_step2.versions_ui.sheet_tp2 = Some(form.card_key.clone());
        self.editor_step2.versions_ui.source_form = Some(form);
        self.editor_step2.versions_ui.sheet = Some(VersionsSheet::EditSource);
    }

    pub(crate) fn accept_scan(
        &mut self,
        generation: u64,
        destination: &std::path::Path,
        result: BTreeMap<String, LocalPresence>,
    ) -> bool {
        if generation != self.generation || destination != self.scan_destination {
            return false;
        }
        self.local_presence = result;
        true
    }

    pub(crate) fn accept_forks(
        &mut self,
        repo: &str,
        result: Result<Vec<Step2DiscoveredFork>, String>,
    ) -> bool {
        if self.editor_step2.forks_list.repo != repo {
            return false;
        }
        match result {
            Ok(forks) => {
                self.editor_step2.mod_download_forks.clone_from(&forks);
                self.editor_step2.forks_list.status = ForksStatus::Ready(forks);
                self.editor_step2.mod_download_forks_popup_error = None;
            }
            Err(error) => {
                self.editor_step2.mod_download_forks_popup_error = Some(error.clone());
                self.editor_step2.forks_list.status = ForksStatus::Failed(error);
            }
        }
        true
    }

    pub(crate) fn accept_releases(
        &mut self,
        repo: &str,
        result: Result<Vec<CachedRelease>, String>,
    ) -> bool {
        let list = &mut self.editor_step2.versions_ui.release_list;
        if list.repo != repo {
            return false;
        }
        list.status = match result {
            Ok(releases) => ReleaseListStatus::Ready(releases),
            Err(error) => ReleaseListStatus::Failed(error),
        };
        true
    }
}
