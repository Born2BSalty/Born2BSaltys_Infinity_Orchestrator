// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Component, Path, PathBuf};

use super::map_to_bio::{build_prompt_summary_index, dedupe_and_join};
use super::model::ParserOutput;
use super::tra_lookup::{ModLayout, decode_text, tra_map_for_tp2};
use crate::app::compat_tp2_blocks::{collect_tp2_component_blocks, tp2_preamble_code};
use crate::app::tp2_component_begin::code_part;
use crate::parser::{PromptSummaryEvent, PromptSummaryIndex};

const COMPONENT_LINE_LIMIT: usize = 4;
const MOD_LINE_LIMIT: usize = 6;
const COMPONENT_NUMBER_NAME: &str = "COMPONENT_NUMBER";
const COMPONENT_NUMBER_TOKEN: &str = "%COMPONENT_NUMBER%";
const READLN: &[u8] = b"READLN";
const INCLUDE_KEYWORDS: [&str; 6] = [
    "INCLUDE",
    "ACTION_INCLUDE",
    "PATCH_INCLUDE",
    "REINCLUDE",
    "ACTION_REINCLUDE",
    "PATCH_REINCLUDE",
];
const SPRINT_KEYWORDS: [&str; 2] = ["OUTER_SPRINT", "OUTER_TEXT_SPRINT"];

pub(super) fn merge_hidden_prompts(
    index: &mut PromptSummaryIndex,
    tp2_path: &Path,
    preferred_lang: Option<&str>,
    preferred_game: Option<&str>,
) {
    let Ok(bytes) = fs::read(tp2_path) else {
        return;
    };
    let tp2_text = decode_text(bytes);
    let preamble = tp2_preamble_code(&tp2_text);
    let blocks = collect_tp2_component_blocks(&tp2_text);
    let variables = text_variables(
        preamble
            .iter()
            .chain(blocks.iter().flat_map(|(_, lines)| lines))
            .copied(),
    );
    let keys = component_keys(&blocks);
    let layout = ModLayout::for_tp2(tp2_path);
    let mut search = TargetedSearch::new(&layout, &variables, &keys);
    search.search_tp2_lines(&preamble, None);
    for (key, lines) in &blocks {
        search.search_tp2_lines(lines, Some(key));
    }
    let targets = search.targets;
    if targets.is_empty() {
        return;
    }
    let mut parser = HiddenFileParser {
        tp2_path,
        tp2_text: &tp2_text,
        preferred_lang,
        preferred_game,
        tra: None,
        parsed: HashMap::new(),
    };
    for target in targets {
        let events = parser.events(&target.key, &target.path);
        merge_events(index, target.owner.as_deref(), events);
    }
}

struct Target {
    key: String,
    path: PathBuf,
    owner: Option<String>,
}

type OwnedPath = (String, Option<String>);

struct TargetedSearch<'a> {
    layout: &'a ModLayout,
    variables: &'a HashMap<String, String>,
    keys: &'a [String],
    computed_in_direct: HashMap<String, Vec<String>>,
    direct_seen: HashSet<OwnedPath>,
    asks: HashMap<String, bool>,
    target_seen: HashSet<OwnedPath>,
    targets: Vec<Target>,
}

impl<'a> TargetedSearch<'a> {
    fn new(
        layout: &'a ModLayout,
        variables: &'a HashMap<String, String>,
        keys: &'a [String],
    ) -> Self {
        Self {
            layout,
            variables,
            keys,
            computed_in_direct: HashMap::new(),
            direct_seen: HashSet::new(),
            asks: HashMap::new(),
            target_seen: HashSet::new(),
            targets: Vec::new(),
        }
    }

    fn search_tp2_lines(&mut self, tp2_lines: &[&str], owner: Option<&str>) {
        let layout = self.layout;
        for literal in include_literals(tp2_lines) {
            let computed = is_computed(layout, &literal);
            for (relative, file_owner) in self.expand(&literal, owner) {
                let Some(path) = layout.resolve(&relative, layout.mod_dir()) else {
                    continue;
                };
                if computed {
                    self.keep_if_it_asks(&path, file_owner.clone());
                }
                self.search_direct_include(&path, file_owner.as_deref());
            }
        }
    }

    fn search_direct_include(&mut self, path: &Path, owner: Option<&str>) {
        let key = path_key(path);
        if !self
            .direct_seen
            .insert((key.clone(), owner.map(str::to_string)))
        {
            return;
        }
        let layout = self.layout;
        let literals = self
            .computed_in_direct
            .entry(key)
            .or_insert_with(|| computed_literals_in(layout, path))
            .clone();
        let includer_dir = path.parent().unwrap_or_else(|| Path::new(""));
        for literal in &literals {
            for (relative, file_owner) in self.expand(literal, owner) {
                if let Some(found) = layout.resolve(&relative, includer_dir) {
                    self.keep_if_it_asks(&found, file_owner);
                }
            }
        }
    }

    fn keep_if_it_asks(&mut self, path: &Path, owner: Option<String>) {
        let key = path_key(path);
        let asks = *self
            .asks
            .entry(key.clone())
            .or_insert_with(|| fs::read(path).is_ok_and(|bytes| contains_readln(&bytes)));
        if asks && self.target_seen.insert((key.clone(), owner.clone())) {
            self.targets.push(Target {
                key,
                path: path.to_path_buf(),
                owner,
            });
        }
    }

    fn expand(&self, literal: &str, owner: Option<&str>) -> Vec<OwnedPath> {
        let Some(path) =
            substitute_variables(&self.layout.with_mod_folder(literal), self.variables)
        else {
            return Vec::new();
        };
        let path = path.replace('\\', "/");
        if !path.contains(COMPONENT_NUMBER_TOKEN) {
            return vec![(path, owner.map(str::to_string))];
        }
        if let Some(number) = owner {
            return vec![(
                path.replace(COMPONENT_NUMBER_TOKEN, number),
                Some(number.to_string()),
            )];
        }
        self.keys
            .iter()
            .map(|key| (path.replace(COMPONENT_NUMBER_TOKEN, key), Some(key.clone())))
            .collect()
    }
}

fn is_computed(layout: &ModLayout, literal: &str) -> bool {
    layout.with_mod_folder(literal).contains('%')
}

fn computed_literals_in(layout: &ModLayout, path: &Path) -> Vec<String> {
    let Ok(bytes) = fs::read(path) else {
        return Vec::new();
    };
    include_literals(&all_code_lines(&decode_text(bytes)))
        .into_iter()
        .filter(|literal| is_computed(layout, literal))
        .collect()
}

fn contains_readln(bytes: &[u8]) -> bool {
    bytes
        .windows(READLN.len())
        .any(|window| window.eq_ignore_ascii_case(READLN))
}

struct HiddenFileParser<'a> {
    tp2_path: &'a Path,
    tp2_text: &'a str,
    preferred_lang: Option<&'a str>,
    preferred_game: Option<&'a str>,
    tra: Option<HashMap<String, String>>,
    parsed: HashMap<String, Vec<PromptSummaryEvent>>,
}

impl HiddenFileParser<'_> {
    fn events(&mut self, key: &str, path: &Path) -> Vec<PromptSummaryEvent> {
        if let Some(events) = self.parsed.get(key) {
            return events.clone();
        }
        let events = self.parse(path);
        self.parsed.insert(key.to_string(), events.clone());
        events
    }

    fn parse(&mut self, path: &Path) -> Vec<PromptSummaryEvent> {
        let Ok(json) = lapdu_parser_rust::parse_path_to_json(path, self.preferred_lang) else {
            return Vec::new();
        };
        let Ok(mut output) = serde_json::from_str::<ParserOutput>(&json) else {
            return Vec::new();
        };
        let tra = self.tra.get_or_insert_with(|| {
            tra_map_for_tp2(self.tp2_path, self.tp2_text, self.preferred_lang)
        });
        translate_output(&mut output, tra);
        let built = build_prompt_summary_index(&output, self.preferred_game);
        let mut bound: Vec<(String, Vec<PromptSummaryEvent>)> =
            built.by_component_id_events.into_iter().collect();
        bound.sort_by(|left, right| left.0.cmp(&right.0));
        let mut events = built.mod_events;
        events.extend(bound.into_iter().flat_map(|(_, list)| list));
        events
    }
}

fn translate_output(output: &mut ParserOutput, tra: &HashMap<String, String>) {
    for event in &mut output.events {
        translate(&mut event.text, tra);
        for option in &mut event.options {
            translate(&mut option.label, tra);
        }
    }
}

fn translate(value: &mut String, tra: &HashMap<String, String>) {
    let trimmed = value.trim();
    let is_reference = trimmed
        .strip_prefix('@')
        .is_some_and(|digits| !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()));
    if !is_reference {
        return;
    }
    if let Some(text) = tra.get(trimmed) {
        value.clone_from(text);
    }
}

fn merge_events(
    index: &mut PromptSummaryIndex,
    owner: Option<&str>,
    events: Vec<PromptSummaryEvent>,
) {
    if events.is_empty() {
        return;
    }
    if let Some(id) = owner {
        let list = index
            .by_component_id_events
            .entry(id.to_string())
            .or_default();
        if !append_unique(list, events) {
            return;
        }
        let summary = dedupe_and_join(summary_lines(list), COMPONENT_LINE_LIMIT);
        index.by_component_id.insert(id.to_string(), summary);
    } else {
        if !append_unique(&mut index.mod_events, events) {
            return;
        }
        let summary = dedupe_and_join(summary_lines(&index.mod_events), MOD_LINE_LIMIT);
        if !summary.is_empty() {
            index.mod_summary = Some(summary);
        }
    }
}

fn append_unique(list: &mut Vec<PromptSummaryEvent>, events: Vec<PromptSummaryEvent>) -> bool {
    let mut seen: HashSet<EventIdentity> = list.iter().map(event_identity).collect();
    let before = list.len();
    list.extend(
        events
            .into_iter()
            .filter(|event| seen.insert(event_identity(event))),
    );
    list.len() > before
}

#[derive(PartialEq, Eq, Hash)]
enum EventIdentity {
    Line(String, u32),
    Text(String, String),
}

fn event_identity(event: &PromptSummaryEvent) -> EventIdentity {
    let source = source_key(&event.source_file);
    match event.line {
        Some(line) => EventIdentity::Line(source, line),
        None => EventIdentity::Text(source, event.text.clone()),
    }
}

fn summary_lines(events: &[PromptSummaryEvent]) -> Vec<String> {
    events
        .iter()
        .map(|event| event.summary_line.clone())
        .collect()
}

fn component_keys(blocks: &[(String, Vec<&str>)]) -> Vec<String> {
    let mut seen = HashSet::<&str>::new();
    blocks
        .iter()
        .filter(|(key, _)| seen.insert(key.as_str()))
        .map(|(key, _)| key.clone())
        .collect()
}

fn all_code_lines(text: &str) -> Vec<&str> {
    let mut lines = tp2_preamble_code(text);
    lines.extend(
        collect_tp2_component_blocks(text)
            .into_iter()
            .flat_map(|(_, block)| block),
    );
    lines
}

fn statement(line: &str) -> Option<(&str, &str)> {
    let code = code_part(line).trim_start();
    let end = code
        .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
        .unwrap_or(code.len());
    (end > 0).then(|| code.split_at(end))
}

fn include_literals(lines: &[&str]) -> Vec<String> {
    lines
        .iter()
        .filter_map(|line| statement(line))
        .filter(|(keyword, _)| {
            INCLUDE_KEYWORDS
                .iter()
                .any(|known| known.eq_ignore_ascii_case(keyword))
        })
        .flat_map(|(_, rest)| delimited_literals(rest))
        .map(str::trim)
        .filter(|literal| !literal.is_empty())
        .map(str::to_string)
        .collect()
}

fn delimited_literals(text: &str) -> Vec<&str> {
    let mut literals = Vec::new();
    let mut rest = text;
    while let Some(start) = rest.find(['~', '"']) {
        let delimiter = &rest[start..=start];
        let body = &rest[start + 1..];
        let Some(end) = body.find(delimiter) else {
            break;
        };
        literals.push(&body[..end]);
        rest = &body[end + 1..];
    }
    literals
}

fn text_variables<'t>(lines: impl Iterator<Item = &'t str>) -> HashMap<String, String> {
    let mut assigned = HashMap::<String, Option<String>>::new();
    for line in lines {
        let Some((keyword, rest)) = statement(line) else {
            continue;
        };
        if !SPRINT_KEYWORDS
            .iter()
            .any(|known| known.eq_ignore_ascii_case(keyword))
        {
            continue;
        }
        let Some((name, value)) = literal_assignment(rest) else {
            continue;
        };
        assigned
            .entry(name.to_string())
            .and_modify(|current| {
                if current.as_deref() != Some(value) {
                    *current = None;
                }
            })
            .or_insert_with(|| Some(value.to_string()));
    }
    assigned
        .into_iter()
        .filter_map(|(name, value)| value.map(|value| (name, value)))
        .collect()
}

fn literal_assignment(rest: &str) -> Option<(&str, &str)> {
    let rest = rest.trim_start();
    let name_end = rest.find(char::is_whitespace)?;
    let (name, after) = rest.split_at(name_end);
    if name.contains(['~', '"', '%']) {
        return None;
    }
    let after = after.trim_start();
    let delimiter = after.chars().next().filter(|c| matches!(c, '~' | '"'))?;
    let body = &after[1..];
    let value = &body[..body.find(delimiter)?];
    (!value.contains('%')).then_some((name, value))
}

fn substitute_variables(text: &str, variables: &HashMap<String, String>) -> Option<String> {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(open) = rest.find('%') {
        out.push_str(&rest[..open]);
        let after = &rest[open + 1..];
        let close = after.find('%')?;
        let name = &after[..close];
        if name == COMPONENT_NUMBER_NAME {
            out.push_str(COMPONENT_NUMBER_TOKEN);
        } else {
            out.push_str(variables.get(name)?);
        }
        rest = &after[close + 1..];
    }
    out.push_str(rest);
    Some(out)
}

fn path_key(path: &Path) -> String {
    let mut parts = Vec::<String>::new();
    for component in path.components() {
        match component {
            Component::CurDir | Component::RootDir => {}
            Component::ParentDir => {
                parts.pop();
            }
            other => parts.push(other.as_os_str().to_string_lossy().to_ascii_lowercase()),
        }
    }
    parts.join("/")
}

fn source_key(source_file: &str) -> String {
    path_key(Path::new(&source_file.replace('\\', "/")))
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::{ModLayout, TargetedSearch, merge_events};
    use crate::parser::{PromptSummaryEvent, PromptSummaryIndex, collect_prompt_summary_index};

    static ROOT_SEQ: AtomicUsize = AtomicUsize::new(0);

    const HEADER: &str = "BACKUP ~mymod/backup~\nAUTHOR ~me~\nVERSION ~1~\n\nLANGUAGE ~English~ ~english~ ~mymod/tra/english/setup.tra~\n";
    const TEXT_FOLDER_HEADER: &str = "BACKUP ~mymod/backup~\nAUTHOR ~me~\nVERSION ~1~\n\nLANGUAGE ~English~ ~english~ ~mymod/text/setup.tra~\n";
    const QUESTION_FILE: &str = "PRINT @10\nACTION_READLN answer\n";
    const TRA_TEXT: &str = "@1 = ~Component~\n@10 = ~Pick one (1 or 2)~\n";

    struct TempModsRoot {
        root: PathBuf,
    }

    impl TempModsRoot {
        fn new() -> Self {
            let root = std::env::temp_dir().join(format!(
                "bio_hidden_prompts_test_{}_{}",
                std::process::id(),
                ROOT_SEQ.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir_all(root.join("mods")).expect("create temp mods root");
            let created = Self { root };
            created.write("mymod/tra/english/setup.tra", TRA_TEXT);
            created
        }

        fn write(&self, relative: &str, text: &str) -> PathBuf {
            let path = self.root.join("mods").join(relative);
            fs::create_dir_all(path.parent().expect("file has a parent")).expect("create dir");
            fs::write(&path, text).expect("write file");
            path
        }

        fn scan(&self, body: &str) -> PromptSummaryIndex {
            self.scan_with(HEADER, body)
        }

        fn scan_with_tra_unseen_by_vendored(
            &self,
            tra_text: &str,
            body: &str,
        ) -> PromptSummaryIndex {
            self.write("mymod/tra/english/setup.tra", "");
            self.write("mymod/text/setup.tra", tra_text);
            self.scan_with(TEXT_FOLDER_HEADER, body)
        }

        fn scan_with(&self, header: &str, body: &str) -> PromptSummaryIndex {
            let tp2 = self.write("mymod/setup-mymod.tp2", &format!("{header}{body}"));
            collect_prompt_summary_index(&tp2, &self.root.join("mods"), Some("en_US"), None)
        }
    }

    impl Drop for TempModsRoot {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    fn on_big_stack(body: impl FnOnce() + Send + 'static) {
        let handle = std::thread::Builder::new()
            .stack_size(32 * 1024 * 1024)
            .spawn(body)
            .expect("spawn test thread");
        if let Err(panic) = handle.join() {
            std::panic::resume_unwind(panic);
        }
    }

    fn summary_of<'i>(index: &'i PromptSummaryIndex, id: &str) -> Option<&'i str> {
        index.by_component_id.get(id).map(String::as_str)
    }

    #[test]
    fn a_component_number_include_flags_only_components_with_a_file() {
        on_big_stack(|| {
            let root = TempModsRoot::new();
            root.write(
                "mymod/lib/always.tpa",
                "DEFINE_ACTION_MACRO m BEGIN\n  INCLUDE ~mymod/lib/comp_%COMPONENT_NUMBER%_prompts.tpa~\nEND\n",
            );
            root.write("mymod/lib/comp_5_prompts.tpa", QUESTION_FILE);
            let index = root.scan(
                "ALWAYS\n  INCLUDE ~mymod/lib/always.tpa~\nEND\n\nBEGIN ~Five~ DESIGNATED 5\nLAM m\n\nBEGIN ~Six~ DESIGNATED 6\nLAM m\n",
            );
            assert!(summary_of(&index, "5").is_some_and(|text| text.contains("Pick one")));
            assert!(!index.by_component_id.contains_key("6"));
            assert!(!index.by_component_id_events.contains_key("6"));
        });
    }

    #[test]
    fn a_variable_set_in_the_tp2_resolves_an_include_inside_a_component() {
        on_big_stack(|| {
            let root = TempModsRoot::new();
            root.write("mymod/include/ask.tpa", QUESTION_FILE);
            let index = root.scan(
                "ALWAYS\n  OUTER_SPRINT PKGNAME ~mymod~\nEND\n\nBEGIN @1 DESIGNATED 7\nINCLUDE ~%PKGNAME%/include/ask.tpa~\n",
            );
            assert!(summary_of(&index, "7").is_some_and(|text| text.contains("Pick one")));
        });
    }

    #[test]
    fn a_variable_with_two_values_is_not_used() {
        on_big_stack(|| {
            let root = TempModsRoot::new();
            root.write("mymod/include/ask.tpa", QUESTION_FILE);
            root.write("othermod/include/ask.tpa", QUESTION_FILE);
            let index = root.scan(
                "ALWAYS\n  OUTER_SPRINT PKGNAME ~mymod~\n  OUTER_SPRINT PKGNAME ~othermod~\nEND\n\nBEGIN @1 DESIGNATED 7\nINCLUDE ~%PKGNAME%/include/ask.tpa~\n",
            );
            assert!(!index.by_component_id.contains_key("7"));
            assert!(!index.by_component_id_events.contains_key("7"));
        });
    }

    #[test]
    fn an_unknown_variable_include_adds_nothing() {
        on_big_stack(|| {
            let root = TempModsRoot::new();
            root.write("mymod/include/ask.tpa", QUESTION_FILE);
            let index = root.scan("BEGIN @1 DESIGNATED 7\nINCLUDE ~%NOPE%/include/ask.tpa~\n");
            assert!(!index.by_component_id.contains_key("7"));
            assert!(!index.by_component_id_events.contains_key("7"));
            assert_eq!(index.mod_events.len(), 0);
            assert!(index.mod_summary.is_none());
        });
    }

    #[test]
    fn a_literally_included_question_is_not_counted_twice() {
        on_big_stack(|| {
            let root = TempModsRoot::new();
            root.write("mymod/lib/lit.tpa", QUESTION_FILE);
            let start =
                "ALWAYS\n  OUTER_SPRINT PKGNAME ~mymod~\nEND\n\nBEGIN ~Nine~ DESIGNATED 9\n";
            let vendored_only = root.scan(&format!("{start}INCLUDE ~mymod/lib/lit.tpa~\n"));
            let both = root.scan(&format!(
                "{start}INCLUDE ~%PKGNAME%/lib/lit.tpa~\nINCLUDE ~mymod/lib/lit.tpa~\n"
            ));
            assert!(
                !vendored_only.by_component_id_events.is_empty()
                    || !vendored_only.mod_events.is_empty()
            );
            assert_eq!(
                both.by_component_id_events.get("9"),
                vendored_only.by_component_id_events.get("9")
            );
            assert_eq!(
                both.by_component_id.get("9"),
                vendored_only.by_component_id.get("9")
            );
            assert_eq!(both.mod_events, vendored_only.mod_events);
        });
    }

    #[test]
    fn a_component_number_inside_a_component_means_that_component() {
        on_big_stack(|| {
            let root = TempModsRoot::new();
            root.write("mymod/lib/comp_5_ask.tpa", QUESTION_FILE);
            root.write("mymod/lib/comp_6_ask.tpa", QUESTION_FILE);
            let index = root.scan(
                "BEGIN ~Five~ DESIGNATED 5\nINCLUDE ~mymod/lib/comp_%COMPONENT_NUMBER%_ask.tpa~\n\nBEGIN ~Six~ DESIGNATED 6\nPRINT ~six~\n",
            );
            assert!(summary_of(&index, "5").is_some_and(|text| text.contains("Pick one")));
            assert!(!index.by_component_id.contains_key("6"));
            assert!(!index.by_component_id_events.contains_key("6"));
        });
    }

    #[test]
    fn a_question_reached_twice_is_listed_once() {
        on_big_stack(|| {
            let root = TempModsRoot::new();
            root.write(
                "mymod/tra/english/setup.tra",
                &format!("{TRA_TEXT}@11 = ~Second question (1 or 2)~\n"),
            );
            root.write(
                "mymod/lib/hidden.tpa",
                &format!("INCLUDE ~sub.tpa~\n{QUESTION_FILE}"),
            );
            root.write("mymod/lib/sub.tpa", "PRINT @11\nACTION_READLN other\n");
            let index = root.scan(
                "ALWAYS\n  OUTER_SPRINT PKGNAME ~mymod~\nEND\n\nBEGIN @1 DESIGNATED 7\nINCLUDE ~%PKGNAME%/lib/hidden.tpa~\nINCLUDE ~%PKGNAME%/lib/sub.tpa~\n",
            );
            let events = index
                .by_component_id_events
                .get("7")
                .expect("component 7 has events");
            let count = |needle: &str| {
                events
                    .iter()
                    .filter(|event| event.text.contains(needle))
                    .count()
            };
            assert_eq!(count("Pick one"), 1);
            assert_eq!(count("Second question"), 1);
        });
    }

    fn count_from<'e>(
        events: impl Iterator<Item = &'e PromptSummaryEvent>,
        file_name: &str,
    ) -> usize {
        let suffix = format!("/{file_name}");
        events
            .filter(|event| {
                event
                    .source_file
                    .replace('\\', "/")
                    .to_lowercase()
                    .ends_with(&suffix)
            })
            .count()
    }

    #[test]
    fn a_question_already_listed_untranslated_is_not_added_again() {
        on_big_stack(|| {
            let root = TempModsRoot::new();
            root.write("mymod/lib/sub.tpa", "PRINT @11\nACTION_READLN other\n");
            root.write("mymod/lib/ask.tpa", QUESTION_FILE);
            let index = root.scan_with_tra_unseen_by_vendored(
                &format!("{TRA_TEXT}@11 = ~Second question (1 or 2)~\n"),
                "ALWAYS\n  OUTER_SPRINT PKGNAME ~mymod~\nEND\n\nBEGIN @1 DESIGNATED 7\nINCLUDE ~mymod/lib/sub.tpa~\nINCLUDE ~%PKGNAME%/lib/sub.tpa~\nINCLUDE ~%PKGNAME%/lib/ask.tpa~\n",
            );
            let events = index
                .by_component_id_events
                .get("7")
                .expect("component 7 has events");
            assert!(events.iter().any(|event| event.text == "@11"));
            assert!(events.iter().any(|event| event.text.contains("Pick one")));
            assert_eq!(count_from(events.iter(), "sub.tpa"), 1);
        });
    }

    #[test]
    fn a_computed_include_two_levels_down_is_not_followed() {
        on_big_stack(|| {
            let root = TempModsRoot::new();
            root.write("mymod/lib/a.tpa", "INCLUDE ~mymod/lib/b.tpa~\n");
            root.write("mymod/lib/b.tpa", "INCLUDE ~%PKGNAME%/include/ask.tpa~\n");
            root.write("mymod/include/ask.tpa", QUESTION_FILE);
            let index = root.scan(
                "ALWAYS\n  OUTER_SPRINT PKGNAME ~mymod~\nEND\n\nBEGIN @1 DESIGNATED 7\nINCLUDE ~mymod/lib/a.tpa~\n",
            );
            let every_event = index
                .mod_events
                .iter()
                .chain(index.by_component_id_events.values().flatten());
            assert_eq!(count_from(every_event, "ask.tpa"), 0);
            assert!(!index.by_component_id.contains_key("7"));
        });
    }

    #[test]
    fn a_computed_include_with_no_owner_goes_to_the_mod_level_list() {
        on_big_stack(|| {
            let root = TempModsRoot::new();
            root.write("mymod/lib/ask.tpa", QUESTION_FILE);
            let index = root.scan(
                "ALWAYS\n  OUTER_SPRINT PKGNAME ~mymod~\n  INCLUDE ~%PKGNAME%/lib/ask.tpa~\nEND\n\nBEGIN ~Five~ DESIGNATED 5\nPRINT ~five~\n\nBEGIN ~Six~ DESIGNATED 6\nPRINT ~six~\n",
            );
            assert_eq!(index.mod_events.len(), 1);
            assert_eq!(count_from(index.mod_events.iter(), "ask.tpa"), 1);
            assert!(index.mod_events[0].text.contains("Pick one"));
            assert!(
                index
                    .mod_summary
                    .as_deref()
                    .is_some_and(|text| text.contains("Pick one"))
            );
            for id in ["5", "6"] {
                assert!(!index.by_component_id.contains_key(id));
                assert!(!index.by_component_id_events.contains_key(id));
            }
        });
    }

    #[test]
    fn a_file_included_by_a_component_passes_its_owner_on() {
        on_big_stack(|| {
            let root = TempModsRoot::new();
            root.write("mymod/lib/a.tpa", "INCLUDE ~%PKGNAME%/include/ask.tpa~\n");
            root.write("mymod/include/ask.tpa", QUESTION_FILE);
            let index = root.scan(
                "ALWAYS\n  OUTER_SPRINT PKGNAME ~mymod~\nEND\n\nBEGIN @1 DESIGNATED 7\nINCLUDE ~mymod/lib/a.tpa~\n\nBEGIN @2 DESIGNATED 8\nPRINT ~eight~\n",
            );
            assert!(summary_of(&index, "7").is_some_and(|text| text.contains("Pick one")));
            assert!(!index.by_component_id.contains_key("8"));
            assert!(!index.by_component_id_events.contains_key("8"));
            assert_eq!(index.mod_events.len(), 0);
            assert!(index.mod_summary.is_none());
        });
    }

    #[test]
    fn a_component_number_in_a_file_included_by_a_component_means_that_component() {
        on_big_stack(|| {
            let root = TempModsRoot::new();
            root.write(
                "mymod/lib/a.tpa",
                "INCLUDE ~mymod/lib/comp_%COMPONENT_NUMBER%_ask.tpa~\n",
            );
            root.write("mymod/lib/comp_7_ask.tpa", QUESTION_FILE);
            root.write("mymod/lib/comp_8_ask.tpa", QUESTION_FILE);
            let index = root.scan(
                "BEGIN @1 DESIGNATED 7\nINCLUDE ~mymod/lib/a.tpa~\n\nBEGIN @2 DESIGNATED 8\nPRINT ~eight~\n",
            );
            assert!(summary_of(&index, "7").is_some_and(|text| text.contains("Pick one")));
            assert!(!index.by_component_id.contains_key("8"));
            assert!(!index.by_component_id_events.contains_key("8"));
            assert_eq!(index.mod_events.len(), 0);
        });
    }

    fn unlined_event(text: &str) -> PromptSummaryEvent {
        PromptSummaryEvent {
            kind: "readln".to_string(),
            text: text.to_string(),
            summary_line: text.to_string(),
            source_file: "mods/mymod/lib/ask.tpa".to_string(),
            line: None,
            ..PromptSummaryEvent::default()
        }
    }

    #[test]
    fn two_unlined_questions_in_one_file_are_both_kept() {
        let mut index = PromptSummaryIndex::default();
        merge_events(
            &mut index,
            Some("7"),
            vec![unlined_event("First"), unlined_event("Second")],
        );
        merge_events(&mut index, Some("7"), vec![unlined_event("First")]);
        let texts: Vec<&str> = index.by_component_id_events["7"]
            .iter()
            .map(|event| event.text.as_str())
            .collect();
        assert_eq!(texts, ["First", "Second"]);
    }

    #[test]
    fn a_backslash_in_a_variable_value_becomes_a_slash() {
        let layout = ModLayout::for_tp2(Path::new("mods/mymod/setup-mymod.tp2"));
        let variables = HashMap::from([("PKGNAME".to_string(), "mymod\\include".to_string())]);
        let search = TargetedSearch::new(&layout, &variables, &[]);
        let paths: Vec<String> = search
            .expand("%PKGNAME%/ask.tpa", None)
            .into_iter()
            .map(|(path, _)| path)
            .collect();
        assert_eq!(paths, ["mymod/include/ask.tpa"]);
    }
}
