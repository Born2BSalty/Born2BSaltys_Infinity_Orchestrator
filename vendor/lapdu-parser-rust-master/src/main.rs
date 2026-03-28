mod generated;

use antlr4rust::common_token_stream::CommonTokenStream;
use antlr4rust::Parser;
use antlr4rust::tree::{ParseTree, ParseTreeVisitorCompat};
use antlr4rust::InputStream;
use encoding_rs::{Encoding, WINDOWS_1250, WINDOWS_1252};
use std::collections::HashMap;
use std::collections::HashSet;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use generated::lapducombinedlexer::LapduCombinedLexer;
use generated::lapducombinedparser::{
    ActionMatchActionContext, ActionMatchActionContextAttrs, ActionReadlnActionContext,
    AnyMatchBranchContextAttrs, ComponentRuleContext, ComponentRuleContextAttrs, LapduCombinedParser,
    LapduCombinedParserContextType, MatchBranchRuleContextAll, QuickMenuContext, QuickMenuContextAttrs,
    QuickMenuComponentRuleContextAttrs, QuickMenuDirectiveRuleContextAttrs,
    QuickMenuEntryRuleContextAttrs, QuickMenuParamsRuleContextAttrs, SpecificMatchBranchContextAttrs,
};
use generated::lapducombinedparservisitor::LapduCombinedParserVisitorCompat;
use serde::Serialize;

#[derive(Debug)]
struct ComponentReadlnInfo {
    name: String,
    action_readln_instances: Vec<String>,
}

#[derive(Debug)]
struct QuickMenuEntryInfo {
    title: String,
    components: Vec<String>,
}

#[derive(Debug)]
struct QuickMenuInfo {
    directives: Vec<String>,
    always_ask: bool,
    entries: Vec<QuickMenuEntryInfo>,
}

#[derive(Debug)]
struct MatchPromptInfo {
    match_value: String,
    options: Vec<String>,
}

#[derive(Debug, Serialize)]
struct ParserOutput {
    schema_version: u32,
    source_file: String,
    tra_language_requested: Option<String>,
    tra_language_used: Option<String>,
    events: Vec<PromptEvent>,
    flow: Vec<FlowNode>,
    warnings: Vec<ParserDiagnostic>,
    errors: Vec<ParserDiagnostic>,
}

#[derive(Debug, Serialize)]
struct PromptEvent {
    kind: String,
    interactive: bool,
    node_id: String,
    parent_id: Option<String>,
    path_id: String,
    text: String,
    options: Vec<PromptOption>,
    source_file: String,
    line: Option<u32>,
    branch_path: Vec<String>,
    condition: Option<String>,
    condition_id: Option<String>,
    game_allow: Vec<String>,
    game_deny: Vec<String>,
}

#[derive(Debug, Serialize, Clone)]
struct PromptOption {
    label: String,
    value: String,
    component_ids: Vec<String>,
}

#[derive(Debug, Serialize)]
struct ParserDiagnostic {
    code: String,
    message: String,
    source_file: Option<String>,
    line: Option<u32>,
}

#[derive(Debug, Serialize, Clone)]
struct FlowNode {
    id: String,
    label: String,
    event_ids: Vec<String>,
    children: Vec<FlowNode>,
}

#[derive(Debug, Default)]
struct TraLookupResult {
    map: HashMap<String, String>,
    language_used: Option<String>,
}

#[derive(Debug, Default, Clone)]
struct GameConditionMeta {
    raw: Option<String>,
    line: Option<u32>,
    allow: Vec<String>,
    deny: Vec<String>,
}

struct ComponentReadlnCollector {
    temp: (),
    current_component: Option<usize>,
    components: Vec<ComponentReadlnInfo>,
    global_action_readln_instances: Vec<String>,
    quick_menus: Vec<QuickMenuInfo>,
    match_prompts: Vec<MatchPromptInfo>,
}

impl ComponentReadlnCollector {
    fn new() -> Self {
        Self {
            temp: (),
            current_component: None,
            components: Vec::new(),
            global_action_readln_instances: Vec::new(),
            quick_menus: Vec::new(),
            match_prompts: Vec::new(),
        }
    }
}

impl<'input> ParseTreeVisitorCompat<'input> for ComponentReadlnCollector {
    type Node = LapduCombinedParserContextType;
    type Return = ();

    fn temp_result(&mut self) -> &mut Self::Return {
        &mut self.temp
    }
}

impl<'input> LapduCombinedParserVisitorCompat<'input> for ComponentReadlnCollector {
    fn visit_componentRule(&mut self, ctx: &ComponentRuleContext<'input>) -> Self::Return {
        let component_name = resolve_component_binding_name(ctx, self.components.len());

        self.components.push(ComponentReadlnInfo {
            name: component_name,
            action_readln_instances: Vec::new(),
        });

        let previous = self.current_component.replace(self.components.len() - 1);
        self.visit_children(ctx);
        self.current_component = previous;
    }

    fn visit_actionReadlnAction(&mut self, ctx: &ActionReadlnActionContext<'input>) -> Self::Return {
        if let Some(component_index) = self.current_component {
            self.components[component_index]
                .action_readln_instances
                .push(ctx.get_text());
        } else {
            self.global_action_readln_instances.push(ctx.get_text());
        }
        self.visit_children(ctx);
    }

    fn visit_quickMenu(&mut self, ctx: &QuickMenuContext<'input>) -> Self::Return {
        let mut quick_menu = QuickMenuInfo {
            directives: Vec::new(),
            always_ask: false,
            entries: Vec::new(),
        };

        if let Some(params) = ctx.quickMenuParamsRule() {
            for directive in params.quickMenuDirectiveRule_all() {
                let value = directive
                    .stringRule()
                    .map_or_else(String::new, |v| v.get_text());
                let normalized = value.trim().to_ascii_uppercase();
                if normalized == "ALWAYS_ASK" {
                    quick_menu.always_ask = true;
                }
                if !value.trim().is_empty() {
                    quick_menu.directives.push(value);
                }
            }

            for entry in params.quickMenuEntryRule_all() {
                let title = entry
                    .traLineRule()
                    .map(|v| v.get_text())
                    .unwrap_or_else(|| "<missing-title>".to_string());
                let components = entry
                    .quickMenuComponentRule_all()
                    .into_iter()
                    .filter_map(|component| component.numberRule().map(|number| number.get_text()))
                    .collect::<Vec<_>>();
                quick_menu.entries.push(QuickMenuEntryInfo { title, components });
            }
        }

        self.quick_menus.push(quick_menu);
        self.visit_children(ctx);
    }

    fn visit_actionMatchAction(&mut self, ctx: &ActionMatchActionContext<'input>) -> Self::Return {
        let match_value = ctx
            .expressionRule()
            .map(|v| v.get_text())
            .unwrap_or_else(|| "<missing-match-value>".to_string());

        let mut options: Vec<String> = Vec::new();
        for branch in ctx.matchBranchRule_all() {
            match branch.as_ref() {
                MatchBranchRuleContextAll::SpecificMatchBranchContext(specific) => {
                    let mut exprs: Vec<String> = specific
                        .expressionRule_all()
                        .into_iter()
                        .map(|v| v.get_text())
                        .collect();

                    if specific.WHEN().is_some() && !exprs.is_empty() {
                        exprs.pop();
                    }

                    for guard in exprs {
                        options.push(guard);
                    }
                }
                MatchBranchRuleContextAll::AnyMatchBranchContext(any_branch) => {
                    options.push("ANY".to_string());
                    if let Some(condition) = any_branch.expressionRule() {
                        options.push(format!("ANY WHEN {}", condition.get_text()));
                    }
                }
                MatchBranchRuleContextAll::Error(_) => {}
            }
        }

        self.match_prompts.push(MatchPromptInfo {
            match_value,
            options,
        });

        self.visit_children(ctx);
    }
}

fn resolve_component_binding_name(ctx: &ComponentRuleContext<'_>, default_index: usize) -> String {
    if let Some(designated) = extract_designated_component_id(ctx) {
        return designated;
    }
    format!("@{default_index}")
}

fn extract_designated_component_id(ctx: &ComponentRuleContext<'_>) -> Option<String> {
    for flag in ctx.componentFlagRule_all() {
        let text = flag.get_text();
        let upper = text.to_ascii_uppercase();
        if !upper.starts_with("DESIGNATED") {
            continue;
        }
        let digits: String = text.chars().filter(|c| c.is_ascii_digit()).collect();
        if let Some(id) = canonical_component_id_from_digits(&digits) {
            return Some(id);
        }
    }
    None
}

fn canonical_component_id_from_digits(digits: &str) -> Option<String> {
    if digits.is_empty() || !digits.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let normalized = digits.trim_start_matches('0');
    if normalized.is_empty() {
        Some("@0".to_string())
    } else {
        Some(format!("@{normalized}"))
    }
}

pub fn main() {
    let handle = std::thread::Builder::new()
        .name("lapdu-main".to_string())
        .stack_size(32 * 1024 * 1024)
        .spawn(run)
        .expect("failed to spawn parser thread");

    match handle.join() {
        Ok(Ok(())) => {}
        Ok(Err(err)) => {
            eprintln!("{err}");
            std::process::exit(1);
        }
        Err(_) => {
            eprintln!("parser thread panicked");
            std::process::exit(1);
        }
    }
}

pub fn run() -> Result<(), String> {
    let cli = parse_cli_args(std::env::args().skip(1).collect())?;
    let input_path = cli
        .input_path
        .unwrap_or_else(|| "setup-Helga.TP2".to_string());
    let root_path = PathBuf::from(input_path);
    let json = parse_path_to_json(&root_path, cli.lang.as_deref())?;
    println!("{json}");
    Ok(())
}

pub fn parse_path_to_json(root_path: &Path, preferred_lang: Option<&str>) -> Result<String, String> {
    let root_display = root_path.to_string_lossy().to_string();
    let mod_root = root_path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    let mut visited: HashSet<PathBuf> = HashSet::new();
    let mut targets: Vec<(PathBuf, String)> = Vec::new();
    collect_parse_targets(&root_path, &mod_root, &mut visited, &mut targets)?;
    let mut include_component_hints: HashMap<PathBuf, HashSet<String>> = HashMap::new();
    let mut include_walk_visited: HashSet<(PathBuf, Option<String>)> = HashSet::new();
    collect_include_component_hints(
        &root_path,
        &mod_root,
        None,
        &mut include_walk_visited,
        &mut include_component_hints,
    )?;

    let mut events: Vec<PromptEvent> = Vec::new();
    let mut warnings: Vec<ParserDiagnostic> = Vec::new();
    let mut tra_language_used: Option<String> = None;
    let root_tra_lookup = load_tra_map_for_source(&root_display, preferred_lang);
    if tra_language_used.is_none() && root_tra_lookup.language_used.is_some() {
        tra_language_used = root_tra_lookup.language_used.clone();
    }
    for (path, source) in targets {
        let source_file = path.to_string_lossy().to_string();
        let source_normalized = normalize_for_visited(&path);
        let component_hints = include_component_hints.get(&source_normalized);
        match parse_source(&source, &path) {
            Ok(visitor) => append_events_from_visitor(
                &mut events,
                &visitor,
                &source,
                &source_file,
                preferred_lang,
                &mut tra_language_used,
                Some(&root_tra_lookup.map),
                component_hints,
            ),
            Err(err) => warnings.push(ParserDiagnostic {
                code: "PARSE_FILE_FAILED".to_string(),
                message: err,
                source_file: Some(source_file),
                line: None,
            }),
        }
    }

    if events.is_empty() {
        warnings.push(ParserDiagnostic {
            code: "NO_PROMPT_EVENTS".to_string(),
            message: "Parsed successfully but no prompt-like events were extracted".to_string(),
            source_file: Some(root_display.clone()),
            line: None,
        });
    }

    let output = ParserOutput {
        schema_version: 2,
        source_file: root_display,
        tra_language_requested: preferred_lang.map(ToString::to_string),
        tra_language_used,
        flow: build_flow(&events),
        events,
        warnings,
        errors: Vec::new(),
    };

    let json = serde_json::to_string_pretty(&output)
        .map_err(|e| format!("failed to serialize parser output: {e}"))?;
    Ok(json)
}

#[derive(Debug)]
pub struct CliArgs {
    input_path: Option<String>,
    lang: Option<String>,
}

pub fn parse_cli_args(args: Vec<String>) -> Result<CliArgs, String> {
    let mut input_path: Option<String> = None;
    let mut lang: Option<String> = None;
    let mut i = 0usize;
    while i < args.len() {
        match args[i].as_str() {
            "--lang" => {
                let Some(value) = args.get(i + 1) else {
                    return Err("missing value for --lang".to_string());
                };
                if value.starts_with("--") {
                    return Err("missing value for --lang".to_string());
                }
                lang = Some(value.clone());
                i += 2;
                continue;
            }
            "--help" | "-h" => {
                return Err(
                    "usage: lapdu-parser-rust <tp2-path> [--lang <locale_or_language>]".to_string(),
                );
            }
            _ => {
                if input_path.is_none() {
                    input_path = Some(args[i].clone());
                }
            }
        }
        i += 1;
    }

    Ok(CliArgs { input_path, lang })
}

fn build_flow(events: &[PromptEvent]) -> Vec<FlowNode> {
    let mut roots: Vec<FlowNode> = Vec::new();

    for event in events {
        let root_id = format!("{}::root", event.source_file);
        let root_idx = match roots.iter().position(|n| n.id == root_id) {
            Some(i) => i,
            None => {
                roots.push(FlowNode {
                    id: root_id.clone(),
                    label: "root".to_string(),
                    event_ids: Vec::new(),
                    children: Vec::new(),
                });
                roots.len() - 1
            }
        };

        if event.branch_path.is_empty() {
            roots[root_idx].event_ids.push(event.node_id.clone());
            continue;
        }

        let mut current = &mut roots[root_idx];
        for depth in 0..event.branch_path.len() {
            let segment = event.branch_path[depth].clone();
            let path_id = make_path_id(&event.source_file, &event.branch_path[..=depth]);
            let child_idx = match current.children.iter().position(|n| n.id == path_id) {
                Some(i) => i,
                None => {
                    current.children.push(FlowNode {
                        id: path_id,
                        label: segment,
                        event_ids: Vec::new(),
                        children: Vec::new(),
                    });
                    current.children.len() - 1
                }
            };
            current = &mut current.children[child_idx];
        }

        current.event_ids.push(event.node_id.clone());
    }

    roots
}

fn parse_source(source: &str, path: &Path) -> Result<ComponentReadlnCollector, String> {
    let path_display = path.to_string_lossy().to_string();
    let mut errors: Vec<String> = Vec::new();
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();

    // Choose parser entry by file type first to avoid noisy mismatch diagnostics.
    if ext == "tph" || ext == "tpa" {
        let mut lexer = LapduCombinedLexer::new(InputStream::new(source));
        lexer.remove_error_listeners();
        let token_stream = CommonTokenStream::new(lexer);
        let mut parser = LapduCombinedParser::new(token_stream);
        parser.remove_error_listeners();
        match parser.tpaFileRule() {
            Ok(tree) => {
                let mut visitor = ComponentReadlnCollector::new();
                visitor.visit(&*tree);
                return Ok(visitor);
            }
            Err(e) => errors.push(format!("tpaFileRule: {e}")),
        }
    } else if ext == "tpp" {
        let mut lexer = LapduCombinedLexer::new(InputStream::new(source));
        lexer.remove_error_listeners();
        let token_stream = CommonTokenStream::new(lexer);
        let mut parser = LapduCombinedParser::new(token_stream);
        parser.remove_error_listeners();
        match parser.tppFileRule() {
            Ok(tree) => {
                let mut visitor = ComponentReadlnCollector::new();
                visitor.visit(&*tree);
                return Ok(visitor);
            }
            Err(e) => errors.push(format!("tppFileRule: {e}")),
        }
    } else if ext == "tp2" {
        let mut lexer = LapduCombinedLexer::new(InputStream::new(source));
        lexer.remove_error_listeners();
        let token_stream = CommonTokenStream::new(lexer);
        let mut parser = LapduCombinedParser::new(token_stream);
        parser.remove_error_listeners();
        match parser.tp2File() {
            Ok(tree) => {
                let mut visitor = ComponentReadlnCollector::new();
                visitor.visit(&*tree);
                return Ok(visitor);
            }
            Err(e) => errors.push(format!("tp2File: {e}")),
        }
    }

    {
        let mut lexer = LapduCombinedLexer::new(InputStream::new(source));
        lexer.remove_error_listeners();
        let token_stream = CommonTokenStream::new(lexer);
        let mut parser = LapduCombinedParser::new(token_stream);
        parser.remove_error_listeners();
        match parser.tp2File() {
            Ok(tree) => {
                let mut visitor = ComponentReadlnCollector::new();
                visitor.visit(&*tree);
                return Ok(visitor);
            }
            Err(e) => errors.push(format!("tp2File: {e}")),
        }
    }

    {
        let mut lexer = LapduCombinedLexer::new(InputStream::new(source));
        lexer.remove_error_listeners();
        let token_stream = CommonTokenStream::new(lexer);
        let mut parser = LapduCombinedParser::new(token_stream);
        parser.remove_error_listeners();
        match parser.tpaFileRule() {
            Ok(tree) => {
                let mut visitor = ComponentReadlnCollector::new();
                visitor.visit(&*tree);
                return Ok(visitor);
            }
            Err(e) => errors.push(format!("tpaFileRule: {e}")),
        }
    }

    {
        let mut lexer = LapduCombinedLexer::new(InputStream::new(source));
        lexer.remove_error_listeners();
        let token_stream = CommonTokenStream::new(lexer);
        let mut parser = LapduCombinedParser::new(token_stream);
        parser.remove_error_listeners();
        match parser.tppFileRule() {
            Ok(tree) => {
                let mut visitor = ComponentReadlnCollector::new();
                visitor.visit(&*tree);
                return Ok(visitor);
            }
            Err(e) => errors.push(format!("tppFileRule: {e}")),
        }
    }

    Err(format!(
        "parse failed for '{}': {}",
        path_display,
        errors.join(" | ")
    ))
}

fn append_events_from_visitor(
    events: &mut Vec<PromptEvent>,
    visitor: &ComponentReadlnCollector,
    source: &str,
    source_file: &str,
    preferred_lang: Option<&str>,
    tra_language_used: &mut Option<String>,
    root_tra_map: Option<&HashMap<String, String>>,
    include_component_hints: Option<&HashSet<String>>,
) {
    let mut quick_menu_search_from_line = 0usize;
    let mut match_prompt_search_from_line = 0usize;
    let mut readln_search_from_line = 0usize;
    let mut seen_readln_keys: HashSet<String> = HashSet::new();
    let mut seen_subcomponent_keys: HashSet<String> = HashSet::new();
    let source_lines: Vec<&str> = source.lines().collect();
    let tra_lookup = load_tra_map_for_source(source_file, preferred_lang);
    if tra_language_used.is_none() && tra_lookup.language_used.is_some() {
        *tra_language_used = tra_lookup.language_used.clone();
    }
    let mut tra_map = tra_lookup.map;
    if let Some(root_map) = root_tra_map {
        for (k, v) in root_map {
            tra_map.entry(k.clone()).or_insert_with(|| v.clone());
        }
    }

    for (qm_idx, quick_menu) in visitor.quick_menus.iter().enumerate() {
        let (question, line) = derive_keyword_prompt(
            &source_lines,
            &tra_map,
            "QUICK_MENU",
            &mut quick_menu_search_from_line,
        );

        let mut options: Vec<PromptOption> = quick_menu
            .entries
            .iter()
            .map(|entry| {
                let label = resolve_atom_text(&entry.title, &tra_map);
                PromptOption {
                    label: sanitize_wrapped_text(&label),
                    value: sanitize_wrapped_text(&label),
                    component_ids: entry.components.clone(),
                }
            })
            .collect();

        // Fallback to parsed PRINT options nearby if QUICK_MENU entry labels are weak/missing.
        if options.is_empty() {
            let print_context = collect_previous_print_texts(
                &source_lines,
                &tra_map,
                line.map(|v| v as usize).unwrap_or(1).saturating_sub(1),
                30,
            );
            let (from_prints, _) = select_options_from_prints(&print_context);
            options = from_prints;
        }

        let condition_meta = line
            .map(|v| infer_game_condition_for_line(&source_lines, v as usize - 1))
            .unwrap_or_default();
        let branch_path = line
            .map(|v| infer_branch_path_for_line(&source_lines, v as usize - 1))
            .unwrap_or_default();
        let id = format!("quick_menu:{}:{}", source_file, qm_idx + 1);
        let path_id = make_path_id(source_file, &branch_path);
        let parent_id = make_parent_path_id(source_file, &branch_path);
        events.push(PromptEvent {
            kind: "quick_menu".to_string(),
            interactive: false,
            node_id: id.clone(),
            parent_id,
            path_id,
            text: if question.is_empty() {
                if quick_menu.always_ask {
                    "QUICK_MENU ALWAYS_ASK".to_string()
                } else {
                    "QUICK_MENU".to_string()
                }
            } else {
                question
            },
            options,
            source_file: source_file.to_string(),
            line,
            branch_path,
            condition: condition_meta.raw.clone(),
            condition_id: condition_meta
                .line
                .map(|ln| format!("{source_file}:{ln}")),
            game_allow: condition_meta.allow,
            game_deny: condition_meta.deny,
        });
    }

    for (idx, match_prompt) in visitor.match_prompts.iter().enumerate() {
        let (friendly_text, mut options, line, options_from_prints) = derive_match_prompt(
            &source_lines,
            &tra_map,
            &match_prompt.match_value,
            &match_prompt.options,
            &mut match_prompt_search_from_line,
        );

        if options.is_empty() {
            for option in &match_prompt.options {
                let resolved = resolve_atom_text(option, &tra_map);
                let cleaned = sanitize_wrapped_text(&resolved);
                options.push(PromptOption {
                    label: cleaned.clone(),
                    value: cleaned,
                    component_ids: Vec::new(),
                });
            }
        }

        // Drop low-signal internal ACTION_MATCH branches from helper libraries.
        // We keep events only when they look like real prompts (print-derived question/options).
        if !options_from_prints && friendly_text.trim().is_empty() {
            continue;
        }
        if is_internal_match_prompt(&friendly_text, &options) {
            continue;
        }

        let condition_meta = line
            .map(|v| infer_game_condition_for_line(&source_lines, v as usize - 1))
            .unwrap_or_default();
        let branch_path = line
            .map(|v| infer_branch_path_for_line(&source_lines, v as usize - 1))
            .unwrap_or_default();
        let id = format!("match_prompt:{}:{}", source_file, idx + 1);
        let path_id = make_path_id(source_file, &branch_path);
        let parent_id = make_parent_path_id(source_file, &branch_path);
        events.push(PromptEvent {
            kind: "match_prompt".to_string(),
            interactive: true,
            node_id: id.clone(),
            parent_id,
            path_id,
            text: if friendly_text.is_empty() {
                format!("ACTION_MATCH {}", match_prompt.match_value)
            } else {
                friendly_text
            },
            options,
            source_file: source_file.to_string(),
            line,
            branch_path,
            condition: condition_meta.raw.clone(),
            condition_id: condition_meta
                .line
                .map(|ln| format!("{source_file}:{ln}")),
            game_allow: condition_meta.allow,
            game_deny: condition_meta.deny,
        });
    }

    for component in &visitor.components {
        for (idx, action) in component.action_readln_instances.iter().enumerate() {
            let var_name = extract_readln_var(action);
            let (friendly_text, options, line) = derive_readln_prompt(
                &source_lines,
                &tra_map,
                &var_name,
                &mut readln_search_from_line,
            );

            let condition_meta = line
                .map(|v| infer_game_condition_for_line(&source_lines, v as usize - 1))
                .unwrap_or_default();
            let final_text = if friendly_text.is_empty() {
                if var_name.is_empty() {
                    format!("ACTION_READLN for {}", component.name)
                } else {
                    format!("ACTION_READLN {} for {}", var_name, component.name)
                }
            } else {
                friendly_text
            };
            let options_key = options
                .iter()
                .map(|o| {
                    format!(
                        "{}={}",
                        o.value.trim().to_ascii_lowercase(),
                        o.label.trim().to_ascii_lowercase()
                    )
                })
                .collect::<Vec<_>>()
                .join("|");
            let dedupe_key = format!(
                "{}|{}|{}|{}|{}",
                component.name.trim().to_ascii_lowercase(),
                var_name.trim().to_ascii_lowercase(),
                final_text.trim().to_ascii_lowercase(),
                condition_meta.raw.as_deref().unwrap_or("").to_ascii_lowercase(),
                options_key
            );
            if !seen_readln_keys.insert(dedupe_key) {
                continue;
            }
            let id = format!("readln:{}:{}:{}", source_file, component.name, idx + 1);
            let branch_path = line
                .map(|v| infer_branch_path_for_line(&source_lines, v as usize - 1))
                .unwrap_or_default();
            let path_id = make_path_id(source_file, &branch_path);
            let parent_id = make_parent_path_id(source_file, &branch_path);
            events.push(PromptEvent {
                kind: "readln".to_string(),
                interactive: true,
                node_id: id.clone(),
                parent_id,
                path_id,
                text: final_text,
                options,
                source_file: source_file.to_string(),
                line,
                branch_path,
                condition: condition_meta.raw.clone(),
                condition_id: condition_meta
                    .line
                    .map(|ln| format!("{source_file}:{ln}")),
                game_allow: condition_meta.allow,
                game_deny: condition_meta.deny,
            });
        }
    }

    // Also include ACTION_READLN prompts found outside explicit BEGIN component blocks,
    // e.g. in included helper libraries that still drive real installer questions.
    for (idx, action) in visitor.global_action_readln_instances.iter().enumerate() {
        let var_name = extract_readln_var(action);
        let (friendly_text, options, line) = derive_readln_prompt(
            &source_lines,
            &tra_map,
            &var_name,
            &mut readln_search_from_line,
        );
        let condition_meta = line
            .map(|v| infer_game_condition_for_line(&source_lines, v as usize - 1))
            .unwrap_or_default();
        let final_text = if friendly_text.is_empty() {
            if var_name.is_empty() {
                "ACTION_READLN".to_string()
            } else {
                format!("ACTION_READLN {}", var_name)
            }
        } else {
            friendly_text
        };
        let inferred_components = infer_component_hints_from_include_context(include_component_hints);

        let options_key = options
            .iter()
            .map(|o| {
                format!(
                    "{}={}",
                    o.value.trim().to_ascii_lowercase(),
                    o.label.trim().to_ascii_lowercase()
                )
            })
            .collect::<Vec<_>>()
            .join("|");
        let bind_targets: Vec<Option<String>> = if inferred_components.is_empty() {
            vec![None]
        } else {
            inferred_components.into_iter().map(Some).collect()
        };
        for bind_target in bind_targets {
            let dedupe_key = format!(
                "{}|{}|{}|{}|{}",
                bind_target.as_deref().unwrap_or("global"),
                var_name.trim().to_ascii_lowercase(),
                final_text.trim().to_ascii_lowercase(),
                condition_meta.raw.as_deref().unwrap_or("").to_ascii_lowercase(),
                options_key
            );
            if !seen_readln_keys.insert(dedupe_key) {
                continue;
            }

            let id = if let Some(component) = bind_target.as_deref() {
                format!("readln:{}:{}:{}", source_file, component, idx + 1)
            } else {
                format!("readln:{}:global:{}", source_file, idx + 1)
            };
            let branch_path = line
                .map(|v| infer_branch_path_for_line(&source_lines, v as usize - 1))
                .unwrap_or_default();
            let path_id = make_path_id(source_file, &branch_path);
            let parent_id = make_parent_path_id(source_file, &branch_path);
            events.push(PromptEvent {
                kind: "readln".to_string(),
                interactive: true,
                node_id: id,
                parent_id,
                path_id,
                text: final_text.clone(),
                options: options.clone(),
                source_file: source_file.to_string(),
                line,
                branch_path,
                condition: condition_meta.raw.clone(),
                condition_id: condition_meta
                    .line
                    .map(|ln| format!("{source_file}:{ln}")),
                game_allow: condition_meta.allow.clone(),
                game_deny: condition_meta.deny.clone(),
            });
        }
    }

    for (idx, prompt) in extract_subcomponent_prompts(&source_lines, &tra_map).into_iter().enumerate() {
        let condition_meta = prompt
            .line
            .map(|v| infer_game_condition_for_line(&source_lines, v as usize - 1))
            .unwrap_or_default();
        let branch_path = prompt
            .line
            .map(|v| infer_branch_path_for_line(&source_lines, v as usize - 1))
            .unwrap_or_default();
        let dedupe_key = format!(
            "{}|{}|{}|{}",
            prompt
                .text
                .trim()
                .to_ascii_lowercase(),
            prompt
                .options
                .iter()
                .map(|o| format!("{}={}", o.value, o.label))
                .collect::<Vec<_>>()
                .join("|")
                .to_ascii_lowercase(),
            condition_meta
                .raw
                .as_deref()
                .unwrap_or("")
                .to_ascii_lowercase(),
            prompt.line.unwrap_or_default()
        );
        if !seen_subcomponent_keys.insert(dedupe_key) {
            continue;
        }

        let id = format!("subcomponent_prompt:{}:{}", source_file, idx + 1);
        let path_id = make_path_id(source_file, &branch_path);
        let parent_id = make_parent_path_id(source_file, &branch_path);
        events.push(PromptEvent {
            kind: "subcomponent_prompt".to_string(),
            interactive: true,
            node_id: id,
            parent_id,
            path_id,
            text: prompt.text,
            options: prompt.options,
            source_file: source_file.to_string(),
            line: prompt.line,
            branch_path,
            condition: condition_meta.raw.clone(),
            condition_id: condition_meta
                .line
                .map(|ln| format!("{source_file}:{ln}")),
            game_allow: condition_meta.allow,
            game_deny: condition_meta.deny,
        });
    }
}

fn infer_component_hints_from_include_context(
    include_component_hints: Option<&HashSet<String>>,
) -> Vec<String> {
    let Some(hints) = include_component_hints else {
        return Vec::new();
    };
    let mut normalized = hints
        .iter()
        .filter_map(|hint| normalize_component_hint(hint))
        .collect::<Vec<_>>();
    normalized.sort();
    normalized.dedup();
    normalized
}

fn normalize_component_hint(hint: &str) -> Option<String> {
    let trimmed = hint.trim();
    if trimmed.is_empty() {
        return None;
    }
    if let Some(digits) = trimmed.strip_prefix('@') {
        return canonical_component_id_from_digits(digits);
    }
    canonical_component_id_from_digits(trimmed)
}

#[derive(Debug)]
struct SubcomponentPromptDraft {
    text: String,
    options: Vec<PromptOption>,
    line: Option<u32>,
}

fn extract_subcomponent_prompts(
    source_lines: &[&str],
    tra_map: &HashMap<String, String>,
) -> Vec<SubcomponentPromptDraft> {
    let mut groups: HashMap<String, (usize, Vec<String>)> = HashMap::new();

    let mut idx = 0usize;
    while idx < source_lines.len() {
        let line = strip_inline_comment(source_lines[idx]).trim();
        let upper = line.to_ascii_uppercase();
        if !upper.starts_with("BEGIN ") {
            idx += 1;
            continue;
        }

        let begin_atom = parse_begin_atom(line);
        let mut j = idx + 1;
        let mut subcomponent_atom: Option<String> = None;
        while j < source_lines.len() {
            let look = strip_inline_comment(source_lines[j]).trim();
            let look_upper = look.to_ascii_uppercase();
            if look_upper.starts_with("BEGIN ") {
                break;
            }
            if look_upper.starts_with("SUBCOMPONENT ") {
                subcomponent_atom = parse_subcomponent_atom(look);
                break;
            }
            if !look.is_empty() {
                break;
            }
            j += 1;
        }

        if let (Some(sub), Some(begin)) = (subcomponent_atom, begin_atom) {
            let entry = groups.entry(sub).or_insert((j, Vec::new()));
            entry.1.push(begin);
        }

        idx += 1;
    }

    let mut out = Vec::new();
    for (question_atom, (line_idx, begin_atoms)) in groups {
        let question = sanitize_wrapped_text(&resolve_atom_text(&question_atom, tra_map));
        if question.is_empty() {
            continue;
        }
        let mut options = Vec::new();
        for (opt_idx, begin_atom) in begin_atoms.iter().enumerate() {
            let label = sanitize_wrapped_text(&resolve_atom_text(begin_atom, tra_map));
            if label.is_empty() {
                continue;
            }
            options.push(PromptOption {
                label,
                value: (opt_idx + 1).to_string(),
                component_ids: vec![sanitize_wrapped_text(begin_atom)],
            });
        }

        if options.len() < 2 {
            continue;
        }
        dedupe_prompt_options(&mut options);
        out.push(SubcomponentPromptDraft {
            text: question,
            options,
            line: Some((line_idx + 1) as u32),
        });
    }

    out
}

fn parse_begin_atom(line: &str) -> Option<String> {
    let rest = line
        .trim_start()
        .strip_prefix("BEGIN")
        .or_else(|| line.trim_start().strip_prefix("begin"))?
        .trim_start();
    parse_first_atom(rest)
}

fn parse_subcomponent_atom(line: &str) -> Option<String> {
    let rest = line
        .trim_start()
        .strip_prefix("SUBCOMPONENT")
        .or_else(|| line.trim_start().strip_prefix("subcomponent"))?
        .trim_start();
    parse_first_atom(rest)
}

fn parse_first_atom(rest: &str) -> Option<String> {
    if let Some(after) = rest.strip_prefix('@') {
        let digits: String = after.chars().take_while(|c| c.is_ascii_digit()).collect();
        if digits.is_empty() {
            return None;
        }
        return Some(format!("@{digits}"));
    }
    if let Some(after) = rest.strip_prefix('~') {
        if let Some(end) = after.find('~') {
            return Some(format!("~{}~", &after[..end]));
        }
    }
    if let Some(after) = rest.strip_prefix('"') {
        if let Some(end) = after.find('"') {
            return Some(format!("\"{}\"", &after[..end]));
        }
    }
    rest.split_whitespace().next().map(|s| s.to_string())
}

fn infer_game_condition_for_line(source_lines: &[&str], line_idx: usize) -> GameConditionMeta {
    if source_lines.is_empty() {
        return GameConditionMeta::default();
    }
    let mut stack: Vec<ActionIfFrame> = Vec::new();
    let end = line_idx.min(source_lines.len().saturating_sub(1));

    for (i, raw_line) in source_lines.iter().take(end + 1).enumerate() {
        let code = strip_inline_comment(raw_line).trim();
        if code.is_empty() {
            continue;
        }
        let upper = code.to_ascii_uppercase();

        if upper == "END" || upper.starts_with("END ") {
            let _ = stack.pop();
            continue;
        }

        if !is_block_begin(&upper) {
            continue;
        }

        if upper.starts_with("ACTION_IF") {
            stack.push(ActionIfFrame {
                line: (i + 1) as u32,
                code: code.to_string(),
            });
        } else {
            stack.push(ActionIfFrame {
                line: (i + 1) as u32,
                code: String::new(),
            });
        }
    }

    let action_ifs = stack
        .iter()
        .filter(|f| !f.code.is_empty())
        .collect::<Vec<_>>();
    if action_ifs.is_empty() {
        return GameConditionMeta::default();
    }

    let raw = action_ifs
        .iter()
        .map(|f| f.code.clone())
        .collect::<Vec<_>>()
        .join(" && ");
    let mut allow = Vec::<String>::new();
    let mut deny = Vec::<String>::new();
    for frame in &action_ifs {
        let upper = frame.code.to_ascii_uppercase();
        if !upper.contains("GAME_IS") {
            continue;
        }
        if let Some(game_meta) = parse_game_is_condition(&frame.code) {
            for g in game_meta.allow {
                if !allow.contains(&g) {
                    allow.push(g);
                }
            }
            for g in game_meta.deny {
                if !deny.contains(&g) {
                    deny.push(g);
                }
            }
        }
    }

    GameConditionMeta {
        raw: Some(raw),
        line: action_ifs.last().map(|f| f.line),
        allow,
        deny,
    }
}

fn infer_branch_path_for_line(source_lines: &[&str], line_idx: usize) -> Vec<String> {
    if source_lines.is_empty() {
        return Vec::new();
    }

    let mut block_stack: Vec<Option<String>> = Vec::new();
    let mut tracked_stack: Vec<String> = Vec::new();
    let end = line_idx.min(source_lines.len().saturating_sub(1));

    for raw_line in source_lines.iter().take(end + 1) {
        let code = strip_inline_comment(raw_line).trim();
        if code.is_empty() {
            continue;
        }

        let upper = code.to_ascii_uppercase();

        if upper == "END" || upper.starts_with("END ") {
            if let Some(frame) = block_stack.pop() {
                if let Some(label) = frame {
                    if tracked_stack.last().is_some_and(|v| v == &label) {
                        let _ = tracked_stack.pop();
                    }
                }
            }
            continue;
        }

        if !is_block_begin(&upper) {
            continue;
        }

        let tracked = if upper.starts_with("ACTION_IF") {
            Some(format!("if:{}", code))
        } else if upper.starts_with("OUTER_WHILE") {
            Some(format!("while:{}", code))
        } else if upper.starts_with("OUTER_FOR") {
            Some(format!("for:{}", code))
        } else {
            None
        };
        if let Some(label) = tracked.clone() {
            tracked_stack.push(label);
        }
        block_stack.push(tracked);
    }

    tracked_stack
}

#[derive(Clone, Debug)]
struct ActionIfFrame {
    line: u32,
    code: String,
}

fn is_block_begin(upper: &str) -> bool {
    upper == "BEGIN" || upper.contains("THEN BEGIN") || upper.ends_with(" BEGIN")
}

fn make_path_id(source_file: &str, branch_path: &[String]) -> String {
    if branch_path.is_empty() {
        return format!("{source_file}::root");
    }
    format!("{source_file}::{}", branch_path.join(" >> "))
}

fn make_parent_path_id(source_file: &str, branch_path: &[String]) -> Option<String> {
    if branch_path.is_empty() {
        return None;
    }
    let parent = &branch_path[..branch_path.len() - 1];
    Some(make_path_id(source_file, parent))
}

fn parse_game_is_condition(code: &str) -> Option<GameConditionMeta> {
    let upper = code.to_ascii_uppercase();
    let game_is_idx = upper.find("GAME_IS")?;
    let before = &upper[..game_is_idx];
    let negated = before.contains("NOT") || before.contains('!');

    let rest = &code[game_is_idx + "GAME_IS".len()..];
    let values = extract_game_values(rest);
    if values.is_empty() {
        return Some(GameConditionMeta {
            raw: Some(code.trim().to_string()),
            line: None,
            allow: Vec::new(),
            deny: Vec::new(),
        });
    }

    if negated {
        Some(GameConditionMeta {
            raw: Some(code.trim().to_string()),
            line: None,
            allow: Vec::new(),
            deny: values,
        })
    } else {
        Some(GameConditionMeta {
            raw: Some(code.trim().to_string()),
            line: None,
            allow: values,
            deny: Vec::new(),
        })
    }
}

fn extract_game_values(rest: &str) -> Vec<String> {
    let mut out = Vec::new();
    let trimmed = rest.trim();

    let payload = if let Some(s) = trimmed.find('~') {
        let tail = &trimmed[s + 1..];
        if let Some(e) = tail.find('~') {
            Some(tail[..e].to_string())
        } else {
            None
        }
    } else if let Some(s) = trimmed.find('"') {
        let tail = &trimmed[s + 1..];
        tail.find('"').map(|e| tail[..e].to_string())
    } else {
        None
    };

    if let Some(data) = payload {
        for tok in data.split_whitespace() {
            let t = tok.trim().to_ascii_lowercase();
            if !t.is_empty() && !out.contains(&t) {
                out.push(t);
            }
        }
    }

    out
}

fn derive_keyword_prompt(
    source_lines: &[&str],
    tra_map: &HashMap<String, String>,
    keyword: &str,
    search_from_line: &mut usize,
) -> (String, Option<u32>) {
    let idx = find_keyword_line_index(source_lines, keyword, *search_from_line)
        .or_else(|| find_keyword_line_index(source_lines, keyword, 0));
    let Some(idx) = idx else {
        return (String::new(), None);
    };
    *search_from_line = idx.saturating_add(1);

    let print_context = collect_previous_print_texts(source_lines, tra_map, idx, 30);
    let (_, options_idx) = select_options_from_prints(&print_context);
    let question = select_question_text(&print_context, options_idx).unwrap_or_default();
    (question, Some((idx + 1) as u32))
}

fn find_keyword_line_index(
    source_lines: &[&str],
    keyword: &str,
    start: usize,
) -> Option<usize> {
    source_lines
        .iter()
        .enumerate()
        .skip(start)
        .find(|(_, line)| strip_inline_comment(line).to_ascii_uppercase().contains(keyword))
        .map(|(idx, _)| idx)
}

fn derive_match_prompt(
    source_lines: &[&str],
    tra_map: &HashMap<String, String>,
    match_value: &str,
    raw_options: &[String],
    search_from_line: &mut usize,
) -> (String, Vec<PromptOption>, Option<u32>, bool) {
    let action_idx = find_action_match_line_index(source_lines, match_value, *search_from_line)
        .or_else(|| find_action_match_line_index(source_lines, match_value, 0));
    let Some(action_idx) = action_idx else {
        return (String::new(), Vec::new(), None, false);
    };
    *search_from_line = action_idx.saturating_add(1);

    let print_context = collect_previous_print_texts(source_lines, tra_map, action_idx, 30);
    let (mut options, options_idx) = select_options_from_prints(&print_context);
    let question = select_question_text(&print_context, options_idx).unwrap_or_default();
    let options_from_prints = !options.is_empty();

    if options.is_empty() {
        for raw in raw_options {
            let resolved = resolve_atom_text(raw, tra_map);
            let cleaned = sanitize_wrapped_text(&resolved);
            if cleaned.is_empty() {
                continue;
            }
            options.push(PromptOption {
                label: cleaned.clone(),
                value: cleaned,
                component_ids: Vec::new(),
            });
        }
    }

    dedupe_prompt_options(&mut options);
    (question, options, Some((action_idx + 1) as u32), options_from_prints)
}

fn find_action_match_line_index(
    source_lines: &[&str],
    match_value: &str,
    start: usize,
) -> Option<usize> {
    let needle = match_value.to_ascii_lowercase();
    for (idx, line) in source_lines.iter().enumerate().skip(start) {
        let code = strip_inline_comment(line).trim();
        let upper = code.to_ascii_uppercase();
        if !upper.starts_with("ACTION_MATCH ") {
            continue;
        }
        if needle.is_empty() || code.to_ascii_lowercase().contains(&needle) {
            return Some(idx);
        }
    }
    None
}

fn collect_parse_targets(
    path: &Path,
    mod_root: &Path,
    visited: &mut HashSet<PathBuf>,
    out: &mut Vec<(PathBuf, String)>,
) -> Result<(), String> {
    let normalized = normalize_for_visited(path);
    if !visited.insert(normalized) {
        return Ok(());
    }

    let source = read_text_with_fallback(path)?;
    out.push((path.to_path_buf(), source.clone()));

    for include in extract_include_paths(&source, path, mod_root) {
        if include.exists() {
            collect_parse_targets(&include, mod_root, visited, out)?;
        }
    }

    Ok(())
}

fn collect_include_component_hints(
    path: &Path,
    mod_root: &Path,
    inherited_component: Option<String>,
    visited: &mut HashSet<(PathBuf, Option<String>)>,
    out: &mut HashMap<PathBuf, HashSet<String>>,
) -> Result<(), String> {
    let normalized = normalize_for_visited(path);
    let key = (normalized.clone(), inherited_component.clone());
    if !visited.insert(key) {
        return Ok(());
    }

    let source = read_text_with_fallback(path)?;
    let mut current_component = inherited_component;
    let mut begin_index = 0usize;

    for raw_line in source.lines() {
        let code = strip_inline_comment(raw_line).trim();
        if code.is_empty() {
            continue;
        }

        if let Some(component) = parse_begin_component_hint(code, begin_index) {
            current_component = Some(component);
            begin_index = begin_index.saturating_add(1);
        }

        if !code.to_ascii_uppercase().starts_with("INCLUDE") {
            continue;
        }

        let Some(raw_include) = extract_include_raw_path(code) else {
            continue;
        };
        let Some(include_path) = resolve_include_path(&raw_include, path, mod_root) else {
            continue;
        };

        let include_normalized = normalize_for_visited(&include_path);
        if let Some(component) = current_component.clone() {
            out.entry(include_normalized.clone())
                .or_default()
                .insert(component);
        }

        if include_path.exists() {
            collect_include_component_hints(
                &include_path,
                mod_root,
                current_component.clone(),
                visited,
                out,
            )?;
        }
    }

    Ok(())
}

fn parse_begin_component_hint(code: &str, default_index: usize) -> Option<String> {
    let upper = code.to_ascii_uppercase();
    if !upper.starts_with("BEGIN ") && upper != "BEGIN" {
        return None;
    }
    if let Some(designated) = extract_designated_component_id_from_line(code) {
        return Some(designated);
    }
    Some(format!("@{default_index}"))
}

fn extract_designated_component_id_from_line(code: &str) -> Option<String> {
    let upper = code.to_ascii_uppercase();
    let pos = upper.find("DESIGNATED")?;
    let after = &code[pos + "DESIGNATED".len()..];
    let digits: String = after
        .chars()
        .skip_while(|c| c.is_whitespace())
        .take_while(|c| c.is_ascii_digit())
        .collect();
    canonical_component_id_from_digits(&digits)
}

fn normalize_for_visited(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

fn extract_include_paths(source: &str, current_file: &Path, mod_root: &Path) -> Vec<PathBuf> {
    let mut includes = Vec::new();

    for line in source.lines() {
        let trimmed = line.trim();
        if !trimmed.to_ascii_uppercase().starts_with("INCLUDE") {
            continue;
        }

        let Some(raw) = extract_include_raw_path(trimmed) else {
            continue;
        };

        if let Some(path) = resolve_include_path(&raw, current_file, mod_root) {
            includes.push(path);
        }
    }

    includes
}

fn extract_include_raw_path(line: &str) -> Option<String> {
    let rest = line
        .trim_start()
        .strip_prefix("INCLUDE")
        .or_else(|| line.trim_start().strip_prefix("include"))?
        .trim_start();
    if let Some(after) = rest.strip_prefix('~') {
        let end = after.find('~')?;
        return Some(after[..end].to_string());
    }
    if let Some(after) = rest.strip_prefix('"') {
        let end = after.find('"')?;
        return Some(after[..end].to_string());
    }
    None
}

fn resolve_include_path(raw: &str, current_file: &Path, mod_root: &Path) -> Option<PathBuf> {
    let mod_name = mod_root.file_name().and_then(OsStr::to_str).unwrap_or_default();
    let mut value = raw.replace('\\', "/");
    value = value.replace("%MOD_FOLDER%", mod_name);
    value = value.replace("%mod_folder%", mod_name);

    let include = PathBuf::from(value);
    if include.is_absolute() {
        return Some(include);
    }

    let current_dir = current_file.parent().unwrap_or(Path::new("."));
    let mut candidates: Vec<PathBuf> = vec![
        mod_root.join(&include),
        current_dir.join(&include),
    ];

    if let Some(parent_of_parent) = current_dir.parent() {
        candidates.push(parent_of_parent.join(&include));
    }

    let include_starts_with_mod_folder = include
        .components()
        .next()
        .and_then(|c| c.as_os_str().to_str())
        .map(|first| first.eq_ignore_ascii_case(mod_name))
        .unwrap_or(false);
    if include_starts_with_mod_folder {
        if let Some(mod_parent) = mod_root.parent() {
            candidates.push(mod_parent.join(&include));
        }
    }

    let mod_prefix = format!("{mod_name}/");
    if let Some(stripped) = include
        .to_string_lossy()
        .strip_prefix(&mod_prefix)
        .map(PathBuf::from)
    {
        candidates.push(mod_root.join(stripped));
    }

    for candidate in &candidates {
        if candidate.exists() {
            return Some(candidate.clone());
        }
    }

    candidates.into_iter().next()
}

fn derive_readln_prompt(
    source_lines: &[&str],
    tra_map: &HashMap<String, String>,
    var_name: &str,
    search_from_line: &mut usize,
) -> (String, Vec<PromptOption>, Option<u32>) {
    let action_idx = find_readln_line_index(source_lines, var_name, *search_from_line)
        .or_else(|| find_readln_line_index(source_lines, var_name, 0));

    let Some(action_idx) = action_idx else {
        return (String::new(), Vec::new(), None);
    };
    *search_from_line = action_idx.saturating_add(1);

    let anchor = find_outer_sprint_for_var(source_lines, var_name, action_idx).unwrap_or(action_idx);
    let start = anchor.saturating_sub(20);
    let print_context = collect_previous_print_texts_in_range(
        source_lines,
        tra_map,
        start,
        action_idx,
    );
    let (mut options, options_idx) = select_options_from_prints(&print_context);
    let mut question = select_question_text(&print_context, options_idx);
    if let Some(q) = question.clone() {
        question = Some(resolve_placeholder_question(
            source_lines,
            tra_map,
            action_idx,
            &q,
            0,
        ));
    }

    if options.is_empty() {
        options = infer_options_from_action_if(source_lines, tra_map, action_idx, var_name, 50);
    }
    if options.is_empty() {
        if let Some(q) = question.as_deref() {
            options = infer_bracket_letter_options(q);
        }
    }
    if options.is_empty() {
        options = infer_yes_no_options(source_lines, action_idx, var_name, question.as_deref());
    }

    (question.unwrap_or_default(), options, Some((action_idx + 1) as u32))
}

fn find_outer_sprint_for_var(source_lines: &[&str], var_name: &str, before_idx: usize) -> Option<usize> {
    if var_name.trim().is_empty() {
        return None;
    }
    let needle = var_name.to_ascii_lowercase();
    for i in (0..before_idx).rev() {
        let code = strip_inline_comment(source_lines[i]).trim().to_ascii_lowercase();
        if code.starts_with("outer_sprint") && code.contains(&needle) {
            return Some(i);
        }
    }
    None
}

fn find_readln_line_index(source_lines: &[&str], var_name: &str, start: usize) -> Option<usize> {
    for (i, line) in source_lines.iter().enumerate().skip(start) {
        let upper = line.to_ascii_uppercase();
        if !upper.contains("ACTION_READLN") {
            continue;
        }
        if var_name.is_empty() {
            return Some(i);
        }
        if line.contains(var_name) || line.contains(&format!("~{}~", var_name)) {
            return Some(i);
        }
        if line
            .to_ascii_lowercase()
            .contains(&var_name.to_ascii_lowercase())
        {
            return Some(i);
        }
    }
    None
}

fn collect_previous_print_texts(
    source_lines: &[&str],
    tra_map: &HashMap<String, String>,
    action_idx: usize,
    max_lines_back: usize,
) -> Vec<String> {
    let start = action_idx.saturating_sub(max_lines_back);
    collect_previous_print_texts_in_range(source_lines, tra_map, start, action_idx)
}

fn collect_previous_print_texts_in_range(
    source_lines: &[&str],
    tra_map: &HashMap<String, String>,
    start: usize,
    action_idx: usize,
) -> Vec<String> {
    let mut out = Vec::new();
    let mut i = start;
    while i < action_idx {
        let trimmed = strip_inline_comment(source_lines[i]).trim();
        if !trimmed.to_ascii_uppercase().starts_with("PRINT ") {
            i += 1;
            continue;
        }
        if let Some((text, consumed)) = parse_print_text_block(source_lines, i, action_idx, tra_map) {
            if !text.trim().is_empty() {
                out.push(text);
            }
            i += consumed.max(1);
        } else {
            i += 1;
        }
    }
    out
}

fn strip_inline_comment(line: &str) -> &str {
    line.split("//").next().unwrap_or(line)
}

fn extract_inline_comment(line: &str) -> Option<String> {
    let (_, comment) = line.split_once("//")?;
    let cleaned = comment.trim();
    if cleaned.is_empty() {
        None
    } else {
        Some(cleaned.to_string())
    }
}

fn parse_print_atom(trimmed_line: &str) -> Option<String> {
    let rest = trimmed_line
        .trim_start()
        .strip_prefix("PRINT")
        .or_else(|| trimmed_line.trim_start().strip_prefix("print"))?
        .trim_start();

    if let Some(after) = rest.strip_prefix('@') {
        let digits: String = after.chars().take_while(|c| c.is_ascii_digit()).collect();
        if digits.is_empty() {
            return None;
        }
        return Some(format!("@{digits}"));
    }

    if let Some(after) = rest.strip_prefix('~') {
        if let Some(end) = after.find('~') {
            return Some(after[..end].to_string());
        }
    }
    if let Some(after) = rest.strip_prefix('"') {
        if let Some(end) = after.find('"') {
            return Some(after[..end].to_string());
        }
    }

    rest.split_whitespace().next().map(|s| s.to_string())
}

fn parse_print_text_block(
    source_lines: &[&str],
    idx: usize,
    stop_before: usize,
    tra_map: &HashMap<String, String>,
) -> Option<(String, usize)> {
    let raw_line = source_lines.get(idx)?;
    let line = strip_inline_comment(raw_line).trim();
    let inline_comment = extract_inline_comment(raw_line);
    let rest = line
        .trim_start()
        .strip_prefix("PRINT")
        .or_else(|| line.trim_start().strip_prefix("print"))?
        .trim_start();

    if rest.starts_with('@') {
        let atom = parse_print_atom(line)?;
        let resolved = resolve_atom_text(&atom, tra_map);
        let text = fallback_comment_if_placeholder(&sanitize_wrapped_text(&resolved), inline_comment.as_deref());
        return Some((text, 1));
    }

    let Some(delim) = rest.chars().next().filter(|c| *c == '~' || *c == '"') else {
        let atom = parse_print_atom(line)?;
        let resolved = resolve_atom_text(&atom, tra_map);
        let text = fallback_comment_if_placeholder(&sanitize_wrapped_text(&resolved), inline_comment.as_deref());
        return Some((text, 1));
    };

    let mut text = String::new();
    let mut consumed = 1usize;
    let mut current = rest[1..].to_string();

    loop {
        if let Some(end) = current.find(delim) {
            text.push_str(&current[..end]);
            break;
        }
        text.push_str(&current);
        let next_idx = idx + consumed;
        if next_idx >= stop_before || next_idx >= source_lines.len() {
            break;
        }
        text.push('\n');
        current = strip_inline_comment(source_lines[next_idx]).trim().to_string();
        consumed += 1;
    }

    let final_text = fallback_comment_if_placeholder(text.trim(), inline_comment.as_deref());
    Some((final_text, consumed))
}

fn fallback_comment_if_placeholder(text: &str, comment: Option<&str>) -> String {
    let cleaned = text.trim();
    let unresolved = (cleaned.starts_with('%') && cleaned.ends_with('%')) || cleaned.contains('%');
    if unresolved {
        if let Some(c) = comment {
            let c = c.trim();
            if !c.is_empty() {
                return c.to_string();
            }
        }
    }
    cleaned.to_string()
}

fn resolve_atom_text(atom: &str, tra_map: &HashMap<String, String>) -> String {
    let token = sanitize_wrapped_text(atom);
    if token.starts_with('@') {
        return tra_map
            .get(&token.to_ascii_lowercase())
            .cloned()
            .unwrap_or(token);
    }
    token
}

fn sanitize_wrapped_text(value: &str) -> String {
    let mut out = value.trim().to_string();
    if out.starts_with('~') && out.ends_with('~') && out.len() >= 2 {
        out = out[1..out.len() - 1].to_string();
    }
    if out.starts_with('"') && out.ends_with('"') && out.len() >= 2 {
        out = out[1..out.len() - 1].to_string();
    }
    out.trim().to_string()
}

fn select_question_text(texts: &[String], options_idx: Option<usize>) -> Option<String> {
    if let Some(idx) = options_idx {
        if let Some(from_same_block) = extract_question_from_options_block(&texts[idx]) {
            if !is_generic_question(&from_same_block) {
                return Some(from_same_block);
            }
        }
        for t in texts[..idx].iter().rev() {
            if is_question_candidate(t) {
                return Some(t.clone());
            }
        }
    }

    for t in texts.iter().rev() {
        if is_question_candidate(t) {
            return Some(t.clone());
        }
    }

    None
}

fn resolve_placeholder_question(
    source_lines: &[&str],
    tra_map: &HashMap<String, String>,
    action_idx: usize,
    question: &str,
    depth: usize,
) -> String {
    if depth >= 4 {
        return question.to_string();
    }
    let Some(var_name) = extract_percent_placeholder(question) else {
        return question.to_string();
    };
    let Some(raw_assigned) = find_latest_outer_assignment(source_lines, action_idx, &var_name) else {
        return question.to_string();
    };
    let resolved = resolve_atom_text(&raw_assigned, tra_map);
    if resolved.trim().is_empty() || resolved == question {
        return question.to_string();
    }
    resolve_placeholder_question(source_lines, tra_map, action_idx, &resolved, depth + 1)
}

fn extract_percent_placeholder(text: &str) -> Option<String> {
    let t = text.trim();
    if !(t.starts_with('%') && t.ends_with('%') && t.len() >= 3) {
        return None;
    }
    let inner = &t[1..t.len() - 1];
    if inner.is_empty() || inner.chars().any(|c| c.is_whitespace()) {
        return None;
    }
    Some(inner.to_ascii_lowercase())
}

fn find_latest_outer_assignment(source_lines: &[&str], before_idx: usize, var_name: &str) -> Option<String> {
    for i in (0..before_idx).rev() {
        let line = strip_inline_comment(source_lines[i]).trim();
        if let Some((lhs, rhs)) = parse_outer_assignment(line) {
            if lhs == var_name {
                return Some(rhs);
            }
        }
        if let Some((lhs, rhs)) = parse_plain_assignment(line) {
            if lhs == var_name {
                return Some(rhs);
            }
        }
    }
    None
}

fn parse_outer_assignment(line: &str) -> Option<(String, String)> {
    let upper = line.to_ascii_uppercase();
    for cmd in ["OUTER_SPRINT", "OUTER_TEXT_SPRINT", "OUTER_SNPRINT", "OUTER_SET"] {
        if !upper.starts_with(cmd) {
            continue;
        }
        let rest = line[cmd.len()..].trim_start();
        let (lhs_raw, after_lhs) = take_first_token(rest)?;
        let lhs = normalize_var_name(&lhs_raw);
        if lhs.is_empty() {
            return None;
        }
        let expr = if cmd == "OUTER_SET" {
            let (_, rhs) = after_lhs.split_once('=')?;
            rhs.trim_start()
        } else {
            after_lhs.trim_start()
        };
        let rhs = extract_leading_atom(expr)?;
        return Some((lhs, rhs));
    }
    None
}

fn parse_plain_assignment(line: &str) -> Option<(String, String)> {
    let (lhs_raw, rhs_raw) = line.split_once('=')?;
    let lhs = normalize_var_name(lhs_raw);
    if lhs.is_empty() {
        return None;
    }
    let rhs = extract_leading_atom(rhs_raw.trim_start())?;
    Some((lhs, rhs))
}

fn take_first_token(input: &str) -> Option<(String, &str)> {
    let s = input.trim_start();
    if s.is_empty() {
        return None;
    }
    if let Some(after) = s.strip_prefix('~') {
        let end = after.find('~')?;
        let token = after[..end].to_string();
        let rest = &after[end + 1..];
        return Some((token, rest));
    }
    if let Some(after) = s.strip_prefix('"') {
        let end = after.find('"')?;
        let token = after[..end].to_string();
        let rest = &after[end + 1..];
        return Some((token, rest));
    }
    let end = s.find(char::is_whitespace).unwrap_or(s.len());
    let token = s[..end].to_string();
    let rest = &s[end..];
    Some((token, rest))
}

fn extract_leading_atom(input: &str) -> Option<String> {
    let s = input.trim_start();
    if s.is_empty() {
        return None;
    }
    if let Some(after) = s.strip_prefix('@') {
        let digits: String = after.chars().take_while(|c| c.is_ascii_digit()).collect();
        if !digits.is_empty() {
            return Some(format!("@{digits}"));
        }
    }
    if let Some(after) = s.strip_prefix('~') {
        let end = after.find('~')?;
        return Some(format!("~{}~", &after[..end]));
    }
    if let Some(after) = s.strip_prefix('"') {
        let end = after.find('"')?;
        return Some(format!("\"{}\"", &after[..end]));
    }
    if let Some(after) = s.strip_prefix('%') {
        let end = after.find('%')?;
        return Some(format!("%{}%", &after[..end]));
    }
    s.split_whitespace().next().map(|v| v.to_string())
}

fn normalize_var_name(raw: &str) -> String {
    sanitize_wrapped_text(raw)
        .trim_matches('%')
        .trim()
        .to_ascii_lowercase()
}

fn extract_question_from_options_block(text: &str) -> Option<String> {
    let mut question_lines: Vec<&str> = Vec::new();
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() {
            continue;
        }
        if parse_numbered_option_line(line).is_some() {
            break;
        }
        question_lines.push(line);
    }
    if question_lines.is_empty() {
        None
    } else {
        Some(question_lines.join(" ").trim().to_string())
    }
}

fn is_question_candidate(t: &str) -> bool {
    let lower = t.to_ascii_lowercase();
    if lower.contains("[1]") {
        return false;
    }
    if lower.contains("please select") {
        return false;
    }
    if lower.starts_with("portrait:") || lower.starts_with("installed ") || lower.starts_with("did not install") {
        return false;
    }
    !t.trim().is_empty()
}

fn is_generic_question(t: &str) -> bool {
    let s = t.trim().to_ascii_lowercase();
    s == "please choose one of the following:"
        || s == "please choose one of the following"
        || s == "choose one of the following:"
        || s == "choose one of the following"
}

fn select_options_from_prints(texts: &[String]) -> (Vec<PromptOption>, Option<usize>) {
    for (idx, t) in texts.iter().enumerate() {
        let parsed = parse_numbered_options_block(t);
        if !parsed.is_empty() {
            return (parsed, Some(idx));
        }
    }
    (Vec::new(), None)
}

fn parse_numbered_options_block(text: &str) -> Vec<PromptOption> {
    let mut out = Vec::new();
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() {
            continue;
        }

        if let Some((value, label)) = parse_numbered_option_line(line) {
            out.push(PromptOption {
                label,
                value,
                component_ids: Vec::new(),
            });
        }
    }
    out
}

fn parse_numbered_option_line(line: &str) -> Option<(String, String)> {
    // [1] Text
    if let Some(rest) = line.strip_prefix('[') {
        let end = rest.find(']')?;
        let number = rest[..end].trim();
        if !number.chars().all(|c| c.is_ascii_digit()) {
            return None;
        }
        let label = rest[end + 1..].trim().trim_start_matches('-').trim();
        if label.is_empty() {
            return None;
        }
        return Some((number.to_string(), label.to_string()));
    }

    // 1 = Text | 1 - Text | 1) Text
    let digits: String = line.chars().take_while(|c| c.is_ascii_digit()).collect();
    if digits.is_empty() {
        return None;
    }
    let rest = line[digits.len()..].trim_start();
    let rest = if let Some(r) = rest.strip_prefix('=') {
        r
    } else if let Some(r) = rest.strip_prefix('-') {
        r
    } else if let Some(r) = rest.strip_prefix(')') {
        r
    } else {
        return None;
    };
    let label = rest.trim();
    if label.is_empty() {
        return None;
    }
    Some((digits, label.to_string()))
}

fn infer_options_from_action_if(
    source_lines: &[&str],
    tra_map: &HashMap<String, String>,
    action_idx: usize,
    var_name: &str,
    max_lines_forward: usize,
) -> Vec<PromptOption> {
    if var_name.is_empty() {
        return Vec::new();
    }
    let needle = var_name.to_ascii_lowercase();
    let mut out = Vec::new();
    let end = (action_idx + max_lines_forward).min(source_lines.len());

    for i in action_idx..end {
        let line = strip_inline_comment(source_lines[i]).trim();
        let upper = line.to_ascii_uppercase();
        if !upper.starts_with("ACTION_IF") {
            continue;
        }
        if !line.to_ascii_lowercase().contains(&needle) {
            continue;
        }
        let Some(value) = extract_if_equals_number(line) else {
            continue;
        };

        let mut label = format!("Option {value}");
        for look_ahead in i + 1..(i + 8).min(end) {
            let candidate = strip_inline_comment(source_lines[look_ahead]).trim();
            if candidate.to_ascii_uppercase().starts_with("PRINT ") {
                if let Some(atom) = parse_print_atom(candidate) {
                    let resolved = resolve_atom_text(&atom, tra_map);
                    if !resolved.trim().is_empty() {
                        label = resolved.trim().to_string();
                        break;
                    }
                }
            }
        }

        if out.iter().any(|o: &PromptOption| o.value == value) {
            continue;
        }
        out.push(PromptOption {
            label,
            value,
            component_ids: Vec::new(),
        });
    }

    dedupe_prompt_options(&mut out);
    out
}

fn infer_yes_no_options(
    source_lines: &[&str],
    action_idx: usize,
    var_name: &str,
    question: Option<&str>,
) -> Vec<PromptOption> {
    let from_text = question
        .map(looks_like_yes_no_prompt)
        .unwrap_or(false);
    let from_conditions = var_name_supports_yes_no(source_lines, action_idx, var_name);
    if !(from_text || from_conditions) {
        return Vec::new();
    }
    vec![
        PromptOption {
            label: "Yes".to_string(),
            value: "y".to_string(),
            component_ids: Vec::new(),
        },
        PromptOption {
            label: "No".to_string(),
            value: "n".to_string(),
            component_ids: Vec::new(),
        },
    ]
}

fn infer_bracket_letter_options(question: &str) -> Vec<PromptOption> {
    let mut out: Vec<PromptOption> = Vec::new();
    let bytes = question.as_bytes();
    let mut i = 0usize;

    while i + 2 < bytes.len() {
        if bytes[i] != b'[' {
            i += 1;
            continue;
        }
        let key = bytes[i + 1] as char;
        if !key.is_ascii_alphabetic() || bytes[i + 2] != b']' {
            i += 1;
            continue;
        }

        let mut j = i + 3;
        while j < bytes.len() && (bytes[j] == b' ' || bytes[j] == b'\t') {
            j += 1;
        }

        let mut tail = String::new();
        while j < bytes.len() {
            let c = bytes[j] as char;
            if c == ',' || c == ')' || c == '?' || c == ';' || c == ':' {
                break;
            }
            if !(c.is_ascii_alphabetic() || c == '-' || c == ' ') {
                break;
            }
            tail.push(c);
            j += 1;
        }

        let mut label = String::new();
        label.push(key.to_ascii_uppercase());
        label.push_str(tail.trim());
        let mut label = label.trim().to_string();
        if label.to_ascii_lowercase().ends_with(" or") {
            label = label[..label.len() - 3].trim_end().to_string();
        }
        if !label.is_empty() {
            out.push(PromptOption {
                label,
                value: key.to_ascii_lowercase().to_string(),
                component_ids: Vec::new(),
            });
        }

        i += 3;
    }

    dedupe_prompt_options(&mut out);
    out
}

fn looks_like_yes_no_prompt(question: &str) -> bool {
    let q = question.to_ascii_lowercase();
    (q.contains("yes") && q.contains("no"))
        || q.contains("[y]es")
        || q.contains("[n]o")
        || q.contains(" y/n")
        || q.contains("y/n ")
        || q.contains("answer y")
}

fn var_name_supports_yes_no(source_lines: &[&str], action_idx: usize, var_name: &str) -> bool {
    if var_name.trim().is_empty() {
        return false;
    }
    let needle = var_name.to_ascii_lowercase();
    let start = action_idx.saturating_sub(80);
    let end = (action_idx + 120).min(source_lines.len());
    let mut has_y = false;
    let mut has_n = false;

    for line in source_lines[start..end].iter() {
        let l = strip_inline_comment(line).to_ascii_lowercase();
        if !l.contains(&needle) {
            continue;
        }
        if l.contains("string_equal_case y")
            || l.contains("string_equal_case ~y~")
            || l.contains("string_equal_case \"y\"")
            || l.contains("string_equal_case 'y'")
        {
            has_y = true;
        }
        if l.contains("string_equal_case n")
            || l.contains("string_equal_case ~n~")
            || l.contains("string_equal_case \"n\"")
            || l.contains("string_equal_case 'n'")
        {
            has_n = true;
        }
        if has_y && has_n {
            return true;
        }
    }
    false
}

fn dedupe_prompt_options(options: &mut Vec<PromptOption>) {
    let mut seen = HashSet::new();
    options.retain(|o| {
        let value = o.value.trim();
        let label = o.label.trim();
        if value.is_empty() || label.is_empty() {
            return false;
        }
        let key = format!("{}::{}", value.to_ascii_lowercase(), label.to_ascii_lowercase());
        seen.insert(key)
    });
}

fn is_internal_match_prompt(text: &str, options: &[PromptOption]) -> bool {
    let t = text.trim();
    if t.is_empty() {
        return false;
    }
    let lower = t.to_ascii_lowercase();
    if lower.starts_with("skipping ") {
        return true;
    }

    let looks_placeholder = t.starts_with('%') && t.ends_with('%') && t.len() >= 3;
    if !looks_placeholder {
        return false;
    }

    if options.len() < 10 {
        return false;
    }

    // Heuristic: helper-menu internals often expose a wide set of 1-char keys (A-Z/0-9).
    options.iter().all(|o| {
        let v = o.value.trim();
        !v.is_empty() && v.chars().count() == 1 && v.chars().all(|c| c.is_ascii_alphanumeric())
    })
}

fn extract_if_equals_number(line: &str) -> Option<String> {
    let eq_pos = line.find('=')?;
    let rest = line[eq_pos + 1..].trim_start();
    let number: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
    if number.is_empty() {
        None
    } else {
        Some(number)
    }
}

fn load_tra_map_for_source(source_file: &str, preferred_lang: Option<&str>) -> TraLookupResult {
    let path = PathBuf::from(source_file);
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    if ext != "tp2" {
        return TraLookupResult::default();
    }
    let Some(mod_root) = path.parent() else {
        return TraLookupResult::default();
    };

    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or_default()
        .to_string();
    let stem = stem.strip_prefix("setup-").unwrap_or(&stem).to_string();
    let stem_lower = stem.to_ascii_lowercase();

    let mut candidate_dirs: Vec<(PathBuf, String)> = Vec::new();
    for lang in preferred_lang_candidates(preferred_lang) {
        candidate_dirs.extend(language_dirs_for_mod_root(mod_root, &lang));
    }
    // Always keep English as fallback.
    if !preferred_lang_candidates(preferred_lang)
        .iter()
        .any(|l| l.eq_ignore_ascii_case("english"))
    {
        candidate_dirs.extend(language_dirs_for_mod_root(mod_root, "english"));
    }

    for (dir, lang_label) in &candidate_dirs {
        if !dir.exists() {
            continue;
        }
        if let Ok(entries) = std::fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.extension().and_then(|e| e.to_str()).map(|e| e.eq_ignore_ascii_case("tra"))
                    != Some(true)
                {
                    continue;
                }
                let file_stem = p
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or_default()
                    .to_ascii_lowercase();
                if file_stem == stem_lower {
                    if let Ok(map) = parse_tra_file(&p) {
                        return TraLookupResult {
                            map,
                            language_used: Some(lang_label.clone()),
                        };
                    }
                }
            }
        }
    }

    // Fallback for mods that keep setup strings in non-stem files (for example setup.tra).
    for (dir, lang_label) in &candidate_dirs {
        if let Ok(map) = load_all_tra_from_dir(dir) {
            if !map.is_empty() {
                return TraLookupResult {
                    map,
                    language_used: Some(lang_label.clone()),
                };
            }
        }
    }

    TraLookupResult::default()
}

fn language_dirs_for_mod_root(mod_root: &Path, lang: &str) -> Vec<(PathBuf, String)> {
    vec![
        (mod_root.join("Tra").join(lang), lang.to_string()),
        (mod_root.join("tra").join(lang), lang.to_string()),
        (mod_root.join("Translations").join(lang), lang.to_string()),
        (mod_root.join("translations").join(lang), lang.to_string()),
        (mod_root.join("languages").join(lang), lang.to_string()),
        (mod_root.join("lang").join(lang), lang.to_string()),
        (mod_root.join("language").join(lang), lang.to_string()),
    ]
}

fn preferred_lang_candidates(preferred_lang: Option<&str>) -> Vec<String> {
    let mut out = Vec::new();
    if let Some(raw) = preferred_lang {
        let norm = raw.trim().to_ascii_lowercase();
        if !norm.is_empty() {
            push_unique(&mut out, norm.clone());
            push_unique(&mut out, norm.replace('-', "_"));
            if let Some(short) = norm.split(['_', '-']).next() {
                push_unique(&mut out, short.to_string());
            }
            match norm.as_str() {
                "en" | "en_us" | "en-gb" | "en_gb" => push_unique(&mut out, "english".to_string()),
                "pl" | "pl_pl" => push_unique(&mut out, "polish".to_string()),
                "de" | "de_de" => push_unique(&mut out, "german".to_string()),
                "fr" | "fr_fr" => push_unique(&mut out, "french".to_string()),
                "it" | "it_it" => push_unique(&mut out, "italian".to_string()),
                "es" | "es_es" => push_unique(&mut out, "spanish".to_string()),
                _ => {}
            }
        }
    } else {
        push_unique(&mut out, "english".to_string());
    }
    out
}

fn push_unique(items: &mut Vec<String>, value: String) {
    if !value.is_empty() && !items.iter().any(|v| v.eq_ignore_ascii_case(&value)) {
        items.push(value);
    }
}

fn load_all_tra_from_dir(dir: &Path) -> Result<HashMap<String, String>, String> {
    if !dir.exists() {
        return Ok(HashMap::new());
    }
    let mut map = HashMap::new();
    let entries = std::fs::read_dir(dir)
        .map_err(|e| format!("failed to read '{}': {e}", dir.display()))?;
    for entry in entries.flatten() {
        let p = entry.path();
        if p.extension().and_then(|e| e.to_str()).map(|e| e.eq_ignore_ascii_case("tra")) != Some(true)
        {
            continue;
        }
        if let Ok(one) = parse_tra_file(&p) {
            for (k, v) in one {
                map.entry(k).or_insert(v);
            }
        }
    }
    Ok(map)
}

fn parse_tra_file(path: &Path) -> Result<HashMap<String, String>, String> {
    let text = read_text_with_fallback(path)?;
    let mut map = HashMap::new();
    let mut i = 0usize;
    let lines: Vec<&str> = text.lines().collect();
    while i < lines.len() {
        let line = lines[i].trim();
        i += 1;
        if !line.starts_with('@') {
            continue;
        }
        let Some(eq_idx) = line.find('=') else {
            continue;
        };
        let key = line[..eq_idx].trim().to_ascii_lowercase();
        let rhs = line[eq_idx + 1..].trim_start();

        if let Some(after) = rhs.strip_prefix('~') {
            let mut value = String::new();
            if let Some(end) = after.find('~') {
                value.push_str(&after[..end]);
            } else {
                value.push_str(after);
                value.push('\n');
                while i < lines.len() {
                    let next = lines[i];
                    i += 1;
                    if let Some(end) = next.find('~') {
                        value.push_str(&next[..end]);
                        break;
                    }
                    value.push_str(next);
                    value.push('\n');
                }
            }
            map.insert(key, value.trim().to_string());
        } else if let Some(after) = rhs.strip_prefix('"') {
            let value = if let Some(end) = after.find('"') {
                &after[..end]
            } else {
                after
            };
            map.insert(key, value.trim().to_string());
        }
    }
    Ok(map)
}

fn read_text_with_fallback(path: &Path) -> Result<String, String> {
    let bytes = std::fs::read(path)
        .map_err(|e| format!("failed to read '{}': {e}", path.display()))?;

    if let Ok(text) = String::from_utf8(bytes.clone()) {
        return Ok(text);
    }

    for encoding in [WINDOWS_1252, WINDOWS_1250] {
        let (decoded, _, had_errors) = encoding.decode(&bytes);
        if !had_errors {
            return Ok(decoded.into_owned());
        }
    }

    let (decoded, _, _) = Encoding::for_label(b"iso-8859-1")
        .unwrap_or(WINDOWS_1252)
        .decode(&bytes);
    Ok(decoded.into_owned())
}

fn extract_readln_var(action_text: &str) -> String {
    action_text
        .strip_prefix("ACTION_READLN")
        .map(|s| s.trim().to_string())
        .unwrap_or_default()
}
