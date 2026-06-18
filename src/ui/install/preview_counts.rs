// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::HashSet;

use crate::mods::component::Component;

pub(crate) fn distinct_mod_count(first_log_text: &str, second_log_text: &str) -> usize {
    let mut mods: HashSet<String> = HashSet::new();
    for text in [first_log_text, second_log_text] {
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with("//") {
                continue;
            }
            if let Ok(component) = Component::parse_weidu_line(line) {
                mods.insert(
                    format!("{}/{}", component.name, component.tp_file).to_ascii_uppercase(),
                );
            }
        }
    }
    mods.len()
}

#[cfg(test)]
mod tests {
    use super::distinct_mod_count;

    #[test]
    fn counts_distinct_tp2_across_both_logs_ignoring_comments_and_blanks() {
        let first_log = "\
~ASCENSION/ASCENSION.TP2~ #0 #0 // Ascension - by David Wallis: v2.0
~ASCENSION/ASCENSION.TP2~ #0 #10 // Ascension - Improved battles
~EET/EET.TP2~ #0 #0 // EET core: v13";
        let second_log = "\
~ascension/ascension.tp2~ #0 #20 // Ascension - Wrath of the Five Gods
~CDTWEAKS/SETUP-CDTWEAKS.TP2~ #0 #3160 // Tweaks Anthology: Anthology";
        assert_eq!(distinct_mod_count(first_log, second_log), 3);
    }

    #[test]
    fn empty_or_commentless_logs_yield_zero() {
        assert_eq!(distinct_mod_count("", ""), 0);
        assert_eq!(distinct_mod_count("// only a comment\n\n   \n", ""), 0);
    }
}
