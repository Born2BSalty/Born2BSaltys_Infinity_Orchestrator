use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, TryLockError};
use std::time::{Duration, Instant, SystemTime};

use eframe::egui;

use crate::app::tool_version_probe::{ProbeOutput, query_path};
use crate::ui::settings::state_settings::PathStatusTone;
use crate::ui::settings::validate_now::resolve_on_path;

const DEBOUNCE: Duration = Duration::from_millis(200);
const PROBE_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ToolKind {
    WeiDu,
    ModInstaller,
}

impl ToolKind {
    const fn index(self) -> usize {
        match self {
            Self::WeiDu => 0,
            Self::ModInstaller => 1,
        }
    }

    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::WeiDu => "WeiDU",
            Self::ModInstaller => "Mod Installer",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ToolPlatform {
    Windows,
    Linux,
    MacOs,
}

impl ToolPlatform {
    pub(crate) const fn current() -> Self {
        if cfg!(target_os = "windows") {
            Self::Windows
        } else if cfg!(target_os = "macos") {
            Self::MacOs
        } else {
            Self::Linux
        }
    }
}

pub(crate) const fn guidance(kind: ToolKind, platform: ToolPlatform) -> &'static str {
    match (kind, platform) {
        (ToolKind::WeiDu, ToolPlatform::Windows) => {
            "Select weidu.exe from the extracted Windows download."
        }
        (ToolKind::WeiDu, ToolPlatform::Linux) => {
            "Select the weidu executable from the extracted Linux download."
        }
        (ToolKind::WeiDu, ToolPlatform::MacOs) => {
            "Select the weidu executable from the extracted macOS download."
        }
        (ToolKind::ModInstaller, ToolPlatform::Windows) => {
            "Select the Windows executable downloaded from the official release."
        }
        (ToolKind::ModInstaller, ToolPlatform::Linux) => {
            "Select the Linux executable downloaded from the official release."
        }
        (ToolKind::ModInstaller, ToolPlatform::MacOs) => {
            "Select the macOS executable downloaded from the official release."
        }
    }
}

pub(crate) const fn release_url(kind: ToolKind) -> &'static str {
    match kind {
        ToolKind::WeiDu => "https://github.com/WeiDUorg/weidu/releases",
        ToolKind::ModInstaller => "https://github.com/dark0dave/mod_installer/releases",
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) enum ToolStatus {
    #[default]
    Empty,
    Pending,
    Detected(String),
    InvalidPath(String),
    Unrecognized,
    Failed(String),
}

impl ToolStatus {
    pub(crate) fn hint(&self, kind: ToolKind) -> String {
        match self {
            Self::Empty => String::new(),
            Self::Pending => "Checking tool…".to_string(),
            Self::Detected(version) => {
                format!("Detected {} — version {version}", kind.label())
            }
            Self::InvalidPath(message) => message.clone(),
            Self::Unrecognized => {
                format!(
                    "This file exists, but it isn't recognized as {}.",
                    kind.label()
                )
            }
            Self::Failed(message) => format!("Could not check this executable: {message}"),
        }
    }

    pub(crate) const fn tone(&self) -> PathStatusTone {
        match self {
            Self::Empty | Self::Pending => PathStatusTone::Neutral,
            Self::Detected(_) => PathStatusTone::Success,
            Self::InvalidPath(_) | Self::Unrecognized | Self::Failed(_) => PathStatusTone::Error,
        }
    }
}

fn numeric_version(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .split('.')
            .all(|part| !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit()))
}

fn installer_version(value: &str) -> Option<&str> {
    if value.len() > 128 {
        return None;
    }
    let value = value.strip_prefix('v').unwrap_or(value);
    let (core_and_pre, build) = value
        .split_once('+')
        .map_or((value, None), |(left, right)| (left, Some(right)));
    let (core, pre) = core_and_pre
        .split_once('-')
        .map_or((core_and_pre, None), |(left, right)| (left, Some(right)));
    let valid_suffix = |suffix: &str| {
        suffix.split('.').all(|part| {
            !part.is_empty()
                && part
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
        })
    };
    if core.split('.').count() != 3
        || !numeric_version(core)
        || core
            .split('.')
            .any(|part| part.len() > 1 && part.starts_with('0'))
        || pre.is_some_and(|suffix| !valid_suffix(suffix))
        || build.is_some_and(|suffix| !valid_suffix(suffix))
    {
        return None;
    }
    Some(value)
}

fn line_version(kind: ToolKind, line: &str) -> Option<String> {
    let line = line.trim();
    let line = if line.starts_with('[') {
        line.rsplit_once(']')?.1.trim_start()
    } else {
        line
    };
    let mut words = line.split_whitespace();
    let name = words.next()?;
    match kind {
        ToolKind::WeiDu => {
            if !name.eq_ignore_ascii_case("weidu") || !words.next()?.eq_ignore_ascii_case("version")
            {
                return None;
            }
            let version = words.next()?;
            numeric_version(version).then(|| version.to_string())
        }
        ToolKind::ModInstaller => {
            if !["mod_installer", "mod-installer", "mod_runner"]
                .iter()
                .any(|expected| name.eq_ignore_ascii_case(expected))
            {
                return None;
            }
            installer_version(words.next()?).map(str::to_string)
        }
    }
}

pub(crate) fn classify(kind: ToolKind, output: &ProbeOutput) -> ToolStatus {
    let version = output
        .stdout
        .lines()
        .chain(output.stderr.lines())
        .find_map(|line| line_version(kind, line));
    match (version, output.success, kind) {
        (Some(version), _, ToolKind::WeiDu) | (Some(version), true, ToolKind::ModInstaller) => {
            ToolStatus::Detected(version)
        }
        (_, false, _) => ToolStatus::Failed("The executable returned an error.".to_string()),
        (None, true, _) => ToolStatus::Unrecognized,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ProbeKey {
    requested: String,
    resolved: PathBuf,
    length: u64,
    modified: Option<SystemTime>,
    readonly: bool,
    #[cfg(unix)]
    mode: u32,
}

impl ProbeKey {
    fn resolve(value: &str) -> Result<Option<Self>, String> {
        let requested = value.trim();
        if requested.is_empty() {
            return Ok(None);
        }
        let path = Path::new(requested);
        let resolved = if path.is_absolute() {
            path.to_path_buf()
        } else if path.is_file() {
            path.canonicalize()
                .map_err(|_| "Executable path could not be resolved.".to_string())?
        } else {
            resolve_on_path(requested)
                .ok_or_else(|| "Executable not found on PATH; select its full path.".to_string())?
        };
        let metadata = resolved
            .metadata()
            .map_err(|_| "Executable not found or could not be read.".to_string())?;
        if !metadata.is_file() {
            return Err("Select an executable file, not a folder.".to_string());
        }
        Ok(Some(Self {
            requested: requested.to_string(),
            resolved,
            length: metadata.len(),
            modified: metadata.modified().ok(),
            readonly: metadata.permissions().readonly(),
            #[cfg(unix)]
            mode: {
                use std::os::unix::fs::MetadataExt;
                metadata.mode()
            },
        }))
    }
}

pub(crate) type ProbeTicket = Arc<Mutex<Option<ToolStatus>>>;

#[derive(Debug, Clone)]
struct RunningProbe {
    key: ProbeKey,
    ticket: ProbeTicket,
}

#[derive(Debug, Clone, Default)]
struct ToolEntry {
    desired: Option<ProbeKey>,
    changed_at: Option<Instant>,
    status: ToolStatus,
    running: Option<RunningProbe>,
}

impl ToolEntry {
    fn completed(&self) -> Option<(ProbeKey, ToolStatus)> {
        let running = self.running.as_ref()?;
        let status = match running.ticket.try_lock() {
            Ok(result) => result.clone(),
            Err(TryLockError::WouldBlock) => None,
            Err(TryLockError::Poisoned(_)) => Some(ToolStatus::Failed(
                "The version worker stopped.".to_string(),
            )),
        }?;
        Some((running.key.clone(), status))
    }

    fn observe(&mut self, value: &str, now: Instant) {
        match ProbeKey::resolve(value) {
            Ok(desired) => {
                if self.desired != desired {
                    self.desired = desired;
                    self.changed_at = Some(now);
                    self.status = if self.desired.is_some() {
                        ToolStatus::Pending
                    } else {
                        ToolStatus::Empty
                    };
                } else if desired.is_none() {
                    self.status = ToolStatus::Empty;
                }
            }
            Err(message) => {
                self.desired = None;
                self.changed_at = None;
                self.status = ToolStatus::InvalidPath(message);
            }
        }
        if let Some((key, status)) = self.completed() {
            self.running = None;
            if self.desired.as_ref() == Some(&key) {
                self.status = status;
            }
        }
    }

    fn ready_path(&self, now: Instant) -> Option<&Path> {
        if self.running.is_some()
            || !matches!(self.status, ToolStatus::Pending)
            || self
                .changed_at
                .is_none_or(|at| now.saturating_duration_since(at) < DEBOUNCE)
        {
            return None;
        }
        self.desired.as_ref().map(|key| key.resolved.as_path())
    }
}

#[derive(Debug, Clone, Default)]
pub(crate) struct ToolValidationState {
    entries: [ToolEntry; 2],
}

impl ToolValidationState {
    pub(crate) fn observe_with(
        &mut self,
        kind: ToolKind,
        value: &str,
        now: Instant,
        launcher: impl FnOnce(ToolKind, &Path) -> Result<ProbeTicket, String>,
    ) -> ToolStatus {
        let entry = &mut self.entries[kind.index()];
        entry.observe(value, now);
        if let Some(path) = entry.ready_path(now) {
            match launcher(kind, path) {
                Ok(ticket) => {
                    entry.running = entry
                        .desired
                        .clone()
                        .map(|key| RunningProbe { key, ticket });
                }
                Err(error) => entry.status = ToolStatus::Failed(error),
            }
        }
        entry.status.clone()
    }

    pub(crate) fn observe(
        &mut self,
        kind: ToolKind,
        value: &str,
        context: &egui::Context,
    ) -> ToolStatus {
        let status = self.observe_with(kind, value, Instant::now(), |kind, path| {
            spawn_probe(kind, path, context.clone())
        });
        if matches!(status, ToolStatus::Pending) {
            context.request_repaint_after(Duration::from_millis(50));
        }
        status
    }
}

fn spawn_probe(kind: ToolKind, path: &Path, context: egui::Context) -> Result<ProbeTicket, String> {
    let ticket = Arc::new(Mutex::new(None));
    let result = Arc::clone(&ticket);
    let path = path.to_path_buf();
    std::thread::Builder::new()
        .name("bio-settings-tool-version".to_string())
        .spawn(move || {
            let status = match query_path(&path, PROBE_TIMEOUT) {
                Ok(output) => classify(kind, &output),
                Err(error) => ToolStatus::Failed(error.to_string().chars().take(160).collect()),
            };
            if let Ok(mut destination) = result.lock() {
                *destination = Some(status);
            }
            context.request_repaint();
        })
        .map_err(|error| error.to_string())?;
    Ok(ticket)
}

#[cfg(test)]
#[path = "tool_validation_tests.rs"]
mod tests;
