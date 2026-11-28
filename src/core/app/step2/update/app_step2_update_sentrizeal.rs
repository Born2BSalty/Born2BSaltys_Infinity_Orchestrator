// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::io::Read;

use super::app_step2_update_check::{
    Step2PackageKind, Step2UpdateCheckOutcome, Step2UpdateCheckRequest,
};
use crate::parser::weidu_version::normalize_version_text;

pub(crate) const PAGE_URL: &str = "https://www.sentrizeal.com/pst_7.htm";
const PAGE_LIMIT: u64 = 256 * 1024;

fn authorized_url(value: &str) -> Option<ureq::RequestUrl> {
    let value = value.trim();
    if value.chars().any(|ch| ch.is_control() || ch == '\\') {
        return None;
    }
    let parsed = ureq::get(value).request_url().ok()?;
    let url = parsed.as_url();
    (matches!(url.scheme(), "http" | "https")
        && matches!(
            url.host_str(),
            Some("sentrizeal.com" | "www.sentrizeal.com")
        )
        && url.username().is_empty()
        && url.password().is_none()
        && url.port().is_none()
        && url.query().is_none()
        && url.fragment().is_none())
    .then_some(parsed)
}

pub(super) fn is_source_url(value: &str) -> bool {
    authorized_url(value).is_some_and(|url| matches!(url.path(), "/pst_7.htm" | "/downloaditm5"))
}

pub(crate) fn download_referer(value: &str) -> Option<&'static str> {
    authorized_url(value)
        .filter(|url| url.path() == "/downloaditm5")
        .map(|_| PAGE_URL)
}

pub(super) fn download_request(agent: &ureq::Agent, url: &str) -> ureq::Request {
    let request = agent.get(url);
    if let Some(page) = download_referer(url) {
        request.set("Referer", page)
    } else {
        request
    }
}

pub(super) fn archive_prefix(reader: &mut dyn Read) -> Result<[u8; 4], String> {
    let mut prefix = [0; 4];
    reader
        .read_exact(&mut prefix)
        .map_err(|err| format!("Sentrizeal archive is empty or truncated: {err}"))?;
    if prefix != *b"PK\x03\x04" {
        return Err("Sentrizeal response is not a ZIP archive".to_string());
    }
    Ok(prefix)
}

pub(super) fn check_sentrizeal_download_page(
    agent: &ureq::Agent,
    request: &Step2UpdateCheckRequest,
) -> Step2UpdateCheckOutcome {
    match resolve(agent, request) {
        Ok(outcome) => outcome,
        Err(error) => Step2UpdateCheckOutcome {
            game_tab: request.game_tab.clone(),
            tp_file: request.tp_file.clone(),
            label: request.label.clone(),
            source_id: request.source_id.clone(),
            source_url: request.source_url.clone(),
            tag: None,
            source_ref: None,
            asset_name: None,
            asset_url: None,
            error: Some(error),
            package_kind: Step2PackageKind::PageArchive,
            version_pin_overridden: None,
            remote_file: None,
        },
    }
}

fn resolve(
    agent: &ureq::Agent,
    request: &Step2UpdateCheckRequest,
) -> Result<Step2UpdateCheckOutcome, String> {
    if !is_source_url(&request.source_url) {
        return Err("Unsupported Sentrizeal source".to_string());
    }
    let response = agent
        .get(PAGE_URL)
        .set("User-Agent", "BIO-update-check")
        .call()
        .map_err(|err| err.to_string())?;
    let mut html = String::new();
    response
        .into_reader()
        .take(PAGE_LIMIT + 1)
        .read_to_string(&mut html)
        .map_err(|err| err.to_string())?;
    if u64::try_from(html.len()).unwrap_or(u64::MAX) > PAGE_LIMIT {
        return Err("Sentrizeal page exceeds size limit".to_string());
    }
    let (version, download_url) = page_archive(&html)?;
    let response = download_request(agent, &download_url)
        .set("User-Agent", "BIO-update-check")
        .set("Range", "bytes=0-3")
        .call()
        .map_err(|err| err.to_string())?;
    let asset_name = response
        .header("Content-Disposition")
        .and_then(archive_filename)
        .ok_or_else(|| "Sentrizeal download has no safe ZIP filename".to_string())?;
    archive_prefix(&mut response.into_reader())?;
    let version_pin_overridden = request
        .requested_version
        .as_ref()
        .filter(|requested| normalize_version_text(requested) != normalize_version_text(&version))
        .cloned();
    Ok(Step2UpdateCheckOutcome {
        game_tab: request.game_tab.clone(),
        tp_file: request.tp_file.clone(),
        label: request.label.clone(),
        source_id: request.source_id.clone(),
        source_url: PAGE_URL.to_string(),
        tag: Some(version),
        source_ref: None,
        asset_name: Some(asset_name),
        asset_url: Some(download_url),
        error: None,
        package_kind: Step2PackageKind::PageArchive,
        version_pin_overridden,
        remote_file: None,
    })
}

fn page_archive(html: &str) -> Result<(String, String), String> {
    let mut text = String::new();
    let mut in_tag = false;
    for ch in html.chars() {
        match ch {
            '<' => {
                in_tag = true;
                text.push(' ');
            }
            '>' => in_tag = false,
            _ if !in_tag => text.push(ch),
            _ => {}
        }
    }
    let lower = text.to_ascii_lowercase();
    let version_start = lower
        .find("current version:")
        .ok_or_else(|| "Sentrizeal page has no current version".to_string())?
        + "current version:".len();
    let version = text[version_start..]
        .split_whitespace()
        .next()
        .filter(|value| {
            value.len() <= 64
                && value.starts_with(|ch: char| ch.is_ascii_digit())
                && value
                    .chars()
                    .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '-' | '_'))
        })
        .ok_or_else(|| "Sentrizeal page has no valid version".to_string())?;
    let lower = html.to_ascii_lowercase();
    let page = ureq::get(PAGE_URL)
        .request_url()
        .map_err(|err| err.to_string())?;
    for (start, _) in lower.match_indices("<a ") {
        let Some(end) = html[start..].find('>') else {
            continue;
        };
        let Some(href) = anchor_href(&html[start..start + end]) else {
            continue;
        };
        let Ok(url) = page.as_url().join(href) else {
            continue;
        };
        if download_referer(url.as_str()).is_some() {
            return Ok((version.to_string(), url.into()));
        }
    }
    Err("Sentrizeal page has no WeiDU pack download link".to_string())
}

fn anchor_href(tag: &str) -> Option<&str> {
    let lower = tag.to_ascii_lowercase();
    for (start, _) in lower.match_indices("href") {
        if !tag[..start].ends_with(char::is_whitespace) {
            continue;
        }
        let tail = tag[start + 4..]
            .trim_start()
            .strip_prefix('=')?
            .trim_start();
        let quote = tail.chars().next()?;
        if !matches!(quote, '\'' | '"') {
            continue;
        }
        let value = &tail[1..];
        return value.find(quote).map(|end| value[..end].trim());
    }
    None
}

fn archive_filename(disposition: &str) -> Option<String> {
    let name = disposition.split(';').find_map(|part| {
        let (key, value) = part.trim().split_once('=')?;
        if !key.trim().eq_ignore_ascii_case("filename") {
            return None;
        }
        let value = value.trim();
        value
            .strip_prefix('"')
            .map_or(Some(value), |quoted| quoted.strip_suffix('"'))
    })?;
    if name.is_empty()
        || name.len() > 200
        || name.starts_with('.')
        || name.ends_with([' ', '.'])
        || !name.to_ascii_lowercase().ends_with(".zip")
        || name.chars().any(|ch| {
            ch.is_control() || matches!(ch, '/' | '\\' | ':' | '<' | '>' | '"' | '|' | '?' | '*')
        })
    {
        return None;
    }
    Some(name.to_string())
}

#[cfg(test)]
pub mod test_support {
    use std::fs;
    use std::io::{Cursor, Read, Write};
    use std::net::{TcpListener, TcpStream};
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
    use std::sync::{Arc, Mutex};
    use std::thread::{self, JoinHandle};
    use std::time::Duration;

    use super::{PAGE_URL, Step2UpdateCheckRequest};

    pub(crate) const DOWNLOAD_URL: &str = "https://www.sentrizeal.com/downloaditm5";
    pub(crate) const HTML: &str = r#"<span class="bluecolor">Current Version: </span>1.6<br><h3>Download(s):</h3><a href="downloaditm5" target=_blank>Download Sentrizeal&#039;s Alternate Avatar Pack ~ WeiDU install</a>"#;
    pub(crate) const DISPOSITION: &str = "attachment; filename=\"SentrizealAvatarPack.zip\"";

    pub(crate) struct TestRoot(pub(crate) PathBuf);

    impl TestRoot {
        pub(crate) fn new() -> Self {
            static COUNTER: AtomicU64 = AtomicU64::new(0);
            Self::with_counter(&std::env::temp_dir(), &COUNTER)
        }

        pub(super) fn with_counter(parent: &Path, counter: &AtomicU64) -> Self {
            loop {
                let path = parent.join(format!(
                    "bio_sentrizeal_{}_{}",
                    std::process::id(),
                    counter.fetch_add(1, Ordering::Relaxed)
                ));
                match fs::create_dir(&path) {
                    Ok(()) => return Self(path),
                    Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => {}
                    Err(err) => panic!("Could not reserve Sentrizeal fixture root: {err}"),
                }
            }
        }
    }

    impl Drop for TestRoot {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    pub(crate) struct Fixture {
        pub(crate) agent: ureq::Agent,
        pub(crate) archive: Vec<u8>,
        requests: Arc<Mutex<Vec<String>>>,
        stop: Arc<AtomicBool>,
        server: Option<JoinHandle<()>>,
    }

    impl Fixture {
        pub(crate) fn pack() -> Self {
            let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
            zip.start_file(
                "Sentrizeal_AA_Pack/Sentrizeal_AA_Pack.tp2",
                zip::write::SimpleFileOptions::default(),
            )
            .unwrap();
            zip.write_all(b"BACKUP ~backup~\nAUTHOR ~fixture~\nVERSION ~1.6~\n")
                .unwrap();
            let archive = zip.finish().unwrap().into_inner();
            Self::new(HTML, Some(DISPOSITION), &archive)
        }

        pub(crate) fn new(html: &str, disposition: Option<&str>, archive: &[u8]) -> Self {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            listener.set_nonblocking(true).unwrap();
            let base = format!("http://{}", listener.local_addr().unwrap());
            let requests = Arc::new(Mutex::new(Vec::new()));
            let stop = Arc::new(AtomicBool::new(false));
            let server_requests = Arc::clone(&requests);
            let server_stop = Arc::clone(&stop);
            let page = html.to_string();
            let name = disposition.map(str::to_string);
            let body = archive.to_vec();
            let server = thread::spawn(move || {
                while !server_stop.load(Ordering::Relaxed) {
                    match listener.accept() {
                        Ok((stream, _)) => {
                            answer(stream, &page, name.as_deref(), &body, &server_requests);
                        }
                        Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => {
                            thread::sleep(Duration::from_millis(2));
                        }
                        Err(_) => break,
                    }
                }
            });
            let agent = ureq::AgentBuilder::new()
                .middleware(FixtureTransport { base })
                .build();
            Self {
                agent,
                archive: archive.to_vec(),
                requests,
                stop,
                server: Some(server),
            }
        }

        pub(crate) fn requests(&self) -> Vec<String> {
            self.requests.lock().unwrap().clone()
        }
    }

    struct FixtureTransport {
        base: String,
    }

    impl ureq::Middleware for FixtureTransport {
        fn handle(
            &self,
            request: ureq::Request,
            _: ureq::MiddlewareNext<'_>,
        ) -> Result<ureq::Response, ureq::Error> {
            let parsed = request.request_url().unwrap();
            assert!(super::is_source_url(request.url()));
            let mut mapped = ureq::get(&format!("{}{}", self.base, parsed.path()))
                .timeout(Duration::from_secs(3));
            for name in ["Referer", "Range", "User-Agent"] {
                if let Some(value) = request.header(name) {
                    mapped = mapped.set(name, value);
                }
            }
            mapped.call()
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            self.stop.store(true, Ordering::Relaxed);
            if let Some(server) = self.server.take() {
                let _ = server.join();
            }
        }
    }

    fn answer(
        mut stream: TcpStream,
        html: &str,
        disposition: Option<&str>,
        archive: &[u8],
        requests: &Mutex<Vec<String>>,
    ) {
        let _ = stream.set_read_timeout(Some(Duration::from_secs(3)));
        let mut bytes = Vec::new();
        let mut buffer = [0; 1024];
        while !bytes.ends_with(b"\r\n\r\n") && bytes.len() < 8192 {
            match stream.read(&mut buffer) {
                Ok(0) | Err(_) => return,
                Ok(count) => bytes.extend_from_slice(&buffer[..count]),
            }
        }
        let request = String::from_utf8_lossy(&bytes).into_owned();
        let page = request.starts_with("GET /pst_7.htm ");
        let authorized = request.lines().any(|line| {
            line.split_once(':').is_some_and(|(key, value)| {
                key.eq_ignore_ascii_case("Referer") && value.trim() == PAGE_URL
            })
        });
        requests.lock().unwrap().push(request);
        let body = if page {
            html.as_bytes()
        } else if authorized {
            archive
        } else {
            &[]
        };
        let name = if page {
            String::new()
        } else {
            disposition.map_or_else(String::new, |name| {
                format!("Content-Disposition: {name}\r\n")
            })
        };
        let header = format!(
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n{name}Connection: close\r\n\r\n",
            body.len()
        );
        let _ = stream.write_all(header.as_bytes());
        let _ = stream.write_all(body);
    }

    pub(crate) fn resolve_for_test(
        agent: &ureq::Agent,
        source: &str,
    ) -> super::Step2UpdateCheckOutcome {
        crate::app::app_step2_update_check::check_latest_release_for_worker(agent, request(source))
    }

    pub(crate) fn request(source: &str) -> Step2UpdateCheckRequest {
        Step2UpdateCheckRequest {
            game_tab: "BGEE".to_string(),
            tp_file: "Sentrizeal_AA_Pack.tp2".to_string(),
            label: "Sentrizeal Animation Pack".to_string(),
            source_id: "primary".to_string(),
            repo: String::new(),
            exact_github: Vec::new(),
            source_url: source.to_string(),
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
}

#[cfg(test)]
mod tests {
    use super::test_support::{DISPOSITION, DOWNLOAD_URL, Fixture, HTML, request};
    use super::*;
    use crate::app::app_step2_update_check::check_latest_release_for_worker;
    use crate::app::mod_downloads::{ModDownloadSource, source_is_auto_resolvable};

    #[test]
    fn sentrizeal_fixture_root_preserves_collisions_and_cleans_normal_and_unwind_paths() {
        use std::sync::atomic::AtomicU64;

        let parent = test_support::TestRoot::new();
        let counter = AtomicU64::new(0);
        let occupied = parent
            .0
            .join(format!("bio_sentrizeal_{}_0", std::process::id()));
        std::fs::create_dir(&occupied).unwrap();
        let sentinel = occupied.join("sentinel");
        std::fs::write(&sentinel, b"preserve").unwrap();
        let normal = test_support::TestRoot::with_counter(&parent.0, &counter);
        let normal_path = normal.0.clone();
        assert_ne!(normal_path, occupied);
        std::fs::write(normal.0.join("fixture"), b"owned").unwrap();
        drop(normal);
        assert!(!normal_path.exists());
        let mut unwind_path = None;
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let root = test_support::TestRoot::with_counter(&parent.0, &counter);
            unwind_path = Some(root.0.clone());
            std::fs::write(root.0.join("partial"), b"owned").unwrap();
            panic!("controlled Sentrizeal fixture setup failure");
        }));
        assert!(result.is_err());
        assert!(!unwind_path.unwrap().exists());
        assert_eq!(std::fs::read(&sentinel).unwrap(), b"preserve");
    }

    #[test]
    fn sentrizeal_only_recognizes_the_weidu_pack_page_and_legacy_endpoint() {
        for value in [
            PAGE_URL,
            DOWNLOAD_URL,
            "http://sentrizeal.com/downloaditm5",
            "https://sentrizeal.com/pst_7.htm",
            "https://www.sentrizeal.com:443/downloaditm5",
        ] {
            assert!(source_is_auto_resolvable(&ModDownloadSource {
                url: value.to_string(),
                ..Default::default()
            }));
        }
        for value in [
            "https://www.sentrizeal.com.evil.test/downloaditm5",
            "https://evil.test/www.sentrizeal.com/downloaditm5",
            "https://sentrizeal.com@evil.test/downloaditm5",
            "https://user@sentrizeal.com/downloaditm5",
            "https://sentrizeal.com:444/downloaditm5",
            "ftp://sentrizeal.com/downloaditm5",
            "https://sentrizeal.com/downloaditm50",
            "https://sentrizeal.com/downloaditm1",
            "https://sentrizeal.com/pst_8.htm",
            "https://sentrizeal.com/downloaditm5/file",
            "https://sentrizeal.com/downloaditm5?x=1",
            "https://sentrizeal.com/downloaditm5#fragment",
        ] {
            assert!(!is_source_url(value), "{value}");
            assert_eq!(download_referer(value), None, "{value}");
        }
        let agent = ureq::AgentBuilder::new().build();
        assert_eq!(
            download_request(&agent, "https://example.test/mod.zip").header("Referer"),
            None
        );
    }

    #[test]
    fn sentrizeal_page_parser_requires_version_and_authorized_pack_anchor() {
        assert_eq!(
            page_archive(HTML).unwrap(),
            ("1.6".to_string(), DOWNLOAD_URL.to_string())
        );
        assert!(page_archive("<a href=\"downloaditm5\">download</a>").is_err());
        assert!(
            page_archive("Current Version: <br><a href=\"downloaditm5\">download</a>").is_err()
        );
        assert!(page_archive("Current Version: 1.6<br>downloaditm5").is_err());
        for href in [
            "downloaditm1",
            "downloaditm50",
            "https://evil.test/downloaditm5",
            "javascript:downloaditm5",
        ] {
            assert!(
                page_archive(&format!(
                    "Current Version: 1.6<br><a href=\"{href}\">download</a>"
                ))
                .is_err()
            );
        }
        assert_eq!(page_archive(&HTML.replace("1.6", "2.7")).unwrap().0, "2.7");
    }

    #[test]
    fn sentrizeal_filename_requires_a_safe_archive_basename() {
        assert_eq!(
            archive_filename(DISPOSITION).as_deref(),
            Some("SentrizealAvatarPack.zip")
        );
        for name in [
            "../pack.zip",
            "C:\\pack.zip",
            "pack.exe",
            ".zip",
            "pack.zip ",
            "bad:pack.zip",
            "pack\n.zip",
            "",
        ] {
            assert!(
                archive_filename(&format!("attachment; filename=\"{name}\"")).is_none(),
                "{name}"
            );
        }
        assert!(archive_filename("attachment").is_none());
        assert!(archive_filename("attachment; filename=\"pack.zip").is_none());
    }

    #[test]
    fn sentrizeal_worker_resolves_canonical_and_legacy_with_page_referer() {
        for source in [PAGE_URL, DOWNLOAD_URL] {
            let fixture = Fixture::pack();
            let response = fixture.agent.get(DOWNLOAD_URL).call().unwrap();
            assert_eq!(response.header("Content-Length"), Some("0"));
            let mut pinned = request(source);
            pinned.requested_version = Some("1.5".to_string());
            let outcome = check_latest_release_for_worker(&fixture.agent, pinned);
            assert_eq!(outcome.error, None);
            assert_eq!(outcome.tag.as_deref(), Some("1.6"));
            assert_eq!(
                outcome.asset_name.as_deref(),
                Some("SentrizealAvatarPack.zip")
            );
            assert_eq!(outcome.asset_url.as_deref(), Some(DOWNLOAD_URL));
            assert_eq!(outcome.version_pin_overridden.as_deref(), Some("1.5"));
            assert!(matches!(
                outcome.package_kind,
                Step2PackageKind::PageArchive
            ));
            let requests = fixture.requests();
            assert!(requests[1].starts_with("GET /pst_7.htm "));
            assert!(requests[2].contains(&format!("Referer: {PAGE_URL}\r\n")));
            assert!(requests[2].contains("Range: bytes=0-3\r\n"));
            let mut matching = request(source);
            matching.requested_version = Some("v1.6".to_string());
            let outcome = check_latest_release_for_worker(&fixture.agent, matching);
            assert_eq!(outcome.version_pin_overridden, None);
        }
    }

    #[test]
    fn sentrizeal_worker_rejects_missing_metadata_empty_and_nonarchive_bodies() {
        for (html, name, body) in [
            (HTML, Some(DISPOSITION), b"".as_slice()),
            (HTML, Some(DISPOSITION), b"<html>blocked</html>".as_slice()),
            (HTML, None, b"PK\x03\x04".as_slice()),
            ("malformed", Some(DISPOSITION), b"PK\x03\x04".as_slice()),
        ] {
            let fixture = Fixture::new(html, name, body);
            let outcome = check_latest_release_for_worker(&fixture.agent, request(PAGE_URL));
            assert!(outcome.error.is_some());
            assert!(outcome.asset_url.is_none());
        }
        let fixture = Fixture::new(
            &"x".repeat(usize::try_from(PAGE_LIMIT + 1).unwrap()),
            None,
            b"",
        );
        let outcome = check_latest_release_for_worker(&fixture.agent, request(PAGE_URL));
        assert!(outcome.error.unwrap().contains("size limit"));
    }
}
