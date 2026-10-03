// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::app::mod_downloads::{self, ModDownloadSource, ModDownloadTp2Rename};
use crate::app::step2_action::ModSourceEditDestination;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SourceKind {
    GitHub,
    WeaselMods,
    MorpheusMart,
    DirectLink,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Follow {
    Commit,
    Tag,
    Branch,
    Release,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) enum AssetPick {
    #[default]
    SourceZip,
    BestForOs,
    Named(String),
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct KeptFields {
    pub(crate) exact_github: Vec<String>,
    pub(crate) tp2_rename: Option<ModDownloadTp2Rename>,
    pub(crate) source_default: bool,
    pub(crate) pkg_windows: Option<String>,
    pub(crate) pkg_linux: Option<String>,
    pub(crate) pkg_macos: Option<String>,
    pub(crate) asset: Option<String>,
    pub(crate) channel_word: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct SourceFormIdentity {
    pub(crate) may_change_id: bool,
    pub(crate) is_new_mod: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SourceForm {
    pub(crate) tp2: String,
    pub(crate) card_key: String,
    pub(crate) name: String,
    pub(crate) source_id: String,
    pub(crate) label: String,
    pub(crate) identity: SourceFormIdentity,
    pub(crate) kind: SourceKind,
    pub(crate) repo: String,
    pub(crate) link: String,
    pub(crate) follow: Follow,
    pub(crate) commit: String,
    pub(crate) tag: String,
    pub(crate) branch: String,
    pub(crate) allow_pre: bool,
    pub(crate) release: String,
    pub(crate) asset: AssetPick,
    pub(crate) pkg_windows: String,
    pub(crate) pkg_linux: String,
    pub(crate) pkg_macos: String,
    pub(crate) aliases_text: String,
    pub(crate) subdir_require: String,
    pub(crate) config_files: Vec<String>,
    pub(crate) config_files_focus_pending: Option<usize>,
    pub(crate) save_to: ModSourceEditDestination,
    pub(crate) advanced_open: bool,
    pub(crate) release_query: String,
    pub(crate) notice: Option<String>,
    pub(crate) error: Option<String>,
    pub(crate) kept: KeptFields,
    pub(crate) note: String,
    pub(crate) note_seed: String,
    pub(crate) note_who: String,
}

const PREONLY_NOTICE: &str =
    "This source takes pre-releases only; saving from the form widens it to releases too.";

#[derive(Debug, Default)]
struct FollowDetails {
    commit: String,
    tag: String,
    branch: String,
    release: String,
    asset: AssetPick,
    allow_pre: bool,
    notice: Option<String>,
    newest_asset: Option<String>,
    channel_word: Option<String>,
}

fn compute_follow(source: &ModDownloadSource, details: &mut FollowDetails) -> Follow {
    if let Some(value) = non_empty(source.commit.as_deref()) {
        details.commit = value.to_string();
        return Follow::Commit;
    }
    if let Some(value) = non_empty(source.tag.as_deref()) {
        details.tag = value.to_string();
        return Follow::Tag;
    }
    if let Some(value) = non_empty(source.branch.as_deref()) {
        details.branch = value.to_string();
        return Follow::Branch;
    }
    if let Some(value) = non_empty(source.release.as_deref()) {
        details.release = value.to_string();
        details.asset = non_empty(source.asset.as_deref()).map_or(AssetPick::BestForOs, |name| {
            AssetPick::Named(name.to_string())
        });
        return Follow::Release;
    }
    let raw_channel = non_empty(source.channel.as_deref());
    let lower_channel = raw_channel.map(str::to_ascii_lowercase);
    let follow = match lower_channel.as_deref() {
        None => Follow::Release,
        Some("master") => Follow::Branch,
        Some("preonly") => {
            details.allow_pre = true;
            details.notice = Some(PREONLY_NOTICE.to_string());
            Follow::Release
        }
        Some("pre-release") => {
            details.allow_pre = true;
            Follow::Release
        }
        Some("release" | "releases") => {
            details.channel_word = raw_channel.map(str::to_string);
            Follow::Release
        }
        Some(_) => {
            let original = raw_channel.unwrap_or_default();
            details.notice = Some(format!(
                "This source uses a setting the form cannot show ({original}); saving replaces it."
            ));
            Follow::Release
        }
    };
    if follow == Follow::Release
        && let Some(asset) = non_empty(source.asset.as_deref())
    {
        details.newest_asset = Some(asset.to_string());
        if details.notice.is_none() {
            details.notice = Some(format!(
                "This source picks the file {asset} from the newest release; the form keeps it."
            ));
        }
    }
    follow
}

pub(crate) fn from_source(
    source: &ModDownloadSource,
    identity: SourceFormIdentity,
    save_to: ModSourceEditDestination,
    card_key: &str,
) -> SourceForm {
    let kind = source_kind(source);
    let (repo, link) = match kind {
        SourceKind::GitHub => (source.github.clone().unwrap_or_default(), String::new()),
        SourceKind::WeaselMods | SourceKind::MorpheusMart | SourceKind::DirectLink => {
            (String::new(), source.url.clone())
        }
    };

    let mut details = FollowDetails::default();
    let follow = compute_follow(source, &mut details);

    SourceForm {
        tp2: source.tp2.clone(),
        card_key: card_key.to_string(),
        name: source.name.clone(),
        source_id: source.source_id.clone(),
        label: source.source_label.clone(),
        identity,
        kind,
        repo,
        link,
        follow,
        commit: details.commit,
        tag: details.tag,
        branch: details.branch,
        allow_pre: details.allow_pre,
        release: details.release,
        asset: details.asset,
        pkg_windows: source.pkg_windows.clone().unwrap_or_default(),
        pkg_linux: source.pkg_linux.clone().unwrap_or_default(),
        pkg_macos: source.pkg_macos.clone().unwrap_or_default(),
        aliases_text: source.aliases.join(", "),
        subdir_require: source.subdir_require.clone().unwrap_or_default(),
        config_files: source.config_files.clone(),
        config_files_focus_pending: None,
        save_to,
        advanced_open: false,
        release_query: String::new(),
        notice: details.notice,
        error: None,
        note: String::new(),
        note_seed: String::new(),
        note_who: String::new(),
        kept: KeptFields {
            exact_github: source.exact_github.clone(),
            tp2_rename: source.tp2_rename.clone(),
            source_default: source.source_default,
            pkg_windows: source.pkg_windows.clone(),
            pkg_linux: source.pkg_linux.clone(),
            pkg_macos: source.pkg_macos.clone(),
            asset: details.newest_asset,
            channel_word: details.channel_word,
        },
    }
}

pub(crate) fn to_source(form: &SourceForm) -> ModDownloadSource {
    let mut source = ModDownloadSource {
        tp2: form.tp2.clone(),
        name: form.name.clone(),
        source_id: form.source_id.clone(),
        source_label: form.label.clone(),
        source_default: form.identity.is_new_mod || form.kept.source_default,
        exact_github: form.kept.exact_github.clone(),
        tp2_rename: form.kept.tp2_rename.clone(),
        config_files: normalised_config_files(&form.config_files),
        aliases: parse_aliases(&form.aliases_text),
        subdir_require: non_empty_owned(&form.subdir_require),
        ..ModDownloadSource::default()
    };

    match form.kind {
        SourceKind::GitHub => {
            let repo = form.repo.trim();
            source.github = Some(repo.to_string());
            source.url = format!("https://github.com/{repo}");
            apply_follow(&mut source, form);
        }
        SourceKind::WeaselMods | SourceKind::MorpheusMart => {
            source.url = form.link.trim().to_string();
            source.kind = Some("page".to_string());
        }
        SourceKind::DirectLink => {
            source.url = form.link.trim().to_string();
            source.kind = Some("url".to_string());
        }
    }

    source
}

fn apply_follow(source: &mut ModDownloadSource, form: &SourceForm) {
    match form.follow {
        Follow::Commit => source.commit = non_empty_owned(&form.commit),
        Follow::Tag => source.tag = non_empty_owned(&form.tag),
        Follow::Branch => {
            source.branch = non_empty_owned(&form.branch);
            if source.branch.is_none() {
                source.channel = Some("master".to_string());
            }
        }
        Follow::Release => apply_release(source, form),
    }
}

fn apply_release(source: &mut ModDownloadSource, form: &SourceForm) {
    let release = form.release.trim();
    if release.is_empty() {
        source.channel = release_newest_channel(form);
        source.pkg_windows = non_empty_owned(&form.pkg_windows);
        source.pkg_linux = non_empty_owned(&form.pkg_linux);
        source.pkg_macos = non_empty_owned(&form.pkg_macos);
        source.asset.clone_from(&form.kept.asset);
        return;
    }
    match &form.asset {
        AssetPick::SourceZip => source.tag = Some(release.to_string()),
        AssetPick::BestForOs => {
            source.release = Some(release.to_string());
            source.pkg_windows.clone_from(&form.kept.pkg_windows);
            source.pkg_linux.clone_from(&form.kept.pkg_linux);
            source.pkg_macos.clone_from(&form.kept.pkg_macos);
        }
        AssetPick::Named(name) => {
            source.release = Some(release.to_string());
            source.asset = non_empty_owned(name);
        }
    }
}

fn release_newest_channel(form: &SourceForm) -> Option<String> {
    if form.allow_pre {
        Some("pre-release".to_string())
    } else {
        form.kept.channel_word.clone()
    }
}

pub(crate) fn source_form_save_text(form: &SourceForm) -> String {
    let source = to_source(form);
    let block = mod_downloads::complete_source_block(&source);
    let header = mod_downloads::template_mod_header(&form.tp2, &form.name);
    format!("{header}\n\n{block}\n")
}

pub(crate) fn will_fetch(form: &SourceForm) -> String {
    match form.kind {
        SourceKind::GitHub => will_fetch_github(form),
        SourceKind::WeaselMods => will_fetch_link(form, "newest file on this Weasel Mods page"),
        SourceKind::MorpheusMart => will_fetch_link(form, "newest file on this Morpheus Mart page"),
        SourceKind::DirectLink => will_fetch_direct_link(form),
    }
}

fn will_fetch_github(form: &SourceForm) -> String {
    let repo = form.repo.trim();
    if repo.is_empty() {
        return "add a repository".to_string();
    }
    match form.follow {
        Follow::Commit => format!(
            "source zip of commit {} from {repo}",
            short_commit(&form.commit)
        ),
        Follow::Tag => format!("source zip of tag {} from {repo}", form.tag.trim()),
        Follow::Branch => {
            let branch = form.branch.trim();
            if branch.is_empty() {
                format!("source zip of the default branch of {repo}")
            } else {
                format!("source zip of branch {branch} from {repo}")
            }
        }
        Follow::Release => will_fetch_release(form, repo),
    }
}

fn will_fetch_release(form: &SourceForm, repo: &str) -> String {
    let release = form.release.trim();
    if release.is_empty() {
        if let Some(asset) = form.kept.asset.as_deref() {
            return format!("{asset} from the newest release of {repo}");
        }
        return if form.allow_pre {
            format!("newest release or pre-release of {repo}")
        } else {
            format!("newest release of {repo}")
        };
    }
    match &form.asset {
        AssetPick::SourceZip => format!("source zip of release {release}"),
        AssetPick::BestForOs => format!("best file for this OS from release {release}"),
        AssetPick::Named(name) => format!("{name} from release {release}"),
    }
}

fn will_fetch_link(form: &SourceForm, sentence: &str) -> String {
    if form.link.trim().is_empty() {
        "paste a link".to_string()
    } else {
        sentence.to_string()
    }
}

fn will_fetch_direct_link(form: &SourceForm) -> String {
    let link = form.link.trim();
    if link.is_empty() {
        return "paste a link".to_string();
    }
    if mod_downloads::is_direct_archive_url(link) {
        format!("{} from {}", file_name_from_url(link), host_from_url(link))
    } else {
        "nothing automatically \u{b7} manual download from this page".to_string()
    }
}

fn source_kind(source: &ModDownloadSource) -> SourceKind {
    if source.github.is_some() {
        SourceKind::GitHub
    } else if mod_downloads::source_is_weaselmods_page_url(&source.url) {
        SourceKind::WeaselMods
    } else if mod_downloads::source_is_morpheus_mart_page_url(&source.url) {
        SourceKind::MorpheusMart
    } else {
        SourceKind::DirectLink
    }
}

fn short_commit(value: &str) -> String {
    value.trim().chars().take(7).collect()
}

fn host_from_url(url: &str) -> String {
    let trimmed = url.trim();
    let without_scheme = trimmed
        .strip_prefix("https://")
        .or_else(|| trimmed.strip_prefix("http://"))
        .unwrap_or(trimmed);
    without_scheme
        .split('/')
        .next()
        .unwrap_or(without_scheme)
        .to_string()
}

fn file_name_from_url(url: &str) -> String {
    let trimmed = url.trim().trim_end_matches('/');
    trimmed.rsplit('/').next().unwrap_or(trimmed).to_string()
}

fn parse_aliases(text: &str) -> Vec<String> {
    text.split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .collect()
}

fn normalised_config_files(rows: &[String]) -> Vec<String> {
    let mut names: Vec<String> = Vec::new();
    for name in rows
        .iter()
        .map(|row| row.trim())
        .filter(|value| !value.is_empty())
        .map(|value| value.replace('\\', "/"))
    {
        if !names.contains(&name) {
            names.push(name);
        }
    }
    names
}

pub(crate) fn config_files_error(form: &SourceForm) -> Option<String> {
    normalised_config_files(&form.config_files)
        .iter()
        .find_map(|entry| {
            crate::app::modlist_config_files::validate_relative_config_path(entry).err()
        })
        .map(|err| format!("Config files: {err}"))
}

fn non_empty(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}

fn non_empty_owned(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_form(kind: SourceKind) -> SourceForm {
        let (repo, link) = match kind {
            SourceKind::GitHub => ("owner/repo".to_string(), String::new()),
            SourceKind::WeaselMods => (
                String::new(),
                "https://downloads.weaselmods.net/download/mod".to_string(),
            ),
            SourceKind::MorpheusMart => (
                String::new(),
                "https://www.morpheus-mart.com/mod".to_string(),
            ),
            SourceKind::DirectLink => (String::new(), "https://example.test/file.zip".to_string()),
        };
        SourceForm {
            tp2: "mod".to_string(),
            card_key: "mod".to_string(),
            name: "Mod".to_string(),
            source_id: "primary".to_string(),
            label: "GitHub".to_string(),
            identity: SourceFormIdentity::default(),
            kind,
            repo,
            link,
            follow: Follow::Release,
            commit: String::new(),
            tag: String::new(),
            branch: String::new(),
            allow_pre: false,
            release: String::new(),
            asset: AssetPick::SourceZip,
            pkg_windows: String::new(),
            pkg_linux: String::new(),
            pkg_macos: String::new(),
            aliases_text: "alt1, alt2".to_string(),
            subdir_require: "core".to_string(),
            config_files: Vec::new(),
            config_files_focus_pending: None,
            save_to: ModSourceEditDestination::ThisModlist,
            advanced_open: false,
            release_query: String::new(),
            notice: None,
            error: None,
            note: String::new(),
            note_seed: String::new(),
            note_who: String::new(),
            kept: KeptFields {
                source_default: true,
                ..KeptFields::default()
            },
        }
    }

    fn round_trip(form: &SourceForm) -> SourceForm {
        let source = to_source(form);
        from_source(&source, form.identity, form.save_to, &form.card_key)
    }

    #[test]
    fn form_round_trips_every_follow_kind() {
        let commit_form = base_form_with(|f| {
            f.follow = Follow::Commit;
            f.commit = "abcdef1234".to_string();
        });
        assert_eq!(round_trip(&commit_form), commit_form);

        let tag_form = base_form_with(|f| {
            f.follow = Follow::Tag;
            f.tag = "v1.0".to_string();
        });
        assert_eq!(round_trip(&tag_form), tag_form);

        let branch_form = base_form_with(|f| {
            f.follow = Follow::Branch;
            f.branch = "main".to_string();
        });
        assert_eq!(round_trip(&branch_form), branch_form);

        let default_branch_form = base_form_with(|f| f.follow = Follow::Branch);
        assert_eq!(round_trip(&default_branch_form), default_branch_form);

        let newest_off = base_form_with(|f| {
            f.follow = Follow::Release;
            f.pkg_windows = "win.zip".to_string();
            f.kept.pkg_windows = Some("win.zip".to_string());
        });
        assert_eq!(round_trip(&newest_off), newest_off);

        let newest_on = base_form_with(|f| {
            f.follow = Follow::Release;
            f.allow_pre = true;
        });
        assert_eq!(round_trip(&newest_on), newest_on);

        let best_for_os = base_form_with(|f| {
            f.follow = Follow::Release;
            f.release = "v2.0".to_string();
            f.asset = AssetPick::BestForOs;
            f.pkg_windows = "win.zip".to_string();
            f.kept.pkg_windows = Some("win.zip".to_string());
        });
        assert_eq!(round_trip(&best_for_os), best_for_os);

        let named_asset = base_form_with(|f| {
            f.follow = Follow::Release;
            f.release = "v2.0".to_string();
            f.asset = AssetPick::Named("linux.tar.gz".to_string());
        });
        assert_eq!(round_trip(&named_asset), named_asset);

        let weasel = base_form(SourceKind::WeaselMods);
        assert_eq!(round_trip(&weasel), weasel);

        let morpheus = base_form(SourceKind::MorpheusMart);
        assert_eq!(round_trip(&morpheus), morpheus);

        let direct_archive = base_form(SourceKind::DirectLink);
        assert_eq!(round_trip(&direct_archive), direct_archive);

        let mut direct_page = base_form(SourceKind::DirectLink);
        direct_page.link = "https://example.test/page".to_string();
        assert_eq!(round_trip(&direct_page), direct_page);
    }

    fn base_form_with(edit: impl FnOnce(&mut SourceForm)) -> SourceForm {
        let mut form = base_form(SourceKind::GitHub);
        edit(&mut form);
        form
    }

    #[test]
    fn newest_with_prereleases_writes_pre_release_channel() {
        let form = base_form_with(|f| {
            f.follow = Follow::Release;
            f.allow_pre = true;
        });
        let source = to_source(&form);
        assert_eq!(source.channel.as_deref(), Some("pre-release"));
    }

    #[test]
    fn chosen_release_with_source_zip_writes_tag() {
        let form = base_form_with(|f| {
            f.follow = Follow::Release;
            f.release = "v35.17".to_string();
            f.asset = AssetPick::SourceZip;
        });
        let source = to_source(&form);
        assert_eq!(source.tag.as_deref(), Some("v35.17"));
        assert!(source.release.is_none());
    }

    #[test]
    fn chosen_release_best_for_os_keeps_packages() {
        let form = base_form_with(|f| {
            f.follow = Follow::Release;
            f.release = "v35.17".to_string();
            f.asset = AssetPick::BestForOs;
            f.kept.pkg_windows = Some("win.zip".to_string());
            f.kept.pkg_linux = Some("linux.tar.gz".to_string());
        });
        let source = to_source(&form);
        assert_eq!(source.release.as_deref(), Some("v35.17"));
        assert!(source.asset.is_none());
        assert_eq!(source.pkg_windows.as_deref(), Some("win.zip"));
        assert_eq!(source.pkg_linux.as_deref(), Some("linux.tar.gz"));
    }

    #[test]
    fn chosen_release_with_file_writes_release_and_asset() {
        let form = base_form_with(|f| {
            f.follow = Follow::Release;
            f.release = "v35.17".to_string();
            f.asset = AssetPick::Named("win-stratagems.exe".to_string());
        });
        let source = to_source(&form);
        assert_eq!(source.release.as_deref(), Some("v35.17"));
        assert_eq!(source.asset.as_deref(), Some("win-stratagems.exe"));
    }

    #[test]
    fn latest_code_block_opens_as_branch_blank() {
        let source = ModDownloadSource {
            github: Some("owner/repo".to_string()),
            channel: Some("master".to_string()),
            ..ModDownloadSource::default()
        };
        let form = from_source(
            &source,
            SourceFormIdentity::default(),
            ModSourceEditDestination::ThisModlist,
            "repo",
        );
        assert_eq!(form.follow, Follow::Branch);
        assert_eq!(form.branch.len(), 0);
        assert!(form.notice.is_none());
    }

    #[test]
    fn blank_branch_saves_as_master_channel() {
        let form = base_form_with(|f| {
            f.follow = Follow::Branch;
            f.branch = "   ".to_string();
        });
        let source = to_source(&form);
        assert_eq!(source.channel.as_deref(), Some("master"));
        assert!(source.branch.is_none());
    }

    #[test]
    fn named_branch_saves_as_branch() {
        let form = base_form_with(|f| {
            f.follow = Follow::Branch;
            f.branch = " develop ".to_string();
        });
        let source = to_source(&form);
        assert_eq!(source.branch.as_deref(), Some("develop"));
        assert!(source.channel.is_none());
    }

    #[test]
    fn preonly_source_reads_as_newest_with_notice() {
        let source = ModDownloadSource {
            github: Some("owner/repo".to_string()),
            channel: Some("preonly".to_string()),
            ..ModDownloadSource::default()
        };
        let form = from_source(
            &source,
            SourceFormIdentity::default(),
            ModSourceEditDestination::ThisModlist,
            "repo",
        );
        assert_eq!(form.follow, Follow::Release);
        assert!(form.allow_pre);
        assert_eq!(form.notice.as_deref(), Some(PREONLY_NOTICE));
    }

    #[test]
    fn ifeellucky_source_reads_with_notice() {
        let source = ModDownloadSource {
            github: Some("owner/repo".to_string()),
            channel: Some("ifeellucky".to_string()),
            ..ModDownloadSource::default()
        };
        let form = from_source(
            &source,
            SourceFormIdentity::default(),
            ModSourceEditDestination::ThisModlist,
            "repo",
        );
        assert_eq!(form.follow, Follow::Release);
        assert_eq!(
            form.notice.as_deref(),
            Some(
                "This source uses a setting the form cannot show (ifeellucky); saving replaces it."
            )
        );
    }

    #[test]
    fn will_fetch_sentences_match_the_table() {
        assert_eq!(
            will_fetch(&base_form_with(|f| {
                f.follow = Follow::Commit;
                f.commit = "abcdef1234".to_string();
            })),
            "source zip of commit abcdef1 from owner/repo"
        );
        assert_eq!(
            will_fetch(&base_form_with(|f| {
                f.follow = Follow::Tag;
                f.tag = "v1.0".to_string();
            })),
            "source zip of tag v1.0 from owner/repo"
        );
        assert_eq!(
            will_fetch(&base_form_with(|f| {
                f.follow = Follow::Branch;
                f.branch = "main".to_string();
            })),
            "source zip of branch main from owner/repo"
        );
        assert_eq!(
            will_fetch(&base_form_with(|f| f.follow = Follow::Branch)),
            "source zip of the default branch of owner/repo"
        );
        assert_eq!(
            will_fetch(&base_form_with(|f| f.follow = Follow::Release)),
            "newest release of owner/repo"
        );
        assert_eq!(
            will_fetch(&base_form_with(|f| {
                f.follow = Follow::Release;
                f.allow_pre = true;
            })),
            "newest release or pre-release of owner/repo"
        );
        assert_eq!(
            will_fetch(&base_form_with(|f| {
                f.follow = Follow::Release;
                f.release = "v35.17".to_string();
                f.asset = AssetPick::SourceZip;
            })),
            "source zip of release v35.17"
        );
        assert_eq!(
            will_fetch(&base_form_with(|f| {
                f.follow = Follow::Release;
                f.release = "v35.17".to_string();
                f.asset = AssetPick::BestForOs;
            })),
            "best file for this OS from release v35.17"
        );
        assert_eq!(
            will_fetch(&base_form_with(|f| {
                f.follow = Follow::Release;
                f.release = "v35.17".to_string();
                f.asset = AssetPick::Named("win-stratagems.exe".to_string());
            })),
            "win-stratagems.exe from release v35.17"
        );
        assert_eq!(
            will_fetch(&base_form(SourceKind::WeaselMods)),
            "newest file on this Weasel Mods page"
        );
        assert_eq!(
            will_fetch(&base_form(SourceKind::MorpheusMart)),
            "newest file on this Morpheus Mart page"
        );
        assert_eq!(
            will_fetch(&base_form(SourceKind::DirectLink)),
            "file.zip from example.test"
        );
        assert_eq!(
            will_fetch(&{
                let mut form = base_form(SourceKind::DirectLink);
                form.link = "https://example.test/page".to_string();
                form
            }),
            "nothing automatically \u{b7} manual download from this page"
        );
        assert_eq!(
            will_fetch(&base_form_with(|f| f.repo.clear())),
            "add a repository"
        );
        assert_eq!(
            will_fetch(&{
                let mut form = base_form(SourceKind::DirectLink);
                form.link.clear();
                form
            }),
            "paste a link"
        );
    }

    #[test]
    fn new_mod_save_text_has_a_mod_header() {
        let form = base_form_with(|f| f.identity.is_new_mod = true);
        let source = to_source(&form);
        let text = source_form_save_text(&form);
        assert!(text.starts_with("[[mods]]"));
        assert!(text.contains("[[mods.sources]]"));
        assert!(source.source_default);
    }

    #[test]
    fn newest_with_named_asset_round_trips_and_keeps_the_file() {
        let source = ModDownloadSource {
            github: Some("owner/repo".to_string()),
            asset: Some("win-stratagems.exe".to_string()),
            ..ModDownloadSource::default()
        };
        let form = from_source(
            &source,
            SourceFormIdentity::default(),
            ModSourceEditDestination::ThisModlist,
            "repo",
        );
        assert_eq!(form.follow, Follow::Release);
        assert_eq!(form.release.len(), 0);
        assert_eq!(form.kept.asset.as_deref(), Some("win-stratagems.exe"));
        assert_eq!(
            form.notice.as_deref(),
            Some(
                "This source picks the file win-stratagems.exe from the newest release; the form keeps it."
            )
        );
        assert_eq!(
            will_fetch(&form),
            "win-stratagems.exe from the newest release of owner/repo"
        );
        let round = to_source(&form);
        assert_eq!(round.asset.as_deref(), Some("win-stratagems.exe"));
        assert!(round.release.is_none());
    }

    #[test]
    fn legacy_releases_word_round_trips_verbatim() {
        let source = ModDownloadSource {
            github: Some("owner/repo".to_string()),
            channel: Some("RELEASES".to_string()),
            ..ModDownloadSource::default()
        };
        let form = from_source(
            &source,
            SourceFormIdentity::default(),
            ModSourceEditDestination::ThisModlist,
            "repo",
        );
        assert_eq!(form.follow, Follow::Release);
        assert!(!form.allow_pre);
        assert!(form.notice.is_none());
        assert_eq!(form.kept.channel_word.as_deref(), Some("RELEASES"));
        let round = to_source(&form);
        assert_eq!(round.channel.as_deref(), Some("RELEASES"));
    }

    #[test]
    fn channel_words_match_case_insensitively() {
        let preonly = ModDownloadSource {
            github: Some("owner/repo".to_string()),
            channel: Some("PreOnly".to_string()),
            ..ModDownloadSource::default()
        };
        let form = from_source(
            &preonly,
            SourceFormIdentity::default(),
            ModSourceEditDestination::ThisModlist,
            "repo",
        );
        assert_eq!(form.follow, Follow::Release);
        assert!(form.allow_pre);
        assert_eq!(form.notice.as_deref(), Some(PREONLY_NOTICE));

        let master = ModDownloadSource {
            github: Some("owner/repo".to_string()),
            channel: Some("MASTER".to_string()),
            ..ModDownloadSource::default()
        };
        let form2 = from_source(
            &master,
            SourceFormIdentity::default(),
            ModSourceEditDestination::ThisModlist,
            "repo",
        );
        assert_eq!(form2.follow, Follow::Branch);
        assert_eq!(form2.branch.len(), 0);
    }

    #[test]
    fn release_source_zip_reads_back_as_tag() {
        let form = base_form_with(|f| {
            f.follow = Follow::Release;
            f.release = "v1.0".to_string();
            f.asset = AssetPick::SourceZip;
        });
        let source = to_source(&form);
        assert_eq!(source.tag.as_deref(), Some("v1.0"));
        let read_back = from_source(&source, form.identity, form.save_to, &form.card_key);
        assert_eq!(read_back.follow, Follow::Tag);
        assert_eq!(read_back.tag, "v1.0");
    }

    #[test]
    fn kept_fields_survive_a_save() {
        let form = base_form_with(|f| {
            f.kept.exact_github = vec!["owner/other".to_string()];
            f.kept.tp2_rename = Some(ModDownloadTp2Rename {
                from: "old.tp2".to_string(),
                to: "new.tp2".to_string(),
            });
        });
        let source = to_source(&form);
        assert_eq!(source.exact_github, vec!["owner/other".to_string()]);
        assert_eq!(
            source.tp2_rename,
            Some(ModDownloadTp2Rename {
                from: "old.tp2".to_string(),
                to: "new.tp2".to_string(),
            })
        );
    }

    #[test]
    fn config_files_round_trip_through_the_rows() {
        let seed = vec!["a.ini".to_string(), "sub/b.ini".to_string()];
        let source = ModDownloadSource {
            github: Some("owner/repo".to_string()),
            config_files: seed.clone(),
            ..ModDownloadSource::default()
        };
        let form = from_source(
            &source,
            SourceFormIdentity::default(),
            ModSourceEditDestination::ThisModlist,
            "repo",
        );
        assert_eq!(form.config_files, seed);
        assert_eq!(to_source(&form).config_files, seed);
    }

    fn rows(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_string()).collect()
    }

    #[test]
    fn config_files_rows_are_trimmed_deduped_and_slash_normalised() {
        let form = base_form_with(|f| {
            f.config_files = rows(&[" a.ini ", "sub\\b.ini", "", "a.ini"]);
        });
        assert_eq!(
            to_source(&form).config_files,
            vec!["a.ini".to_string(), "sub/b.ini".to_string()]
        );
    }

    #[test]
    fn config_files_error_names_the_bad_entry() {
        let bad = base_form_with(|f| f.config_files = rows(&["a.ini", "../x.ini"]));
        let error = config_files_error(&bad).expect("a bad entry is refused");
        assert!(error.starts_with("Config files: "), "got: {error}");

        let dot_only = base_form_with(|f| f.config_files = rows(&["a.ini", ".../x.ini"]));
        let error = config_files_error(&dot_only).expect("a dot-only segment is refused");
        assert!(error.starts_with("Config files: "), "got: {error}");

        let good = base_form_with(|f| f.config_files = rows(&["a.ini"]));
        assert_eq!(config_files_error(&good), None);

        let empty = base_form_with(|f| f.config_files.clear());
        assert_eq!(config_files_error(&empty), None);

        let blank = base_form_with(|f| f.config_files = vec![String::new()]);
        assert_eq!(config_files_error(&blank), None);
    }
}
