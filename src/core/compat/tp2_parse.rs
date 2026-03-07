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
    let lines: Vec<&str> = content.lines().collect();
    let mut i = 0usize;

    while i < lines.len() {
        let trimmed = lines[i].trim();
        if trimmed.is_empty() || trimmed.starts_with("//") {
            i += 1;
            continue;
        }

        let upper = trimmed.to_ascii_uppercase();

        if upper.contains("BEGIN") {
            // New BEGIN starts a new component scope.
            // If there is no explicit DESIGNATED on/near this BEGIN, do not carry
            // previous component id forward, otherwise rules can be mis-attributed.
            current_component = parse_begin_component(&upper);
            i += 1;
            continue;
        }
        if let Some(comp_id) = parse_designated_line(&upper) {
            current_component = Some(comp_id);
            i += 1;
            continue;
        }

        let Some(comp_id) = current_component else {
            i += 1;
            continue;
        };

        let (raw_expr, consumed_to) = if upper.contains("REQUIRE_PREDICATE") {
            collect_predicate_expression(&lines, i)
        } else {
            (trimmed.to_string(), i)
        };
        let upper_expr = raw_expr.to_ascii_uppercase();

        let rule_line = i + 1;
        if let Some(rule) = parse_require_component(&upper_expr, &raw_expr, rule_line) {
            rules.push((comp_id, rule));
        } else if let Some(rule) = parse_forbid_component(&upper_expr, &raw_expr, rule_line) {
            rules.push((comp_id, rule));
        } else if let Some(rule) =
            parse_require_predicate_game_or_installed_any(&upper_expr, &raw_expr, rule_line)
        {
            rules.push((comp_id, rule));
        } else if let Some(rule) = parse_require_predicate_game_is(&upper_expr, &raw_expr, rule_line) {
            rules.push((comp_id, rule));
        } else if let Some(rule) = parse_forbid_predicate_mod_installed(&upper_expr, &raw_expr, rule_line) {
            rules.push((comp_id, rule));
        } else if let Some(rule) = parse_require_predicate_mod_installed(&upper_expr, &raw_expr, rule_line) {
            rules.push((comp_id, rule));
        } else if let Some(rule) = parse_action_if_mod_installed(&upper_expr, &raw_expr, rule_line) {
            rules.push((comp_id, rule));
        } else if let Some(rule) = parse_action_if_mod_missing(&upper_expr, &raw_expr, rule_line) {
            rules.push((comp_id, rule));
        }

        i = consumed_to + 1;
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

fn collect_predicate_expression(lines: &[&str], start_idx: usize) -> (String, usize) {
    let mut expr = lines[start_idx].trim().to_string();
    let mut paren_balance = paren_delta(&expr);
    let mut last = start_idx;

    while last + 1 < lines.len() {
        let next = lines[last + 1].trim();
        if next.is_empty() {
            last += 1;
            continue;
        }

        let next_upper = next.to_ascii_uppercase();
        let starts_new_statement = next_upper.starts_with("BEGIN ")
            || next_upper.starts_with("REQUIRE_")
            || next_upper.starts_with("FORBID_")
            || next_upper.starts_with("ACTION_IF")
            || next_upper.starts_with("LABEL ")
            || next_upper.starts_with("INCLUDE ")
            || next_upper.starts_with("COPY ")
            || next_upper.starts_with("OUTER_")
            || next_upper.starts_with("END");

        let expr_upper = expr.to_ascii_uppercase();
        let can_stop = paren_balance <= 0
            && !expr_upper.ends_with(" OR")
            && !expr_upper.ends_with(" ||")
            && !expr_upper.ends_with("AND")
            && !expr_upper.ends_with("&&");

        if starts_new_statement && can_stop {
            break;
        }

        expr.push(' ');
        expr.push_str(next);
        paren_balance += paren_delta(next);
        last += 1;

        let expr_upper = expr.to_ascii_uppercase();
        let should_stop = paren_balance <= 0
            && !expr_upper.ends_with(" OR")
            && !expr_upper.ends_with(" ||")
            && !expr_upper.ends_with("AND")
            && !expr_upper.ends_with("&&");
        if should_stop {
            break;
        }
    }

    (expr, last)
}

fn paren_delta(input: &str) -> i32 {
    let open = input.chars().filter(|c| *c == '(').count() as i32;
    let close = input.chars().filter(|c| *c == ')').count() as i32;
    open - close
}
#[path = "tp2_parse_rules.rs"]
mod rules;

use rules::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_require_component() {
        let line = r#"REQUIRE_COMPONENT ~STRATAGEMS/SETUP-STRATAGEMS.TP2~ 1000 @123"#;
        let upper = line.to_ascii_uppercase();
        let rule = parse_require_component(&upper, line, 1).unwrap();
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
        let rule = parse_forbid_component(&upper, line, 1).unwrap();
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
        let rule = parse_forbid_predicate_mod_installed(&upper, line, 1).unwrap();
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
        let rule = parse_require_predicate_mod_installed(&upper, line, 1).unwrap();
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
        let rule = parse_action_if_mod_missing(&upper, line, 1).unwrap();
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

    #[test]
    fn test_parse_multiline_require_predicate_game_is_or_chain() {
        let content = r#"
BEGIN @0 DESIGNATED 0
REQUIRE_PREDICATE (((GAME_IS ~bgee~)  AND (FILE_EXISTS ~eefixpack/files/tph/bgee.tph~)) OR
                   ((GAME_IS ~bg2ee~) AND (FILE_EXISTS ~eefixpack/files/tph/bg2ee.tph~)) OR
                   ((GAME_IS ~iwdee~) AND (FILE_EXISTS ~eefixpack/files/tph/iwdee.tph~)) OR
                   ((GAME_IS ~pstee~) AND (FILE_EXISTS ~eefixpack/files/tph/pstee.tph~))) @4
"#;

        let rules = extract_rules(content);
        let (_, rule) = rules
            .iter()
            .find(|(comp, rule)| *comp == 0 && matches!(rule, Tp2Rule::RequireGame { .. }))
            .expect("missing RequireGame rule");

        match rule {
            Tp2Rule::RequireGame { allowed_games, .. } => {
                assert_eq!(
                    allowed_games,
                    &vec![
                        "bgee".to_string(),
                        "bg2ee".to_string(),
                        "iwdee".to_string(),
                        "pstee".to_string()
                    ]
                );
            }
            _ => panic!("expected RequireGame"),
        }
    }

    #[test]
    fn test_parse_negated_paren_mod_is_installed_as_forbid() {
        let line = r#"REQUIRE_PREDICATE !(MOD_IS_INSTALLED ~dw_talents/dw_talents.tp2~ 40600) @50016"#;
        let upper = line.to_ascii_uppercase();
        let forbid = parse_forbid_predicate_mod_installed(&upper, line, 1).unwrap();
        assert!(matches!(forbid, Tp2Rule::ForbidInstalledMod { .. }));
        assert!(parse_require_predicate_mod_installed(&upper, line, 1).is_none());
    }
}
