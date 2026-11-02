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

pub(super) fn union_recognition_rules(
    groups: &[&ParserRecognitionRules],
) -> Result<ParserRecognitionRules, String> {
    let mut merged = ParserRecognitionRules::new();
    for group in groups {
        validate_recognition_rules(group)?;
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
    validate_recognition_rules(&merged)?;
    Ok(merged)
}

pub(super) fn recognition_rules_with_legacy(
    rules: &ParserRecognitionRules,
    phrases: &[String],
) -> Result<ParserRecognitionRules, String> {
    super::validate_phrases(phrases)?;
    let legacy = if phrases.is_empty() {
        ParserRecognitionRules::new()
    } else {
        ParserRecognitionRules::from([("choice_phrase".to_string(), phrases.to_vec())])
    };
    union_recognition_rules(&[rules, &legacy])
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
