// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use super::app_step2_update_check::{
    Step2PackageKind, Step2UpdateCheckOutcome, Step2UpdateCheckRequest,
};
use crate::app::mod_downloads::normalize_mod_download_tp2;
use crate::parser::weidu_version::normalize_version_text;

pub(super) fn check_weaselmods_download_page(
    agent: &ureq::Agent,
    request: &Step2UpdateCheckRequest,
) -> Step2UpdateCheckOutcome {
    let response = match agent
        .get(request.source_url.trim())
        .set("User-Agent", "BIO-update-check")
        .call()
    {
        Ok(response) => response,
        Err(err) => return failed_weaselmods_outcome(request, &err.to_string()),
    };
    let html = match response.into_string() {
        Ok(value) => value,
        Err(err) => return failed_weaselmods_outcome(request, &err.to_string()),
    };
    let Some(version) = weaselmods_sidebar_value(&html, "Version") else {
        return failed_weaselmods_outcome(request, "weaselmods page has no version");
    };
    let pin_overridden = version_override(request.requested_version.as_deref(), &version);
    let Some(download_url) = weaselmods_download_url(&html) else {
        return failed_weaselmods_outcome(request, "weaselmods page has no download url");
    };
    let file_stem = normalize_mod_download_tp2(&request.tp_file);
    let asset_name = format!("{file_stem}-{version}.zip");
    Step2UpdateCheckOutcome {
        game_tab: request.game_tab.clone(),
        tp_file: request.tp_file.clone(),
        label: request.label.clone(),
        source_id: request.source_id.clone(),
        tag: Some(version),
        source_ref: None,
        asset_name: Some(asset_name),
        asset_url: Some(download_url),
        error: None,
        package_kind: Step2PackageKind::PageArchive,
        version_pin_overridden: pin_overridden,
    }
}

fn version_override(requested: Option<&str>, current: &str) -> Option<String> {
    let req = requested?;
    if normalize_version_text(req) == normalize_version_text(current) {
        return None;
    }
    Some(req.to_string())
}

fn failed_weaselmods_outcome(
    request: &Step2UpdateCheckRequest,
    error: &str,
) -> Step2UpdateCheckOutcome {
    Step2UpdateCheckOutcome {
        game_tab: request.game_tab.clone(),
        tp_file: request.tp_file.clone(),
        label: request.label.clone(),
        source_id: request.source_id.clone(),
        tag: None,
        source_ref: None,
        asset_name: None,
        asset_url: None,
        error: Some(error.to_string()),
        package_kind: Step2PackageKind::PageArchive,
        version_pin_overridden: None,
    }
}

fn weaselmods_sidebar_value(html: &str, label: &str) -> Option<String> {
    let needle = format!("<strong>{label}</strong><br/>");
    let start = html.find(&needle)? + needle.len();
    let tail = &html[start..];
    let end = tail.find('<').unwrap_or(tail.len());
    let value = tail[..end].trim();
    if value.is_empty() {
        None
    } else {
        Some(html_entity_decode_basic(value))
    }
}

fn weaselmods_download_url(html: &str) -> Option<String> {
    let needle = "data-downloadurl=\"";
    let start = html.find(needle)? + needle.len();
    let tail = &html[start..];
    let end = tail.find('"')?;
    let value = tail[..end].trim();
    if value.is_empty() {
        None
    } else {
        Some(html_entity_decode_basic(value))
    }
}

fn html_entity_decode_basic(value: &str) -> String {
    value
        .replace("&amp;", "&")
        .replace("&#038;", "&")
        .replace("&quot;", "\"")
        .replace("&#8217;", "'")
}

#[cfg(test)]
mod tests {
    use super::version_override;

    #[test]
    fn mismatch_returns_the_requested_version() {
        let result = version_override(Some("6.5.5"), "6.5.6");
        assert_eq!(result, Some("6.5.5".to_string()));
    }

    #[test]
    fn match_returns_none() {
        let result = version_override(Some("6.5.6"), "6.5.6");
        assert_eq!(result, None);
    }

    #[test]
    fn no_pin_returns_none() {
        let result = version_override(None, "6.5.6");
        assert_eq!(result, None);
    }

    #[test]
    fn normalized_match_with_v_prefix_returns_none() {
        let result = version_override(Some("v6.5.6"), "6.5.6");
        assert_eq!(result, None);
    }
}
