// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use anyhow::{Result, anyhow};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Component {
    pub tp_file: String,
    pub name: String,
    pub lang: String,
    pub component: String,
    pub component_name: String,
    pub sub_component: String,
    pub version: String,
    pub wlb_inputs: Option<String>,
}

impl Component {
    pub fn parse_weidu_line(line: &str) -> Result<Self> {
        let wlb_inputs = extract_wlb_inputs(line);
        let mut parts = line.split('~');
        let install_path = parts
            .nth(1)
            .ok_or_else(|| anyhow!("missing install path in line: {line}"))?;
        if install_path.trim().is_empty() {
            return Err(anyhow!("invalid install path in line: {line}"));
        }

        let (name, tp_file) = install_path
            .rsplit_once('\\')
            .or_else(|| install_path.rsplit_once('/'))
            .unwrap_or(("", install_path));

        let lang_component_part = parts
            .next()
            .ok_or_else(|| anyhow!("missing lang/component in line: {line}"))?;
        let mut tail = lang_component_part.split("//");
        let mut lang_and_component = tail.next().unwrap_or_default().split_whitespace();
        let lang = lang_and_component
            .next()
            .unwrap_or_default()
            .replace('#', "");
        let component = lang_and_component
            .next()
            .unwrap_or_default()
            .replace('#', "");

        let details = tail.next().unwrap_or_default().trim();
        let (names_text, version) = details
            .rsplit_once(':')
            .filter(|(_, version)| looks_like_version_tail(version))
            .map(|(names, version)| (names.trim_end(), version.trim().to_string()))
            .unwrap_or((details, String::new()));
        let mut names = names_text.split("->");
        let component_name = names.next().unwrap_or_default().trim().to_string();
        let sub_component = names.next().unwrap_or_default().trim().to_string();

        Ok(Self {
            tp_file: tp_file.to_string(),
            name: name.to_string(),
            lang,
            component,
            component_name,
            sub_component,
            version,
            wlb_inputs,
        })
    }

    #[must_use]
    pub fn key_eq(&self, other: &Self) -> bool {
        self.tp_file.eq_ignore_ascii_case(&other.tp_file)
            && self.name.eq_ignore_ascii_case(&other.name)
            && self.lang.eq_ignore_ascii_case(&other.lang)
            && self.component.eq_ignore_ascii_case(&other.component)
    }

    #[must_use]
    pub fn strict_eq(&self, other: &Self) -> bool {
        self.key_eq(other)
            && self.component_name == other.component_name
            && self.sub_component == other.sub_component
            && self.version == other.version
    }
}

fn looks_like_version_tail(tail: &str) -> bool {
    let trimmed = tail.trim();
    if trimmed.is_empty() {
        return false;
    }
    if trimmed.contains("->") || trimmed.contains('(') || trimmed.contains(')') {
        return false;
    }
    let words: Vec<&str> = trimmed.split_whitespace().collect();
    match words.as_slice() {
        [word] => {
            word.bytes().any(|b| b.is_ascii_digit())
                || word
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || matches!(c, '.' | '-' | '_'))
        }
        [_, _] => trimmed.bytes().any(|b| b.is_ascii_digit()),
        _ => false,
    }
}

fn extract_wlb_inputs(line: &str) -> Option<String> {
    let marker = "@wlb-inputs:";
    let lower = line.to_ascii_lowercase();
    let start = lower.find(marker)?;
    let tail = line[start + marker.len()..].trim();
    if tail.is_empty() {
        None
    } else {
        Some(tail.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::{Component, looks_like_version_tail};

    #[test]
    fn parse_windows_line() {
        let line = r"~BG1UB\BG1UB.TP2~ #0 #3 // Angelo Notices Shar-teel: v17.1";
        let c = Component::parse_weidu_line(line).expect("parse should succeed");
        assert_eq!(c.name, "BG1UB");
        assert_eq!(c.tp_file, "BG1UB.TP2");
        assert_eq!(c.lang, "0");
        assert_eq!(c.component, "3");
        assert_eq!(c.component_name, "Angelo Notices Shar-teel");
        assert_eq!(c.version, "v17.1");
    }

    #[test]
    fn parse_root_level_line_has_no_folder() {
        let line = "~SETUP-D0QUESTPACK.TP2~ #0 #5 // Additional Shadow Thieves Content: v3.5";
        let c = Component::parse_weidu_line(line).expect("parse should succeed");
        assert_eq!(c.name, "");
        assert_eq!(c.tp_file, "SETUP-D0QUESTPACK.TP2");
        assert_eq!(c.lang, "0");
        assert_eq!(c.component, "5");
        assert_eq!(c.component_name, "Additional Shadow Thieves Content");
        assert_eq!(c.version, "v3.5");
    }

    #[test]
    fn parse_empty_install_path_still_fails() {
        let err = Component::parse_weidu_line("~~ #0 #0 // x").expect_err("parse should fail");
        assert!(err.to_string().contains("invalid install path"));
    }

    #[test]
    fn parse_unix_line_with_subcomponent() {
        let line = "~EET/EET.TP2~ #0 #0 // EET core (resource importation)->Default: v14.0";
        let c = Component::parse_weidu_line(line).expect("parse should succeed");
        assert_eq!(c.name, "EET");
        assert_eq!(c.tp_file, "EET.TP2");
        assert_eq!(c.lang, "0");
        assert_eq!(c.component, "0");
        assert_eq!(c.component_name, "EET core (resource importation)");
        assert_eq!(c.sub_component, "Default");
        assert_eq!(c.version, "v14.0");
    }

    #[test]
    fn strict_and_key_match_behave_differently() {
        let a = Component {
            tp_file: "A.TP2".into(),
            name: "A".into(),
            lang: "0".into(),
            component: "1".into(),
            component_name: "X".into(),
            sub_component: String::new(),
            version: "v1".into(),
            wlb_inputs: None,
        };
        let b = Component {
            component_name: "Y".into(),
            ..a.clone()
        };
        assert!(a.key_eq(&b));
        assert!(!a.strict_eq(&b));
    }

    #[test]
    fn parse_component_name_with_colon_before_version() {
        let line = r"~BG1AERIE\SETUP-BG1AERIE.TP2~ #0 #5000 // Aerie for BG:EE: 2.5";
        let c = Component::parse_weidu_line(line).expect("parse should succeed");
        assert_eq!(c.component_name, "Aerie for BG:EE");
        assert_eq!(c.version, "2.5");
    }

    #[test]
    fn parse_subcomponent_with_colons_before_version() {
        let line = r"~SOA\SETUP-SOA.TP2~ #0 #0 // The Stone of Askavar for TotSC/Tutu/BGT/BGEE -> Default version: areas connected by travel triggers: 2.8";
        let c = Component::parse_weidu_line(line).expect("parse should succeed");
        assert_eq!(
            c.component_name,
            "The Stone of Askavar for TotSC/Tutu/BGT/BGEE"
        );
        assert_eq!(
            c.sub_component,
            "Default version: areas connected by travel triggers"
        );
        assert_eq!(c.version, "2.8");
    }

    #[test]
    fn parse_wlb_inputs_marker() {
        let line = r"~EET\EET.TP2~ #0 #0 // EET core: v14.0 // @wlb-inputs: y,D:\test1";
        let c = Component::parse_weidu_line(line).expect("parse should succeed");
        assert_eq!(c.wlb_inputs.as_deref(), Some("y,D:\\test1"));
    }

    #[test]
    fn version_tail_rule_accepts_the_reference_corpus() {
        let accept = [
            "1.6.9",
            "30",
            "1.02",
            "35.21",
            "v18",
            "v1.3.10-alpha",
            "v0.4.1-alpha",
            "V 2.4",
            "Alpha 3",
            "devel",
        ];
        for tail in accept {
            assert!(looks_like_version_tail(tail), "expected accept: {tail}");
        }
    }

    #[test]
    fn version_tail_rule_rejects_label_tails() {
        let reject = [
            "Items",
            "Inspirations",
            "EE",
            "EE)",
            "Main Component",
            "Troubadour Kit",
            "Skald Overhaul",
            "Not-So-Indestructible Rats",
            "Enhanced Edition",
            "Race Text Patch",
            "-> Artificer (Bard, EEex required)",
            "3e Alignment for Bards (any non-lawful)",
            "Edwin -> Use BG2 values",
            "EE Colors (Green icons for summoning spells)",
            "LShift-on-hover to view spells affecting creature",
        ];
        for tail in reject {
            assert!(!looks_like_version_tail(tail), "expected reject: {tail}");
        }
    }

    #[test]
    fn parse_line_without_version_keeps_a_colon_inside_the_label() {
        let line = r"~EEex\EEEX.TP2~ #0 #2 // Enable effect menu module: LShift-on-hover to view spells affecting creature";
        let c = Component::parse_weidu_line(line).expect("parse should succeed");
        assert_eq!(
            c.component_name,
            "Enable effect menu module: LShift-on-hover to view spells affecting creature"
        );
        assert_eq!(c.version, "");
    }

    #[test]
    fn parse_line_without_version_keeps_a_colon_and_a_subcomponent() {
        let line =
            r"~CDTweaks\SETUP-CDTWEAKS.TP2~ #0 #4031 // Consistent Stats: Edwin -> Use BG2 values";
        let c = Component::parse_weidu_line(line).expect("parse should succeed");
        assert_eq!(c.component_name, "Consistent Stats: Edwin");
        assert_eq!(c.sub_component, "Use BG2 values");
        assert_eq!(c.version, "");
    }

    #[test]
    fn parse_line_with_game_suffix_and_no_version() {
        let line =
            r"~HOUSETWEAKS\HOUSETWEAKS.TP2~ #0 #14 // House Tweaks: Improved Dialogues (BG:EE)";
        let c = Component::parse_weidu_line(line).expect("parse should succeed");
        assert_eq!(c.component_name, "House Tweaks: Improved Dialogues (BG:EE)");
        assert_eq!(c.version, "");
    }

    #[test]
    fn parse_line_with_two_word_version() {
        let line = r"~EEFIXPACK\SETUP-EEFIXPACK.TP2~ #0 #0 // Core Fixes: Alpha 3";
        let c = Component::parse_weidu_line(line).expect("parse should succeed");
        assert_eq!(c.component_name, "Core Fixes");
        assert_eq!(c.version, "Alpha 3");
    }

    #[test]
    fn parse_line_with_devel_version() {
        let line = r"~EEEX\EEEX.TP2~ #0 #2 // Enable effect menu module: LShift-on-hover to view spells affecting creature: devel";
        let c = Component::parse_weidu_line(line).expect("parse should succeed");
        assert_eq!(
            c.component_name,
            "Enable effect menu module: LShift-on-hover to view spells affecting creature"
        );
        assert_eq!(c.version, "devel");
    }

    #[test]
    fn parse_line_with_version_tail_and_prompt_marker() {
        let line = r"~EET\EET.TP2~ #0 #0 // EET core (resource importation): v14.0 // @wlb-inputs: y,C:\BIO\Baldur's Gate Enhanced Edition";
        let c = Component::parse_weidu_line(line).expect("parse should succeed");
        assert_eq!(c.component_name, "EET core (resource importation)");
        assert_eq!(c.version, "v14.0");
        assert_eq!(
            c.wlb_inputs.as_deref(),
            Some(r"y,C:\BIO\Baldur's Gate Enhanced Edition")
        );
    }

    #[test]
    fn parse_line_with_single_word_label_tail() {
        let line = r"~BARDICWONDERS\SETUP-BARDICWONDERS.TP2~ #0 #1 // Bardic Wonders: Items";
        let c = Component::parse_weidu_line(line).expect("parse should succeed");
        assert_eq!(c.component_name, "Bardic Wonders: Items");
        assert_eq!(c.version, "");
    }
}
