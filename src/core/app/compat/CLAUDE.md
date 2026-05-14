# src/core/app/compat/

TP2/modlist compatibility rule engine. Two layers: **scans** (built-in heuristics over scanned components) and **rules** (declarative TOML overrides loaded from `step2_compat_rules.toml`).

All files here are flattened into `crate::app::compat_*` via `#[path]` in `core/app/mod.rs`.

## Issue kinds (see `compat_issue.rs`)

`REQ_MISSING` (missing dependency), `FORBID_HIT` (conflict), `GAME_MISMATCH` (wrong game target), `CONDITIONAL` (conditional patch), `ORDER_WARN` (install-order warning), plus deprecated-mod scan.

## Layers

### Scans — built-in heuristics, run after every Step 2 scan

| File | Detects |
|------|---------|
| `compat_missing_dep_scan.rs` | Required deps not present. |
| `compat_conflict_scan.rs` (+ `_parse.rs`, `_runtime.rs`) | Forbid/conflict hits. |
| `compat_mismatch_scan.rs` (+ `_classify`, `_guards`) + `compat_mismatch_eval*.rs` | Game-target mismatches (BGEE vs BG2EE vs EET context). |
| `compat_path_scan.rs` (+ `_eval.rs`, `_runtime.rs`) | Required file/folder paths. |
| `compat_deprecated_scan.rs` | Mods marked deprecated. |

`compat_logic.rs::apply_step2_compat_rules` is the orchestrator that runs all of the above in sequence after a scan.

### Rule engine — declarative overrides

| File | Role |
|------|------|
| `compat_rules.rs` | Loads `step2_compat_rules.toml` (user override → default fallback) via `load_rules()`. |
| `compat_rules_model.rs` | `CompatRule` struct + serde model. |
| `compat_rule_runtime.rs` (+ `_active.rs`, `_matches.rs`, `_relations.rs`) | Evaluates rules against the active selection — direct rules vs relation rules, kind matching, mod-key normalization. |
| `compat_dependency_parse.rs`, `compat_dependency_runtime.rs` | Parses & evaluates the dependency expression mini-language. |

### Issue plumbing & UI bridges

| File | Role |
|------|------|
| `compat_issue.rs`, `compat_issue_text.rs` | Issue enum + human-readable rendering. |
| `compat_popup_nav.rs`, `compat_popup_targets.rs` | Drives the Step 2 compat popup (which file/component to jump to). |
| `compat_step3_rules.rs` | Step 3 re-evaluation against ordered selection. |
| `compat_setup_tra.rs` | TRA (translation file) discovery for evaluation context. |

## Adding a new compat rule kind

1. Add the kind to `compat_issue.rs::IssueKind` and its text in `compat_issue_text.rs`.
2. If declarative-only: extend `compat_rules_model.rs` and `compat_rule_runtime*` matching.
3. If a built-in scan: add a `compat_<name>_scan.rs` and call it from `compat_logic::apply_step2_compat_rules`.
4. UI side: add rendering in `src/ui/step2/compat/` (popup) and `src/ui/step2/tree/tree_compat_display_step2.rs` (inline pills).
