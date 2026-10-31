// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(in crate::app) enum FamilyIdentity {
    Literal(String),
    Translation(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::app) enum PredicateFlagKind {
    Requirement,
    Option,
}

pub(in crate::app) struct PredicateFlag {
    pub(crate) kind: PredicateFlagKind,
    pub(crate) expression: String,
    pub(crate) prefix: String,
    pub(crate) line: usize,
}

pub(in crate::app) struct ForbidFlag {
    pub(crate) target_reference: String,
    pub(crate) component_reference: String,
    pub(crate) message_token: String,
    pub(crate) raw_line: String,
    pub(crate) line: usize,
}

pub(in crate::app) struct ComponentFlags {
    pub(crate) component_id: String,
    pub(crate) family: Option<FamilyIdentity>,
    pub(crate) predicates: Vec<PredicateFlag>,
    pub(crate) forbids: Vec<ForbidFlag>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum TokenKind {
    Word,
    Literal,
    Translation,
    Variable,
    Symbol,
    Invalid,
}

struct SourceToken<'a> {
    text: &'a str,
    kind: TokenKind,
    line: usize,
    leading: bool,
    start: usize,
    end: usize,
}

impl SourceToken<'_> {
    fn is_word(&self, word: &str) -> bool {
        self.kind == TokenKind::Word && self.text.eq_ignore_ascii_case(word)
    }

    const fn is_label(&self) -> bool {
        matches!(
            self.kind,
            TokenKind::Literal | TokenKind::Translation | TokenKind::Variable
        )
    }
}

pub(in crate::app) fn collect_component_flags(text: &str) -> Vec<ComponentFlags> {
    let tokens = source_tokens(text);
    let starts: Vec<usize> = tokens
        .iter()
        .enumerate()
        .filter_map(|(index, token)| {
            (token.leading
                && token.is_word("BEGIN")
                && tokens.get(index + 1).is_some_and(SourceToken::is_label))
            .then_some(index)
        })
        .collect();
    let mut next_number = 0u64;
    starts
        .iter()
        .enumerate()
        .map(|(index, &start)| {
            let end = starts.get(index + 1).copied().unwrap_or(tokens.len());
            let mut component = parse_header(&tokens[start + 2..end], text);
            if component.component_id.is_empty() {
                component.component_id = next_number.to_string();
            }
            next_number = component
                .component_id
                .parse::<u64>()
                .map_or(next_number, |number| number.saturating_add(1));
            component
        })
        .collect()
}

fn parse_header(tokens: &[SourceToken<'_>], text: &str) -> ComponentFlags {
    let mut component = ComponentFlags {
        component_id: String::new(),
        family: None,
        predicates: Vec::new(),
        forbids: Vec::new(),
    };
    let mut cursor = 0;
    while let Some(token) = tokens.get(cursor) {
        let upper = token.text.to_ascii_uppercase();
        if token.kind != TokenKind::Word {
            break;
        }
        match upper.as_str() {
            "LABEL" => {
                let Some(end) = label_end(tokens, cursor + 1) else {
                    break;
                };
                cursor = end;
            }
            "FORBID_COMPONENT" => {
                let Some(flag) = forbid_flag(tokens, cursor, text) else {
                    break;
                };
                component.forbids.push(flag);
                cursor += 4;
            }
            "DESIGNATED" => {
                let Some(number) = tokens.get(cursor + 1).filter(|number| {
                    !number.text.is_empty() && number.text.bytes().all(|byte| byte.is_ascii_digit())
                }) else {
                    break;
                };
                component.component_id = normalized_number(number.text);
                cursor += 2;
            }
            "SUBCOMPONENT" | "FORCED_SUBCOMPONENT" | "GROUP" => {
                let Some(label) = tokens.get(cursor + 1).filter(|label| label.is_label()) else {
                    break;
                };
                let option = upper != "GROUP";
                let start = cursor + 2;
                let (end, complete) = expression_end(tokens, start);
                if option {
                    component.family = complete.then(|| family_identity(label)).flatten();
                }
                if option && complete && end > start {
                    component.predicates.push(PredicateFlag {
                        kind: PredicateFlagKind::Option,
                        expression: token_text(&tokens[start..end]),
                        prefix: format!("{} {}", token.text, label.text),
                        line: token.line,
                    });
                }
                if !complete {
                    break;
                }
                cursor = end;
            }
            "REQUIRE_PREDICATE" => {
                let start = cursor + 1;
                let (end, complete) = expression_end(tokens, start);
                let expression_end = without_message(tokens, start, end);
                if complete && expression_end > start {
                    component.predicates.push(PredicateFlag {
                        kind: PredicateFlagKind::Requirement,
                        expression: token_text(&tokens[start..expression_end]),
                        prefix: "REQUIRE_PREDICATE".to_string(),
                        line: token.line,
                    });
                }
                if !complete {
                    break;
                }
                cursor = end;
            }
            _ => {
                let Some(arguments) = ordinary_flag_arguments(&upper) else {
                    break;
                };
                let Some(arguments_end) = cursor.checked_add(arguments + 1) else {
                    break;
                };
                if arguments_end > tokens.len()
                    || tokens[cursor + 1..arguments_end]
                        .iter()
                        .any(|argument| argument.kind == TokenKind::Invalid || is_flag(argument))
                {
                    break;
                }
                cursor = arguments_end;
            }
        }
    }
    component
}

fn forbid_flag(tokens: &[SourceToken<'_>], cursor: usize, text: &str) -> Option<ForbidFlag> {
    let arguments = tokens.get(cursor + 1..cursor + 4)?;
    if arguments.iter().any(|argument| {
        argument.kind == TokenKind::Invalid || is_flag(argument) || is_action(argument)
    }) || !matches!(
        arguments[0].kind,
        TokenKind::Literal | TokenKind::Variable | TokenKind::Word
    ) || !matches!(arguments[1].kind, TokenKind::Word | TokenKind::Literal)
        || !matches!(
            arguments[2].kind,
            TokenKind::Literal | TokenKind::Translation
        )
    {
        return None;
    }
    Some(ForbidFlag {
        target_reference: arguments[0].text.to_string(),
        component_reference: arguments[1].text.to_string(),
        message_token: arguments[2].text.to_string(),
        raw_line: text[tokens[cursor].start..arguments[2].end].to_string(),
        line: tokens[cursor].line,
    })
}

fn label_end(tokens: &[SourceToken<'_>], start: usize) -> Option<usize> {
    let first = tokens.get(start)?;
    if is_flag(first) || is_action(first) {
        return None;
    }
    if first.kind == TokenKind::Literal {
        return Some(start + 1);
    }
    if first.kind != TokenKind::Word {
        return None;
    }
    let mut end = start + 1;
    while let Some(next) = tokens.get(end) {
        if tokens[end - 1].end != next.start {
            break;
        }
        if next.kind != TokenKind::Word && next.text != "-" {
            return None;
        }
        end += 1;
    }
    (tokens[end - 1].kind == TokenKind::Word).then_some(end)
}

fn family_identity(token: &SourceToken<'_>) -> Option<FamilyIdentity> {
    if token.kind == TokenKind::Translation {
        Some(FamilyIdentity::Translation(normalized_number(
            &token.text[1..],
        )))
    } else if token.kind == TokenKind::Literal && !token.text.contains('%') {
        Some(FamilyIdentity::Literal(
            token.text[1..token.text.len() - 1].to_string(),
        ))
    } else {
        None
    }
}

fn normalized_number(text: &str) -> String {
    let normalized = text.trim_start_matches('0');
    if normalized.is_empty() {
        "0"
    } else {
        normalized
    }
    .to_string()
}

fn ordinary_flag_arguments(flag: &str) -> Option<usize> {
    match flag {
        "INSTALL_BY_DEFAULT" | "NO_LOG_RECORD" | "NOT_IN_LOG" => Some(0),
        "LABEL" | "DEPRECATED" | "METADATA" => Some(1),
        "REQUIRE_FILE" | "FORBID_FILE" => Some(2),
        "REQUIRE_COMPONENT" | "FORBID_COMPONENT" => Some(3),
        _ => None,
    }
}

fn is_flag(token: &SourceToken<'_>) -> bool {
    token.kind == TokenKind::Word
        && (matches!(
            token.text.to_ascii_uppercase().as_str(),
            "DESIGNATED" | "GROUP" | "SUBCOMPONENT" | "FORCED_SUBCOMPONENT" | "REQUIRE_PREDICATE"
        ) || ordinary_flag_arguments(&token.text.to_ascii_uppercase()).is_some())
}

fn is_action(token: &SourceToken<'_>) -> bool {
    token.kind == TokenKind::Word
        && matches!(
            token.text.to_ascii_uppercase().as_str(),
            "COPY"
                | "COPY_EXISTING"
                | "COPY_EXISTING_REGEXP"
                | "COPY_LARGE"
                | "LAF"
                | "LAM"
                | "LAUNCH_ACTION_FUNCTION"
                | "LAUNCH_ACTION_MACRO"
                | "INCLUDE"
                | "REINCLUDE"
                | "ACTION_IF"
                | "ACTION_FOR_EACH"
                | "ACTION_PHP_EACH"
                | "OUTER_SET"
                | "OUTER_SPRINT"
                | "OUTER_TEXT_SPRINT"
                | "OUTER_PATCH"
                | "OUTER_PATCH_SAVE"
                | "PRINT"
                | "FAIL"
                | "WARN"
                | "MKDIR"
                | "COMPILE"
                | "EXTEND_TOP"
                | "EXTEND_BOTTOM"
                | "APPEND"
                | "APPEND_OUTER"
                | "AT_NOW"
                | "AT_EXIT"
                | "AT_INTERACTIVE_EXIT"
                | "ADD_KIT"
                | "ADD_SPELL"
                | "ADD_ITEM_TYPE"
                | "LOAD_TRA"
                | "ACTION_READLN"
                | "RANDOM_SEED"
                | "UNINSTALL"
                | "SILENT"
        )
}

fn expression_end(tokens: &[SourceToken<'_>], start: usize) -> (usize, bool) {
    let mut depth = 0usize;
    for (index, token) in tokens.iter().enumerate().skip(start) {
        if token.kind == TokenKind::Invalid {
            return (index, false);
        }
        if depth == 0 && (is_flag(token) || is_action(token)) {
            return (index, true);
        }
        if depth == 0
            && token.leading
            && token.kind == TokenKind::Word
            && !is_expression_word(token.text)
        {
            let complete = index == start || !is_dangling_operator(&tokens[index - 1]);
            return (index, complete);
        }
        if token.text == "(" {
            depth += 1;
        } else if token.text == ")" {
            let Some(next) = depth.checked_sub(1) else {
                return (index, false);
            };
            depth = next;
        }
    }
    (tokens.len(), depth == 0)
}

fn is_dangling_operator(token: &SourceToken<'_>) -> bool {
    token.is_word("AND")
        || token.is_word("OR")
        || token.is_word("NOT")
        || matches!(token.text, "(" | "||" | "&&" | "!")
}

fn is_expression_word(text: &str) -> bool {
    matches!(
        text.to_ascii_uppercase().as_str(),
        "AND"
            | "OR"
            | "NOT"
            | "TRUE"
            | "FALSE"
            | "GAME_IS"
            | "ENGINE_IS"
            | "GAME_INCLUDES"
            | "MOD_IS_INSTALLED"
            | "FILE_EXISTS"
            | "FILE_EXISTS_IN_GAME"
            | "STRING_EQUAL"
            | "STRING_EQUAL_CASE"
            | "STRING_COMPARE"
            | "STRING_COMPARE_CASE"
            | "STRING_COMPARE_REGEXP"
            | "STRING_MATCHES_REGEXP"
            | "STRING_CONTAINS_REGEXP"
            | "STRING_LENGTH"
            | "BAND"
            | "BOR"
            | "BXOR"
            | "BNOT"
            | "BLSL"
            | "BLSR"
            | "BASR"
            | "ABS"
            | "MODULO"
            | "IS_AN_INT"
            | "IS_SILENT"
            | "VARIABLE_IS_SET"
            | "FILE_CONTAINS"
            | "FILE_CONTAINS_EVALUATED"
            | "FILE_IS_IN_COMPRESSED_BIFF"
            | "FILE_MD5"
            | "FILE_SIZE"
            | "RESOURCE_CONTAINS"
            | "DIRECTORY_EXISTS"
            | "IDS_OF_SYMBOL"
            | "STATE_WHICH_SAYS"
            | "TRA_ENTRY_EXISTS"
            | "ID_OF_LABEL"
    )
}

fn without_message(tokens: &[SourceToken<'_>], start: usize, end: usize) -> usize {
    let Some(last) = end.checked_sub(1).filter(|&last| last >= start) else {
        return end;
    };
    if tokens[last].kind == TokenKind::Translation {
        return last;
    }
    if tokens[last].kind != TokenKind::Literal || last == start {
        return end;
    }
    let candidate = token_text(&tokens[start..last]);
    if crate::app::compat_mismatch_eval::render_requirement_evidence(&candidate).is_some() {
        last
    } else {
        end
    }
}

fn token_text(tokens: &[SourceToken<'_>]) -> String {
    tokens
        .iter()
        .map(|token| token.text)
        .collect::<Vec<_>>()
        .join(" ")
}

fn source_tokens(text: &str) -> Vec<SourceToken<'_>> {
    let bytes = text.as_bytes();
    let mut tokens = Vec::new();
    let mut cursor = 0;
    let mut line = 1;
    let mut leading = true;
    let mut inline_file = false;
    while cursor < bytes.len() {
        if bytes[cursor].is_ascii_whitespace() {
            if bytes[cursor] == b'\n' {
                line += 1;
                leading = true;
            }
            cursor += 1;
            continue;
        }
        if leading && (inline_file || text[cursor..].starts_with("<<<<<<<<")) {
            inline_file = !text[cursor..].starts_with(">>>>>>>>");
            cursor += text[cursor..].find('\n').unwrap_or(bytes.len() - cursor);
            continue;
        }
        if text[cursor..].starts_with("//") {
            cursor += text[cursor..].find('\n').unwrap_or(bytes.len() - cursor);
            continue;
        }
        if text[cursor..].starts_with("/*") {
            let end = text[cursor + 2..]
                .find("*/")
                .map_or(bytes.len(), |end| cursor + end + 4);
            let newlines = text[cursor..end].matches('\n').count();
            line += newlines;
            leading |= newlines > 0;
            cursor = end;
            continue;
        }
        let start = cursor;
        let start_line = line;
        let kind = if matches!(bytes[cursor], b'~' | b'"' | b'%') {
            let quote = bytes[cursor];
            cursor += 1;
            while cursor < bytes.len() && bytes[cursor] != quote {
                if bytes[cursor] == b'\n' {
                    line += 1;
                }
                cursor += 1;
            }
            if cursor < bytes.len() {
                cursor += 1;
                if quote == b'%' {
                    TokenKind::Variable
                } else {
                    TokenKind::Literal
                }
            } else {
                TokenKind::Invalid
            }
        } else if bytes[cursor] == b'@' {
            cursor += 1;
            while cursor < bytes.len() && bytes[cursor].is_ascii_digit() {
                cursor += 1;
            }
            if cursor > start + 1 {
                TokenKind::Translation
            } else {
                TokenKind::Invalid
            }
        } else if bytes[cursor].is_ascii_alphanumeric() || bytes[cursor] == b'_' {
            cursor += 1;
            while cursor < bytes.len()
                && (bytes[cursor].is_ascii_alphanumeric() || bytes[cursor] == b'_')
            {
                cursor += 1;
            }
            TokenKind::Word
        } else {
            cursor += text[cursor..].chars().next().map_or(1, char::len_utf8);
            if matches!(bytes[start], b'&' | b'|') && bytes.get(cursor) == Some(&bytes[start]) {
                cursor += 1;
            }
            TokenKind::Symbol
        };
        tokens.push(SourceToken {
            text: &text[start..cursor],
            kind,
            line: start_line,
            leading,
            start,
            end: cursor,
        });
        leading = false;
    }
    tokens
}
