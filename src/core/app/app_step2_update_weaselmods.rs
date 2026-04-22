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
    if let Some(requested_version) = request.requested_version.as_deref()
        && normalize_version_text(&version) != normalize_version_text(requested_version)
    {
        return failed_weaselmods_outcome(
            request,
            &format!("exact version not found: {requested_version}"),
        );
    }
    let Some(download_url) = weaselmods_download_url(&html) else {
        return failed_weaselmods_outcome(request, "weaselmods page has no download url");
    };
    let file_stem = normalize_mod_download_tp2(&request.tp_file);
    let asset_name = format!("{file_stem}-{version}.zip");
    Step2UpdateCheckOutcome {
        game_tab: request.game_tab.clone(),
        tp_file: request.tp_file.clone(),
        label: request.label.clone(),
        tag: Some(version),
        asset_name: Some(asset_name),
        asset_url: Some(download_url),
        error: None,
        package_kind: Step2PackageKind::PageArchive,
    }
}

fn failed_weaselmods_outcome(
    request: &Step2UpdateCheckRequest,
    error: &str,
) -> Step2UpdateCheckOutcome {
    Step2UpdateCheckOutcome {
        game_tab: request.game_tab.clone(),
        tp_file: request.tp_file.clone(),
        label: request.label.clone(),
        tag: None,
        asset_name: None,
        asset_url: None,
        error: Some(error.to_string()),
        package_kind: Step2PackageKind::PageArchive,
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
