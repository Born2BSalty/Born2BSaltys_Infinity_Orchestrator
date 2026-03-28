// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::HashMap;
use std::fs;
use std::sync::{Mutex, OnceLock};
use std::time::SystemTime;

use super::compat_rule_runtime::normalize_mod_key;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ComponentRequirement {
    pub(crate) raw_line: String,
    pub(crate) target_mod: String,
    pub(crate) target_component_id: String,
    pub(crate) message: Option<String>,
}

pub(crate) fn load_component_requirements(
    tp2_path: &str,
) -> HashMap<String, Vec<ComponentRequirement>> {
    if tp2_path.trim().is_empty() {
        return HashMap::new();
    }
    let cache = requirement_cache();
    let mut cache = cache.lock().expect("compat dependency cache lock poisoned");
    let stamp = cache_stamp(tp2_path);

    if let Some(entry) = cache.get(tp2_path)
        && entry.stamp == stamp
    {
        return entry.requirements.clone();
    }

    let requirements = load_component_requirements_uncached(tp2_path);
    cache.insert(
        tp2_path.to_string(),
        CachedRequirements {
            stamp,
            requirements: requirements.clone(),
        },
    );
    requirements
}

fn load_component_requirements_uncached(
    tp2_path: &str,
) -> HashMap<String, Vec<ComponentRequirement>> {
    let Ok(tp2_text) = fs::read_to_string(tp2_path) else {
        return HashMap::new();
    };

    let mut out = HashMap::<String, Vec<ComponentRequirement>>::new();
    let lines: Vec<&str> = tp2_text.lines().collect();
    let mut index = 0usize;

    while index < lines.len() {
        let line = lines[index].trim_start();
        if !line.to_ascii_uppercase().starts_with("BEGIN ") {
            index += 1;
            continue;
        }

        let start = index;
        index += 1;
        while index < lines.len() {
            let next = lines[index].trim_start().to_ascii_uppercase();
            if next.starts_with("BEGIN ") {
                break;
            }
            index += 1;
        }

        let block = &lines[start..index];
        let Some(component_id) = block
            .iter()
            .find_map(|entry| parse_designated_id(&entry.to_ascii_uppercase()))
        else {
            continue;
        };

        let requirements = collect_component_requirements(block);
        if !requirements.is_empty() {
            out.insert(component_id, requirements);
        }
    }

    out
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FileCacheStamp {
    modified: Option<SystemTime>,
    len: u64,
}

#[derive(Debug, Clone)]
struct CachedRequirements {
    stamp: FileCacheStamp,
    requirements: HashMap<String, Vec<ComponentRequirement>>,
}

fn requirement_cache() -> &'static Mutex<HashMap<String, CachedRequirements>> {
    static CACHE: OnceLock<Mutex<HashMap<String, CachedRequirements>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn cache_stamp(tp2_path: &str) -> FileCacheStamp {
    match fs::metadata(tp2_path) {
        Ok(meta) => FileCacheStamp {
            modified: meta.modified().ok(),
            len: meta.len(),
        },
        Err(_) => FileCacheStamp {
            modified: None,
            len: 0,
        },
    }
}

fn collect_component_requirements(block: &[&str]) -> Vec<ComponentRequirement> {
    let mut out = Vec::<ComponentRequirement>::new();
    for line in block {
        let Some(requirement) = parse_requirement_line(line) else {
            continue;
        };
        out.push(requirement);
    }
    out
}

fn parse_requirement_line(line: &str) -> Option<ComponentRequirement> {
    let stripped = strip_inline_comments(line);
    let trimmed = stripped.trim();
    let upper = trimmed.to_ascii_uppercase();
    if !upper.starts_with("REQUIRE_COMPONENT") {
        return None;
    }

    let tail = trimmed["REQUIRE_COMPONENT".len()..].trim_start();
    if tail.is_empty() {
        return None;
    }

    let mut index = 0usize;
    let tp2_ref = read_statement_token(tail, &mut index)?;
    let component_ref = read_statement_token(tail, &mut index)?;
    let target_mod = normalize_mod_key(&tp2_ref);
    let target_component_id = normalize_component_id(&component_ref)?;
    let message = read_statement_token(tail, &mut index).and_then(|value| {
        let value = value.trim();
        if value.is_empty() || value.starts_with('@') {
            None
        } else {
            Some(value.to_string())
        }
    });

    if target_mod.is_empty() {
        return None;
    }

    Some(ComponentRequirement {
        raw_line: trimmed.to_string(),
        target_mod,
        target_component_id,
        message,
    })
}

fn strip_inline_comments(line: &str) -> String {
    let chars: Vec<char> = line.chars().collect();
    let mut out = String::new();
    let mut index = 0usize;
    let mut quote = None::<char>;

    while index < chars.len() {
        let ch = chars[index];
        if let Some(active) = quote {
            out.push(ch);
            if ch == active {
                quote = None;
            }
            index += 1;
            continue;
        }

        if ch == '~' || ch == '"' {
            quote = Some(ch);
            out.push(ch);
            index += 1;
            continue;
        }

        if ch == '/' && chars.get(index + 1) == Some(&'/') {
            break;
        }
        if ch == '/' && chars.get(index + 1) == Some(&'*') {
            break;
        }

        out.push(ch);
        index += 1;
    }

    out
}

fn read_statement_token(input: &str, index: &mut usize) -> Option<String> {
    let bytes = input.as_bytes();
    while *index < bytes.len() && bytes[*index].is_ascii_whitespace() {
        *index += 1;
    }
    if *index >= bytes.len() {
        return None;
    }

    let quote = bytes[*index];
    if quote == b'~' || quote == b'"' {
        *index += 1;
        let start = *index;
        while *index < bytes.len() && bytes[*index] != quote {
            *index += 1;
        }
        let value = input[start..(*index).min(bytes.len())].trim().to_string();
        if *index < bytes.len() {
            *index += 1;
        }
        return Some(value);
    }

    let start = *index;
    while *index < bytes.len() && !bytes[*index].is_ascii_whitespace() {
        *index += 1;
    }
    Some(input[start..*index].trim().to_string())
}

fn normalize_component_id(value: &str) -> Option<String> {
    let trimmed = value
        .trim()
        .trim_matches(|ch: char| matches!(ch, '~' | '"' | '\''));
    let digits: String = trimmed.chars().take_while(|ch| ch.is_ascii_digit()).collect();
    if digits.is_empty() {
        return None;
    }
    let normalized = digits.trim_start_matches('0');
    if normalized.is_empty() {
        Some("0".to_string())
    } else {
        Some(normalized.to_string())
    }
}

fn parse_designated_id(upper_line: &str) -> Option<String> {
    if upper_line.trim_start().starts_with("//") {
        return None;
    }
    let index = upper_line.find("DESIGNATED")?;
    let tail = upper_line[index + "DESIGNATED".len()..].trim_start();
    let digits: String = tail.chars().take_while(|ch| ch.is_ascii_digit()).collect();
    if digits.is_empty() {
        return None;
    }
    let normalized = digits.trim_start_matches('0');
    if normalized.is_empty() {
        Some("0".to_string())
    } else {
        Some(normalized.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::{normalize_component_id, parse_requirement_line};

    #[test]
    fn parses_tilde_requirement_component() {
        let parsed = parse_requirement_line(
            r#"REQUIRE_COMPONENT ~bg1npc/bg1npc.tp2~ 0 @1004 /* comment */"#,
        )
        .expect("requirement should parse");
        assert_eq!(parsed.target_mod, "bg1npc");
        assert_eq!(parsed.target_component_id, "0");
        assert_eq!(parsed.message, None);
    }

    #[test]
    fn parses_quoted_requirement_component() {
        let parsed = parse_requirement_line(
            r#"REQUIRE_COMPONENT "setup-arestorationp.tp2" "11" ~Requires the previous component.~"#,
        )
        .expect("requirement should parse");
        assert_eq!(parsed.target_mod, "arestorationp");
        assert_eq!(parsed.target_component_id, "11");
        assert_eq!(
            parsed.message.as_deref(),
            Some("Requires the previous component.")
        );
    }

    #[test]
    fn ignores_commented_requirement_component() {
        assert!(parse_requirement_line(r#"// REQUIRE_COMPONENT ~foo.tp2~ 0 @1"#).is_none());
        assert!(parse_requirement_line(r#"/* REQUIRE_COMPONENT ~foo.tp2~ 0 @1 */"#).is_none());
    }

    #[test]
    fn normalizes_component_ids() {
        assert_eq!(normalize_component_id("007").as_deref(), Some("7"));
        assert_eq!(normalize_component_id("~000~").as_deref(), Some("0"));
        assert_eq!(normalize_component_id("abc"), None);
    }
}
