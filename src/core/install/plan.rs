// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::mods::component::Component;
use crate::mods::log_file::LogFile;

#[derive(Debug, Clone)]
pub struct InstallPlan {
    pub components: Vec<Component>,
}

impl InstallPlan {
    pub fn from_log_file(log_file: &LogFile) -> Self {
        Self {
            components: log_file.components().to_vec(),
        }
    }

    pub fn filter_installed(
        &mut self,
        installed: &LogFile,
        skip_installed: bool,
        strict_matching: bool,
    ) {
        if !skip_installed {
            return;
        }
        self.components.retain(|candidate| {
            !installed.components().iter().any(|existing| {
                if strict_matching {
                    existing.strict_eq(candidate)
                } else {
                    existing.key_eq(candidate)
                }
            })
        });
    }
}

#[cfg(test)]
mod tests {
    use super::InstallPlan;
    use crate::mods::component::Component;
    use crate::mods::log_file::LogFile;

    fn comp(name: &str, tp_file: &str, component: &str, component_name: &str) -> Component {
        Component {
            tp_file: tp_file.to_string(),
            name: name.to_string(),
            lang: "0".to_string(),
            component: component.to_string(),
            component_name: component_name.to_string(),
            sub_component: String::new(),
            version: "v1".to_string(),
            wlb_inputs: None,
        }
    }

    #[test]
    fn skip_installed_false_keeps_all() {
        let target = LogFile::from_components(vec![comp("BG1UB", "BG1UB.TP2", "3", "A")]);
        let installed = LogFile::from_components(vec![comp("BG1UB", "BG1UB.TP2", "3", "A")]);
        let mut plan = InstallPlan::from_log_file(&target);
        plan.filter_installed(&installed, false, false);
        assert_eq!(plan.components.len(), 1);
    }

    #[test]
    fn non_strict_filters_on_key_match() {
        let target = LogFile::from_components(vec![comp("BG1UB", "BG1UB.TP2", "3", "A")]);
        let installed = LogFile::from_components(vec![comp("bg1ub", "bg1ub.tp2", "3", "B")]);
        let mut plan = InstallPlan::from_log_file(&target);
        plan.filter_installed(&installed, true, false);
        assert_eq!(plan.components.len(), 0);
    }

    #[test]
    fn strict_keeps_when_details_differ() {
        let target = LogFile::from_components(vec![comp("BG1UB", "BG1UB.TP2", "3", "A")]);
        let installed = LogFile::from_components(vec![comp("BG1UB", "BG1UB.TP2", "3", "B")]);
        let mut plan = InstallPlan::from_log_file(&target);
        plan.filter_installed(&installed, true, true);
        assert_eq!(plan.components.len(), 1);
    }
}
