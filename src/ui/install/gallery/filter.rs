// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::registry::model::Game;
use crate::ui::install::gallery::catalog::GalleryEntry;

pub const MIN_CARD_WIDTH_PX: f32 = 300.0;
pub const CARD_GAP_PX: f32 = 20.0;
const MAX_COLUMNS: usize = 4;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GalleryFilter {
    pub search: String,
    pub game: Option<Game>,
    pub starter_only: bool,
}

impl GalleryFilter {
    #[must_use]
    pub fn matches(&self, entry: &GalleryEntry) -> bool {
        if self.starter_only && !entry.starter {
            return false;
        }
        if let Some(game) = self.game
            && entry.game != game
        {
            return false;
        }
        let needle = self.search.trim().to_lowercase();
        if needle.is_empty() {
            return true;
        }
        entry.name.to_lowercase().contains(&needle)
            || entry.author.to_lowercase().contains(&needle)
            || entry
                .game
                .to_legacy_string()
                .to_lowercase()
                .contains(&needle)
            || entry
                .tags
                .iter()
                .any(|tag| tag.to_lowercase().contains(&needle))
    }
}

#[must_use]
pub fn column_count(available_width: f32) -> usize {
    let slot = MIN_CARD_WIDTH_PX + CARD_GAP_PX;
    let budget = available_width + CARD_GAP_PX;
    let mut needed = slot;
    let mut columns = 1;
    while columns < MAX_COLUMNS {
        needed += slot;
        if budget < needed {
            break;
        }
        columns += 1;
    }
    columns
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::install::gallery::catalog;

    fn entry(name: &str) -> &'static GalleryEntry {
        catalog::entries()
            .iter()
            .find(|e| e.name == name)
            .expect("entry is in the catalog")
    }

    #[test]
    fn empty_filter_matches_everything() {
        let filter = GalleryFilter::default();
        assert_eq!(
            catalog::entries()
                .iter()
                .filter(|e| filter.matches(e))
                .count(),
            catalog::entries().len()
        );
    }

    #[test]
    fn search_is_case_insensitive_over_name_author_and_tags() {
        let by_name = GalleryFilter {
            search: "IcEwInD".to_string(),
            ..Default::default()
        };
        assert!(by_name.matches(entry("Icewind Dale Essentials")));
        assert!(!by_name.matches(entry("EET Essentials")));

        let by_author = GalleryFilter {
            search: "bio team".to_string(),
            ..Default::default()
        };
        assert!(by_author.matches(entry("EET Essentials")));
        assert!(!by_author.matches(entry("BGEE Vanilla+")));

        let by_tag = GalleryFilter {
            search: "fixes".to_string(),
            ..Default::default()
        };
        assert!(by_tag.matches(entry("EET + Fixes")));
        assert!(!by_tag.matches(entry("EET Essentials")));
    }

    #[test]
    fn search_matches_the_game_label_and_ignores_surrounding_whitespace() {
        let filter = GalleryFilter {
            search: "  iwd  ".to_string(),
            ..Default::default()
        };
        assert_eq!(
            catalog::entries()
                .iter()
                .filter(|e| filter.matches(e))
                .count(),
            1
        );
        assert!(filter.matches(entry("Icewind Dale Essentials")));
    }

    #[test]
    fn game_filter_keeps_only_that_game() {
        let filter = GalleryFilter {
            game: Some(Game::BGEE),
            ..Default::default()
        };
        assert_eq!(
            catalog::entries()
                .iter()
                .filter(|e| filter.matches(e))
                .count(),
            1
        );
    }

    #[test]
    fn starter_only_keeps_the_starter_lists() {
        let filter = GalleryFilter {
            starter_only: true,
            ..Default::default()
        };
        assert_eq!(
            catalog::entries()
                .iter()
                .filter(|e| filter.matches(e))
                .count(),
            3
        );
    }

    #[test]
    fn filters_combine_as_an_intersection() {
        let filter = GalleryFilter {
            search: "eet".to_string(),
            game: Some(Game::EET),
            starter_only: true,
        };
        assert_eq!(
            catalog::entries()
                .iter()
                .filter(|e| filter.matches(e))
                .count(),
            2
        );
    }

    #[test]
    fn column_count_never_drops_below_one_or_climbs_past_four() {
        assert_eq!(column_count(0.0), 1);
        assert_eq!(column_count(-100.0), 1);
        assert_eq!(column_count(299.0), 1);
        assert_eq!(column_count(10_000.0), 4);
    }

    #[test]
    fn column_count_steps_at_each_card_plus_gap() {
        assert_eq!(column_count(300.0), 1);
        assert_eq!(column_count(619.0), 1);
        assert_eq!(column_count(620.0), 2);
        assert_eq!(column_count(939.0), 2);
        assert_eq!(column_count(940.0), 3);
        assert_eq!(column_count(1259.0), 3);
        assert_eq!(column_count(1260.0), 4);
    }
}
