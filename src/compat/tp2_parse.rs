// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::fs;
use std::path::Path;

use super::model::{Tp2Metadata, Tp2Rule};

pub fn parse_tp2_rules(tp2_path: &Path) -> Tp2Metadata {
    let tp_file = tp2_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default()
        .to_string();

    let content = match fs::read_to_string(tp2_path) {
        Ok(c) => c,
        Err(_) => {
            return Tp2Metadata {
                tp_file,
                rules: Vec::new(),
            };
        }
    };

    let rules = extract_rules(&content);
    Tp2Metadata { tp_file, rules }
}

fn extract_rules(content: &str) -> Vec<(u32, Tp2Rule)> {
    let mut rules = Vec::new();
    let mut current_component: Option<u32> = None;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with("//") {
            continue;
        }

        let upper = trimmed.to_ascii_uppercase();

        if upper.contains("BEGIN") {
            // New BEGIN starts a new component scope.
            // If there is no explicit DESIGNATED on/near this BEGIN, do not carry
            // previous component id forward, otherwise rules can be mis-attributed.
            current_component = parse_begin_component(&upper);
            continue;
        }
        if let Some(comp_id) = parse_designated_line(&upper) {
            current_component = Some(comp_id);
            continue;
        }

        let Some(comp_id) = current_component else {
            continue;
        };

        if let Some(rule) = parse_require_component(&upper, trimmed) {
            rules.push((comp_id, rule));
        } else if let Some(rule) = parse_forbid_component(&upper, trimmed) {
            rules.push((comp_id, rule));
        } else if let Some(rule) = parse_require_predicate_game_or_installed_any(&upper, trimmed) {
            rules.push((comp_id, rule));
        } else if let Some(rule) = parse_require_predicate_game_is(&upper, trimmed) {
            rules.push((comp_id, rule));
        } else if let Some(rule) = parse_forbid_predicate_mod_installed(&upper, trimmed) {
            rules.push((comp_id, rule));
        } else if let Some(rule) = parse_require_predicate_mod_installed(&upper, trimmed) {
            rules.push((comp_id, rule));
        } else if let Some(rule) = parse_action_if_mod_installed(&upper, trimmed) {
            rules.push((comp_id, rule));
        } else if let Some(rule) = parse_action_if_mod_missing(&upper, trimmed) {
            rules.push((comp_id, rule));
        }
    }

    rules
}

fn parse_begin_component(upper: &str) -> Option<u32> {
    if !upper.contains("BEGIN") {
        return None;
    }
    parse_designated_line(upper)
}

fn parse_designated_line(upper: &str) -> Option<u32> {
    let designated_idx = upper.find("DESIGNATED")?;
    let after_designated = &upper[designated_idx + "DESIGNATED".len()..];
    let trimmed = after_designated
        .trim_start()
        .trim_start_matches(['=', ',', ':'])
        .trim_start();

    let digits: String = trimmed.chars().take_while(|c| c.is_ascii_digit()).collect();
    if digits.is_empty() {
        return None;
    }

    digits.parse().ok()
}

fn parse_require_component(upper: &str, raw: &str) -> Option<Tp2Rule> {
    let idx = upper.find("REQUIRE_COMPONENT")?;
    let after = &raw[idx + "REQUIRE_COMPONENT".len()..];
    let (target_mod, target_component) = parse_mod_component_args(after)?;

    Some(Tp2Rule::Require {
        target_mod,
        target_component,
        raw_line: raw.to_string(),
    })
}

fn parse_forbid_component(upper: &str, raw: &str) -> Option<Tp2Rule> {
    let idx = upper.find("FORBID_COMPONENT")?;
    let after = &raw[idx + "FORBID_COMPONENT".len()..];
    let (target_mod, target_component) = parse_mod_component_args(after)?;

    Some(Tp2Rule::Forbid {
        target_mod,
        target_component,
        raw_line: raw.to_string(),
    })
}

fn parse_mod_component_args(after: &str) -> Option<(String, u32)> {
    let trimmed = after.trim_start();

    let mod_name = if trimmed.starts_with('~') {
        let end = trimmed[1..].find('~')?;
        trimmed[1..1 + end].to_string()
    } else if trimmed.starts_with('"') {
        let end = trimmed[1..].find('"')?;
        trimmed[1..1 + end].to_string()
    } else if trimmed.starts_with('%') {
        let end = trimmed[1..].find('%')?;
        trimmed[1..1 + end].to_string()
    } else {
        return None;
    };

    let after_mod = if trimmed.starts_with('~') {
        let end = trimmed[1..].find('~')?;
        &trimmed[2 + end..]
    } else if trimmed.starts_with('"') {
        let end = trimmed[1..].find('"')?;
        &trimmed[2 + end..]
    } else if trimmed.starts_with('%') {
        let end = trimmed[1..].find('%')?;
        &trimmed[2 + end..]
    } else {
        return None;
    };

    let comp_str = after_mod
        .trim_start()
        .trim_start_matches(['=', ',', ':'])
        .trim_start()
        .trim_start_matches('#');
    let digits: String = comp_str.chars().take_while(|c| c.is_ascii_digit()).collect();
    let component_id: u32 = digits.parse().ok()?;

    Some((normalize_tp2_ref(&mod_name), component_id))
}

fn parse_require_predicate_game_is(upper: &str, raw: &str) -> Option<Tp2Rule> {
    if !upper.contains("REQUIRE_PREDICATE") || !upper.contains("GAME_IS") {
        return None;
    }
    // Ignore negated predicates (`!GAME_IS ...` or `NOT GAME_IS ...`):
    // those are "forbid" semantics, not "require allowed games".
    // Parsing them as RequireGame creates false mismatches.
    let normalized = upper.split_whitespace().collect::<Vec<_>>().join(" ");
    if normalized.contains("!GAME_IS") || normalized.contains("NOT GAME_IS") {
        return None;
    }
    let idx = upper.find("GAME_IS")?;
    let after = &raw[idx + "GAME_IS".len()..];
    let tokens = parse_token_group(after);
    if tokens.is_empty() {
        return None;
    }
    let allowed_games = tokens
        .into_iter()
        .map(|t| t.to_ascii_lowercase())
        .collect::<Vec<_>>();
    Some(Tp2Rule::RequireGame {
        allowed_games,
        raw_line: raw.to_string(),
    })
}

fn parse_require_predicate_game_or_installed_any(upper: &str, raw: &str) -> Option<Tp2Rule> {
    if !upper.contains("REQUIRE_PREDICATE")
        || !upper.contains("GAME_IS")
        || !upper.contains("MOD_IS_INSTALLED")
    {
        return None;
    }
    // This rule is meant for OR-style predicates:
    // REQUIRE_PREDICATE GAME_IS ~iwdee~ || MOD_IS_INSTALLED ...
    if !(upper.contains("||") || upper.contains(" OR ")) {
        return None;
    }
    // Ignore negated game checks here.
    let normalized = upper.split_whitespace().collect::<Vec<_>>().join(" ");
    if normalized.contains("!GAME_IS") || normalized.contains("NOT GAME_IS") {
        return None;
    }
    let idx = upper.find("GAME_IS")?;
    let after = &raw[idx + "GAME_IS".len()..];
    let allowed_games = parse_token_group(after)
        .into_iter()
        .map(|t| t.to_ascii_lowercase())
        .collect::<Vec<_>>();
    if allowed_games.is_empty() {
        return None;
    }
    let targets = parse_all_mod_is_installed(upper, raw);
    if targets.is_empty() {
        return None;
    }
    Some(Tp2Rule::RequireGameOrInstalledAny {
        allowed_games,
        targets,
        raw_line: raw.to_string(),
    })
}

fn parse_require_predicate_mod_installed(upper: &str, raw: &str) -> Option<Tp2Rule> {
    if !upper.contains("REQUIRE_PREDICATE") || !upper.contains("MOD_IS_INSTALLED") {
        return None;
    }
    // Negated predicates are "forbid" semantics, not requirements.
    if upper.contains("!MOD_IS_INSTALLED")
        || upper
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .contains("NOT MOD_IS_INSTALLED")
    {
        return None;
    }
    let targets = parse_all_mod_is_installed(upper, raw);
    if targets.is_empty() {
        return None;
    }
    if targets.len() == 1 {
        let (target_mod, target_component) = targets[0].clone();
        Some(Tp2Rule::RequireInstalledMod {
            target_mod,
            target_component,
            raw_line: raw.to_string(),
        })
    } else {
        Some(Tp2Rule::RequireInstalledAny {
            targets,
            raw_line: raw.to_string(),
        })
    }
}

fn parse_forbid_predicate_mod_installed(upper: &str, raw: &str) -> Option<Tp2Rule> {
    if !upper.contains("REQUIRE_PREDICATE") || !upper.contains("MOD_IS_INSTALLED") {
        return None;
    }
    let normalized = upper.split_whitespace().collect::<Vec<_>>().join(" ");
    let is_negated = upper.contains("!MOD_IS_INSTALLED")
        || normalized.contains("NOT MOD_IS_INSTALLED");
    if !is_negated {
        return None;
    }
    let idx = upper.find("MOD_IS_INSTALLED")?;
    let after = &raw[idx + "MOD_IS_INSTALLED".len()..];
    let (target_mod, target_component) = parse_mod_component_optional(after)?;
    Some(Tp2Rule::ForbidInstalledMod {
        target_mod,
        target_component,
        raw_line: raw.to_string(),
    })
}

fn parse_action_if_mod_installed(upper: &str, raw: &str) -> Option<Tp2Rule> {
    if !upper.contains("ACTION_IF") || !upper.contains("MOD_IS_INSTALLED") {
        return None;
    }
    let normalized = upper.split_whitespace().collect::<Vec<_>>().join(" ");
    let is_negated = upper.contains("!MOD_IS_INSTALLED")
        || normalized.contains("NOT MOD_IS_INSTALLED");
    if is_negated {
        return None;
    }
    let idx = upper.find("MOD_IS_INSTALLED")?;
    let after = &raw[idx + "MOD_IS_INSTALLED".len()..];
    let (target_mod, target_component) = parse_mod_component_optional(after)?;
    Some(Tp2Rule::ConditionalOnInstalled {
        target_mod,
        target_component,
        raw_line: raw.to_string(),
    })
}

fn parse_action_if_mod_missing(upper: &str, raw: &str) -> Option<Tp2Rule> {
    if !upper.contains("ACTION_IF") || !upper.contains("MOD_IS_INSTALLED") {
        return None;
    }
    let normalized = upper.split_whitespace().collect::<Vec<_>>().join(" ");
    let is_negated = upper.contains("!MOD_IS_INSTALLED")
        || normalized.contains("NOT MOD_IS_INSTALLED");
    if !is_negated {
        return None;
    }
    let idx = upper.find("MOD_IS_INSTALLED")?;
    let after = &raw[idx + "MOD_IS_INSTALLED".len()..];
    let (target_mod, target_component) = parse_mod_component_optional(after)?;
    Some(Tp2Rule::ConditionalOnMissing {
        target_mod,
        target_component,
        raw_line: raw.to_string(),
    })
}

fn parse_mod_component_optional(after: &str) -> Option<(String, Option<u32>)> {
    let trimmed = after.trim_start();
    let (mod_name, after_mod) = parse_quoted_or_tilde_token(trimmed)?;
    let mut comp: Option<u32> = None;
    let comp_str = after_mod
        .trim_start()
        .trim_start_matches(['=', ',', ':'])
        .trim_start();
    let comp_str = if let Some(rest) = comp_str.strip_prefix('#') {
        rest.trim_start()
    } else {
        comp_str
    };
    let digits: String = if let Some((token, _)) = parse_quoted_or_tilde_token(comp_str) {
        token.chars().take_while(|c| c.is_ascii_digit()).collect()
    } else {
        comp_str.chars().take_while(|c| c.is_ascii_digit()).collect()
    };
    if !digits.is_empty() {
        comp = digits.parse().ok();
    }
    Some((normalize_tp2_ref(&mod_name), comp))
}

fn parse_all_mod_is_installed(upper: &str, raw: &str) -> Vec<(String, Option<u32>)> {
    let mut out: Vec<(String, Option<u32>)> = Vec::new();
    let mut offset = 0usize;
    while let Some(rel_idx) = upper[offset..].find("MOD_IS_INSTALLED") {
        let idx = offset + rel_idx;
        let after = &raw[idx + "MOD_IS_INSTALLED".len()..];
        if let Some(target) = parse_mod_component_optional(after)
            && !out.contains(&target)
        {
            out.push(target);
        }
        offset = idx + "MOD_IS_INSTALLED".len();
    }
    out
}

fn parse_quoted_or_tilde_token(input: &str) -> Option<(String, &str)> {
    if let Some(rest) = input.strip_prefix('~') {
        let end = rest.find('~')?;
        return Some((rest[..end].to_string(), &rest[end + 1..]));
    }
    if let Some(rest) = input.strip_prefix('"') {
        let end = rest.find('"')?;
        return Some((rest[..end].to_string(), &rest[end + 1..]));
    }
    if let Some(rest) = input.strip_prefix('%') {
        let end = rest.find('%')?;
        return Some((rest[..end].to_string(), &rest[end + 1..]));
    }
    None
}

fn parse_token_group(input: &str) -> Vec<String> {
    let trimmed = input.trim_start();
    let mut body = "";
    if let Some(rest) = trimmed.strip_prefix('~') {
        if let Some(end) = rest.find('~') {
            body = &rest[..end];
        }
    } else if let Some(rest) = trimmed.strip_prefix('"') {
        if let Some(end) = rest.find('"') {
            body = &rest[..end];
        }
    } else if let Some(rest) = trimmed.strip_prefix('%')
        && let Some(end) = rest.find('%')
    {
        body = &rest[..end];
    }
    body.split_whitespace().map(|s| s.to_string()).collect()
}

fn normalize_tp2_ref(value: &str) -> String {
    let lower = value.to_ascii_lowercase();
    let file = if let Some(idx) = lower.rfind(['/', '\\']) {
        &lower[idx + 1..]
    } else {
        &lower
    };
    let without_ext = file.strip_suffix(".tp2").unwrap_or(file);
    without_ext
        .strip_prefix("setup-")
        .unwrap_or(without_ext)
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_require_component() {
        let line = r#"REQUIRE_COMPONENT ~STRATAGEMS/SETUP-STRATAGEMS.TP2~ 1000 @123"#;
        let upper = line.to_ascii_uppercase();
        let rule = parse_require_component(&upper, line).unwrap();
        match rule {
            Tp2Rule::Require { target_mod, target_component, .. } => {
                assert_eq!(target_mod, "stratagems");
                assert_eq!(target_component, 1000);
            }
            _ => panic!("expected Require"),
        }
    }

    #[test]
    fn test_parse_forbid_component() {
        let line = r#"FORBID_COMPONENT ~EET/EET.TP2~ 0 @456"#;
        let upper = line.to_ascii_uppercase();
        let rule = parse_forbid_component(&upper, line).unwrap();
        match rule {
            Tp2Rule::Forbid { target_mod, target_component, .. } => {
                assert_eq!(target_mod, "eet");
                assert_eq!(target_component, 0);
            }
            _ => panic!("expected Forbid"),
        }
    }

    #[test]
    fn test_normalize_tp2_ref() {
        assert_eq!(normalize_tp2_ref("STRATAGEMS/SETUP-STRATAGEMS.TP2"), "stratagems");
        assert_eq!(normalize_tp2_ref("EET.TP2"), "eet");
        assert_eq!(normalize_tp2_ref("setup-mymod.tp2"), "mymod");
    }

    #[test]
    fn test_parse_forbid_predicate_mod_is_installed_not() {
        let line =
            r#"REQUIRE_PREDICATE NOT MOD_IS_INSTALLED ~1pp/1pp.tp2~ ~113~ @24"#;
        let upper = line.to_ascii_uppercase();
        let rule = parse_forbid_predicate_mod_installed(&upper, line).unwrap();
        match rule {
            Tp2Rule::ForbidInstalledMod {
                target_mod,
                target_component,
                ..
            } => {
                assert_eq!(target_mod, "1pp");
                assert_eq!(target_component, Some(113));
            }
            _ => panic!("expected ForbidInstalledMod"),
        }
    }

    #[test]
    fn test_parse_require_predicate_mod_is_installed_or() {
        let line = r#"REQUIRE_PREDICATE (MOD_IS_INSTALLED ~Emily.tp2~ ~0~) OR (MOD_IS_INSTALLED ~SkitiaNPCs.tp2~ ~0~) @24"#;
        let upper = line.to_ascii_uppercase();
        let rule = parse_require_predicate_mod_installed(&upper, line).unwrap();
        match rule {
            Tp2Rule::RequireInstalledAny { targets, .. } => {
                assert_eq!(targets.len(), 2);
                assert!(targets.contains(&("emily".to_string(), Some(0))));
                assert!(targets.contains(&("skitianpcs".to_string(), Some(0))));
            }
            _ => panic!("expected RequireInstalledAny"),
        }
    }

    #[test]
    fn test_parse_action_if_not_mod_is_installed() {
        let line = r#"ACTION_IF (NOT MOD_IS_INSTALLED ~HiddenGameplayOptions.tp2~ 35) BEGIN"#;
        let upper = line.to_ascii_uppercase();
        let rule = parse_action_if_mod_missing(&upper, line).unwrap();
        match rule {
            Tp2Rule::ConditionalOnMissing {
                target_mod,
                target_component,
                ..
            } => {
                assert_eq!(target_mod, "hiddengameplayoptions");
                assert_eq!(target_component, Some(35));
            }
            _ => panic!("expected ConditionalOnMissing"),
        }
    }
}
