// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub enum NavDestination {
    #[default]
    Home,
    Install,
    Create,
    Settings,
    Workspace {
        modlist_id: Option<String>,
    },
}

impl NavDestination {
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
            Self::Home => "\u{2302}",
            Self::Install => "\u{2193}",
            Self::Create => "\u{270E}",
            Self::Settings => "\u{2699}",
            Self::Workspace { .. } => "\u{2630}",
        }
    }

    #[must_use]
    pub const fn rail_items() -> [Self; 4] {
        [Self::Home, Self::Install, Self::Create, Self::Settings]
    }

    #[must_use]
    pub const fn is_rail_item(&self) -> bool {
        matches!(
            self,
            Self::Home | Self::Install | Self::Create | Self::Settings,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rail_items_returns_four_in_order() {
        let items = NavDestination::rail_items();
        assert_eq!(items.len(), 4);
        assert_eq!(items[0], NavDestination::Home);
        assert_eq!(items[1], NavDestination::Install);
        assert_eq!(items[2], NavDestination::Create);
        assert_eq!(items[3], NavDestination::Settings);
    }

    #[test]
    fn labels_match_spec_2_1() {
        assert_eq!(NavDestination::Home.label(), "Home");
        assert_eq!(NavDestination::Install.label(), "Install");
        assert_eq!(NavDestination::Create.label(), "Create");
        assert_eq!(NavDestination::Settings.label(), "Settings");
    }

    #[test]
    fn icons_match_spec_2_1() {
        assert_eq!(NavDestination::Home.icon(), "\u{2302}");
        assert_eq!(NavDestination::Install.icon(), "\u{2193}");
        assert_eq!(NavDestination::Create.icon(), "\u{270E}");
        assert_eq!(NavDestination::Settings.icon(), "\u{2699}");
    }

    #[test]
    fn workspace_carries_optional_modlist_id() {
        let a = NavDestination::Workspace { modlist_id: None };
        let b = NavDestination::Workspace {
            modlist_id: Some("modlist-1".to_string()),
        };
        assert_ne!(a, b);
        assert!(!a.is_rail_item());
        assert!(!b.is_rail_item());
    }

    #[test]
    fn default_is_home() {
        assert_eq!(NavDestination::default(), NavDestination::Home);
    }
}
