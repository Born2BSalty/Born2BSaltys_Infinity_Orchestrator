// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// `NavDestination` — top-level orchestrator destinations.
//
// Per Phase 2 P2.T1: the four rail items (Home / Install / Create / Settings)
// in SPEC §2.1 order, plus an off-rail `Workspace` destination (Phase 6 wires
// the real modlist_id; Phase 2 leaves it `None`).
//
// `rail_items()` returns an **owned** `[NavDestination; 4]` (per L6) — a
// `&'static [NavDestination]` cannot be const-constructed because
// `Workspace { modlist_id: Option<String> }` carries a heap-allocated
// `String` and the enum is not `'static`-constructible at const-eval time.
// The four rail variants are all unit variants, so construction at call time
// is cheap (4 enum discriminants).
//
// SPEC §2.1: Explore is intentionally omitted from v1 alpha (Appendix C
// lists it as a future v2 track).

/// The top-level orchestrator destination router. Variants 1-4 are the rail
/// items in SPEC §2.1 order; `Workspace` is off-rail and reached from the
/// Home stub's dev-mode button in Phase 2 (real entry from Home cards in
/// Phase 5).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum NavDestination {
    Home,
    Install,
    Create,
    Settings,
    Workspace { modlist_id: Option<String> },
}

impl Default for NavDestination {
    fn default() -> Self {
        NavDestination::Home
    }
}

impl NavDestination {
    /// The wireframe-verbatim rail label (SPEC §2.1).
    pub fn label(&self) -> &'static str {
        match self {
            NavDestination::Home => "Home",
            NavDestination::Install => "Install",
            NavDestination::Create => "Create",
            NavDestination::Settings => "Settings",
            NavDestination::Workspace { .. } => "Workspace",
        }
    }

    /// The glyph shown in the rail next to the label (SPEC §2.1).
    pub fn icon(&self) -> &'static str {
        match self {
            NavDestination::Home => "\u{2302}",       // ⌂
            NavDestination::Install => "\u{2193}",    // ↓
            NavDestination::Create => "\u{270E}",     // ✎
            NavDestination::Settings => "\u{2699}",   // ⚙
            NavDestination::Workspace { .. } => "\u{2630}", // ☰ (workspace — not shown in rail)
        }
    }

    /// The four rail items in SPEC §2.1 order. Owned array (L6); the four
    /// rail variants are unit variants so construction is trivial.
    pub fn rail_items() -> [NavDestination; 4] {
        [
            NavDestination::Home,
            NavDestination::Install,
            NavDestination::Create,
            NavDestination::Settings,
        ]
    }

    /// True for the four rail destinations; false for `Workspace`. Used by the
    /// rail-active-state highlight (Home stays "active" while in Workspace per
    /// SPEC §2.1, since Workspace is reached from a Home card).
    pub fn is_rail_item(&self) -> bool {
        matches!(
            self,
            NavDestination::Home
                | NavDestination::Install
                | NavDestination::Create
                | NavDestination::Settings,
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
        let b = NavDestination::Workspace { modlist_id: Some("modlist-1".to_string()) };
        assert_ne!(a, b);
        assert!(!a.is_rail_item());
        assert!(!b.is_rail_item());
    }

    #[test]
    fn default_is_home() {
        assert_eq!(NavDestination::default(), NavDestination::Home);
    }
}
