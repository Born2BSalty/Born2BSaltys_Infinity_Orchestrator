use toml_edit::{Array, DocumentMut, InlineTable, Item, Table, Value};

pub(super) fn render(base: &[u8], merged: &toml::Value) -> Result<Vec<u8>, String> {
    let source = std::str::from_utf8(base).map_err(|error| error.to_string())?;
    let crlf = source.contains("\r\n");
    let normalized = source.replace("\r\n", "\n");
    if crlf && normalized.contains('\r') {
        return Err("Parser has unsupported mixed carriage-return layout".to_string());
    }
    if crlf && normalized.matches('\n').count() != source.matches("\r\n").count() {
        return Err("Parser has unsupported mixed line endings".to_string());
    }
    let mut document = normalized
        .parse::<DocumentMut>()
        .map_err(|error| error.to_string())?;
    if document.to_string() != normalized {
        return Err("Parser layout cannot be retained without rewriting source".to_string());
    }
    let original: toml::Value = toml::from_str(source).map_err(|error| error.to_string())?;
    let desired = toml::to_string(merged)
        .map_err(|error| error.to_string())?
        .parse::<DocumentMut>()
        .map_err(|error| error.to_string())?;
    merge_table(
        document.as_table_mut(),
        desired.as_table(),
        &original,
        merged,
    )?;
    let rendered = document.to_string();
    let rendered = if crlf {
        rendered.replace('\n', "\r\n")
    } else {
        rendered
    };
    let verified: toml::Value = toml::from_str(&rendered).map_err(|error| error.to_string())?;
    if &verified != merged {
        return Err("Layout-preserving parser merge changed effective settings".to_string());
    }
    Ok(rendered.into_bytes())
}

fn merge_table(
    current: &mut Table,
    desired: &Table,
    original: &toml::Value,
    merged: &toml::Value,
) -> Result<(), String> {
    for (key, desired_item) in desired {
        let desired_value = merged
            .get(key)
            .ok_or_else(|| format!("Missing merged parser setting: {key}"))?;
        if original.get(key) == Some(desired_value) {
            continue;
        }
        if let Some(current_item) = current.get_mut(key) {
            update_item(current_item, desired_item, original.get(key), desired_value)?;
        } else {
            current.insert(key, desired_item.clone());
        }
    }
    Ok(())
}

fn update_item(
    current: &mut Item,
    desired: &Item,
    original: Option<&toml::Value>,
    merged: &toml::Value,
) -> Result<(), String> {
    if let (Some(current_table), Some(desired_table), Some(original)) =
        (current.as_table_mut(), desired.as_table(), original)
    {
        return merge_table(current_table, desired_table, original, merged);
    }
    if let (Some(current_table), Some(desired_table), Some(original)) =
        (current.as_inline_table_mut(), desired.as_table(), original)
    {
        return merge_inline_table(current_table, desired_table, original, merged);
    }
    if let (Some(current_array), Some(desired_array)) = (current.as_array_mut(), desired.as_array())
    {
        rewrite_array(current_array, desired_array)?;
        return Ok(());
    }
    let decoration = current.as_value().map(|value| value.decor().clone());
    let mut replacement = desired.clone();
    if let (Some(decoration), Some(value)) = (decoration, replacement.as_value_mut()) {
        *value.decor_mut() = decoration;
    }
    *current = replacement;
    Ok(())
}

fn merge_inline_table(
    current: &mut InlineTable,
    desired: &Table,
    original: &toml::Value,
    merged: &toml::Value,
) -> Result<(), String> {
    for (key, desired_item) in desired {
        let desired_value = merged
            .get(key)
            .ok_or_else(|| format!("Missing merged parser setting: {key}"))?;
        if original.get(key) == Some(desired_value) {
            continue;
        }
        if let Some(value) = current.get_mut(key) {
            if let (Some(table), Some(desired_table), Some(original)) = (
                value.as_inline_table_mut(),
                desired_item.as_table(),
                original.get(key),
            ) {
                merge_inline_table(table, desired_table, original, desired_value)?;
            } else {
                let mut replacement = inline_value(desired_item)?;
                *replacement.decor_mut() = value.decor().clone();
                *value = replacement;
            }
        } else {
            current.insert(key, inline_value(desired_item)?);
        }
    }
    Ok(())
}

fn inline_value(item: &Item) -> Result<Value, String> {
    item.as_value().map_or_else(
        || {
            item.as_table().map_or_else(
                || Err("Cannot preserve parser inline-table layout for this setting".to_string()),
                |table| Ok(Value::InlineTable(table.clone().into_inline_table())),
            )
        },
        |value| Ok(value.clone()),
    )
}

fn prefix(value: &Value) -> &str {
    value
        .decor()
        .prefix()
        .and_then(toml_edit::RawString::as_str)
        .unwrap_or_default()
}

fn leading_layout(raw: &str) -> String {
    raw.split_once('\n')
        .map_or_else(|| raw.to_string(), |(_, rest)| format!("\n{rest}"))
}

fn preceding_comment(raw: &str) -> &str {
    raw.split_once('\n').map_or("", |(line, _)| line)
}

fn rewrite_array(current: &mut Array, desired: &Array) -> Result<(), String> {
    if desired.iter().any(|value| !value.is_str()) {
        return Err("Cannot preserve changed non-string parser array layout".to_string());
    }
    let existing: Vec<Value> = current.iter().cloned().collect();
    let trailing = current.trailing().as_str().unwrap_or_default().to_string();
    let multiline =
        trailing.contains('\n') || existing.iter().any(|value| prefix(value).contains('\n'));
    let decoration = current.decor().clone();
    let trailing_comma = current.trailing_comma();
    let mut replacement = current.clone();
    replacement.clear();
    let mut opening =
        preceding_comment(existing.first().map_or(trailing.as_str(), prefix)).to_string();
    let mut previous_comment = String::new();
    let indent = existing.last().map_or("", |value| {
        prefix(value).rsplit('\n').next().unwrap_or_default()
    });
    let indent = if indent.chars().all(|ch| matches!(ch, ' ' | '\t')) {
        indent
    } else {
        ""
    };
    for desired_value in desired {
        let matched = existing
            .iter()
            .position(|value| value.as_str() == desired_value.as_str());
        let mut value =
            matched.map_or_else(|| desired_value.clone(), |index| existing[index].clone());
        if multiline {
            let leading = matched.map_or_else(
                || format!("\n{indent}"),
                |index| leading_layout(prefix(&existing[index])),
            );
            value
                .decor_mut()
                .set_prefix(format!("{opening}{previous_comment}{leading}"));
            opening.clear();
            previous_comment = matched.map_or_else(String::new, |index| {
                let following = existing.get(index + 1).map_or(trailing.as_str(), prefix);
                preceding_comment(following).to_string()
            });
        } else if matched.is_none() {
            value.decor_mut().set_prefix(" ");
        }
        replacement.push_formatted(value);
    }
    if multiline {
        let closing = trailing
            .split_once('\n')
            .map_or_else(|| "\n".to_string(), |(_, rest)| format!("\n{rest}"));
        replacement.set_trailing(format!("{opening}{previous_comment}{closing}"));
    } else {
        replacement.set_trailing(trailing);
    }
    replacement.set_trailing_comma(trailing_comma);
    *replacement.decor_mut() = decoration;
    *current = replacement;
    Ok(())
}
