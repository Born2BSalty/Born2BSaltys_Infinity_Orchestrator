// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use super::scripted_inputs::extract_field;

const SUCCESS_PHRASE: &str = "SUCCESSFULLY INSTALLED";
const BATCH_ENTRY_MARKER: &str = "WeiduComponent {";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct BatchComponent {
    pub(super) tp2_stem: String,
    pub(super) component_id: String,
    pub(super) component_name: String,
}

const BATCH_MARKER: &str = "INSTALLING MOD WEIDUBATCHEDCOMPONENTS(";

pub(super) fn is_batch_line(line: &str) -> bool {
    let upper = line.to_ascii_uppercase();
    upper.contains("MOD_INSTALLER::INSTALLERS") && upper.contains(BATCH_MARKER)
}

pub(super) fn parse_batch_line(line: &str) -> Option<Vec<BatchComponent>> {
    let upper = line.to_ascii_uppercase();
    if !upper.contains("MOD_INSTALLER::INSTALLERS") {
        return None;
    }
    let batch_start = upper.find(BATCH_MARKER)? + BATCH_MARKER.len();
    let entries: Vec<BatchComponent> = line[batch_start..]
        .split(BATCH_ENTRY_MARKER)
        .skip(1)
        .map(parse_batch_entry)
        .collect::<Option<Vec<_>>>()?;
    if entries.is_empty() {
        None
    } else {
        Some(entries)
    }
}

fn parse_batch_entry(entry: &str) -> Option<BatchComponent> {
    let tp_file = extract_field(entry, "tp_file")?;
    let component_id = extract_field(entry, "component")?.trim().to_string();
    if component_id.is_empty() {
        return None;
    }
    let component_name = extract_field(entry, "component_name")
        .map(|name| name.trim().to_string())
        .unwrap_or_default();
    Some(BatchComponent {
        tp2_stem: tp2_stem(&tp_file),
        component_id,
        component_name,
    })
}

pub(super) fn tp2_stem(tp_file: &str) -> String {
    let file_name = tp_file.rsplit(['/', '\\']).next().unwrap_or(tp_file);
    let without_extension = file_name
        .rsplit_once('.')
        .map_or(file_name, |(stem, _)| stem);
    let setup_prefix = "SETUP-";
    let without_setup = if without_extension.len() >= setup_prefix.len()
        && without_extension.is_char_boundary(setup_prefix.len())
        && without_extension[..setup_prefix.len()].eq_ignore_ascii_case(setup_prefix)
    {
        &without_extension[setup_prefix.len()..]
    } else {
        without_extension
    };
    without_setup.to_ascii_uppercase()
}

pub(super) fn prefixed_success_line(line: &str, batch: &[BatchComponent]) -> Option<String> {
    let phrase_start = line.to_ascii_uppercase().find(SUCCESS_PHRASE)?;
    let after_phrase = phrase_start + SUCCESS_PHRASE.len();
    let rest = &line[after_phrase..];
    let success_text = rest.trim_start_matches(' ');
    if success_text.len() == rest.len() || success_text.trim().is_empty() {
        return None;
    }
    let insert_at = line.len() - success_text.len();
    let component = matching_component(success_text.trim_end(), batch)?;
    Some(format!(
        "{}{} #{} {}",
        &line[..insert_at],
        component.tp2_stem,
        component.component_id,
        success_text
    ))
}

fn matching_component<'a>(
    success_text: &str,
    batch: &'a [BatchComponent],
) -> Option<&'a BatchComponent> {
    let by_name = batch.iter().find(|component| {
        !component.component_name.is_empty()
            && (success_text == component.component_name
                || success_text
                    .strip_prefix(component.component_name.as_str())
                    .is_some_and(|tail| tail.starts_with(" -> ")))
    });
    match (by_name, batch) {
        (Some(component), _) | (None, [component]) => Some(component),
        (None, _) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{BatchComponent, parse_batch_line, prefixed_success_line, tp2_stem};

    const TWO_ENTRY_BATCH: &str = "[2026-09-29T03:32:10Z INFO  mod_installer::installers] Installing mod WeiduBatchedComponents([WeiduComponent { tp_file: \"SETUP-EEFIXPACK.TP2\", name: \"EEFixPack\", lang: \"0\", component: \"0\", component_name: \"Core Fixes\", sub_component: \"\", version: \"\" }, WeiduComponent { tp_file: \"SETUP-EEFIXPACK.TP2\", name: \"EEFixPack\", lang: \"0\", component: \"2\", component_name: \"Game Text Update\", sub_component: \"\", version: \"\" }])";
    const DLC_BATCH: &str = "[2026-09-29T03:30:35Z INFO  mod_installer::installers] Installing mod WeiduBatchedComponents([WeiduComponent { tp_file: \"DLCMERGER.TP2\", name: \"DlcMerger\", lang: \"0\", component: \"1\", component_name: \"Merge DLC into game\", sub_component: \"Siege of Dragonspear\", version: \"1.8\" }])";
    const DLC_SUCCESS: &str = "[2026-09-29T03:31:14Z INFO  mod_installer::parser] SUCCESSFULLY INSTALLED      Merge DLC into game -> Merge \"Siege of Dragonspear\" DLC";

    fn component(stem: &str, id: &str, name: &str) -> BatchComponent {
        BatchComponent {
            tp2_stem: stem.to_string(),
            component_id: id.to_string(),
            component_name: name.to_string(),
        }
    }

    #[test]
    fn batch_line_yields_every_component_entry() {
        let batch = parse_batch_line(TWO_ENTRY_BATCH).expect("batch");
        assert_eq!(
            batch,
            vec![
                component("EEFIXPACK", "0", "Core Fixes"),
                component("EEFIXPACK", "2", "Game Text Update"),
            ]
        );
        assert_eq!(parse_batch_line(DLC_SUCCESS), None);
        assert_eq!(
            parse_batch_line(&TWO_ENTRY_BATCH.replace("Installing mod", "Installed mod")),
            None
        );
    }

    #[test]
    fn a_batch_with_an_unparsable_entry_is_not_a_batch() {
        let broken = TWO_ENTRY_BATCH.replacen("component: \"2\"", "component: \"\"", 1);
        assert_eq!(parse_batch_line(&broken), None);
        let core_fixes = "[2026-09-29T03:33:00Z INFO  mod_installer::parser] SUCCESSFULLY INSTALLED      Core Fixes";
        assert_eq!(prefixed_success_line(core_fixes, &[]), None);
    }

    #[test]
    fn stem_drops_setup_prefix_and_extension() {
        assert_eq!(tp2_stem("SETUP-EEFIXPACK.TP2"), "EEFIXPACK");
        assert_eq!(tp2_stem("DLCMERGER.TP2"), "DLCMERGER");
        assert_eq!(tp2_stem("zstweaks/zstweaks.tp2"), "ZSTWEAKS");
    }

    #[test]
    fn success_line_is_matched_by_component_name_with_or_without_subcomponent() {
        let two = parse_batch_line(TWO_ENTRY_BATCH).expect("batch");
        let game_text = "[2026-09-29T03:33:00Z INFO  mod_installer::parser] SUCCESSFULLY INSTALLED      Game Text Update";
        assert_eq!(
            prefixed_success_line(game_text, &two).as_deref(),
            Some(
                "[2026-09-29T03:33:00Z INFO  mod_installer::parser] SUCCESSFULLY INSTALLED      EEFIXPACK #2 Game Text Update"
            )
        );
        let dlc = parse_batch_line(DLC_BATCH).expect("batch");
        assert!(
            prefixed_success_line(DLC_SUCCESS, &dlc)
                .is_some_and(|line| line.contains("DLCMERGER #1 Merge DLC into game -> "))
        );
        let unmatched = "[2026-09-29T03:33:00Z INFO  mod_installer::parser] SUCCESSFULLY INSTALLED      Something Else";
        assert_eq!(prefixed_success_line(unmatched, &two), None);
    }

    #[test]
    fn prefix_lands_after_the_padding_and_keeps_the_phrase() {
        let dlc = parse_batch_line(DLC_BATCH).expect("batch");
        let prefixed = prefixed_success_line(DLC_SUCCESS, &dlc).expect("prefixed");
        assert_eq!(
            prefixed
                .matches("SUCCESSFULLY INSTALLED      DLCMERGER #1 Merge DLC into game -> Merge \"Siege of Dragonspear\" DLC")
                .count(),
            1
        );
        assert_eq!(prefixed.matches("SUCCESSFULLY INSTALLED").count(), 1);
    }
}
