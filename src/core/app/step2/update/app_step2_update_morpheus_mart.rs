// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::sync::Arc;

use super::app_step2_update_check::{
    Step2PackageKind, Step2UpdateCheckOutcome, Step2UpdateCheckRequest,
};
use crate::parser::weidu_version::normalize_version_text;

pub(super) fn check_morpheus_mart_download_page(
    agent: &ureq::Agent,
    request: &Step2UpdateCheckRequest,
) -> Step2UpdateCheckOutcome {
    let response = match fetch_morpheus_mart_page(agent, request.source_url.trim()) {
        Ok(response) => response,
        Err(err) => return failed_morpheus_mart_outcome(request, &err),
    };
    let html = match response.into_string() {
        Ok(value) => value,
        Err(err) => return failed_morpheus_mart_outcome(request, &err.to_string()),
    };
    let Some(download_url) = morpheus_mart_download_url(&html) else {
        return failed_morpheus_mart_outcome(request, "morpheus-mart page has no download url");
    };
    let Some(asset_name) = filename_from_url(&download_url) else {
        return failed_morpheus_mart_outcome(request, "morpheus-mart download has no filename");
    };
    let Some(version) = version_from_filename(&asset_name) else {
        return failed_morpheus_mart_outcome(request, "morpheus-mart filename has no version");
    };
    if let Some(requested_version) = request.requested_version.as_deref()
        && normalize_version_text(&version) != normalize_version_text(requested_version)
    {
        return failed_morpheus_mart_outcome(
            request,
            &format!("exact version not found: {requested_version}"),
        );
    }
    Step2UpdateCheckOutcome {
        game_tab: request.game_tab.clone(),
        tp_file: request.tp_file.clone(),
        label: request.label.clone(),
        source_id: request.source_id.clone(),
        tag: Some(version),
        source_ref: None,
        asset_name: Some(asset_name),
        asset_url: Some(force_dropbox_download(&download_url)),
        error: None,
        package_kind: Step2PackageKind::PageArchive,
    }
}

fn fetch_morpheus_mart_page(agent: &ureq::Agent, url: &str) -> Result<ureq::Response, String> {
    match agent.get(url).set("User-Agent", "BIO-update-check").call() {
        Ok(response) => Ok(response),
        Err(err) if url.starts_with("https://") && err.to_string().contains("UnknownIssuer") => {
            let mut builder = native_tls::TlsConnector::builder();
            builder.danger_accept_invalid_certs(true);
            let connector = builder.build().map_err(|err| err.to_string())?;
            let insecure_agent = ureq::AgentBuilder::new()
                .tls_connector(Arc::new(connector))
                .timeout_connect(std::time::Duration::from_secs(10))
                .timeout_read(std::time::Duration::from_secs(20))
                .build();
            insecure_agent
                .get(url)
                .set("User-Agent", "BIO-update-check")
                .call()
                .map_err(|err| err.to_string())
        }
        Err(err) => Err(err.to_string()),
    }
}

fn failed_morpheus_mart_outcome(
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
    }
}

fn morpheus_mart_download_url(html: &str) -> Option<String> {
    let label = html.find(">Download<")?;
    let anchor = &html[..label];
    let start = anchor.rfind("<a ")?;
    let href = anchor[start..].find("href=")? + start;
    let tail = html[href + "href=".len()..].trim_start();
    let quote = tail.chars().next()?;
    if quote != '"' && quote != '\'' {
        return None;
    }
    let tail = &tail[quote.len_utf8()..];
    let end = tail.find(quote)?;
    let value = tail[..end].trim();
    if value.is_empty() {
        None
    } else {
        Some(html_entity_decode_basic(value))
    }
}

fn filename_from_url(url: &str) -> Option<String> {
    let path = url.split('?').next().unwrap_or(url);
    let name = path.rsplit('/').next()?.trim();
    if name.is_empty() {
        None
    } else {
        Some(name.to_string())
    }
}

fn version_from_filename(name: &str) -> Option<String> {
    let stem = name
        .trim()
        .strip_suffix(".zip")
        .or_else(|| name.trim().strip_suffix(".7z"))
        .or_else(|| name.trim().strip_suffix(".rar"))
        .unwrap_or(name.trim());
    let version = stem.rsplit_once('-')?.1.trim();
    if version.is_empty() {
        None
    } else {
        Some(
            version
                .trim_start_matches('v')
                .trim_start_matches('V')
                .to_string(),
        )
    }
}

fn force_dropbox_download(url: &str) -> String {
    if url.contains("dl=0") {
        url.replacen("dl=0", "dl=1", 1)
    } else if url.contains("dl=1") {
        url.to_string()
    } else if url.contains('?') {
        format!("{url}&dl=1")
    } else {
        format!("{url}?dl=1")
    }
}

fn html_entity_decode_basic(value: &str) -> String {
    value
        .replace("&amp;", "&")
        .replace("&#038;", "&")
        .replace("&quot;", "\"")
        .replace("&#8217;", "'")
}
