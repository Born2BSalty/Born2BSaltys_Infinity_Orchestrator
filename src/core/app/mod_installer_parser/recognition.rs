use std::collections::HashSet;

use super::ParserRecognitionRules;

const FIELDS: [&str; 7] = [
    "in_progress_words",
    "useful_status_words",
    "choice_words",
    "choice_phrase",
    "completed_with_warnings",
    "failed_with_error",
    "finished",
];

pub(super) fn validate_recognition_rules(rules: &ParserRecognitionRules) -> Result<(), String> {
    let mut bytes = 0usize;
    for (key, values) in rules {
        if !FIELDS.contains(&key.as_str()) {
            return Err(format!("Unsupported parser recognition field: {key}"));
        }
        super::validate_phrases(values)?;
        bytes = bytes.saturating_add(key.len());
        for value in values {
            bytes = bytes.saturating_add(value.len());
        }
    }
    if bytes > 512 * 1024 {
        return Err("Parser recognition rules exceed 512 KiB".to_string());
    }
    Ok(())
}

const OUTCOME_FIELDS: [&str; 3] = ["completed_with_warnings", "failed_with_error", "finished"];

pub(super) fn merge_recognition_rules(
    groups: &[&ParserRecognitionRules],
) -> ParserRecognitionRules {
    let mut merged = ParserRecognitionRules::new();
    for group in groups {
        for (key, values) in *group {
            let output: &mut Vec<String> = merged.entry(key.clone()).or_default();
            let mut seen: HashSet<String> = output.iter().cloned().collect();
            output.extend(
                values
                    .iter()
                    .filter(|value| seen.insert((*value).clone()))
                    .cloned(),
            );
        }
    }
    merged
}

pub(super) fn union_recognition_rules(
    groups: &[&ParserRecognitionRules],
) -> Result<ParserRecognitionRules, String> {
    for group in groups {
        validate_recognition_rules(group)?;
    }
    let merged = merge_recognition_rules(groups);
    validate_recognition_rules(&merged)?;
    Ok(merged)
}

pub(super) fn legacy_choice_rules(phrases: &[String]) -> ParserRecognitionRules {
    if phrases.is_empty() {
        ParserRecognitionRules::new()
    } else {
        ParserRecognitionRules::from([("choice_phrase".to_string(), phrases.to_vec())])
    }
}

#[cfg(test)]
pub(super) fn recognition_rules_with_legacy(
    rules: &ParserRecognitionRules,
    phrases: &[String],
) -> Result<ParserRecognitionRules, String> {
    super::validate_phrases(phrases)?;
    union_recognition_rules(&[rules, &legacy_choice_rules(phrases)])
}

fn shown(text: &str) -> String {
    let head: String = text.chars().take(64).collect();
    if head.len() < text.len() {
        format!("{head:?}...")
    } else {
        format!("{head:?}")
    }
}

fn unsafe_value_reason(key: &str, value: &str) -> Option<&'static str> {
    let trimmed = value.trim();
    if value.chars().any(char::is_control) {
        Some("it contains a control character")
    } else if trimmed.chars().count() < 3 {
        Some("it is shorter than 3 characters")
    } else if OUTCOME_FIELDS.contains(&key) && trimmed.split_whitespace().count() < 2 {
        Some("this list needs at least two words")
    } else if value.len() > 8192 {
        Some("it is longer than 8192 bytes")
    } else {
        None
    }
}

pub(super) fn sanitize_recognition_rules(
    rules: &ParserRecognitionRules,
) -> (ParserRecognitionRules, Vec<String>) {
    let mut kept = ParserRecognitionRules::new();
    let mut dropped = Vec::new();
    for (key, values) in rules {
        if !FIELDS.contains(&key.as_str()) {
            dropped.push(format!(
                "unknown list {} with {} value(s)",
                shown(key),
                values.len()
            ));
            continue;
        }
        let mut safe = Vec::with_capacity(values.len());
        for value in values {
            if let Some(reason) = unsafe_value_reason(key, value) {
                dropped.push(format!("{key} value {}: {reason}", shown(value)));
            } else {
                safe.push(value.clone());
            }
        }
        if !safe.is_empty() {
            kept.insert(key.clone(), safe);
        }
    }
    if let Err(error) = validate_recognition_rules(&kept) {
        dropped.push(format!("every list: {error}"));
        kept.clear();
    }
    (kept, dropped)
}

pub(super) fn rules_from_json(value: &serde_json::Value) -> Result<ParserRecognitionRules, String> {
    let object = value
        .as_object()
        .ok_or_else(|| "Parser recognition rules must be an object".to_string())?;
    let mut rules = ParserRecognitionRules::new();
    for (key, entry) in object {
        let strings = entry.as_array().and_then(|array| {
            array
                .iter()
                .map(|item| item.as_str().map(str::to_string))
                .collect::<Option<Vec<_>>>()
        });
        if FIELDS.contains(&key.as_str()) {
            let values =
                strings.ok_or_else(|| format!("Parser {key} must be an array of strings"))?;
            rules.insert(key.clone(), values);
        } else {
            rules.insert(key.clone(), strings.unwrap_or_default());
        }
    }
    Ok(rules)
}

pub(super) fn from_document(document: &toml::Value) -> Result<ParserRecognitionRules, String> {
    document
        .as_table()
        .ok_or_else(|| "Parser root must be a table".to_string())?;
    let mut rules = ParserRecognitionRules::new();
    for key in FIELDS {
        let Some(value) = document.get(key) else {
            continue;
        };
        let array = value
            .as_array()
            .ok_or_else(|| format!("Parser {key} must be an array of strings"))?;
        let values = array
            .iter()
            .map(|entry| {
                entry
                    .as_str()
                    .map(str::to_string)
                    .ok_or_else(|| format!("Parser {key} must contain only strings"))
            })
            .collect::<Result<Vec<_>, _>>()?;
        rules.insert(key.to_string(), values);
    }
    validate_recognition_rules(&rules)?;
    Ok(rules)
}
