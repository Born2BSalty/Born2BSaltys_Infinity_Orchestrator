// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use serde::{Deserialize, Serialize};

use crate::ui::shared::redesign_tokens::ThemePalette;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct RedesignSettings {
    pub user_name: String,
    #[serde(
        default = "default_theme_palette",
        serialize_with = "serialize_theme_palette",
        deserialize_with = "deserialize_theme_palette"
    )]
    pub theme_palette: ThemePalette,
    pub language: String,
    pub validate_paths_on_startup: bool,
    pub diagnostic_mode: bool,
}

impl Default for RedesignSettings {
    fn default() -> Self {
        Self {
            user_name: "@b2bs".to_string(),
            theme_palette: ThemePalette::Dark,
            language: "English".to_string(),
            validate_paths_on_startup: true,
            diagnostic_mode: false,
        }
    }
}

const fn default_theme_palette() -> ThemePalette {
    ThemePalette::Dark
}

#[expect(
    clippy::trivially_copy_pass_by_ref,
    reason = "serde serialize_with requires a reference to the serialized field"
)]
fn serialize_theme_palette<S>(palette: &ThemePalette, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    let value = match palette {
        ThemePalette::Light => "light",
        ThemePalette::Dark => "dark",
    };
    serializer.serialize_str(value)
}

fn deserialize_theme_palette<'de, D>(deserializer: D) -> Result<ThemePalette, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = String::deserialize(deserializer)?;
    Ok(match value.as_str() {
        "light" | "Light" => ThemePalette::Light,
        _ => ThemePalette::Dark,
    })
}
