// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use super::app_step2_update_check::{
    Step2PackageKind, Step2UpdateCheckOutcome, Step2UpdateCheckRequest, failed_outcome,
};
use super::app_step2_update_source_refs::RemoteFileFacts;

const ARCHIVE_SUFFIXES_LONGEST_FIRST: [&str; 9] = [
    ".tar.bz2", ".tar.gz", ".tar.xz", ".tbz2", ".zip", ".rar", ".tgz", ".txz", ".7z",
];

pub(crate) fn direct_archive_names(url: &str) -> Option<(String, String)> {
    let trimmed = url.trim();
    let without_suffix = trimmed
        .find(['?', '#'])
        .map_or(trimmed, |cut| &trimmed[..cut]);
    let path = without_suffix.trim_end_matches('/');
    let file_name = path.rsplit('/').next().unwrap_or(path);
    if file_name.is_empty() {
        return None;
    }
    let lower = file_name.to_ascii_lowercase();
    let suffix = ARCHIVE_SUFFIXES_LONGEST_FIRST
        .iter()
        .find(|suffix| lower.ends_with(*suffix))?;
    let stem = &file_name[..file_name.len() - suffix.len()];
    if stem.is_empty() {
        return None;
    }
    Some((file_name.to_string(), stem.to_string()))
}

pub(crate) type RemoteFileProbe<'a> = &'a dyn Fn(&str) -> Result<RemoteFileFacts, String>;

const PROBE_USER_AGENT: &str = "BIO-update-check";

fn header_value(response: &ureq::Response, name: &str) -> Option<String> {
    response
        .header(name)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn content_length(response: &ureq::Response) -> Option<u64> {
    header_value(response, "Content-Length").and_then(|value| value.parse().ok())
}

fn content_range_total(response: &ureq::Response) -> Option<u64> {
    header_value(response, "Content-Range")
        .and_then(|value| value.rsplit('/').next().map(str::trim).map(str::to_string))
        .and_then(|total| total.parse().ok())
}

fn facts_from(response: &ureq::Response, size: Option<u64>) -> Result<RemoteFileFacts, String> {
    let facts = RemoteFileFacts {
        size,
        last_modified: header_value(response, "Last-Modified"),
        etag: header_value(response, "ETag"),
    };
    if facts == RemoteFileFacts::default() {
        return Err("server gives no file details".into());
    }
    Ok(facts)
}

pub(super) fn probe_remote_file(agent: &ureq::Agent, url: &str) -> Result<RemoteFileFacts, String> {
    if let Ok(response) = agent.head(url).set("User-Agent", PROBE_USER_AGENT).call()
        && (200..300).contains(&response.status())
    {
        return facts_from(&response, content_length(&response));
    }
    match agent
        .get(url)
        .set("User-Agent", PROBE_USER_AGENT)
        .set("Range", "bytes=0-0")
        .call()
    {
        Ok(response) => match response.status() {
            206 => facts_from(&response, content_range_total(&response)),
            200 => facts_from(&response, content_length(&response)),
            status => Err(format!("HTTP {status}")),
        },
        Err(ureq::Error::Status(status, _)) => Err(format!("HTTP {status}")),
        Err(err) => Err(err.to_string()),
    }
}

pub(super) fn check_direct_archive(
    request: &Step2UpdateCheckRequest,
    probe: RemoteFileProbe<'_>,
) -> Step2UpdateCheckOutcome {
    let Some((file_name, stem)) = direct_archive_names(&request.source_url) else {
        return failed_outcome(request.clone(), "direct link has no file name");
    };
    let facts = match probe(&request.source_url) {
        Ok(facts) => facts,
        Err(message) => return failed_outcome(request.clone(), &message),
    };
    Step2UpdateCheckOutcome {
        game_tab: request.game_tab.clone(),
        tp_file: request.tp_file.clone(),
        label: request.label.clone(),
        source_id: request.source_id.clone(),
        source_url: String::new(),
        tag: Some(stem),
        source_ref: None,
        asset_name: Some(file_name),
        asset_url: Some(request.source_url.trim().to_string()),
        error: None,
        package_kind: Step2PackageKind::ReleaseAsset,
        version_pin_overridden: None,
        remote_file: Some(facts),
    }
}

#[cfg(test)]
mod tests {
    use super::{check_direct_archive, direct_archive_names};
    use crate::app::app_step2_update_check::Step2UpdateCheckRequest;
    use crate::app::app_step2_update_source_refs::RemoteFileFacts;

    const QUESTPACK_URL: &str = "https://pocketplane.net/mods/questpack-v35-win.zip";

    fn questpack_request() -> Step2UpdateCheckRequest {
        Step2UpdateCheckRequest {
            game_tab: "BGEE".to_string(),
            tp_file: "d0questpack/setup-d0questpack.tp2".to_string(),
            label: "d0questpack".to_string(),
            source_id: "pocket-plane-group".to_string(),
            repo: String::new(),
            exact_github: vec![],
            source_url: QUESTPACK_URL.to_string(),
            channel: None,
            tag: None,
            commit: None,
            branch: None,
            release: None,
            asset: None,
            pkg: None,
            requested_version: None,
        }
    }

    fn pocketplane_facts() -> RemoteFileFacts {
        RemoteFileFacts {
            size: Some(21_707_615),
            last_modified: Some("Thu, 10 Sep 2020 17:25:37 GMT".to_string()),
            etag: None,
        }
    }

    #[test]
    fn probe_facts_ride_on_the_outcome() {
        let probed = std::cell::RefCell::new(Vec::new());
        let probe = |url: &str| {
            probed.borrow_mut().push(url.to_string());
            Ok(pocketplane_facts())
        };

        let outcome = check_direct_archive(&questpack_request(), &probe);

        assert_eq!(probed.into_inner(), vec![QUESTPACK_URL.to_string()]);
        assert!(outcome.error.is_none());
        assert_eq!(outcome.remote_file, Some(pocketplane_facts()));
        assert_eq!(outcome.tag.as_deref(), Some("questpack-v35-win"));
        assert_eq!(outcome.asset_name.as_deref(), Some("questpack-v35-win.zip"));
        assert_eq!(outcome.asset_url.as_deref(), Some(QUESTPACK_URL));
    }

    #[test]
    fn probe_failure_is_a_failed_outcome() {
        let probe = |_: &str| Err("HTTP 404".to_string());

        let outcome = check_direct_archive(&questpack_request(), &probe);

        assert_eq!(outcome.error.as_deref(), Some("HTTP 404"));
        assert_eq!(outcome.tag, None);
        assert_eq!(outcome.remote_file, None);
    }

    fn names(file_name: &str, stem: &str) -> (String, String) {
        (file_name.to_string(), stem.to_string())
    }

    #[test]
    fn direct_archive_names_splits_a_plain_zip() {
        assert_eq!(
            direct_archive_names("https://pocketplane.net/mods/questpack-v35-win.zip"),
            Some(names("questpack-v35-win.zip", "questpack-v35-win"))
        );
    }

    #[test]
    fn direct_archive_names_drops_query_and_fragment() {
        assert_eq!(
            direct_archive_names("https://h/x/mod.tar.gz?dl=1#top"),
            Some(names("mod.tar.gz", "mod"))
        );
    }

    #[test]
    fn direct_archive_names_is_case_insensitive() {
        assert_eq!(
            direct_archive_names("https://h/BGEETenya15c.RAR"),
            Some(names("BGEETenya15c.RAR", "BGEETenya15c"))
        );
    }

    #[test]
    fn direct_archive_names_refuses_a_bare_extension() {
        assert_eq!(direct_archive_names("https://h/.zip"), None);
    }

    #[test]
    fn direct_archive_names_refuses_a_folder_url() {
        assert_eq!(direct_archive_names("https://h/mods/"), None);
    }
}
