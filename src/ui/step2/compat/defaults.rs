// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::fs;
use std::path::PathBuf;

use crate::platform_defaults::app_config_file;

const STEP2_COMPAT_FILE_NAME: &str = "step2_compat_rules.toml";

pub fn create_default_step2_compat_rules_file() -> std::io::Result<PathBuf> {
    let path = step2_compat_rules_path();
    if path.exists() {
        return Ok(path);
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&path, default_step2_rules_content())?;
    Ok(path)
}

pub fn step2_compat_rules_path() -> PathBuf {
    app_config_file(STEP2_COMPAT_FILE_NAME, "config")
}

fn default_step2_rules_content() -> &'static str {
    r#"# Compatibility rules for greying out components after scan.
# Each rule matches by mod header name and optional component substring or id.
# issue types: included, not_needed, not_compatible, warning, conflict
#
# Example (copy/paste and edit):
# [[rules]]
# mod = "bg1ub"
# component = "Ice Island Level Two Restoration"
# mode = ["EET", "BGEE", "BG2EE"]   # optional: only apply when selected game mode matches
# tab = ["BGEE", "BG2EE"]           # optional: only apply on these Step 2 tabs
# kind = "included"
# message = "Already included in BG:EE v2.5+."
# source = "step2_compat_rules.toml" # optional: shown in Details pane
# related_mod = "SOME_MOD"           # optional: related conflict target
# related_component = "2"            # optional: related conflict component id

[[rules]]
mod = "bg1ub"
component = "Restored Elfsong Tavern Movie"
mode = ["EET", "BGEE", "BG2EE"]
tab = ["bgee", "bg2ee"]
kind = "not_compatible"
message = "Not compatible with BG:EE."

[[rules]]
mod = "bg1ub"
component = "Finishable Kagain Caravan Quest"
mode = ["EET", "BGEE", "BG2EE"]
tab = ["bgee", "bg2ee"]
kind = "included"
message = "Already included in BG:EE v2.5+."

[[rules]]
mod = "bg1ub"
component = "The Mysterious Vial"
mode = ["EET", "BGEE", "BG2EE"]
tab = ["bgee", "bg2ee"]
kind = "included"
message = "Already included in BG:EE v2.5+."

[[rules]]
mod = "bg1ub"
component = "Additional Elminster Encounter"
mode = ["EET", "BGEE", "BG2EE"]
tab = ["bgee", "bg2ee"]
kind = "included"
message = "Already fixed in BGEE/BGT."

[[rules]]
mod = "bg1ub"
component = "Flaming Fist Mercenary Reinforcements"
mode = ["EET", "BGEE", "BG2EE"]
tab = ["bgee", "bg2ee"]
kind = "included"
message = "Already included in BG:EE."

[[rules]]
mod = "bg1ub"
component = "Appropriate Albert and Rufie Reward"
mode = ["EET", "BGEE", "BG2EE"]
tab = ["bgee", "bg2ee"]
kind = "included"
message = "Already included in BG:EE."

[[rules]]
mod = "bg1ub"
component = "Prism and the Emeralds Tweak"
mode = ["EET", "BGEE", "BG2EE"]
tab = ["bgee", "bg2ee"]
kind = "included"
message = "Already included in BG:EE v2.5+."

[[rules]]
mod = "bg1ub"
component = "Branwen and Tranzig"
mode = ["EET", "BGEE", "BG2EE"]
tab = ["bgee", "bg2ee"]
kind = "included"
message = "Already included in BG:EE v2.5+."

[[rules]]
mod = "bg1ub"
component = "Kivan and Tazok"
mode = ["EET", "BGEE", "BG2EE"]
tab = ["bgee", "bg2ee"]
kind = "included"
message = "Already included in BG:EE v2.5+."

[[rules]]
mod = "bg1ub"
component = "Coran and the Wyverns"
mode = ["EET", "BGEE", "BG2EE"]
tab = ["bgee", "bg2ee"]
kind = "included"
message = "Already included in BG:EE."

[[rules]]
mod = "bg1ub"
component = "Place Entar Silvershield in His Home"
mode = ["EET", "BGEE", "BG2EE"]
tab = ["bgee", "bg2ee"]
kind = "included"
message = "Already included in BG:EE v2.5+."

[[rules]]
mod = "bg1ub"
component = "Safana the Flirt"
mode = ["EET", "BGEE", "BG2EE"]
tab = ["bgee", "bg2ee"]
kind = "included"
message = "Already included in BG:EE."

[[rules]]
mod = "bg1ub"
component = "Audio Restorations"
mode = ["EET", "BGEE", "BG2EE"]
tab = ["bgee", "bg2ee"]
kind = "included"
message = "Already included in BG:EE."

[[rules]]
mod = "bg1ub"
component = "Area Corrections and Restorations"
mode = ["EET", "BGEE", "BG2EE"]
tab = ["bgee", "bg2ee"]
kind = "included"
message = "Already included in BG:EE."

[[rules]]
mod = "bg1ub"
component = "Permanent Corpses"
mode = ["EET", "BGEE", "BG2EE"]
tab = ["bgee", "bg2ee"]
kind = "included"
message = "Already included in BG:EE."

[[rules]]
mod = "bg1ub"
component = "Elven Charm"
mode = ["EET", "BGEE", "BG2EE"]
tab = ["bgee", "bg2ee"]
kind = "included"
message = "Already included in BG:EE."

[[rules]]
mod = "bg1ub"
component = "Original Saga Music Playlist Corrections"
mode = ["EET", "BGEE", "BG2EE"]
tab = ["bgee", "bg2ee"]
kind = "included"
message = "Already included in BG:EE."

[[rules]]
mod = "bg1ub"
component = "Sarevok's Diary Corrections"
mode = ["EET", "BGEE", "BG2EE"]
tab = ["bgee", "bg2ee"]
kind = "included"
message = "Already included in BG:EE."
"#
}
