// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum NavDestination {
    Home,
    Install,
    Create,
    Settings,
    Workspace { modlist_id: Option<String> },
}

impl NavDestination {
    pub fn rail_items() -> [NavDestination; 4] {
        [
            NavDestination::Home,
            NavDestination::Install,
            NavDestination::Create,
            NavDestination::Settings,
        ]
    }

    pub fn label(&self) -> &'static str {
        match self {
            NavDestination::Home => "Home",
            NavDestination::Install => "Install",
            NavDestination::Create => "Create",
            NavDestination::Settings => "Settings",
            NavDestination::Workspace { .. } => "Workspace",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            NavDestination::Home => "⌂",
            NavDestination::Install => "↓",
            NavDestination::Create => "✎",
            NavDestination::Settings => "⚙",
            NavDestination::Workspace { .. } => "",
        }
    }
}
