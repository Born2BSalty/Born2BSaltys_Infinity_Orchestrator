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
    #[must_use]
    pub const fn rail_items() -> [Self; 4] {
        [Self::Home, Self::Install, Self::Create, Self::Settings]
    }

    #[must_use]
    pub const fn label(&self) -> &'static str {
        match self {
            Self::Home => "Home",
            Self::Install => "Install",
            Self::Create => "Create",
            Self::Settings => "Settings",
            Self::Workspace { .. } => "Workspace",
        }
    }

    #[must_use]
    pub const fn icon(&self) -> &'static str {
        match self {
            Self::Home => "⌂",
            Self::Install => "↓",
            Self::Create => "✎",
            Self::Settings => "⚙",
            Self::Workspace { .. } => "",
        }
    }
}
