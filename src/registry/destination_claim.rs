// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::registry::model::ModlistRegistry;
use crate::registry::operations::{self, DestinationOwnership};

pub struct ClaimContext<'a> {
    pub destination: &'a str,
    pub held_id: Option<&'a str>,
    pub installing_id: Option<&'a str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClaimRefusal {
    OwnerIsInstalling(String),
    InsideAnother(String),
    ContainsOthers(Vec<String>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DestinationClaim {
    Free,
    Adopt(String),
    Replace(Vec<String>),
    Refused(ClaimRefusal),
}

impl DestinationClaim {
    #[must_use]
    pub const fn blocks(&self) -> bool {
        matches!(self, Self::Refused(_))
    }
}

impl ClaimRefusal {
    #[must_use]
    pub fn message(&self, registry: &ModlistRegistry) -> String {
        let name_of = |id: &str| {
            registry
                .find(id)
                .map_or(id, |e| e.name.as_str())
                .to_string()
        };
        match self {
            Self::OwnerIsInstalling(id) => {
                format!(
                    "\"{}\" is installing into this folder right now",
                    name_of(id)
                )
            }
            Self::InsideAnother(id) => {
                format!("this folder is inside \"{}\"'s install folder", name_of(id))
            }
            Self::ContainsOthers(ids) => {
                let names = ids
                    .iter()
                    .map(|id| format!("\"{}\"", name_of(id)))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("this folder contains other modlists: {names}")
            }
        }
    }
}

#[must_use]
pub fn resolve_destination_claim(
    registry: &ModlistRegistry,
    ctx: &ClaimContext<'_>,
) -> DestinationClaim {
    match operations::classify_destination(ctx.destination, registry) {
        DestinationOwnership::Free => DestinationClaim::Free,
        DestinationOwnership::ExactOwners(ids) => {
            let filtered: Vec<String> =
                ids.into_iter().filter(|id| !id.trim().is_empty()).collect();
            if let Some(held) = ctx.held_id
                && filtered.iter().any(|id| id == held)
            {
                DestinationClaim::Adopt(held.to_string())
            } else if let Some(active) = ctx.installing_id
                && filtered.iter().any(|id| id == active)
            {
                DestinationClaim::Refused(ClaimRefusal::OwnerIsInstalling(active.to_string()))
            } else if filtered.is_empty() {
                DestinationClaim::Free
            } else {
                DestinationClaim::Replace(filtered)
            }
        }
        DestinationOwnership::InsideOwner(id) => {
            DestinationClaim::Refused(ClaimRefusal::InsideAnother(id))
        }
        DestinationOwnership::ContainsOwners(ids) => {
            DestinationClaim::Refused(ClaimRefusal::ContainsOthers(ids))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::model::{Game, ModlistEntry, ModlistState};

    fn entry(id: &str, name: &str, dest: &str) -> ModlistEntry {
        ModlistEntry {
            id: id.to_string(),
            name: name.to_string(),
            game: Game::EET,
            destination_folder: dest.to_string(),
            state: ModlistState::InProgress,
            ..Default::default()
        }
    }

    fn registry(entries: Vec<ModlistEntry>) -> ModlistRegistry {
        ModlistRegistry {
            entries,
            ..Default::default()
        }
    }

    fn dest_str() -> &'static str {
        if cfg!(windows) {
            "C:\\Games\\EET"
        } else {
            "/games/eet"
        }
    }

    fn ctx<'a>(
        destination: &'a str,
        held_id: Option<&'a str>,
        installing_id: Option<&'a str>,
    ) -> ClaimContext<'a> {
        ClaimContext {
            destination,
            held_id,
            installing_id,
        }
    }

    #[test]
    fn free_folder_is_free() {
        let reg = registry(vec![]);
        let claim = resolve_destination_claim(&reg, &ctx(dest_str(), None, None));
        assert_eq!(claim, DestinationClaim::Free);
    }

    #[test]
    fn held_owner_is_adopted() {
        let d = dest_str();
        let reg = registry(vec![entry("OWNER0001", "Owner", d)]);
        let claim = resolve_destination_claim(&reg, &ctx(d, Some("OWNER0001"), None));
        assert_eq!(claim, DestinationClaim::Adopt("OWNER0001".to_string()));
    }

    #[test]
    fn held_owner_wins_over_installing() {
        let d = dest_str();
        let reg = registry(vec![entry("OWNER0001", "Owner", d)]);
        let claim = resolve_destination_claim(&reg, &ctx(d, Some("OWNER0001"), Some("OWNER0001")));
        assert_eq!(claim, DestinationClaim::Adopt("OWNER0001".to_string()));
    }

    #[test]
    fn installing_owner_is_refused() {
        let d = dest_str();
        let reg = registry(vec![entry("BUSY0001", "Busy", d)]);
        let claim = resolve_destination_claim(&reg, &ctx(d, None, Some("BUSY0001")));
        assert_eq!(
            claim,
            DestinationClaim::Refused(ClaimRefusal::OwnerIsInstalling("BUSY0001".to_string()))
        );
    }

    #[test]
    fn other_owner_is_replaced() {
        let d = dest_str();
        let reg = registry(vec![entry("OTHER0001", "Other", d)]);
        let claim = resolve_destination_claim(&reg, &ctx(d, None, None));
        assert_eq!(
            claim,
            DestinationClaim::Replace(vec!["OTHER0001".to_string()])
        );
    }

    #[test]
    fn two_other_owners_are_both_replaced() {
        let d = dest_str();
        let reg = registry(vec![
            entry("OTHER0001", "Other one", d),
            entry("OTHER0002", "Other two", d),
        ]);
        let claim = resolve_destination_claim(&reg, &ctx(d, None, None));
        let DestinationClaim::Replace(ids) = claim else {
            panic!("expected Replace");
        };
        assert_eq!(ids.len(), 2);
        assert!(ids.contains(&"OTHER0001".to_string()));
        assert!(ids.contains(&"OTHER0002".to_string()));
    }

    #[test]
    fn blank_id_owner_is_ignored() {
        let d = dest_str();
        let reg = registry(vec![entry("", "Blank", d)]);
        let claim = resolve_destination_claim(&reg, &ctx(d, None, None));
        assert_eq!(claim, DestinationClaim::Free);

        let reg2 = registry(vec![entry("", "Blank", d), entry("REAL0001", "Real", d)]);
        let claim2 = resolve_destination_claim(&reg2, &ctx(d, None, None));
        assert_eq!(
            claim2,
            DestinationClaim::Replace(vec!["REAL0001".to_string()])
        );
    }

    #[test]
    fn inside_another_is_refused() {
        let owner_dest = dest_str();
        let inside = if cfg!(windows) {
            "C:\\Games\\EET\\mods"
        } else {
            "/games/eet/mods"
        };
        let reg = registry(vec![entry("OWNER0001", "Owner", owner_dest)]);
        let claim = resolve_destination_claim(&reg, &ctx(inside, None, None));
        assert_eq!(
            claim,
            DestinationClaim::Refused(ClaimRefusal::InsideAnother("OWNER0001".to_string()))
        );
    }

    #[test]
    fn contains_others_is_refused() {
        let owner_dest = dest_str();
        let outer = if cfg!(windows) { "C:\\Games" } else { "/games" };
        let reg = registry(vec![entry("OWNER0001", "Owner", owner_dest)]);
        let claim = resolve_destination_claim(&reg, &ctx(outer, None, None));
        assert_eq!(
            claim,
            DestinationClaim::Refused(ClaimRefusal::ContainsOthers(vec!["OWNER0001".to_string()]))
        );
    }

    #[test]
    fn held_id_that_does_not_own_the_folder_does_not_adopt() {
        let d = dest_str();
        let reg = registry(vec![entry("Y", "Owner Y", d)]);
        let claim = resolve_destination_claim(&reg, &ctx(d, Some("X"), None));
        assert_eq!(claim, DestinationClaim::Replace(vec!["Y".to_string()]));
    }

    #[test]
    fn messages_name_the_entries() {
        let d = dest_str();
        let reg = registry(vec![
            entry("BUSY0001", "Busy List", d),
            entry("OWNER0001", "Owner List", d),
            entry("OTHER0001", "Other One", d),
            entry("OTHER0002", "Other Two", d),
        ]);

        let busy = ClaimRefusal::OwnerIsInstalling("BUSY0001".to_string());
        assert_eq!(
            busy.message(&reg),
            "\"Busy List\" is installing into this folder right now"
        );

        let inside = ClaimRefusal::InsideAnother("OWNER0001".to_string());
        assert_eq!(
            inside.message(&reg),
            "this folder is inside \"Owner List\"'s install folder"
        );

        let contains =
            ClaimRefusal::ContainsOthers(vec!["OTHER0001".to_string(), "OTHER0002".to_string()]);
        assert_eq!(
            contains.message(&reg),
            "this folder contains other modlists: \"Other One\", \"Other Two\""
        );
    }
}
