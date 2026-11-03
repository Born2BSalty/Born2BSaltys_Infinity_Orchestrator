use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

#[derive(Clone)]
struct Token {
  text: String,
  literal: bool,
  line: usize,
}

struct Function {
  name: String,
  start: usize,
  body: usize,
  end: usize,
  parameters: HashMap<String, String>,
}

struct Call {
  name: String,
  start: usize,
  end: usize,
  arguments: HashMap<String, String>,
  returns: HashMap<String, String>,
}

struct Unit {
  path: PathBuf,
  tokens: Vec<Token>,
  functions: Vec<Function>,
  calls: Vec<Call>,
  blocks: Vec<Vec<usize>>,
}

pub(super) struct Resolver {
  units: Vec<Unit>,
}

#[derive(Clone, PartialEq, Eq)]
pub(super) enum Resolution {
  Text(String),
  Missing,
  Ambiguous,
}

type Environment = HashMap<String, Resolution>;
type Visit = (usize, Option<usize>, usize, String);

impl Resolver {
  pub(super) fn new(targets: &[(PathBuf, String)]) -> Self {
    let units = targets
      .iter()
      .map(|(path, source)| {
        let tokens = tokenize(source);
        let calls = collect_calls(&tokens);
        let functions = collect_functions(&tokens, &calls);
        let blocks = collect_blocks(&tokens, &calls);
        Unit {
          path: super::normalize_for_visited(path),
          tokens,
          functions,
          calls,
          blocks,
        }
      })
      .collect();
    Self { units }
  }

  pub(super) fn resolve(
    &self,
    path: &Path,
    line: usize,
    text: &str,
    tra: &HashMap<String, String>,
  ) -> Resolution {
    let path = super::normalize_for_visited(path);
    let Some(unit) = self.units.iter().position(|unit| unit.path == path) else {
      return Resolution::Missing;
    };
    let before = self.units[unit]
      .tokens
      .partition_point(|token| token.line < line);
    let scope = self.scope(unit, before);
    self.atom(
      unit,
      scope,
      before,
      text,
      tra,
      &Environment::new(),
      &mut HashSet::new(),
      0,
    )
  }

  fn scope(&self, unit: usize, at: usize) -> Option<usize> {
    self.units[unit]
      .functions
      .iter()
      .position(|function| at >= function.start && at <= function.end)
  }

  fn atom(
    &self,
    unit: usize,
    scope: Option<usize>,
    before: usize,
    text: &str,
    tra: &HashMap<String, String>,
    environment: &Environment,
    visited: &mut HashSet<Visit>,
    depth: usize,
  ) -> Resolution {
    if depth >= 20 {
      return Resolution::Ambiguous;
    }
    let text = text.trim();
    if let Some(name) = super::extract_percent_placeholder(text) {
      return self.variable(
        unit,
        scope,
        before,
        &name,
        tra,
        environment,
        visited,
        depth + 1,
      );
    }
    if text.starts_with('@') && text[1..].bytes().all(|byte| byte.is_ascii_digit()) {
      return Resolution::Text(
        tra
          .get(&text.to_ascii_lowercase())
          .cloned()
          .unwrap_or_else(|| text.to_string()),
      );
    }
    if text.is_empty() {
      Resolution::Missing
    } else {
      Resolution::Text(text.to_string())
    }
  }

  fn variable(
    &self,
    unit: usize,
    scope: Option<usize>,
    before: usize,
    name: &str,
    tra: &HashMap<String, String>,
    environment: &Environment,
    visited: &mut HashSet<Visit>,
    depth: usize,
  ) -> Resolution {
    let key = (unit, scope, before, name.to_ascii_lowercase());
    if depth >= 20 || !visited.insert(key.clone()) {
      return Resolution::Ambiguous;
    }
    let result = self.binding(unit, scope, before, name, tra, environment, visited, depth);
    visited.remove(&key);
    result
  }

  fn binding(
    &self,
    unit: usize,
    scope: Option<usize>,
    before: usize,
    name: &str,
    tra: &HashMap<String, String>,
    environment: &Environment,
    visited: &mut HashSet<Visit>,
    depth: usize,
  ) -> Resolution {
    let source = &self.units[unit];
    let start = scope.map_or(0, |index| source.functions[index].body + 1);
    let mut bindings = Vec::new();
    let mut index = start;
    while index < before && index < source.tokens.len() {
      if let Some(call) = source.calls.iter().find(|call| call.start == index) {
        if let Some(returned) = call.returns.get(name) {
          bindings.push((
            index,
            self.returned(
              unit,
              scope,
              call,
              returned,
              tra,
              environment,
              visited,
              depth + 1,
            ),
          ));
        }
        index = call.end + 1;
        continue;
      }
      if scope.is_none() {
        if let Some(function) = source
          .functions
          .iter()
          .find(|function| function.start == index)
        {
          index = function.end + 1;
          continue;
        }
      }
      let command = &source.tokens[index];
      let supported = [
        "OUTER_SPRINT",
        "OUTER_TEXT_SPRINT",
        "SPRINT",
        "TEXT_SPRINT",
        "OUTER_PATCH_SAVE",
      ];
      if !command.literal
        && supported
          .iter()
          .any(|item| command.text.eq_ignore_ascii_case(item))
        && source
          .tokens
          .get(index + 1)
          .is_some_and(|token| normalize(&token.text) == name)
      {
        if let Some((value, _)) = atom_after(&source.tokens, index + 2, before) {
          let safe = !command.text.eq_ignore_ascii_case("OUTER_PATCH_SAVE")
            || template_patch(&source.tokens, index, before, name);
          let value = if safe {
            self.atom(
              unit,
              scope,
              index,
              &value,
              tra,
              environment,
              visited,
              depth + 1,
            )
          } else {
            Resolution::Ambiguous
          };
          bindings.push((index, value));
        }
      }
      index += 1;
    }
    if !bindings.is_empty() {
      return self.choose_bindings(unit, before, bindings);
    }
    if let Some(value) = environment.get(name) {
      return value.clone();
    }
    if let Some(scope) = scope {
      let function = &source.functions[scope];
      if let Some(default) = function.parameters.get(name) {
        let mut values = Vec::new();
        for (caller_unit, caller) in self.units.iter().enumerate() {
          for call in &caller.calls {
            if call.name == function.name {
              let caller_scope = self.scope(caller_unit, call.start);
              let value = if let Some(argument) = call.arguments.get(name) {
                self.atom(
                  caller_unit,
                  caller_scope,
                  call.start,
                  argument,
                  tra,
                  &Environment::new(),
                  visited,
                  depth + 1,
                )
              } else {
                self.atom(
                  unit,
                  Some(scope),
                  function.body,
                  default,
                  tra,
                  environment,
                  visited,
                  depth + 1,
                )
              };
              values.push(value);
              if values.len() > 128 {
                return Resolution::Ambiguous;
              }
            }
          }
        }
        if values.is_empty() {
          return self.atom(
            unit,
            Some(scope),
            function.body,
            default,
            tra,
            environment,
            visited,
            depth + 1,
          );
        }
        return agree(values);
      }
    }
    Resolution::Missing
  }

  fn choose_bindings(
    &self,
    unit: usize,
    before: usize,
    bindings: Vec<(usize, Resolution)>,
  ) -> Resolution {
    let source = &self.units[unit];
    let current = source.blocks.get(before).map_or(&[][..], Vec::as_slice);
    let mut possible = vec![Resolution::Missing];
    for (index, value) in bindings {
      let path = &source.blocks[index];
      if current.starts_with(path) {
        possible = vec![value];
      } else if path.starts_with(current) {
        possible.push(value);
      }
    }
    agree(possible)
  }

  fn returned(
    &self,
    caller_unit: usize,
    caller_scope: Option<usize>,
    call: &Call,
    returned: &str,
    tra: &HashMap<String, String>,
    caller_environment: &Environment,
    visited: &mut HashSet<Visit>,
    depth: usize,
  ) -> Resolution {
    let definitions = self
      .units
      .iter()
      .enumerate()
      .flat_map(|(unit, source)| {
        source
          .functions
          .iter()
          .enumerate()
          .filter_map(move |(scope, function)| {
            (function.name == call.name).then_some((unit, scope, function))
          })
      })
      .collect::<Vec<_>>();
    let Some((unit, scope, function)) = definitions.first() else {
      return Resolution::Missing;
    };
    let original = &self.units[*unit].tokens[function.start..function.end];
    if definitions.iter().skip(1).any(|(other_unit, _, other)| {
      let candidate = &self.units[*other_unit].tokens[other.start..other.end];
      !original
        .iter()
        .map(|token| (&token.text, token.literal))
        .eq(candidate.iter().map(|token| (&token.text, token.literal)))
    }) {
      return Resolution::Ambiguous;
    }
    let mut environment = Environment::new();
    for (name, default) in &function.parameters {
      let value = if let Some(argument) = call.arguments.get(name) {
        self.atom(
          caller_unit,
          caller_scope,
          call.start,
          argument,
          tra,
          caller_environment,
          visited,
          depth + 1,
        )
      } else {
        self.atom(
          *unit,
          Some(*scope),
          function.body,
          default,
          tra,
          caller_environment,
          visited,
          depth + 1,
        )
      };
      environment.insert(name.clone(), value);
    }
    self.variable(
      *unit,
      Some(*scope),
      function.end,
      returned,
      tra,
      &environment,
      visited,
      depth + 1,
    )
  }
}

fn agree(values: Vec<Resolution>) -> Resolution {
  let mut result = None;
  for value in values {
    match value {
      Resolution::Text(value) => {
        if result.as_ref().is_some_and(|previous| previous != &value) {
          return Resolution::Ambiguous;
        }
        result = Some(value);
      }
      Resolution::Missing | Resolution::Ambiguous => return Resolution::Ambiguous,
    }
  }
  result.map_or(Resolution::Missing, Resolution::Text)
}

fn normalize(value: &str) -> String {
  value
    .trim()
    .trim_matches(['~', '"', '%'])
    .to_ascii_lowercase()
}

fn atom_after(tokens: &[Token], mut index: usize, end: usize) -> Option<(String, usize)> {
  while index < end && tokens[index].text.eq_ignore_ascii_case("EVAL") {
    index += 1;
  }
  let token = tokens.get(index)?;
  (index < end).then(|| (token.text.clone(), index + 1))
}

fn template_patch(tokens: &[Token], start: usize, end: usize, name: &str) -> bool {
  let Some((input, mut index)) = atom_after(tokens, start + 2, end) else {
    return false;
  };
  if super::extract_percent_placeholder(&input).as_deref() != Some(name) {
    return false;
  }
  while index < end {
    if tokens[index].text.eq_ignore_ascii_case("END") {
      return true;
    }
    if tokens[index].text.eq_ignore_ascii_case("REPLACE_TEXTUALLY") {
      let Some((pattern, _)) = atom_after(tokens, index + 1, end) else {
        return false;
      };
      if !pattern.contains('<') && !pattern.contains('%') {
        return false;
      }
    } else if !tokens[index].literal
      && !["BEGIN", "EVAL", "CASE_SENSITIVE", "CASE_INSENSITIVE"]
        .iter()
        .any(|word| tokens[index].text.eq_ignore_ascii_case(word))
    {
      return false;
    }
    index += 1;
  }
  false
}

fn collect_calls(tokens: &[Token]) -> Vec<Call> {
  let mut calls = Vec::new();
  for start in 0..tokens.len() {
    if tokens[start].literal || !tokens[start].text.eq_ignore_ascii_case("LAF") {
      continue;
    }
    let Some(name) = tokens.get(start + 1) else {
      continue;
    };
    let mut end = start + 2;
    let mut depth = 1;
    while end < tokens.len() {
      if !tokens[end].literal && tokens[end].text.eq_ignore_ascii_case("LAF") {
        depth += 1;
      }
      if !tokens[end].literal && tokens[end].text.eq_ignore_ascii_case("END") {
        depth -= 1;
        if depth == 0 {
          break;
        }
      }
      end += 1;
    }
    if end == tokens.len() {
      continue;
    }
    let mut arguments = HashMap::new();
    let mut returns = HashMap::new();
    let mut section = "";
    let mut index = start + 2;
    while index < end {
      let token = &tokens[index];
      if !token.literal
        && ["STR_VAR", "INT_VAR", "RET", "RET_ARRAY"]
          .iter()
          .any(|word| token.text.eq_ignore_ascii_case(word))
      {
        section = if token.text.eq_ignore_ascii_case("STR_VAR") {
          "strings"
        } else if token.text.eq_ignore_ascii_case("RET") {
          "returns"
        } else {
          ""
        };
        index += 1;
        continue;
      }
      let key = normalize(&token.text);
      if section.is_empty() || key.is_empty() || token.text == "=" {
        index += 1;
        continue;
      }
      if tokens.get(index + 1).is_some_and(|token| token.text == "=") {
        if let Some((value, next)) = atom_after(tokens, index + 2, end) {
          if section == "strings" {
            arguments.insert(key, value);
          } else {
            returns.insert(key, normalize(&value));
          }
          index = next;
          continue;
        }
      } else if section == "strings" {
        arguments.insert(key.clone(), format!("%{key}%"));
      } else {
        returns.insert(key.clone(), key);
      }
      index += 1;
    }
    calls.push(Call {
      name: normalize(&name.text),
      start,
      end,
      arguments,
      returns,
    });
  }
  calls
}

fn collect_functions(tokens: &[Token], calls: &[Call]) -> Vec<Function> {
  let mut result = Vec::new();
  let mut index = 0;
  while index < tokens.len() {
    if tokens[index].literal
      || !["DEFINE_ACTION_FUNCTION", "DEFINE_DIMORPHIC_FUNCTION"]
        .iter()
        .any(|word| tokens[index].text.eq_ignore_ascii_case(word))
    {
      index += 1;
      continue;
    }
    let start = index;
    let Some(name) = tokens.get(start + 1) else {
      break;
    };
    let Some(body) = (start + 2..tokens.len())
      .find(|index| !tokens[*index].literal && tokens[*index].text.eq_ignore_ascii_case("BEGIN"))
    else {
      break;
    };
    let mut parameters = HashMap::new();
    let mut strings = false;
    index = start + 2;
    while index < body {
      if !tokens[index].literal
        && ["STR_VAR", "INT_VAR", "RET", "RET_ARRAY"]
          .iter()
          .any(|word| tokens[index].text.eq_ignore_ascii_case(word))
      {
        strings = tokens[index].text.eq_ignore_ascii_case("STR_VAR");
      } else if strings && tokens.get(index + 1).is_some_and(|token| token.text == "=") {
        if let Some((value, next)) = atom_after(tokens, index + 2, body) {
          parameters.insert(normalize(&tokens[index].text), value);
          index = next;
          continue;
        }
      }
      index += 1;
    }
    let mut depth = 1;
    index = body + 1;
    while index < tokens.len() {
      if let Some(call) = calls.iter().find(|call| call.start == index) {
        index = call.end + 1;
        continue;
      }
      if !tokens[index].literal {
        if tokens[index].text.eq_ignore_ascii_case("BEGIN")
          || tokens[index].text.eq_ignore_ascii_case("ACTION_MATCH")
        {
          depth += 1;
        }
        if tokens[index].text.eq_ignore_ascii_case("END") {
          depth -= 1;
          if depth == 0 {
            break;
          }
        }
      }
      index += 1;
    }
    result.push(Function {
      name: normalize(&name.text),
      start,
      body,
      end: index.min(tokens.len()),
      parameters,
    });
    index += 1;
  }
  result
}

fn collect_blocks(tokens: &[Token], calls: &[Call]) -> Vec<Vec<usize>> {
  let mut result = Vec::with_capacity(tokens.len());
  let mut stack = Vec::new();
  let mut index = 0;
  while index < tokens.len() {
    if let Some(call) = calls.iter().find(|call| call.start == index) {
      while index <= call.end {
        result.push(stack.clone());
        index += 1;
      }
      continue;
    }
    result.push(stack.clone());
    if !tokens[index].literal {
      if tokens[index].text.eq_ignore_ascii_case("BEGIN")
        || tokens[index].text.eq_ignore_ascii_case("ACTION_MATCH")
      {
        stack.push(index);
      } else if tokens[index].text.eq_ignore_ascii_case("END") {
        stack.pop();
      }
    }
    index += 1;
  }
  result
}

fn tokenize(source: &str) -> Vec<Token> {
  let chars = source.chars().collect::<Vec<_>>();
  let mut tokens = Vec::new();
  let mut index = 0;
  let mut line = 0;
  while index < chars.len() {
    if chars[index].is_whitespace() {
      line += usize::from(chars[index] == '\n');
      index += 1;
      continue;
    }
    if chars[index] == '/' && chars.get(index + 1) == Some(&'/') {
      while index < chars.len() && chars[index] != '\n' {
        index += 1;
      }
      continue;
    }
    if chars[index] == '/' && chars.get(index + 1) == Some(&'*') {
      index += 2;
      while index < chars.len() && !(chars[index] == '*' && chars.get(index + 1) == Some(&'/')) {
        line += usize::from(chars[index] == '\n');
        index += 1;
      }
      index = (index + 2).min(chars.len());
      continue;
    }
    let start_line = line;
    let literal = chars[index] == '~' || chars[index] == '"';
    let mut text = String::new();
    if literal {
      let delimiter = chars[index];
      index += 1;
      while index < chars.len() && chars[index] != delimiter {
        line += usize::from(chars[index] == '\n');
        text.push(chars[index]);
        index += 1;
      }
      index = (index + 1).min(chars.len());
    } else if chars[index] == '=' {
      text.push('=');
      index += 1;
    } else {
      while index < chars.len()
        && !chars[index].is_whitespace()
        && !['=', '~', '"'].contains(&chars[index])
      {
        text.push(chars[index]);
        index += 1;
      }
    }
    tokens.push(Token {
      text,
      literal,
      line: start_line,
    });
  }
  tokens
}
