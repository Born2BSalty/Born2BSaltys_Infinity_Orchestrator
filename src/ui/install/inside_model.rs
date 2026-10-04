// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::BTreeMap;

use crate::app::app_step2_update_source_refs::ModSourceRefsFile;
use crate::app::mod_downloads::{
    ModDownloadSource, SourceTier, SourceTiers, normalize_mod_download_tp2, source_link_label,
    source_open_url,
};
use crate::app::modlist_share::ModlistSharePreview;
use crate::mods::component::Component;
use crate::ui::install::preview_counts::distinct_mod_count;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct InsideModel {
    pub(crate) sections: Vec<GameSection>,
    pub(crate) mod_count: usize,
    pub(crate) component_count: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct GameSection {
    pub(crate) game: String,
    pub(crate) mods: Vec<ModGroup>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ModGroup {
    pub(crate) folder: String,
    pub(crate) tp_file: String,
    pub(crate) version: String,
    pub(crate) source: ResolvedSource,
    pub(crate) components: Vec<ComponentRow>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ComponentRow {
    pub(crate) id: String,
    pub(crate) label: String,
    pub(crate) wlb_inputs: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum ResolvedSource {
    Link {
        url: String,
        label: String,
        tier: SourceTier,
    },
    None,
}

pub(crate) fn build(preview: &ModlistSharePreview, tiers: &SourceTiers) -> InsideModel {
    let installed_refs = installed_refs_map(&preview.installed_refs_text);
    let sections = section_texts(preview)
        .into_iter()
        .map(|(game, text)| GameSection {
            game,
            mods: parse_section(text, tiers, &installed_refs),
        })
        .collect();

    InsideModel {
        sections,
        mod_count: distinct_mod_count(&preview.bgee_log_text, &preview.bg2ee_log_text),
        component_count: preview.bgee_entries + preview.bg2ee_entries,
    }
}

pub(crate) fn section_texts(preview: &ModlistSharePreview) -> Vec<(String, &str)> {
    if preview.game_install.eq_ignore_ascii_case("EET") {
        vec![
            ("BGEE".to_string(), preview.bgee_log_text.as_str()),
            ("BG2EE".to_string(), preview.bg2ee_log_text.as_str()),
        ]
    } else if preview.game_install.eq_ignore_ascii_case("BG2EE") {
        vec![(
            preview.game_install.clone(),
            preview.bg2ee_log_text.as_str(),
        )]
    } else {
        vec![(preview.game_install.clone(), preview.bgee_log_text.as_str())]
    }
}

pub(crate) fn group_matches(group: &ModGroup, query_lower: &str) -> bool {
    group.folder.to_ascii_lowercase().contains(query_lower)
        || group.tp_file.to_ascii_lowercase().contains(query_lower)
}

pub(crate) fn component_matches(row: &ComponentRow, query_lower: &str) -> bool {
    row.label.to_ascii_lowercase().contains(query_lower)
        || row.id.to_ascii_lowercase().contains(query_lower)
}

pub(crate) fn filter_section(section: &GameSection, query_lower: &str) -> Vec<(usize, Vec<usize>)> {
    if query_lower.is_empty() {
        return section
            .mods
            .iter()
            .enumerate()
            .map(|(index, group)| (index, (0..group.components.len()).collect()))
            .collect();
    }

    section
        .mods
        .iter()
        .enumerate()
        .filter_map(|(index, group)| {
            if group_matches(group, query_lower) {
                return Some((index, (0..group.components.len()).collect()));
            }
            let rows: Vec<usize> = group
                .components
                .iter()
                .enumerate()
                .filter(|(_, row)| component_matches(row, query_lower))
                .map(|(row_index, _)| row_index)
                .collect();
            (!rows.is_empty()).then_some((index, rows))
        })
        .collect()
}

fn parse_section(
    text: &str,
    tiers: &SourceTiers,
    installed_refs: &BTreeMap<String, String>,
) -> Vec<ModGroup> {
    let mut groups: Vec<ModGroup> = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with("//") {
            continue;
        }
        let Ok(component) = Component::parse_weidu_line(line) else {
            continue;
        };
        let row = ComponentRow {
            id: component.component.clone(),
            label: component_label(&component),
            wlb_inputs: component.wlb_inputs.clone(),
        };
        let folder = if component.name.is_empty() {
            component.tp_file.clone()
        } else {
            component.name.clone()
        };
        if let Some(last) = groups.last_mut()
            && last.folder.eq_ignore_ascii_case(&folder)
            && last.tp_file.eq_ignore_ascii_case(&component.tp_file)
        {
            last.components.push(row);
            continue;
        }
        let resolved = tiers.resolve(&component.tp_file);
        groups.push(ModGroup {
            folder,
            tp_file: component.tp_file.clone(),
            version: display_version(&component, resolved.as_ref(), installed_refs),
            source: resolve_source(resolved),
            components: vec![row],
        });
    }
    groups
}

fn installed_refs_map(text: &str) -> BTreeMap<String, String> {
    toml::from_str::<ModSourceRefsFile>(text)
        .map(|file| file.refs)
        .unwrap_or_default()
}

fn display_version(
    component: &Component,
    resolved: Option<&(ModDownloadSource, SourceTier)>,
    installed_refs: &BTreeMap<String, String>,
) -> String {
    let log_version = component.version.trim();
    if !log_version.is_empty() {
        return log_version.to_string();
    }
    if let Some(pin) = resolved.and_then(|(source, _)| pin_label(source)) {
        return pin;
    }
    installed_refs
        .get(&normalize_mod_download_tp2(&component.tp_file))
        .and_then(|value| installed_ref_label(value))
        .unwrap_or_default()
}

fn pin_label(source: &ModDownloadSource) -> Option<String> {
    let tag = source
        .tag
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());
    let branch = source
        .branch
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());
    tag.map(str::to_string)
        .or_else(|| branch.map(|branch| format!("branch: {branch}")))
}

fn installed_ref_label(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() || trimmed.starts_with("commit@") {
        return None;
    }
    if let Some((branch, _)) = trimmed.split_once('@') {
        return Some(format!("branch: {branch}"));
    }
    Some(trimmed.to_string())
}

fn component_label(component: &Component) -> String {
    if component.sub_component.is_empty() {
        component.component_name.clone()
    } else {
        format!(
            "{} -> {}",
            component.component_name, component.sub_component
        )
    }
}

fn resolve_source(resolved: Option<(ModDownloadSource, SourceTier)>) -> ResolvedSource {
    let Some((source, tier)) = resolved else {
        return ResolvedSource::None;
    };
    let Some(url) = source_open_url(&source) else {
        return ResolvedSource::None;
    };
    let label = source_link_label(&url);
    ResolvedSource::Link { url, label, tier }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::mod_downloads::source_tiers_from_texts;

    fn sample_preview() -> ModlistSharePreview {
        ModlistSharePreview {
            bio_version: "0.1.0-test".to_string(),
            game_install: "EET".to_string(),
            game_version: None,
            install_mode: "start_from_weidu_logs_then_review_edit".to_string(),
            bgee_entries: 0,
            bg2ee_entries: 0,
            has_source_overrides: false,
            has_installed_refs: false,
            bgee_log_text: String::new(),
            bg2ee_log_text: String::new(),
            source_overrides_text: String::new(),
            installed_refs_text: String::new(),
            mod_config_count: 0,
            mod_configs_text: String::new(),
            allow_auto_install: true,
            name: None,
            author: None,
            description: None,
            forked_from: Vec::new(),
            unresolved_mods: Vec::new(),
        }
    }

    fn empty_tiers() -> SourceTiers {
        source_tiers_from_texts("", "", "")
    }

    fn empty_refs() -> BTreeMap<String, String> {
        BTreeMap::new()
    }

    #[test]
    fn eet_preview_yields_bgee_then_bg2ee_sections_in_log_order() {
        let mut preview = sample_preview();
        preview.bgee_log_text = "~A\\A.TP2~ #0 #0 // A: v1".to_string();
        preview.bg2ee_log_text = "~B\\B.TP2~ #0 #0 // B: v1".to_string();

        let sections = section_texts(&preview);
        assert_eq!(sections.len(), 2);
        assert_eq!(sections[0].0, "BGEE");
        assert_eq!(sections[0].1, preview.bgee_log_text);
        assert_eq!(sections[1].0, "BG2EE");
        assert_eq!(sections[1].1, preview.bg2ee_log_text);
    }

    #[test]
    fn single_game_preview_yields_one_section_named_after_the_game() {
        let mut preview = sample_preview();
        preview.game_install = "BGEE".to_string();
        preview.bgee_log_text = "~A\\A.TP2~ #0 #0 // A: v1".to_string();

        let sections = section_texts(&preview);
        assert_eq!(sections.len(), 1);
        assert_eq!(sections[0].0, "BGEE");
        assert_eq!(sections[0].1, preview.bgee_log_text);
    }

    #[test]
    fn iwdee_preview_from_an_iwdee_keyed_payload_yields_one_iwdee_section() {
        let json = r#"{
            "format_version": 1,
            "game_install": "IWDEE",
            "install_mode": "start_from_scratch",
            "weidu_logs": { "iwdee": "~MOD/MOD.TP2~ #0 #0 // A component: 1.0" }
        }"#;
        let code = crate::app::modlist_share::encode_share_payload_text(json).expect("encode");
        let preview = crate::app::modlist_share::preview_modlist_share_code(&code)
            .expect("preview must build");

        let sections = section_texts(&preview);
        assert_eq!(sections.len(), 1);
        assert_eq!(sections[0].0, "IWDEE");
        assert_eq!(sections[0].1, preview.bgee_log_text);
        assert_eq!(preview.bgee_entries, 1);
        assert!(sections[0].1.contains("A component"));
    }

    #[test]
    fn bg2ee_only_preview_reads_the_bg2ee_log() {
        let mut preview = sample_preview();
        preview.game_install = "BG2EE".to_string();
        preview.bg2ee_log_text = "~A\\A.TP2~ #0 #0 // A: v1".to_string();

        let sections = section_texts(&preview);
        assert_eq!(sections.len(), 1);
        assert_eq!(sections[0].0, "BG2EE");
        assert_eq!(sections[0].1, preview.bg2ee_log_text);
    }

    #[test]
    fn consecutive_lines_of_one_mod_form_one_group_and_a_repeat_forms_a_new_one() {
        let text = "\
~A\\A.TP2~ #0 #0 // A comp one: v1
~A\\A.TP2~ #0 #1 // A comp two: v1
~B\\B.TP2~ #0 #0 // B comp one: v1
~A\\A.TP2~ #0 #2 // A comp three: v1";
        let groups = parse_section(text, &empty_tiers(), &empty_refs());

        assert_eq!(groups.len(), 3);
        assert_eq!(groups[0].folder, "A");
        assert_eq!(groups[0].components.len(), 2);
        assert_eq!(groups[1].folder, "B");
        assert_eq!(groups[1].components.len(), 1);
        assert_eq!(groups[2].folder, "A");
        assert_eq!(groups[2].components.len(), 1);
    }

    #[test]
    fn a_line_without_a_folder_is_titled_by_its_file_and_groups_with_its_siblings() {
        let text = "\
~SETUP-D0QUESTPACK.TP2~ #0 #5 // Additional Shadow Thieves Content: v3.5
~SETUP-D0QUESTPACK.TP2~ #0 #6 // Alternative Harper/Xzar Plot: v3.5";
        let groups = parse_section(text, &empty_tiers(), &empty_refs());

        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].folder, "SETUP-D0QUESTPACK.TP2");
        assert_eq!(groups[0].tp_file, "SETUP-D0QUESTPACK.TP2");
        assert_eq!(groups[0].components.len(), 2);
    }

    #[test]
    fn component_label_joins_the_subcomponent_with_an_arrow() {
        let text = "~EET/EET.TP2~ #0 #0 // EET core (resource importation)->Default: v14.0";
        let groups = parse_section(text, &empty_tiers(), &empty_refs());

        assert_eq!(
            groups[0].components[0].label,
            "EET core (resource importation) -> Default"
        );
    }

    #[test]
    fn wlb_inputs_ride_on_the_component_row() {
        let text = r"~EET\EET.TP2~ #0 #0 // EET core: v14.0 // @wlb-inputs: y,D:\test1";
        let groups = parse_section(text, &empty_tiers(), &empty_refs());

        assert_eq!(
            groups[0].components[0].wlb_inputs.as_deref(),
            Some("y,D:\\test1")
        );
    }

    #[test]
    fn comment_and_blank_lines_are_ignored() {
        let text = "\
// header comment

~A\\A.TP2~ #0 #0 // A comp one: v1

// trailing comment";
        let groups = parse_section(text, &empty_tiers(), &empty_refs());

        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].components.len(), 1);
    }

    #[test]
    fn group_and_component_matching_is_case_insensitive_over_folder_tp2_label_and_id() {
        let group = ModGroup {
            folder: "DlcMerger".to_string(),
            tp_file: "DLCMERGER.TP2".to_string(),
            version: String::new(),
            source: ResolvedSource::None,
            components: Vec::new(),
        };
        assert!(group_matches(&group, "dlcmerger"));
        assert!(group_matches(&group, "tp2"));

        let row = ComponentRow {
            id: "5000".to_string(),
            label: "EET End".to_string(),
            wlb_inputs: None,
        };
        assert!(component_matches(&row, "eet end"));
        assert!(component_matches(&row, "5000"));
    }

    #[test]
    fn filter_keeps_every_row_of_a_matching_group_and_only_matching_rows_otherwise() {
        let section = GameSection {
            game: "BGEE".to_string(),
            mods: vec![
                ModGroup {
                    folder: "FixPack".to_string(),
                    tp_file: "FIXPACK.TP2".to_string(),
                    version: String::new(),
                    source: ResolvedSource::None,
                    components: vec![
                        ComponentRow {
                            id: "0".to_string(),
                            label: "Fix one".to_string(),
                            wlb_inputs: None,
                        },
                        ComponentRow {
                            id: "1".to_string(),
                            label: "Other".to_string(),
                            wlb_inputs: None,
                        },
                    ],
                },
                ModGroup {
                    folder: "DlcMerger".to_string(),
                    tp_file: "DLCMERGER.TP2".to_string(),
                    version: String::new(),
                    source: ResolvedSource::None,
                    components: vec![ComponentRow {
                        id: "0".to_string(),
                        label: "Fix related".to_string(),
                        wlb_inputs: None,
                    }],
                },
            ],
        };

        let filtered = filter_section(&section, "fix");
        assert_eq!(filtered.len(), 2);
        assert_eq!(filtered[0], (0, vec![0, 1]));
        assert_eq!(filtered[1], (1, vec![0]));
    }

    #[test]
    fn resolved_source_carries_url_label_and_tier_and_unknown_is_none() {
        let default_text = "[[mods]]\nname = \"EET\"\ntp2 = \"eet\"\n\n  [[mods.sources]]\n  id = \"main\"\n  label = \"Main\"\n  type = \"github\"\n  url = \"https://github.com/Owner/Repo\"\n";
        let tiers = source_tiers_from_texts(default_text, "", "");

        let known = resolve_source(tiers.resolve("EET.TP2"));
        assert_eq!(
            known,
            ResolvedSource::Link {
                url: "https://github.com/Owner/Repo".to_string(),
                label: "github.com/Owner/Repo".to_string(),
                tier: SourceTier::Default,
            }
        );

        let unknown = resolve_source(tiers.resolve("UNKNOWN.TP2"));
        assert_eq!(unknown, ResolvedSource::None);
    }

    #[test]
    fn group_version_prefers_the_log_line_version() {
        let modlist_toml = r#"[[mods]]
tp2 = "eeex"

  [[mods.sources]]
  id = "bubb13"
  label = "Bubb13"
  type = "github"
  url = "https://github.com/Bubb13/EEex"
  repo = "Bubb13/EEex"
  tag = "v1.2.0"
"#;
        let tiers = source_tiers_from_texts("", "", modlist_toml);
        let text = "~EEEX\\EEEX.TP2~ #0 #0 // EEex: v1.1.0";

        let groups = parse_section(text, &tiers, &empty_refs());

        assert_eq!(groups[0].version, "v1.1.0");
    }

    #[test]
    fn group_version_falls_back_to_the_tag_pin() {
        let modlist_toml = r#"[[mods]]
tp2 = "eeex"

  [[mods.sources]]
  id = "bubb13"
  label = "Bubb13"
  type = "github"
  url = "https://github.com/Bubb13/EEex"
  repo = "Bubb13/EEex"
  tag = "v1.2.0"
"#;
        let tiers = source_tiers_from_texts("", "", modlist_toml);
        let text = "~EEEX\\EEEX.TP2~ #0 #0 // EEex";

        let groups = parse_section(text, &tiers, &empty_refs());

        assert_eq!(groups[0].version, "v1.2.0");
    }

    #[test]
    fn group_version_shows_the_branch_pin_by_name() {
        let modlist_toml = r#"[[mods]]
tp2 = "cdtweaks"

  [[mods.sources]]
  id = "gibberlings3"
  label = "Gibberlings3"
  type = "github"
  url = "https://github.com/Gibberlings3/Tweaks-Anthology"
  repo = "Gibberlings3/Tweaks-Anthology"
  branch = "master"
"#;
        let tiers = source_tiers_from_texts("", "", modlist_toml);
        let text = "~CDTWEAKS\\CDTWEAKS.TP2~ #0 #0 // CDTweaks";

        let groups = parse_section(text, &tiers, &empty_refs());

        assert_eq!(groups[0].version, "branch: master");
    }

    #[test]
    fn group_version_hides_a_commit_pin() {
        let modlist_toml = r#"[[mods]]
tp2 = "eeex"

  [[mods.sources]]
  id = "bubb13"
  label = "Bubb13"
  type = "github"
  url = "https://github.com/Bubb13/EEex"
  repo = "Bubb13/EEex"
  commit = "a1b2c3d4e5f60718293a4b5c6d7e8f9012345678"
"#;
        let tiers = source_tiers_from_texts("", "", modlist_toml);
        let text = "~EEEX\\EEEX.TP2~ #0 #0 // EEex";

        let groups = parse_section(text, &tiers, &empty_refs());

        assert_eq!(groups[0].version, "");
    }

    #[test]
    fn group_version_falls_back_to_the_installed_ref() {
        let text = "~EEEX\\EEEX.TP2~ #0 #0 // EEex";

        let mut refs = empty_refs();
        refs.insert("eeex".to_string(), "v1.2.0".to_string());
        let groups = parse_section(text, &empty_tiers(), &refs);
        assert_eq!(groups[0].version, "v1.2.0");
    }

    #[test]
    fn installed_branch_ref_shows_the_branch_name() {
        assert_eq!(
            installed_ref_label("master@7649ced6cd25"),
            Some("branch: master".to_string())
        );
    }

    #[test]
    fn installed_commit_ref_shows_nothing() {
        assert_eq!(
            installed_ref_label("commit@bfd167f7a52dfa6c9e694955a074a85991b0c358"),
            None
        );
    }

    #[test]
    fn group_version_is_empty_without_any_source() {
        let text = "~EEEX\\EEEX.TP2~ #0 #0 // EEex";

        let groups = parse_section(text, &empty_tiers(), &empty_refs());

        assert_eq!(groups[0].version, "");
    }
}
