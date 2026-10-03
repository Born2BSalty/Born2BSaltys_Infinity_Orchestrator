// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use super::success_prefix::tp2_stem;

const DUMP_MARKER: &str = "Dumping log:";
const DUMP_END_MARKER: &str = "Weidu command failed";
const TP2_OPEN_MARKER: &str = " for [";
const COMPONENT_LINE_MARKER: &str = "] component ";
const INSTALLING_MARKER: &str = "ERROR Installing [";
const RUN_START_MARKER: &str = "=== Run #";
const RUN_STARTED_MARKER: &str = " started ===";
const MAX_BLOCK_LINES: usize = 100;

pub(in crate::app::terminal) fn extract_error_block(output: &str) -> String {
    let lines: Vec<&str> = output.lines().collect();
    let Some((start, end)) = dump_span(&lines) else {
        return error_lines_fallback(output);
    };
    let span = &lines[start..=end];
    let block = &span[span.len().saturating_sub(MAX_BLOCK_LINES)..];
    let mut out: Vec<String> = Vec::with_capacity(block.len() + 1);
    if let Some(header) = component_header(span) {
        out.push(header);
    }
    out.extend(block.iter().map(|line| (*line).to_string()));
    out.join("\n")
}

fn dump_span(lines: &[&str]) -> Option<(usize, usize)> {
    let run_start = lines
        .iter()
        .rposition(|line| line.contains(RUN_START_MARKER) && line.contains(RUN_STARTED_MARKER))
        .unwrap_or(0);
    let start = run_start
        + lines[run_start..]
            .iter()
            .rposition(|line| line.contains(DUMP_MARKER))?;
    let end = lines
        .iter()
        .enumerate()
        .skip(start + 1)
        .find(|(_, line)| line.contains(DUMP_END_MARKER))
        .map_or(lines.len() - 1, |(index, _)| index);
    Some((start, end))
}

fn component_header(block: &[&str]) -> Option<String> {
    let component_line = block
        .iter()
        .find(|line| line.contains(TP2_OPEN_MARKER) && line.contains(COMPONENT_LINE_MARKER))?;
    let (tp2, id) = component_parts(component_line)?;
    let stem = tp2_stem(tp2);
    let name = block
        .iter()
        .find(|line| line.contains(INSTALLING_MARKER))
        .and_then(|line| installing_name(line));
    Some(name.map_or_else(
        || format!("{stem} #{id}"),
        |name| format!("{stem} #{id} {name}"),
    ))
}

fn component_parts(line: &str) -> Option<(&str, &str)> {
    let tp2_start = line.find(TP2_OPEN_MARKER)? + TP2_OPEN_MARKER.len();
    let rest = &line[tp2_start..];
    let tp2_end = rest.find(']')?;
    let tp2 = &rest[..tp2_end];
    let after_tp2 = &rest[tp2_end..];
    let id_start = after_tp2.find(COMPONENT_LINE_MARKER)? + COMPONENT_LINE_MARKER.len();
    let id_text = &after_tp2[id_start..];
    let id_len = id_text
        .find(|ch: char| !ch.is_ascii_digit())
        .unwrap_or(id_text.len());
    if id_len == 0 {
        return None;
    }
    Some((tp2, &id_text[..id_len]))
}

fn installing_name(line: &str) -> Option<&str> {
    let name_start = line.find(INSTALLING_MARKER)? + INSTALLING_MARKER.len();
    let rest = &line[name_start..];
    let name_end = rest.find(']')?;
    Some(&rest[..name_end])
}

fn error_lines_fallback(output: &str) -> String {
    let mut out = Vec::new();
    for line in output.lines().rev() {
        let u = line.to_ascii_uppercase();
        if u.contains("ERROR")
            || u.contains("FATAL")
            || u.contains("NOT INSTALLED DUE TO ERRORS")
            || u.contains("PARSE ERROR")
            || u.contains("WEIDU COMMAND FAILED")
        {
            out.push(line.to_string());
            if out.len() >= 30 {
                break;
            }
        }
    }
    out.reverse();
    if out.is_empty() {
        "No error lines found in current console output.".to_string()
    } else {
        out.join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::extract_error_block;

    const PARSER_LINE: &str =
        "[2026-09-24T14:02:10Z INFO  mod_installer::parser] Installing mod component";
    const DUMP_LINE: &str = "[2026-09-24T14:02:15Z ERROR mod_installer::runner] Dumping log: [weidu.exe] WeiDU version 24900";
    const END_LINE: &str = "[2026-09-24T14:02:15Z ERROR mod_installer] Weidu command failed with exit status: exit code: 2";

    fn failed_run(body: &[String]) -> String {
        let mut lines = vec![PARSER_LINE.to_string(), PARSER_LINE.to_string()];
        lines.push(DUMP_LINE.to_string());
        lines.extend(body.iter().cloned());
        lines.push(END_LINE.to_string());
        lines.join("\n")
    }

    fn component_body(tp2: &str, id: u32, name: Option<&str>) -> Vec<String> {
        let mut body = vec![
            "    Copying and patching 1 file ...".to_string(),
            "    SET_STRING 209556 out of range 0 -- 104810".to_string(),
        ];
        if let Some(name) = name {
            body.push(format!(
                "    ERROR Installing [{name}], rolling back to previous state"
            ));
        }
        body.push(format!(
            "    Will uninstall  26 files for [{tp2}] component {id}."
        ));
        if let Some(name) = name {
            body.push(format!("    NOT INSTALLED DUE TO ERRORS {name}"));
        }
        body
    }

    #[test]
    fn error_block_is_the_last_dump_headed_by_its_component() {
        let output = failed_run(&component_body(
            "SKILLS-AND-ABILITIES/SKILLS-AND-ABILITIES.TP2",
            150,
            Some("Add New Fighter Abilities"),
        ));
        let block = extract_error_block(&output);
        let lines: Vec<&str> = block.lines().collect();
        assert_eq!(
            lines[0],
            "SKILLS-AND-ABILITIES #150 Add New Fighter Abilities"
        );
        assert_eq!(lines[1], DUMP_LINE);
        assert!(lines.contains(&"    SET_STRING 209556 out of range 0 -- 104810"));
        assert!(lines.contains(
            &"    Will uninstall  26 files for [SKILLS-AND-ABILITIES/SKILLS-AND-ABILITIES.TP2] component 150."
        ));
        assert_eq!(lines.last().copied(), Some(END_LINE));
        assert!(!block.contains("mod_installer::parser]"));
    }

    #[test]
    fn error_block_takes_the_last_dump_when_two_runs_failed() {
        let first = failed_run(&component_body(
            "EEFIXPACK/EEFIXPACK.TP2",
            0,
            Some("Core Fixes"),
        ));
        let second = failed_run(&component_body(
            "SKILLS-AND-ABILITIES/SKILLS-AND-ABILITIES.TP2",
            150,
            Some("Add New Fighter Abilities"),
        ));
        let output = format!("{first}\n=== Run #2 started ===\n{second}");
        let block = extract_error_block(&output);
        assert_eq!(
            block.lines().next(),
            Some("SKILLS-AND-ABILITIES #150 Add New Fighter Abilities")
        );
        assert!(!block.contains("EEFIXPACK"));
    }

    #[test]
    fn error_block_ignores_a_dump_from_an_earlier_run() {
        let first = failed_run(&component_body(
            "EEFIXPACK/EEFIXPACK.TP2",
            0,
            Some("Core Fixes"),
        ));
        let error_line = "[2026-09-24T14:05:11Z ERROR mod_installer] PARSE ERROR in BG1UB.TP2";
        let output = format!("{first}\n=== Run #2 started ===\n{PARSER_LINE}\n{error_line}");
        let block = extract_error_block(&output);
        assert_eq!(block.lines().last(), Some(error_line));
        assert!(!block.contains("EEFIXPACK #0"));
    }

    #[test]
    fn error_block_keeps_the_last_100_lines_of_a_long_dump() {
        let body: Vec<String> = (1..=150).map(|n| format!("    line {n}")).collect();
        let block = extract_error_block(&failed_run(&body));
        let lines: Vec<&str> = block.lines().collect();
        assert_eq!(lines.len(), 100);
        assert_eq!(lines[0], "    line 52");
        assert_eq!(lines.last().copied(), Some(END_LINE));
    }

    #[test]
    fn error_block_header_omits_the_name_without_an_installing_line() {
        let output = failed_run(&component_body(
            "SKILLS-AND-ABILITIES/SKILLS-AND-ABILITIES.TP2",
            150,
            None,
        ));
        let block = extract_error_block(&output);
        assert_eq!(block.lines().next(), Some("SKILLS-AND-ABILITIES #150"));
    }

    #[test]
    fn error_block_without_a_dump_keeps_the_error_lines() {
        let error_line = "[2026-09-24T14:02:11Z ERROR mod_installer] PARSE ERROR in BG1UB.TP2";
        let output = format!("{PARSER_LINE}\n{error_line}");
        let block = extract_error_block(&output);
        assert!(block.contains(error_line));
        assert!(!block.contains(PARSER_LINE));
        assert_eq!(
            extract_error_block(""),
            "No error lines found in current console output."
        );
    }
}
