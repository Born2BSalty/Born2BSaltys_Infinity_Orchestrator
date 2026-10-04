// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::gallery_feed::index::FeedEntry;
use crate::registry::model::Game;

pub const MIN_CARD_WIDTH_PX: f32 = 300.0;
pub const CARD_GAP_PX: f32 = 20.0;
const MAX_COLUMNS: usize = 4;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GalleryFilter {
    pub search: String,
    pub game: Option<Game>,
    pub featured_only: bool,
}

impl GalleryFilter {
    #[must_use]
    pub fn matches(&self, entry: &FeedEntry) -> bool {
        if self.featured_only && !entry.featured {
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

    fn entry(id: &str, name: &str, game: Game, featured: bool, tags: &[&str]) -> FeedEntry {
        FeedEntry {
            id: id.to_string(),
            name: name.to_string(),
            author: "BIO Team".to_string(),
            description: String::new(),
            tags: tags.iter().map(|tag| (*tag).to_string()).collect(),
            game,
            featured,
            version: "1.0.0".to_string(),
            requirements: String::new(),
            code: String::new(),
            cover_png: None,
            game_version: None,
            order: 0,
        }
    }

    fn fixture() -> Vec<FeedEntry> {
        vec![
            entry(
                "eet-essentials",
                "EET Essentials",
                Game::EET,
                true,
                &["Starter", "Vanilla+"],
            ),
            entry(
                "eet-plus-fixes",
                "EET + Fixes",
                Game::EET,
                true,
                &["Starter", "Fixes"],
            ),
            entry(
                "iwdee-essentials",
                "Icewind Dale Essentials",
                Game::IWDEE,
                true,
                &["Starter"],
            ),
            entry(
                "bgee-vanilla-plus",
                "BGEE Vanilla+ (with DLC)",
                Game::BGEE,
                false,
                &["Vanilla+"],
            ),
            entry(
                "bgee-vanilla-plus-no-dlc",
                "BGEE Vanilla+ (no DLC)",
                Game::BGEE,
                false,
                &["Vanilla+"],
            ),
        ]
    }

    fn find<'a>(entries: &'a [FeedEntry], name: &str) -> &'a FeedEntry {
        entries
            .iter()
            .find(|e| e.name == name)
            .expect("entry is in the fixture")
    }

    #[test]
    fn empty_filter_matches_everything() {
        let entries = fixture();
        let filter = GalleryFilter::default();
        assert_eq!(
            entries.iter().filter(|e| filter.matches(e)).count(),
            entries.len()
        );
    }

    #[test]
    fn search_is_case_insensitive_over_name_author_and_tags() {
        let entries = fixture();
        let by_name = GalleryFilter {
            search: "IcEwInD".to_string(),
            ..Default::default()
        };
        assert!(by_name.matches(find(&entries, "Icewind Dale Essentials")));
        assert!(!by_name.matches(find(&entries, "EET Essentials")));

        let by_author = GalleryFilter {
            search: "bio team".to_string(),
            ..Default::default()
        };
        assert!(by_author.matches(find(&entries, "EET Essentials")));

        let by_tag = GalleryFilter {
            search: "fixes".to_string(),
            ..Default::default()
        };
        assert!(by_tag.matches(find(&entries, "EET + Fixes")));
        assert!(!by_tag.matches(find(&entries, "EET Essentials")));
    }

    #[test]
    fn search_matches_the_game_label_and_ignores_surrounding_whitespace() {
        let entries = fixture();
        let filter = GalleryFilter {
            search: "  iwd  ".to_string(),
            ..Default::default()
        };
        assert_eq!(entries.iter().filter(|e| filter.matches(e)).count(), 1);
        assert!(filter.matches(find(&entries, "Icewind Dale Essentials")));
    }

    #[test]
    fn game_filter_keeps_only_that_game() {
        let entries = fixture();
        let filter = GalleryFilter {
            game: Some(Game::BGEE),
            ..Default::default()
        };
        assert_eq!(entries.iter().filter(|e| filter.matches(e)).count(), 2);
    }

    #[test]
    fn featured_only_keeps_the_featured_lists() {
        let entries = fixture();
        let filter = GalleryFilter {
            featured_only: true,
            ..Default::default()
        };
        assert_eq!(entries.iter().filter(|e| filter.matches(e)).count(), 3);
    }

    #[test]
    fn filters_combine_as_an_intersection() {
        let entries = fixture();
        let filter = GalleryFilter {
            search: "eet".to_string(),
            game: Some(Game::EET),
            featured_only: true,
        };
        assert_eq!(entries.iter().filter(|e| filter.matches(e)).count(), 2);
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
