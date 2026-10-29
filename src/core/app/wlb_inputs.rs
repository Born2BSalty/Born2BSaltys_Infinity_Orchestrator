use crate::app::mod_downloads::normalize_mod_download_tp2;
use crate::mods::component::Component;

const MARKER: &str = "@wlb-inputs:";
const JSON_PREFIX: &str = "bio-json-v1:";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WlbIdentity {
    pub(crate) normalized_tp2: String,
    pub(crate) component_id: i64,
    pub(crate) language: u8,
}

pub(crate) fn parse_identity(raw_line: &str) -> Option<WlbIdentity> {
    let raw = raw_line.trim_start().strip_prefix('~')?;
    let (tp2, tail) = raw.split_once('~')?;
    if tp2.trim().is_empty() {
        return None;
    }
    let mut fields = tail.split_whitespace();
    let language = fields.next()?.strip_prefix('#')?.parse::<u8>().ok()?;
    let component_id = fields.next()?.strip_prefix('#')?.parse::<i64>().ok()?;
    Component::parse_weidu_line(raw_line).ok()?;
    Some(WlbIdentity {
        normalized_tp2: normalize_mod_download_tp2(tp2),
        component_id,
        language,
    })
}

fn validate_answers(answers: &[String]) -> Result<(), String> {
    if answers
        .iter()
        .any(|answer| answer.contains(['\r', '\n', '\0']))
    {
        return Err("WLB answers must each contain a single line without NUL.".to_string());
    }
    Ok(())
}

pub(crate) fn decode_answers(spec: &str) -> Result<Vec<String>, String> {
    if spec.contains(['\r', '\n', '\0']) {
        return Err("WLB answers must each contain a single line without NUL.".to_string());
    }
    let answers = if let Some(json) = spec.strip_prefix(JSON_PREFIX) {
        serde_json::from_str::<Vec<String>>(json)
            .map_err(|err| format!("Invalid tagged WLB answers: {err}"))?
    } else if spec.starts_with("bio-json-") {
        return Err("Unsupported tagged WLB answer version.".to_string());
    } else {
        spec.split(',')
            .map(|part| part.trim().to_string())
            .collect()
    };
    validate_answers(&answers)?;
    Ok(answers)
}

pub(crate) fn encode_answers(answers: &[String]) -> Result<Option<String>, String> {
    validate_answers(answers)?;
    if answers.is_empty() {
        return Ok(None);
    }
    let legacy = answers.join(",");
    if !legacy.is_empty() && decode_answers(&legacy).is_ok_and(|value| value == answers) {
        return Ok(Some(legacy));
    }
    serde_json::to_string(answers)
        .map(|json| Some(format!("{JSON_PREFIX}{json}")))
        .map_err(|err| format!("Cannot encode WLB answers: {err}"))
}

pub(crate) fn marker_spec(raw_line: &str) -> Option<&str> {
    let start = raw_line.to_ascii_lowercase().find(MARKER)?;
    Some(raw_line[start + MARKER.len()..].trim())
}

pub(crate) fn replace_marker(raw_line: &str, spec: &str) -> Result<String, String> {
    decode_answers(spec)?;
    let lower = raw_line.to_ascii_lowercase();
    let mut base = lower
        .find(MARKER)
        .map_or(raw_line, |start| &raw_line[..start])
        .trim_end();
    if lower.contains(MARKER) {
        base = base.strip_suffix("//").unwrap_or(base).trim_end();
    }
    Ok(format!("{base} // @wlb-inputs: {spec}"))
}
