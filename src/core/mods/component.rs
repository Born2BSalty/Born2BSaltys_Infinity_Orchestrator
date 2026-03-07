// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use anyhow::{anyhow, Result};

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

        let (name, tp_file) = install_path
            .rsplit_once('\\')
            .or_else(|| install_path.rsplit_once('/'))
            .ok_or_else(|| anyhow!("invalid install path in line: {line}"))?;

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

        let mut details = tail.next().unwrap_or_default().split(':');
        let mut names = details.next().unwrap_or_default().split("->");
        let component_name = names.next().unwrap_or_default().trim().to_string();
        let sub_component = names.next().unwrap_or_default().trim().to_string();
        let version = details.next().unwrap_or_default().trim().to_string();

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

    pub fn key_eq(&self, other: &Self) -> bool {
        self.tp_file.eq_ignore_ascii_case(&other.tp_file)
            && self.name.eq_ignore_ascii_case(&other.name)
            && self.lang.eq_ignore_ascii_case(&other.lang)
            && self.component.eq_ignore_ascii_case(&other.component)
    }

    pub fn strict_eq(&self, other: &Self) -> bool {
        self.key_eq(other)
            && self.component_name == other.component_name
            && self.sub_component == other.sub_component
            && self.version == other.version
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
    use super::Component;

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
            sub_component: "".into(),
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
    fn parse_wlb_inputs_marker() {
        let line = r"~EET\EET.TP2~ #0 #0 // EET core: v14.0 // @wlb-inputs: y,D:\test1";
        let c = Component::parse_weidu_line(line).expect("parse should succeed");
        assert_eq!(c.wlb_inputs.as_deref(), Some("y,D:\\test1"));
    }
}
