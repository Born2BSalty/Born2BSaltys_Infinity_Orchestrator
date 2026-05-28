# Phase 8 — Full visual-fidelity reskin sweep, automatic flag policies, polish

## Summary

Phase 8 covers the **full visual-fidelity sweep** across every BIO source file the orchestrator embeds — Step 2 tree + Details panel, Step 3 reorder list + toolbar pills, Step 5 console + status row + install row + menus + cancel-confirm, and the popup group (Compat, Prompt, Update Check, GitHub OAuth) — under SPEC §1 CRITICAL DIRECTIVE carve-out #6 (state-aware theme-token reads). Every BIO file that renders into the orchestrator's Workspace surfaces is in scope; each is annotated with the carve-out(s) it falls under (#1 pure value substitution, #2 window-chrome flip, or #6 state-aware token reads) and the specific conditional sites whose inline `theme_global::*()` accessors swap for `redesign_*(palette)` accessors.

The two pre-existing carve-outs (#1 + #2) cover the popup group and a handful of unconditional-color files. Carve-out #6 covers the conditional sites in the Step 2 tree / Step 2 Details / Step 3 reorder list / Step 5 sub-renderers — every site that today branches on hover / selected / disabled / conflict-kind / install-running / prep-running / dev-mode / etc. **The conditional structure of each function is preserved exactly**; only the inline `theme_global::*()` accessor inside each branch swaps for a `redesign_*(palette)` accessor, and each touched function gains a `palette: ThemePalette` parameter that call sites thread through. No new branches, no removed branches, no reordered branches, no logic mutations, no behavior changes.

This phase also wires the **residual** SPEC §13.12 flag policies — **#6 + #7 only** — plus the dotted radial background, toast notifications, hover affordances, and the final smoke pass. **#2 / #3 / #4 are NOT Phase 8:** their per-install directory creation (`-u` `weidu_component_logs/`, EET `-p`/`-n` phase dirs, single-game `-g` dir) is install-critical and is owned by **Phase 7 P7.T17** (SPEC §13.12a) — a Phase-7 install cannot run without those dirs, so they cannot be deferred here.

## What ships after this phase

- `cargo build --bin infinity_orchestrator --release` succeeds; the binary is feature-complete for v1 alpha.
- **Every workspace surface visually matches the wireframe.** Specifically:
  - **Step 2 tree** — component rows, parent rows, mod headers, header markers, compat pills, PROMPT pill, and the `compat_colors` / `parent_compat_summary` color palette all read from `redesign_*(palette)` accessors. Dark theme renders dark teal-on-slate; Light theme renders pale blueprint.
  - **Step 2 Details panel** — pane title, exact-log status banner, selection grid (Checked / State / Reason), paths grid (missing-amber on absent paths), package grid all read from redesign tokens.
  - **Step 3 reorder list** — list rows (lock icons, compat pills, PROMPT pill), toolbar tab issue badges, compat-rules-error warning all read from redesign tokens.
  - **Step 5** — console line tones (per SPEC §9.2), install row state banner (Preparing / Installing), dev-mode RUST_LOG status line, status phase indicator (Idle / Preparing / Running / Waiting Input / Cancelling / Finished), error-state copy, cancel-confirm warning all read from redesign tokens. The embedded terminal's internal cell colors stay BIO-default (out of token-extraction scope; SPEC §9 doesn't require recolor).
  - **Popups** — Compat popup, Prompt popup (text + toolbar variants), Update Check popup (+ confirm-latest-fallback inner dialog + source editor + forks), GitHub OAuth popup. Every popup also gets the egui-native title-bar collapse chevron (`.collapsible(true)`).
- **Top-nav chrome** (orchestrator-side `workspace_nav_bar.rs`) already uses redesign tokens via its Phase 6 new-file status. BIO's legacy `update_app.rs` / `app_nav_ui.rs` are part of the `BIO` binary only — not invoked by `OrchestratorApp` — and stay BIO-default per the CRITICAL DIRECTIVE's "anything else is disallowed" rule for the legacy surface.
- All SPEC §13.12 automatic flag policies in effect (implemented across Phase 7 P7.T16 [#1/#5] + P7.T17 [#2/#3/#4 per-install dirs] + this phase [#6/#7]):
  - `-u` always ON, path under `<destination>/weidu_component_logs/` (policy #2) — **implemented Phase 7 P7.T17** (install-critical)
  - `-p` / `-n` always ON for EET, paths derived from destination (policy #3) — **implemented Phase 7 P7.T17** (install-critical)
  - `-g` always ON for single-game installs, path derived from destination (policy #4) — **implemented Phase 7 P7.T17** (install-critical)
  - `prepare_target_dirs_before_install` / `backup_targets_before_eet_copy` from `DestinationNotEmptyWarning` choice (policy #6)
  - `-autolog` / `-logapp` / `-log-extern` always ON (policy #7)
  - Plus Phase 7's #1 (`-s`/`-c`) and #5 (`--download`)
- Toast notifications work across Home, Workspace, and Settings (all "this happened" feedback).
- The dotted radial background pattern (SPEC §12.3, `--dot` token) renders behind the shell.
- Final smoke pass: every screen, every popup, every flow runs without panic. Visual diff against `wireframe-preview/build.html` shows no token regressions on any workspace surface.

## What's still missing

- Nothing within the SPEC's v1 alpha scope.
- The Explore tab (SPEC's v2 community track) is **not** part of the v1 alpha — explicitly out of scope.
- Per-platform asset packaging UI (Appendix B.11) — explicitly resolved as "no UI added".
- A real Nexus Mods / Mega integration — Appendix A.17 stubs only.
- A game launcher for the `play` button — fallback opens the folder.
- The embedded terminal widget's internal cell colors stay BIO-default; SPEC §9 doesn't require terminal recolor (only the line-tone classifier in `status_console_step5.rs`, which **is** covered by carve-out #6).

## Dependencies

- Phases 1-7 must be merged.
- Phase 1's `redesign_tokens.rs` must expose the full `redesign_*(palette)` accessor surface that the carve-out #6 swaps consume. The token map (P8.T1) extends `redesign_tokens.rs` to add any missing accessors needed by today's BIO accessor list (`accent_path`, `accent_numbers`, `accent_comment`, `prompt_text/fill/stroke`, the eight `terminal_*` tones, `success/success_bright`, `warning/warning_soft/warning_emphasis/warning_fill/warning_parent`, `error/error_emphasis`, `conflict/conflict_fill/conflict_parent`, `info/info_fill`, `included/included_fill`, `game_mismatch/game_mismatch_fill`, `conditional/conditional_fill`, `status_running/status_preparing/status_idle`, `text_primary/text_muted/text_disabled`). Each gets a `redesign_*(palette)` counterpart with light + dark values per SPEC §12.1 / §12.2.

## File inventory

### New files

**Note:** flag-policy files for `-u` weidu_component_logs (#2), EET `-p`/`-n` (#3), and single-game `-g` (#4) are SUPERSEDED — moved to Phase 7 P7.T17 (`install_runtime/per_install_dirs.rs`) per SPEC §13.12a, since those per-install dirs are install-critical.

| Path | Purpose | Depends on |
|------|---------|-----------|
| `src/install_runtime/flag_policies_dest_choice.rs` | Maps the `DestChoice` (clear / backup / continue) to `prepare_target_dirs_before_install` / `backup_targets_before_eet_copy` flag values per SPEC §13.12 #6. | — |
| `src/ui/shared/redesign_dot_background.rs` | Renders the 20×20px dotted radial background pattern as an `egui::Painter` custom draw. Called from `shell_chrome::render_shell` body. | redesign theme tokens |
| `src/ui/orchestrator/widgets/clipboard.rs` | Shared clipboard helper used by all redesign copy sites. | `arboard` or `egui_extras` |
| `src/ui/orchestrator/widgets/toast.rs` | Generalized toast helper (relocated from Phase 5's `home/toast.rs`). Source of all "this happened" feedback overlays. | redesign theme tokens |
| `src/ui/orchestrator/widgets/popup_collapse_anchor.rs` | Thin wrapper around `egui::Window` that captures `Window::current_pos` at the collapse transition and pins the window's top-Y so the title bar stays put when the body collapses. Used only if egui's native collapse re-centers (see P8.T6). | egui::Window |

### BIO files read from / consumed (no modifications)

- Every BIO file referenced from earlier phases not appearing in the carve-out tables below.

### BIO files needing allowed mild refactor — carve-out #1 + #2 (unconditional swaps + window-chrome flips)

Strict value substitution: replace each `theme_global::*()` accessor call with the matching `redesign_*(palette)` accessor; replace `.collapsible(false)` with `.collapsible(true)` where listed. **No conditional logic touched.** The functions that today take a `WizardState` (or none) gain a `palette: ThemePalette` parameter so call sites thread the palette through.

| File | Carve-out | Change (strict) — cite line numbers |
|------|-----------|--------------------------------------|
| `src/ui/step2/compat/compat_window_step2.rs` | #2 | Line 16: `.collapsible(false)` → `.collapsible(true)`. No inline colors. |
| `src/ui/step2/prompt/prompt_popup_step2.rs` | #1 + #2 | Line 27: `.collapsible(false)` → `.collapsible(true)` on main popup. Line 123: same flip on toolbar variant. Unconditional `theme_global::*` calls: line 56 (`accent_numbers`); line 91 (`prompt_text`); line 95 (`prompt_fill`); line 98 (`prompt_stroke`); line 147 (`accent_numbers`). Each swaps for the matching `redesign_*(palette)` accessor. Function `render_prompt_popup` and `render_prompt_toolbar_popup` and `draw_prompt_toolbar_badge` and `render_prompt_toolbar_popup` each gain a `palette: ThemePalette` parameter. |
| `src/ui/step2/update_check/update_check_popup_step2.rs` | #2 | Line 67: `.collapsible(false)` → `.collapsible(true)` on main popup. Line 589: same flip on the `Download Latest Instead?` confirm dialog. Line 631: same flip on the forks popup. No inline colors. |
| `src/ui/step2/update_check/update_check_popup_lists_step2.rs` | #1 | `render_section_header`: line 209 `theme_global::text_primary()` → `redesign_text_primary(palette)`; line 211 `theme_global::info_fill()` → `redesign_pill_info(palette)`. Function gains `palette: ThemePalette` arg; all three callers in `update_check_popup_step2.rs::render` (the `render_list` call sites + the section header invocations) and in `render_forks_popup` thread the palette through. |
| `src/ui/step2/update_check/update_check_popup_source_editor_step2.rs` | #2 | Line 20: `.collapsible(false)` → `.collapsible(true)`. No inline colors. |
| `src/ui/step1/github_auth_popup_step1.rs` | #2 | Line 18: `.collapsible(false)` → `.collapsible(true)`. No inline colors. |
| `src/ui/step2/format_step2.rs` | #1 | `colored_component_widget_text`: lines 55-56 unconditional `theme_global::accent_numbers()` / `theme_global::accent_comment()` swap for `redesign_accent_numbers(palette)` / `redesign_accent_comment(palette)`. The conditional structure at lines 62-71 (which picks *between* the two pre-resolved colors) is preserved exactly — only the top-of-function resolution lines change. Function gains `palette: ThemePalette` arg; threaded from `tree_component_row_step2.rs::render_component_row` (line 95) and any other callers. |
| `src/ui/step3/format_step3.rs` | #1 | `weidu_colored_widget_text`: lines 12-14 unconditional `theme_global::accent_path()` / `theme_global::accent_numbers()` / `theme_global::success()` swap for the redesign counterparts. The conditional at lines 16-33 is preserved exactly. Function gains `palette: ThemePalette` arg; threaded from `list_rows_step3.rs::render_step3_rows` (line 151) and any other callers. |
| `src/ui/step2/tree/tree_parent_step2.rs` | #1 | `render_parent_row`: the parent-summary pill (lines 122-147) receives `(text_color, bg, label)` pre-resolved from `parent_compat_summary` — no swap needed at this site. The PROMPT pill (lines 148-175) has unconditional `theme_global::prompt_text/prompt_fill/prompt_stroke` (lines 153, 156, 159). Each swaps for the `redesign_prompt_*(palette)` counterpart. Function gains `palette: ThemePalette` arg; threaded from `tree_render_step2.rs::render_mod_tree`. |

### BIO files needing allowed mild refactor — carve-out #6 (state-aware token reads)

Each file below has at least one conditional site where the inline `theme_global::*()` accessor inside a branch swaps for a `redesign_*(palette)` accessor. **The conditional structure is preserved exactly** — no new branches, no removed branches, no reordered branches, no logic mutations. Only the color expressions inside each branch change. Each function on the call path gains a `palette: ThemePalette` parameter; call sites thread the palette through.

| File | Carve-out | Conditional sites (cite line numbers) |
|------|-----------|----------------------------------------|
| `src/ui/step2/compat/compat_popup_step2.rs` | #6 | `compat_popup_details::render_details` lines 116-124: `match issue.status_tone { Neutral => theme::text_muted(), Blocking => theme::error_emphasis(), Warning => theme::warning_soft() }`. Each arm's accessor swaps to `redesign_*(palette)`; the three-arm match is preserved exactly. `compat_popup_details::render_filter_row` line 208: `theme::text_disabled()` inside the `if !compat_filter_matches(option, current_kind)` branch swaps to `redesign_text_disabled(palette)`. Both functions gain `palette` arg; threaded from `compat_window_step2.rs::render` (which itself gains `palette`). |
| `src/ui/step2/tree/tree_compat_display_step2.rs` | #6 | `compat_colors` lines 11-58: 9-arm `match` on `kind` returns a `(text_color, fill_color, label)` tuple; each arm's two `theme_global::*()` calls swap for the matching `redesign_*(palette)` counterparts. `parent_compat_summary` lines 61-103: `if conflicts > 0 / if order_blocks > 0 / if warnings > 0` cascade — each branch's two `theme_global::*()` calls swap for `redesign_*(palette)` counterparts. Both functions gain `palette: ThemePalette` arg; threaded from every call site (`tree_component_row_step2.rs::render_component_row` line 74 and line 103; `tree_parent_step2.rs::render_parent_row` line 39; `list_rows_step3.rs::render_step3_rows` line 155; `toolbar_compat_step2.rs::draw_active_tab_issue_badge` line 137). |
| `src/ui/step2/tree/tree_component_row_step2.rs` | #6 | `render_component_row` lines 89-96: `if effectively_disabled { ... .color(theme::text_disabled()) } else { colored_component_widget_text(ui, ...) }`. The `theme::text_disabled()` accessor inside the `if` branch swaps for `redesign_text_disabled(palette)`; the `else` branch's helper now takes `palette` (see carve-out #1 row for `format_step2.rs`). The PROMPT pill (lines 156-165) is unconditional — swap `theme::prompt_text/fill/stroke` for `redesign_prompt_*(palette)`. The compat pill (lines 120-134) receives pre-resolved colors from `compat_colors` so no swap is needed at this site (the swap happens in `tree_compat_display_step2.rs`). Function gains `palette: ThemePalette` arg; threaded from `tree_components_step2.rs::render_component_rows` (caller). |
| `src/ui/step2/tree/tree_header_marker_step2.rs` | #6 | `render` lines 18-26: outer `if mod_state.update_locked` chooses `theme::text_muted()`; inner `match marker.as_str()` chooses between `theme::success()` (the `"+"` arm), `theme::error()` (the `"!"` arm), and `ui.visuals().text_color()` (default arm). Three `theme_global::*` accessors swap for redesign counterparts. The `ui.visuals().text_color()` fallback inside the default-arm stays as-is (it reads egui's runtime style, which the orchestrator updates via the palette at frame start — see Phase 1 P1.T2's "Palette ownership and per-frame propagation" note). Function gains `palette: ThemePalette` arg; threaded from `tree_parent_step2.rs::render_parent_row` line 121. |
| `src/ui/step2/details/details_pane_step2.rs` | #6 | `details_pane_content::render_exact_log_status` lines 59-66: `let (headline, color) = if ready { (..., theme::success_bright()) } else { (..., theme::error()) }` — two-arm conditional, each branch's accessor swaps for `redesign_*(palette)`. Line 74: `theme::warning()` inside `if !state.step2.exact_log_mod_list_checked` branch swaps for `redesign_pill_warn(palette)` (or the matching warn-tone accessor). The outer `render_pane` (line 6) and `render` (line 89) functions gain `palette: ThemePalette` arg; threaded from the workspace step router. |
| `src/ui/step2/details/details_paths_step2.rs` | #6 | `render_path_row` lines 275-277: `if value.is_none() && missing_amber { text = text.color(theme_global::warning()); }` — single conditional color application, swaps for `redesign_pill_warn(palette)`. All other rendering in this file (text/labels/buttons) reads no inline colors. Function gains `palette: ThemePalette` arg; threaded from `details_pane_step2.rs::details_pane_content::render_details_content`. |
| `src/ui/step2/details/details_selection_step2.rs` | #6 | `render_checked_row` lines 170-175: `match checked { true => theme::success(), false => theme::text_muted() }` — two-arm match, each accessor swaps for redesign counterparts. `render_state_row` lines 195-201: `if is_disabled { theme::warning() } else { theme::success() }`. `render_reason_row` line 296: unconditional `theme::warning()` (single accessor swap). Each function gains `palette: ThemePalette` arg; threaded from `render_selection_grid` which itself gains `palette`, threaded from `details_pane_step2.rs::details_pane_content::render_details_content` (line 125). |
| `src/ui/step3/list_rows_step3.rs` | #6 | `render_step3_rows` lines 88-95: `if is_locked { theme::warning() } else { theme::text_disabled() }` for the lock-icon glyph — two-arm conditional, each accessor swaps. The compat pill (lines 153-186) receives pre-resolved colors from `compat_colors` (see the `tree_compat_display_step2.rs` row). The PROMPT pill (lines 187-203) is unconditional — swap `theme::prompt_text/fill/stroke` for `redesign_prompt_*(palette)`. Function gains `palette: ThemePalette` arg; threaded from `list_step3.rs::render` (caller). |
| ~~`src/ui/step3/content_step3.rs`~~ | ~~#6~~ — **DROPPED** | Phase-6 P6.T2d superseded this: Step-3's top chrome is net-new redesign code (`src/ui/workspace/step3/`); the orchestrator never calls `content_step3::render` / `render_toolbar`. Step-3's carve-out #6 surface reduces to `list_rows_step3.rs` only (rationale: see [`revision-log.md`](revision-log.md) 2026-05-17 Step-3 C4 entry). |
| `src/ui/step2/toolbar/toolbar_compat_step2.rs` | #6 | `draw_active_tab_issue_badge` lines 137-142: `compat_colors(kind_key).map(...).unwrap_or((theme_global::text_muted(), ui.visuals().widgets.inactive.bg_fill))` — the `unwrap_or` fallback branch carries one `theme_global::*()` accessor that swaps for `redesign_text_muted(palette)`. The `compat_colors` call itself receives `palette` (per the `tree_compat_display_step2.rs` row). Function gains `palette: ThemePalette` arg; threaded from `frame_step2.rs` toolbar invocation. |
| `src/ui/step5/content/content_install_row_step5.rs` | #6 | `render_install_row` lines 32-44: `if state.step5.prep_running { theme::accent_path() } else if state.step5.install_running { theme::accent_path() }` — two-branch state-conditional, both accessors swap for `redesign_accent_path(palette)`. (The two branches independently read the same color today; the swap preserves that.) Line 175: unconditional `theme::error()` inside the share-error sub-popup. Function gains `palette: ThemePalette` arg; threaded from `content_step5.rs::render` (line 35). |
| `src/ui/step5/content/content_cancel_step5.rs` | #6 + #2 | Line 19: `.collapsible(false)` → `.collapsible(true)` on the confirm-cancel egui::Window. Line 34: `theme::warning()` inside `if state.step5.cancel_force_checked` branch swaps for `redesign_pill_warn(palette)`. Function gains `palette: ThemePalette` arg; threaded from `content_install_row_step5.rs::render_install_row` (line 157). |
| `src/ui/step5/content/content_dev_header_step5.rs` | #6 | `render_dev_header` lines 28-32: `let color = if has_rust_log { theme::success() } else { theme::accent_path() }` — two-arm conditional, each accessor swaps for redesign counterparts. Function gains `palette: ThemePalette` arg; threaded from `content_step5.rs::render` (line 31). |
| `src/ui/step5/status/status_phase_step5.rs` | #6 | `compute_phase` lines 13-51: cascading `if/return` blocks each constructing `PhaseInfo { color: theme::* }` — six accessors total (`warning`, `accent_path`, `status_running`, `status_preparing`, `text_muted`, `status_idle`). Each accessor inside its branch swaps for the matching `redesign_*(palette)` accessor; the conditional cascade is preserved. `render_phase` lines 80-86: `let status_color = if status_text.starts_with("Install start failed:") || status_text.contains("os error") { theme::error() } else { theme::text_muted() }` — two-arm conditional, each accessor swaps. Both functions gain `palette: ThemePalette` arg; threaded from `status_bar_step5.rs::render_status_and_input` (lines 69, 81). |
| `src/ui/step5/status/status_console_step5.rs` | #6 | `token_color` lines 46-85: pre-resolves eight `theme_global::*` accessors as locals (`default`, `red`, `debug_blue`, `sent_blue`, `info_green`, `amber`, `sand`, `dim`), then dispatches via an `if n == ... return ...` cascade. The eight local accessor calls (lines 49-56) swap for `redesign_terminal_*(palette)` counterparts; the cascade is preserved exactly. `render_styled_line` lines 87-112: pre-resolves `theme_global::success()` at line 90; inner `let color = if success_line && (n == "SUCCESSFULLY" || n == "INSTALLED") { success_green } else { token_color(&token) }` is the conditional — `success_green` already a local resolved at line 90. Single accessor swap on line 90 (`success() → redesign_success(palette)`); structure preserved. Function `render_console_panel` (line 114) also gains `palette: ThemePalette` arg; threaded from `status_bar_step5.rs::render_console` (line 17). |

### BIO files audited and confirmed out of carve-out scope (no swap needed)

These files contain no `theme_global::*()` color calls and no inline `egui::Color32` literals at their render sites — nothing for carve-out #1 or #6 to substitute. Orchestration shims that have callees with color sites gain a `palette: ThemePalette` arg and thread it; pure layout / pure data-mapper files are no-touch.

- **Orchestration shims (gain `palette` arg, thread through):** `src/ui/step5/content/content_step5.rs`, `src/ui/step5/status/status_bar_step5.rs`, `src/ui/step2/tree/tree_render_step2.rs`, `src/ui/step3/list_step3.rs`, `src/ui/step5/top_panels_step5.rs`.
- **No-color renderers (no touch):** `src/ui/step2/details/details_meta_step2.rs` (data mapping), `src/ui/step2/update_check/update_check_popup_report_step2.rs` (text builder), `src/ui/step5/status/status_input_row_step5.rs`, `src/ui/step5/menus_step5.rs`, `src/ui/step5/prompts/prompt_answers_step5.rs` (+ `_table`, `_rows`, `_top_bar` siblings).
- **Out-of-orchestrator-path (carve-out #6 does not reach here):** `src/ui/app_nav_ui.rs` (action-handlers only; legacy nav rendering lives in `src/ui/frame/update_app.rs`, part of the `BIO` binary, not invoked by `OrchestratorApp` — orchestrator builds its own redesign-token-styled `workspace_nav_bar.rs` in Phase 6).

## Implementation tasks

### P8.T1 — Add the missing theme-token helpers

- **What:** Extend `src/ui/shared/redesign_tokens.rs` with `redesign_*(palette: ThemePalette) -> egui::Color32` accessors that mirror every BIO `theme_global::*` accessor consumed by the files above. The full set:
  - Text tones: `redesign_text_primary`, `redesign_text_muted`, `redesign_text_disabled`, `redesign_text_faint` (already present per Phase 1 P1.T2).
  - Accent tones: `redesign_accent_path`, `redesign_accent_numbers`, `redesign_accent_comment` (new — used in `format_step2.rs`, `format_step3.rs`, `content_install_row_step5.rs`, `status_phase_step5.rs`, `content_dev_header_step5.rs`).
  - Status / phase tones: `redesign_status_running`, `redesign_status_preparing`, `redesign_status_idle` (new — used in `status_phase_step5.rs`).
  - Success / warning / error: `redesign_success`, `redesign_success_bright`, `redesign_warning`, `redesign_warning_soft`, `redesign_warning_emphasis`, `redesign_warning_fill`, `redesign_warning_parent`, `redesign_error`, `redesign_error_emphasis` (extension of Phase 1 P1.T2's pill tones).
  - Compat fill tones: `redesign_conflict`, `redesign_conflict_fill`, `redesign_conflict_parent`, `redesign_included`, `redesign_included_fill`, `redesign_info`, `redesign_info_fill`, `redesign_game_mismatch`, `redesign_game_mismatch_fill`, `redesign_conditional`, `redesign_conditional_fill` (new — used in `tree_compat_display_step2.rs`).
  - Prompt pill tones: `redesign_prompt_text`, `redesign_prompt_fill`, `redesign_prompt_stroke` (new — used in `prompt_popup_step2.rs`, `tree_component_row_step2.rs`, `tree_parent_step2.rs`, `list_rows_step3.rs`).
  - Terminal tones: `redesign_terminal_default`, `redesign_terminal_error`, `redesign_terminal_debug`, `redesign_terminal_sent`, `redesign_terminal_info`, `redesign_terminal_amber`, `redesign_terminal_sand`, `redesign_terminal_dim` (new — used in `status_console_step5.rs`).
  - Each accessor follows SPEC §12.1 / §12.2 light + dark values. Where the SPEC doesn't provide an exact mapping, use the BIO source `Color32::from_rgb(...)` value as the Dark-theme default and derive a Light-theme value by inverting luminance (per SPEC §12.1's general rule).
- **Where:** Edit `src/ui/shared/redesign_tokens.rs` (a Phase-1 new file — additive accessors are unrestricted).
- **Acceptance:** Every `theme_global::*()` call in the files in the carve-out tables above has a one-to-one `redesign_*(palette)` counterpart. Unit tests verify both palette values return distinct `Color32` values per accessor.

### P8.T2 — Popup window-chrome flips (carve-out #2)

- **What:** In each `egui::Window` builder call in the popup-group files, change `.collapsible(false)` to `.collapsible(true)`. Per-file changes:
  - `compat_window_step2.rs` line 16
  - `prompt_popup_step2.rs` line 27 + line 123 (two windows)
  - `update_check_popup_step2.rs` line 67 + line 589 (confirm-latest-fallback) + line 631 (forks popup)
  - `update_check_popup_source_editor_step2.rs` line 20
  - `github_auth_popup_step1.rs` line 18
  - `content_cancel_step5.rs` line 19 (Step 5 confirm-cancel modal)
- **Where:** Each file gets a single-keyword flip per `.collapsible` call. No other edits.
- **Acceptance:** Each popup gains a working title-bar collapse chevron; clicking collapses the body while the title bar stays put (egui's native anchor; if it re-centers, escalate to P8.T6 wrapper).
- **SPEC:** §10 (Collapse chevron global popup pattern + Collapse direction).

### P8.T3 — Popups + unconditional sites (carve-out #1)

- **What:** Apply the carve-out #1 row from the inventory table. Each touched function gains a `palette: ThemePalette` parameter; call sites thread the palette through. Files in scope:
  - `prompt_popup_step2.rs` (5 unconditional swaps)
  - `update_check_popup_lists_step2.rs::render_section_header` (2 swaps)
  - `tree_parent_step2.rs::render_parent_row` PROMPT pill (3 swaps)
  - `format_step2.rs::colored_component_widget_text` (2 swaps)
  - `format_step3.rs::weidu_colored_widget_text` (3 swaps)
- **Where:** Edit each file. The `git diff` per file shows only: (a) the function's signature gains `palette: ThemePalette`, (b) call sites thread `palette` through, (c) inline `theme_global::*()` calls swap for `redesign_*(palette)` calls. **No new branches, no removed branches, no logic mutations.**
- **Acceptance:** Visual diff against `wireframe-preview/build.html` for the PROMPT pill and color-coded WeiDU lines shows redesign tokens applied. Light theme also produces correct values.
- **SPEC:** §6.7, §6.11, §10.4, §10.5, §13.2.

### P8.T4 — Step 2 tree + Details (carve-out #6 + #1 mixed)

- **What:** Apply the carve-out #6 swaps for the Step 2 tree + Details cluster. Files in scope:
  - `compat_popup_step2.rs` — match on `status_tone` (3 arms), filter row disabled fallback
  - `tree_compat_display_step2.rs` — `compat_colors` 9-arm match, `parent_compat_summary` 3-arm cascade
  - `tree_component_row_step2.rs` — `if effectively_disabled` branch + PROMPT pill unconditional
  - `tree_header_marker_step2.rs` — outer `if update_locked` + inner `match marker`
  - `details_pane_step2.rs` — `if ready` branch + warning unconditional
  - `details_paths_step2.rs` — `if value.is_none() && missing_amber` branch
  - `details_selection_step2.rs` — checked / state / reason rows
- **Where:** Edit each file. The `git diff` per file shows: signature changes adding `palette`, threaded call sites, and inline accessor swaps **inside the existing conditional structure only**. No new branches, no removed branches, no logic mutations.
- **Acceptance:** Open Step 2 in the workspace. Click various components. Verify: compat pills (conflict / included / mismatch / order / warning) all render in redesign tokens; the parent-summary pills (conflict / order / warning) render redesign tokens; the `?` Details pane reads redesign tokens; disabled components render in `redesign_text_disabled(palette)`. Light theme also produces correct values.
- **SPEC:** §6.

### P8.T5 — Step 3 reorder list (carve-out #6)

- **What:** Apply the carve-out #6 swaps for the Step-3 reused **list body only**. Files in scope:
  - `list_rows_step3.rs` — `if is_locked` branch + PROMPT pill unconditional
  - `toolbar_compat_step2.rs` — `unwrap_or` fallback for compat color (the compat-pill colour resolver `list_rows_step3.rs` consumes via `compat_colors`)
- **`content_step3.rs` is NOT in scope** — Phase-6 P6.T2d replaced Step-3's top chrome with net-new redesign code (`src/ui/workspace/step3/`, the C4 treatment); the orchestrator no longer calls `content_step3::render` / `render_toolbar`, so its tab/badge row + compat-rules-error label render only in the legacy `BIO` binary (un-reskinned by design). The Step-3 tab/badge/pill chrome the user sees is the **net-new redesign** `step3_tab_row` (already redesign-token-styled in Phase 6) — nothing for carve-out #6 to swap there.
- **Where:** Edit `list_rows_step3.rs` + `toolbar_compat_step2.rs`. Same diff bar as P8.T4.
- **Acceptance:** Open Step 3 (the orchestrator's C4 chrome from P6.T2d). Drag components to create / break conflicts. Verify in the **reused list rows**: per-row compat pills render redesign tokens; lock-icon glyph renders `redesign_warning(palette)` when locked, `redesign_text_disabled(palette)` when unlocked; the PROMPT pill renders redesign tokens. (The aggregate tab conflict/prompt pills are the net-new `step3_tab_row` redesign pills — already correct from Phase 6, not a P8.T5 concern.) Light theme also produces correct values.
- **SPEC:** §7.

### P8.T6 — Step 5 sub-renderers (carve-out #6 + a few #1 + #2)

- **What:** Apply the carve-out #6 + #1 + #2 swaps for Step 5. Files in scope:
  - `status_console_step5.rs` — 8 terminal-tone locals + success-line conditional (carve-out #6)
  - `content_install_row_step5.rs` — `if prep_running / else if install_running` branches + share-error unconditional
  - `content_cancel_step5.rs` — `if cancel_force_checked` warning branch + `.collapsible` flip
  - `content_dev_header_step5.rs` — `if has_rust_log` branch
  - `status_phase_step5.rs` — `compute_phase` cascade + `render_phase` error-state branch
  - Plus the orchestration shims (`content_step5.rs`, `status_bar_step5.rs`, `top_panels_step5.rs`) gain a `palette` arg they thread through, even though they themselves do no color swaps.
- **Where:** Edit each file. Same diff bar.
- **Acceptance:** Start an install. Verify: console line tones (success / error / warning / debug / info / sent / sand / dim / default) all render redesign tokens; the install-state banner (Preparing / Installing) renders redesign tokens; the dev-mode RUST_LOG status line renders redesign tokens; the phase indicator (Idle / Preparing / Running / Waiting Input / Cancelling / Finished) renders redesign tokens; the cancel-confirm warning renders redesign tokens. Light theme also produces correct values.
- **SPEC:** §8, §9, §9.2.

### P8.T7 — Verify anchor-on-collapse behavior

- **What:** Per SPEC §10 ("If egui's native collapse already anchors the window's top-Y…"), test the egui default after the P8.T2 flips. If egui re-centers vertically when the height shrinks, route the affected popups through `widgets/popup_collapse_anchor.rs`, a thin wrapper that captures `Window::current_pos` at the collapse transition and pins the window there. The wrapper is a **new file** — it does not modify BIO source.
- **Where:** New file `src/ui/orchestrator/widgets/popup_collapse_anchor.rs` (if needed). The wrapper is invoked from the popup call site, which for shared popups lives in the orchestrator's `render_shared_popups` analogue (Phase 2). If the popup is invoked from inside a BIO file, the wrapper cannot be threaded in without modifying that file — in which case the default is (a) accept egui's native behavior. The `.collapsible(true)` flip itself is already authorized by carve-out #2 (single-keyword change); threading an additional wrapper through the BIO invocation site would exceed that scope, so the fallback is acceptance, not a new carve-out. Escalate only if SPEC §10's "title bar stays put" is non-negotiable for a popup whose invocation lives inside a BIO file.
- **Acceptance:** Collapse any popup — title bar stays put. Expand it — body reappears below.
- **SPEC:** §10 (Collapse direction).

### P8.T8 — Automatic flag policies #2, #3, #4, #6, #7

- **What:** Implement remaining policies from SPEC §13.12:
  - #2: `-u` always ON, path = `<destination>/weidu_component_logs/`. Create directory at install start.
  - #3: For EET installs, create `<destination>/Baldur's Gate Enhanced Edition/` and `<destination>/Baldur's Gate II Enhanced Edition/`; pass them via `-p` and `-n`. Clone the source-game folders into them.
  - #4: For single-game installs, create `<destination>/<game-folder-name>/` and clone the source game into it; pass via `-g`.
  - #6: Map `DestChoice::Clear` → `prepare_target_dirs_before_install = true`, backup off; `DestChoice::Backup` → both true; `DestChoice::Continue` → both false (skip).
  - #7: `-autolog`, `-logapp`, `-log-extern` always ON; no surface.
- **Where:** New files `src/install_runtime/flag_policies_eet.rs`, `flag_policies_single_game.rs`, `flag_policies_log.rs`, `flag_policies_dest_choice.rs`. Each is called from `install_runtime/start_hooks.rs::on_install_start` (Phase 7) before delegating to BIO. They write into the existing `pub` fields of `Step1State` / `Step5State` that BIO's install flow already reads (`weidu_log_folder`, `eet_pre_dir`, `eet_new_dir`, `prepare_target_dirs_before_install`, `backup_targets_before_eet_copy`, etc.).
- **Acceptance:** Inspect the WeiDU command line in BIO's Command card for each combination; every policy is respected. Confirm that the existing BIO state fields these policies populate are set correctly before BIO's install flow runs.
- **SPEC:** §13.12.

### P8.T9 — Dotted radial background

- **What:** Implement `src/ui/shared/redesign_dot_background.rs` with `pub fn paint_dot_background(painter: &egui::Painter, rect: egui::Rect, palette: ThemePalette)`. Uses `painter.circle_filled` at a 20px grid with 1px dots in `redesign_dot(palette)` color. Called from `shell_chrome::render_shell` body's first paint pass (Phase 1 file).
- **Scroll behavior:** The dot pattern is painted into the shell's **outermost frame** (behind the entire app shell, not inside any scrollable region). It is **fixed** — does not scroll with content. Matches wireframe `index.html:64-76` where the radial-gradient is on `body`, not on `.sk-main`. Use `egui::Context::layer_painter` at a background layer-id to draw the pattern under everything else.
- **Where:** New file + edit Phase-1 `shell_chrome.rs` (editable as a Phase-1 new file).
- **Acceptance:** The page background shows the subtle dot pattern at 20px spacing, matching the wireframe.
- **SPEC:** §12.3.

### P8.T10 — Toast notification rollout

- **What:** Move Phase 5's Home toast helper to `src/ui/orchestrator/widgets/toast.rs`. Wire toasts for: clipboard copies (already in Home), Settings → "Validation complete" (optional), workspace → "✓ saved!" inline button flash (already in Phase 6), share dialog `✓ copied to clipboard` inline (already in Phase 7). Audit every "this happened" feedback site to ensure consistency.
- **Where:** Move Phase-5 `home/toast.rs` → `orchestrator/widgets/toast.rs`; update consumer imports.
- **Acceptance:** All toasts auto-dismiss at ~1.8s. Visual style identical across sites.
- **SPEC:** §10.8.

### P8.T11 — Hover affordances polish

- **What:** Audit hover behavior across all redesign widgets per SPEC §12.3:
  - Tree rows (BIO Step 2 rows now read redesign tokens; their **hover-overlay tint** is rendered via `ui.selectable_label`'s default hover behavior — egui's runtime visuals are updated by the orchestrator at frame start per Phase 1's "Palette ownership and per-frame propagation", so the hover tint already picks up the redesign palette).
  - Pills: 1px hover-lift + brightness boost + 2px drop-shadow. Pills inside BIO files (`tree_component_row_step2.rs`, `tree_parent_step2.rs`, `list_rows_step3.rs`, `compat_popup_step2.rs::render_filter_row`) render via `egui::Button` — egui's hover already lifts and brightens; the drop-shadow is rendered by the redesign button widget when the call site uses it. **BIO-file pills retain egui's default hover behavior**; the wireframe-style 2px drop-shadow is delivered only through orchestrator-side widgets. SPEC §12.3 does not require the in-BIO pills to upgrade to drop-shadow rendering.
  - Details rows: `[?]` action button reveal on hover or selected — orchestrator-side widgets only.
  - Buttons: subtle press-down on click (`transform: translate(1px, 1px)`) — orchestrator-side widgets only.
- **Where:** Edit redesign widget files in `src/ui/orchestrator/widgets/` only. The BIO Step 2/3/5 renderers keep their existing hover behavior (the carve-out #6 swaps already give them the redesign tones); their hover-overlay tint comes from egui's runtime visuals.
- **Acceptance:** Visual diff against the wireframe `index.html` hover CSS rules (lines 198–218) confirms parity in the orchestrator-side widgets. The BIO renderers' hover overlays use redesign colors.
- **SPEC:** §12.3.

### P8.T12 — Copy-to-clipboard everywhere

- **What:** Route all SPEC-mandated copy actions through `src/ui/orchestrator/widgets/clipboard.rs::copy(text)` (using `arboard` or `egui_extras`):
  - DetailsRow value column (Step 2 details panel) — **note:** the BIO details renderer's existing copy behavior continues to work via `ui.ctx().copy_text` in `details_paths_step2.rs` and `details_pane_step2.rs`. This is read-only consumption from the orchestrator's perspective.
  - Step 5 Command card Copy button — the Command card is part of BIO's embedded `page_step5::render`, so its existing copy behavior continues to work.
  - SharePasteCodeDialog Copy button (Phase 7).
  - Home Kebab → Copy import code (Phase 5).
  - Workspace WeiDU Line collapsible row's copy icon (Phase 6).
- **Where:** New file `widgets/clipboard.rs` + audit call sites in orchestrator widgets only.
- **Acceptance:** Every copy action writes to clipboard and shows the SPEC-specified inline confirmation (`Copied!` tag, `✓ copied to clipboard`, toast).
- **SPEC:** §6.8.4, §9.1 (Command card Copy), §10.3 (SharePasteCodeDialog), §3.1 (Home toast).

### P8.T13 — Final smoke pass

- **What:** Walk through every screen and every flow listed in earlier phases' Verification sections. Confirm no regressions from earlier phases. Verify the wireframe visual fidelity rule (SPEC §1) — when in doubt, the wireframe wins. Specifically confirm:
  - Every workspace surface (Step 2 tree, Step 2 Details, Step 3 list, Step 5 console + install row + status row + dev header + cancel-confirm) renders in redesign tokens under both Light and Dark palettes.
  - The popup group (Compat, Prompt, Update Check + sub-dialogs + forks + source editor, GitHub OAuth) renders in redesign tokens and shows the collapse chevron.
  - The legacy `BIO` binary still compiles and runs; its nav buttons still render in BIO-default colors (no regression from the fact that `update_app.rs` was NOT modified).
- **Where:** Manual.
- **Acceptance:** All earlier-phase Verification checklists pass.

### P8.T14 — Per-modlist data ownership refactor

- **What:** Implement SPEC §13.12b. Umbrella covering five sub-tasks T14.1–T14.5: the in-memory workflow-state refactor, per-install path derivation off `wizard_state.step1`, the per-modlist file-storage plumbing, the consumer swap from global TOMLs to per-modlist TOMLs, and the orchestrator-side share-code import replacement (with the accompanying carve-out #5 schema-additive `compat_overrides` field). Promotes `.claude/reference/state-architecture.md`'s "Shape of a clean refactor" candidate (a)/(b)/(c) from descriptive to authorized and extends it with the on-disk artifact piece.
- **SPEC:** §13.12b. Cross-refs: §13.3 (Compat-rule overrides), §1 carve-out #5 (Compat-rule overlay application).
- **Dependencies:** Phases 1–7 plus P8.T1–P8.T13 (the visual-fidelity sweep) merged.

#### P8.T14.1 — `CurrentWorkflow` enum + `transition_to`

- **What:** Implement the single workflow tag on `OrchestratorApp` and the single transition function. Six variants: `Idle`, `Paste`, `CreateScratch`, `ForkImport`, `WorkspaceEdit`, `Reinstall`. Each carries the data today scattered across `install_screen_state` (`InstallScreenState`), `create_screen_state` (`CreateScreenState`), `active_install_modlist_id: Option<String>`, `pending_reinstall_id: Option<String>`, `post_install_reset_gate: PostInstallResetGate`. `transition_to(&mut self, new: CurrentWorkflow)` pattern-matches on `(self.workflow, &new)` to choose what to persist from the old variant, which subsystem cleanup runs, and what to arm in the new variant.
- **Where (new):** `src/ui/orchestrator/workflow/{mod,current_workflow,transition}.rs`.
- **Where (modified, orchestrator-only):** every call site of the seven retired reset functions, in `orchestrator_app.rs`, `page_router.rs`, `page_create.rs`, `stage_fork_download.rs`. Each becomes one `app.transition_to(...)` call.
- **Retires (deletes — verified by grep):** `OrchestratorApp::reset_install_screen_to_paste()`, `page_router::reset_completed_install_runtime()`, `page_router::flush_workspace_on_nav_away()`, `page_router::clear_pending_reinstall_on_nav_away_from_install()`, `page_create::fork_extract_complete_route_to_workspace()`, the partial-clear block inside `stage_fork_download::render_live`, and the `!from_workspace` partial-clear in `maybe_flip_to_installed_on_clean_exit`. The shared `InstallScreenState` struct retires; its data moves onto the workflow variants. BIO's `reset_workflow_keep_step1()` STAYS (BIO source, called from the appropriate `transition_to` arm).
- **Acceptance:** `git grep -E "reset_install_screen_to_paste|reset_completed_install_runtime|flush_workspace_on_nav_away|clear_pending_reinstall_on_nav_away_from_install|fork_extract_complete_route_to_workspace|post_install_reset_gate|InstallScreenState"` returns empty in orchestrator-owned files. Item #4's six-run regression scenarios (R1 double-prepare, wrong-write-path, post-install banner wipe, etc.) all still pass under their existing regression tests. New tests cover the full matrix of `(from, to)` variant transitions.
- **SPEC:** §13.12b (in-memory).

#### P8.T14.2 — Per-install paths derived on-demand

- **What:** Move per-install path derivation off the persistence path. The 11 per-install string fields (`mods_folder`, `weidu_log_folder`, `bgee_log_folder` / `bg2ee_log_folder` / `eet_bgee_log_folder` / `eet_bg2ee_log_folder`, `bgee_log_file` / `bg2ee_log_file`, `eet_pre_dir` / `eet_new_dir`, `generate_directory`) + the 5 booleans (`weidu_log_log_component`, `have_weidu_logs`, `new_pre_eet_dir_enabled`, `new_eet_dir_enabled`, `generate_directory_enabled`) + the `weidu_log_mode` token are computed on-demand from the active workflow variant's destination + game + `AppSettings` / `RedesignSettings`, and written into `wizard_state.step1` immediately before BIO's install runner reads. Never round-trip through `bio_settings.json`.
- **New helper:** `install_runtime::derive_step1::derive_step1_for_install(workflow: &CurrentWorkflow, settings: &AppSettings, redesign_settings: &RedesignSettings) -> Step1ForInstall` returning the 16 fields + the token; called from `install_runtime::start_hooks::on_install_start` just-in-time. The existing `derive_per_install_dirs` may stay as the internal derivation engine; the new gate is the just-in-time write.
- **Where (new):** `src/install_runtime/derive_step1.rs`.
- **Where (modified, orchestrator-only):** `src/install_runtime/start_hooks.rs` (the new just-in-time call), `src/registry/workspace_state_loader.rs::populate_wizard_state_from_workspace` (drop the per-install-path sync into step1; the global `sync_paths_from_settings` for genuine global fields stays). `OrchestratorApp::bio_settings_snapshot` no longer needs the sanitizer pass.
- **Retires:** `install_runtime::settings_sanitizer` module + `sanitize_step1_for_settings_persistence` function — no longer needed, per-install fields literally can't be in settings.
- **Acceptance:** `bio_settings.json` is byte-identical pre / post a full install cycle for every workflow variant. The `settings_sanitizer` module is absent from `git grep`. Run-6's wrong-write-path regression scenario passes — a Step-4 → Step-5 nav with a Settings edit mid-flight does not clobber the per-install destination.
- **SPEC:** §13.12b (in-memory).

#### P8.T14.3 — Per-modlist data resolver + typed overlay modules

- **What:** Build the per-modlist file storage plumbing. A `DataOverlayKind` enum (`ModDownloadsUser`, `CompatRulesUser`, `InstalledSourceRefs`) + a resolver `redesign_data_paths::per_modlist(modlist_id: &str, kind: DataOverlayKind) -> PathBuf` returning `<config_dir>/bio/modlists/<modlist_id>/<filename>`. Each overlay gets a typed load / save module with the same atomic temp-file-then-rename writes as `WorkspaceStore`. BIO's global resolvers (`bio::app::mod_downloads::mod_downloads_user_path`, equivalents) are not called from orchestrator code; they stay valid for the legacy `BIO` binary.
- **DATA-LOSS invariant (mandatory):** the resolver MUST expose a temp-path injection variant — `per_modlist_with_root(root: &Path, modlist_id: &str, kind: DataOverlayKind) -> PathBuf` — and every typed load / save module MUST take an explicit root or store handle parameter that tests can repoint at a `std::env::temp_dir()` path. The same precedent as `RegistryStore::new_default()` test repointing (the `dev_seed.rs` `temp_path()` mechanism). **No unit test may bind the real config dir** — that is a directive-grade failure on the level of a BIO-source-guard hit (per the orchestrator skill's DATA-LOSS invariant). Tests that call the no-root convenience resolver must use a `tempfile::TempDir` for the entire test body's writes.
- **Where (new):**
  - `src/registry/data_overlays/mod.rs` — `DataOverlayKind` + the `per_modlist` + `per_modlist_with_root` resolvers
  - `src/registry/data_overlays/mod_downloads_user.rs` — typed load / save (root-injectable)
  - `src/registry/data_overlays/compat_rules_user.rs` — same shape
  - `src/registry/data_overlays/installed_source_refs.rs` — same shape
- **Persistence wiring (modified):** `src/registry/persistence_cycle.rs` — extend the per-modlist debounce machinery to handle the three new overlay files alongside each modlist's `workspace.json`. Dirty-bits live alongside the workspace dirty-bit; drop-time `flush_all_now` flushes all four. No BIO source touched.
- **Acceptance:** Unit tests assert resolver paths for each kind, **using only the temp-path variant — verified by `git grep` showing no `per_modlist(` call in test modules without a temp-root pairing**. Save → load round-trip byte-identical for each TOML. The DATA-LOSS sentinel widens to include the three new per-modlist files (any existing modlist's three TOMLs byte-stable across any test run).
- **SPEC:** §13.12b (on-disk).

#### P8.T14.4 — Consumers swap to per-modlist resolver

- **What:** Redirect the consumers that today read / write the three global TOMLs to use the per-modlist resolver:
  - **Set Source save action.** `Step2Action::SaveModDownloadSourceEditor` is intercepted at the orchestrator's Step-2 wrapper layer; the side-effect routes to `data_overlays::mod_downloads_user::save(active_modlist_id, ...)`. BIO's `app_step2_router::set_mod_download_source` + `bio::app::mod_downloads::save_user_mod_download_source_block` are no longer reached on the orchestrator path. `state.step2.selected_source_ids` is unchanged — already per-WizardState which is per-modlist.
  - **Compat-rule user additions.** Identify the call site in `src/core/app/compat/` that writes `step2_compat_rules_user.toml` (locate during impl). Intercept at the orchestrator's Step-2 / compat-popup wrapper; route to `data_overlays::compat_rules_user::save`. If today's UI has no user-write path (file is edit-on-disk only), the write surface is added net-new here.
  - **Installed-source-refs post-install write.** The post-install write of `installed_source_refs.toml` (verify exact call site — likely in the install-completion path) is intercepted in `install_runtime::start_hooks` / `flip_to_installed`; routes to `data_overlays::installed_source_refs::save(modlist_id, ...)`.
  - **Share-code export reads.** `src/registry/share_export.rs` (the `pack_meta` envelope inputs) gains parallel reads via the per-modlist resolver. BIO's `export_modlist_share_code` is composed unchanged; only the on-disk source files it reads are routed per-modlist.
- **Where (modified, orchestrator-only):** `src/ui/workspace/step2/*` (action interception), `src/install_runtime/start_hooks.rs` (post-install write redirect), `src/registry/share_export.rs` (export reads per-modlist). No BIO source touched.
- **Acceptance:** Set Source in modlist A's workspace writes to `<config_dir>/bio/modlists/<A>/mod_downloads_user.toml`; modlists B / C / D's per-modlist files + the global file are byte-stable. Equivalent assertions for compat-rule additions and post-install installed-refs writes. Share-code export of modlist A reads modlist A's per-modlist TOMLs, NOT the global file (verifiable: a global file with arbitrary content does not appear in modlist A's exported share code).
- **SPEC:** §13.12b (on-disk + share-code).

#### P8.T14.5 — Orchestrator-side `import_modlist_share_code` replacement + `compat_overrides` schema-additive field

- **What:** Net-new orchestrator-side share-code import that decodes the BIO-MODLIST-V1 payload and routes the embedded user-overlay TOMLs (`source_overrides.mod_downloads_user_toml`, `installed_refs.mod_installed_refs_toml`, `compat_overrides.step2_compat_rules_user_toml`) to per-modlist storage for the importing modlist — never to the global files. Other payload sections ride existing paths: `weidu_logs` + `mod_configs` go to per-install destination locations BIO already handles; the provenance trio (`name`, `author`, `forked_from`) + `allow_auto_install` + `archive_meta` ride the registry / workspace.json. Then drives the existing auto-build / scan / install pipeline.
- **Schema-additive field (carve-out #5).** Add `compat_overrides: ModlistShareCompatOverrides` to `ModlistSharePayload` + `ModlistSharePreview` in `src/core/app/modlist_share.rs` per SPEC §1 carve-out #5 "Compat-rule overlay application". Nested struct `ModlistShareCompatOverrides { step2_compat_rules_user_toml: Option<String> }` defined alongside the existing `ModlistShareSourceOverrides` / `ModlistShareInstalledRefs`. One propagation line in `share_preview()`. Default (empty, `None` excerpt) preserves today's BIO behavior bit-for-bit. BIO's `import_modlist_share_code` ignores the field via `#[serde(default)]` — the legacy `BIO` binary's importer does not write compat overrides to disk.
- **Composes** (does not patch) the existing envelope codec the orchestrator already owns for `pack_meta` / `set_allow_auto_install`. BIO's private `base64url_*` / `zlib_*` / `decode_share_payload` are not touched.
- **Where (new):** `src/install_runtime/import_share_code.rs`.
- **Where (modified, orchestrator-only):** the three orchestrator entry points that today call BIO's `import_modlist_share_code` — Install-Modlist-paste arm (`stage_downloading::arm_pipeline_once`), Create-fork arm (`install_runtime::fork_pipeline_arm::mint_and_arm`), Reinstall arm (`page_router::reinstall_route`) — swap their import call for the new orchestrator-side version. BIO's `import_modlist_share_code` stays valid (still used by the legacy `BIO` binary's Step-1 flow).
- **Where (modified, BIO source under carve-out #5):** one field addition + one nested struct definition + one propagation line in `src/core/app/modlist_share.rs`. Per SPEC §1 carve-out #5 "Compat-rule overlay application" — same shape as the existing provenance application.
- **Acceptance:** A fork-import from a share code with non-default `mod_downloads_user.toml` content writes that content to `<config_dir>/bio/modlists/<new-fork-id>/mod_downloads_user.toml`; the user's other modlists' per-modlist TOMLs + the global TOML are byte-stable. Equivalent assertion for `installed_refs` and `compat_overrides`. **Cross-modlist-overwrite hazard closed** — two modlists with distinct Set Source overrides, then fork-import a third, leaves both originals byte-stable. `bio::app::modlist_share::import_modlist_share_code` does not appear in orchestrator-side `git grep`.
- **SPEC:** §13.12b (on-disk + share-code) + §13.3 Compat-rule overrides + §1 carve-out #5 Compat-rule overlay application.

#### Landing strategy

Two-PR split: Part A = T14.1 + T14.2 + T14.3 (foundation — in-memory + on-disk plumbing); Part B = T14.4 + T14.5 (consumer swap + import replacement + the carve-out #5 schema-additive field). Part A is internally consistent (plumbing exists but unused — global files still drive everything). Part B switches consumers, lands the BIO carve-out #5 edit, and closes the cross-modlist-overwrite hazard. One-PR landing is fine too; the user holds the call.

#### Verification (cross-subtask)

1. `cargo build --bin infinity_orchestrator --release` no-op rebuild at end of each subtask + at the umbrella close.
2. `cargo build --bin BIO --release` continues to succeed; the legacy binary's behavior is unchanged.
3. `cargo test --lib` count holds + each subtask's new tests land green.
4. `git grep` for each retired identifier returns empty in orchestrator-owned files (the seven reset functions; `InstallScreenState`; `post_install_reset_gate`; `settings_sanitizer`; `bio::app::modlist_share::import_modlist_share_code` on orchestrator paths).
5. **Widened DATA-LOSS sentinel** — `modlists.json` + every modlist's `workspace.json` + every modlist's three per-modlist TOMLs are byte-stable across the test suite, across a full modlist-A install, and across a fork-import targeting a fresh modlist-D.
6. Live re-test of every Item #4 regression scenario passes.
7. `.claude/reference/state-architecture.md` moves from "descriptive of what exists today" to "historical: superseded by SPEC §13.12b + plan P8.T14". The doc's "What future agents should NOT do" cautions about `install_screen_state` reuse and the seven scattered reset functions become obsolete under this refactor — readers post-T14 should treat that section as describing the pre-T14 hazard, not active guidance.
8. The `compat_overrides` field on `ModlistSharePayload` + `ModlistSharePreview` is the only BIO-source edit; verified by inspecting the touched-files diff against the BIO-source path set.

#### File inventory summary

**New (orchestrator-owned, net-new):**
- `src/ui/orchestrator/workflow/{mod,current_workflow,transition}.rs`
- `src/registry/data_overlays/{mod,mod_downloads_user,compat_rules_user,installed_source_refs}.rs`
- `src/install_runtime/derive_step1.rs`
- `src/install_runtime/import_share_code.rs`

**Modified (orchestrator-owned, additive):** `orchestrator_app.rs`, `page_router.rs`, `page_create.rs`, `stage_*.rs` (install + fork stages), `src/ui/workspace/step2/*`, `start_hooks.rs`, `fork_pipeline_arm.rs`, `share_export.rs`, `workspace_state_loader.rs`, `persistence_cycle.rs`.

**BIO source touched (under existing carve-out #5):** `src/core/app/modlist_share.rs` — one schema-additive field on `ModlistSharePayload` + symmetric field on `ModlistSharePreview` + one propagation line in `share_preview()` + the `ModlistShareCompatOverrides` nested struct definition. No other BIO source touched.

## Open questions / risks

- **Token extraction scope creep.** During extraction it's tempting to also rename a function, move a constant, simplify a branch, add a helper, or "clean up" the conditional structure. **All forbidden** by carve-outs #1 and #6. If a token extraction surfaces a clear bug or improvement opportunity, file an issue; do not change behavior in this phase. The bar for each touched BIO file is: the `git diff` shows only value substitutions inside existing branches, signature additions of `palette: ThemePalette`, and (where applicable) one `.collapsible()` keyword flip — nothing else. **No new branches, no removed branches, no reordered branches, no logic mutations.**
- **Light theme coverage.** Some BIO theme_global accessors don't have a SPEC §12.1 light counterpart (the SPEC focuses on shell-bg / accent / text tones; compat fill tones aren't enumerated). Where the SPEC is silent, the planner derives a Light-theme value by inverting the Dark-theme value's luminance, matching the wireframe's `index.html` Light palette where applicable. New `redesign_*` accessors live in `redesign_tokens.rs` / `theme_global.rs`.
- **Embedded terminal coloring.** The `EmbeddedTerminal` widget (`third_party/egui_term`) has its own internal color handling. Carve-out #6 does not reach inside the terminal widget. Acceptable scope for Phase 8: skin the surrounding Console Box and the line-tone classifier in `status_console_step5.rs` (P8.T6); the terminal's internal cell colors stay BIO-default. SPEC §9 doesn't require the terminal to recolor.
- **`.collapsible(true)` interactions with focus traps.** Today's BIO popups use `.collapsible(false)` because they behave like modal dialogs. Flipping to `.collapsible(true)` should be safe (collapse only affects body visibility, not modality), but verify no popup state breaks when collapsed mid-operation (e.g., collapsing the Update Check popup while a check is running). Test before merging each flip.
- **Per-frame palette propagation.** Phase 1 P1.T2's "no global theme value" rule means every render function on the carve-out path gains a `palette: ThemePalette` parameter. This is a wide-but-shallow signature change — many files, one new arg each. The orchestrator's frame entry (`OrchestratorApp::update`) reads `self.theme_palette` and threads it through `workspace_step_router::render(ui, self, palette)` → `page_stepN::render(ui, &mut state, dev_mode, exe_fingerprint, palette)` → all sub-renderers. The `palette: ThemePalette` add is the only signature change per BIO function; otherwise functions stay byte-identical.
- **Carve-out #6 cliff edge.** A few sites read `ui.visuals().widgets.*` or `ui.visuals().text_color()` instead of a `theme_global::*` accessor (e.g., `tree_component_row_step2.rs::render_filter_row` lines 195-203 use `visuals.widgets.active.bg_fill` / `bg_stroke`). These are **already palette-driven** because Phase 1 P1.T2's per-frame palette application writes the redesign palette into `egui::Context::style_mut()` at the start of every frame. So they automatically pick up redesign colors without any source change. The carve-out #6 scope is strictly about explicit `theme_global::*()` accessor calls — anything reading egui's runtime style is already covered.

## P8.T15 — Step 3 list-body wireframe fidelity (Item 2)

### Background and constraint

The Step-3 list body is currently rendered by BIO's `list_step3::render` → `list_rows_step3::render_rows`, called from the orchestrator's net-new chrome at `workspace_step3.rs:65`. The existing P8.T5 task (carve-out #6) addresses color-token swaps only. This task addresses the remaining visual gaps between the rendered list body and the wireframe `ComponentsPanel` (`screens.jsx:3057–3172`).

**Hard constraint (no exceptions):** no modification to drag/reorder logic, undo/redo stack, block-selection rules, locked/collapsed block sets, the `Step3ItemState` struct (`src/core/app/state/state_step3.rs`), the per-modlist `workspace.json` schema, or any service function that mutates state. All changes are render-side only.

### Q1 — Reusability audit of BIO services

All four `pub(crate)` functions in `service_drag_ops_step3.rs` (re-exported via `service_step3::drag_ops`) are reachable from orchestrator-owned code under carve-out #3's `pub(crate)` reachability:

| Function | Reachable? | Notes |
|---|---|---|
| `finalize_on_release` (`service_drag_ops_step3.rs:46`) | YES | Takes `DragFinalizeContext`, all fields `pub(crate)` |
| `draw_insert_marker` (`service_drag_ops_step3.rs:93`) | YES | Takes `&egui::Ui` + plain slices |
| `update_drag_target_from_pointer` (`service_drag_ops_step3.rs:125`) | YES | Takes `DragPointerContext`, all fields `pub(crate)` |
| `apply_live_reorder` (`service_drag_ops_step3.rs:167`) | YES | Takes `LiveReorderContext`, all fields `pub(crate)` |
| `block_selection_step3::single_child_main_parent_block_indices` (line 9) | YES | `pub(crate)` |
| `block_selection_step3::selected_full_main_parent_block_indices` (line 25) | YES | `pub(crate)` |
| `service_component_uncheck_step3::apply_component_unchecks` (line 6) | YES | `pub(crate)` |
| `service_prompt_actions_step3::apply_prompt_actions` (line 9) | YES | `pub(crate)` |
| `service_prompt_actions_step3::render` (line 13) | YES | `pub(crate)` |
| `service_step3::apply_row_selection` (`service_step3.rs:7`) | YES | `pub fn` |

No visibility flips are needed. All functions the sibling renderer requires to delegate behavior are already reachable.

### Q2 — Inline render-side mutations in `list_rows_step3.rs`

The following mutations happen inside `list_rows_step3.rs` without going through a service function, and a sibling must replicate them:

| Location | Mutation | Egui response point | Drift risk / note |
|---|---|---|---|
| `update_locked_blocks` (`list_rows_step3.rs:165-173`) | Toggles `locked_blocks` vec | `.clicked()` on lock-icon small-button | Low: two-line vec mutate; trivial to replicate |
| `render_parent_toggle` (`list_rows_step3.rs:175-198`) | Pushes/retains `collapsed_blocks` | `CollapsingState::show_toggle_button` toggle | Low: pure egui CollapsingState dance; pattern stable |
| `handle_drag_start:update_drag_indices` (`list_rows_step3.rs:440-458`) | Writes `drag_from`, `drag_indices`, `selected`, clears them on locked | `.drag_started()` on `drag_response` | Medium: logic references `selected_full_main_parent_block_indices` + `single_child_main_parent_block_indices` + `blocks::block_indices` — all reachable; replicate exactly |
| `handle_drag_start:update_drag_grab_geometry` (`list_rows_step3.rs:461-477`) | Writes `drag_grab_offset`, `drag_grab_pos_in_block`, `drag_row_h` | `.drag_started()` on `drag_response` | Low: pure geometric computation off `visible_rows` |
| `handle_drag_start` (`list_rows_step3.rs:436-438`) | Sets `last_insert_at = None`, `drag_over = Some(idx + 1)` | `.drag_started()` | Low: two-line init |
| `render_parent_context_menu` (`list_rows_step3.rs:327-338`) | Calls `step3_history::push_undo_snapshot` + `blocks::clone_parent_empty_block` | `.context_menu` > button `.clicked()` | Low: delegated to `pub(crate)` helpers; replicate the two-line call |
| `handle_row_selection` (`list_rows_step3.rs:397-414`) | Delegates to `service_step3::apply_row_selection` | `.clicked()` on label or drag response | No inline mutation; already a service call |
| `handle_jump_to_selected` (`list_rows_step3.rs:385-395`) | Calls `ui.scroll_to_rect` + clears `jump_to_selected_requested` | per-row, after push to `visible_rows` | Low: scroll helper; replicate two-line guard |

No mutation requires a new carve-out or additive helper extraction. All are short-enough to replicate with zero drift risk given the hard constraint that their logic is not touched.

### Q3 — Drag orchestration sequence (contract for the sibling)

Per-frame sequence for a drag cycle, derived from `list_rows_step3.rs` + `list_step3.rs`:

1. **Setup** (`list_step3.rs:94-140`): `state_step3::active_list_mut(state)` extracts all drag/selection state as mutable refs; `blocks::visible_indices` computes which rows are visible given `collapsed_blocks`.
2. **Row render loop** (`list_rows_step3.rs:53-58`): for each visible index, call `render_row` — renders the label, detects drag-start, detects click-selection.
   - **Inside each row** (`list_rows_step3.rs:98-112`): `render_row_label` (paint visual) → `ui.interact` on label rect with `Sense::click_and_drag()` → `render_row_context_menu` → push to `visible_rows` → `handle_jump_to_selected` → `handle_row_selection` → `handle_drag_start`.
   - **handle_drag_start** (fires when `drag_response.drag_started()`): locks check → `push_undo_snapshot` → write `drag_from` → `update_drag_indices` → `update_drag_grab_geometry` → clear `last_insert_at` → set `drag_over = Some(idx + 1)`.
3. **Post-loop drag pipeline** (`list_step3.rs:141-155`): after `render_rows` returns `row_outcome`:
   - `update_drag_target_from_pointer(ui, &mut pointer_ctx)` — reads pointer position, recomputes `drag_over`.
   - `draw_insert_marker(ui, items, drag_active, drag_over, visible_rows)` — paints the 2px teal drop-line.
   - `apply_live_reorder(ui, &mut reorder_ctx)` — if pointer still down and `drag_over` changed, splices `items` in-place; updates `selected` and `drag_indices` to new positions.
   - `finalize_on_release(ui, &mut finalize_ctx)` — on pointer-release: clears drag state, calls `repair_orphan_children` + `merge_adjacent_same_mod_blocks` + `prune_empty_parent_blocks`; rebuilds `selected` from key-stable IDs.
4. **Outcome application** (`list_step3.rs:223-257`): compat/prompt popup open requests + uncheck requests + prompt action requests are flushed to state.

The sibling must replicate steps 2-3 in its own row renderer; step 4 is identical (`apply_row_outcome` is a standalone function the sibling can call or inline).

### Q4 — Visual fidelity gap (wireframe vs. current render)

Gaps between `ComponentsPanel` (`screens.jsx:3057–3172`) and the current list render:

| Element | Wireframe value (`screens.jsx`) | Current render | Gap |
|---|---|---|---|
| List-body box | `<Box style={{ padding: 10, ... }}>` (line 3057) | `ui.group(...)` in `list_step3.rs:20` | Box uses `ui.group` (BIO frame style); wireframe uses redesign Box chrome with `padding: 10` + `border: 1.5px solid border-strong` + 6px corner radius |
| Mod-header row bg | `background: "var(--rail-bg)"` (line 3101) | `ui.selectable_label` default BIO theme | No `rail-bg` fill on parent rows |
| Mod-header border | `border: "1px dashed #b9b09a"` (line 3102) | No border | Missing dashed border |
| Mod-header font | `fontSize: 13, fontWeight: 500` (line 3104) | `typography_global::strong(title)` | Matches (strong = weight 500); size may differ |
| Lock glyph | `🔒` emoji (line 3111) | `strong("🔒").color(theme::warning())` (list_rows_step3.rs:141-143) | Color swap done in P8.T5; glyph already present |
| Chevron glyph | `🔗 ▾` / `🔗 ▸` (line 3116) | egui `CollapsingState` toggle button (default icon) | Missing `🔗` link glyph; wireframe uses `🔗` prefix before the expand/collapse chevron |
| Mod-name + suffix | `g.modName + <faint>(copy)</faint>` (line 3117) | `parent_row_title` emits "(split target)" / "(split target 2)" for `parent_placeholder` rows (`list_rows_step3.rs:200-216`) | **No gap — sibling renders `parent_row_title`'s output verbatim. Wireframe's "(copy)" is mock text; "(split target)" is the canonical wording (USER 2026-05-27).** |
| Component count + version | `({g.items.length}) v{g.version}` (line 3118-3119) | `({child_count})` only; no version (list_rows_step3.rs:208-213) | Version string missing from parent header |
| Child-row indent | `paddingLeft: 18` (line 3142) | `ui.add_space(25.0)` (list_rows_step3.rs:231) | 25px vs 18px wireframe intent — tune empirically |
| Child-row separator | `borderBottom: "1px dashed var(--border-dashed-light)"` (line 3146) | None | Missing per-row dashed bottom separator |
| Drag handle `≡` | `≡` glyph, `color: text-faint`, `fontSize: 16` (line 3152) | None | Missing drag-handle glyph on child rows |
| Order number | Right-aligned, faint, monospace, `fontSize: 11` (line 3153-3159) | None | Missing line-number column on child rows (Step 4 has it via `weidu_line.rs`; Step 3 does not) |
| Inline compat/prompt pills | After WeiDU line (lines 3161-3163) | Present (`render_compat_marker_pill`, `render_prompt_pill`) | Already implemented; color-token gap closed by P8.T5 |
| Drop-line indicator | `height: 2, background: accent, borderRadius: 1` (line 3009) | `draw_insert_marker` uses `ui.painter().line_segment` 1.5px (service_drag_ops_step3.rs:117-122) | Functionally equivalent; minor style difference (line vs. filled rect) |

**Step 4 visual gaps (Q7 cross-reference):** `step4_review_list.rs` already renders a redesign Box with `border-strong` + corner radius + inner `ScrollArea`. Line numbers use `weidu_line::render_weidu_line` with FiraCode monospace right-aligned. Three-hue WeiDU coloring is in `weidu_line.rs`. This matches the wireframe `OrderPanel` (`screens.jsx:3199-3222`) closely. Gaps are minor (see Q7).

### Q5 — Architectural options

#### Option A — Net-new sibling renderer at `src/ui/workspace/step3/step3_list_body.rs`

The sibling:
- Reads state via `state_step3::active_list_mut` (already `pub`).
- Calls `blocks::visible_indices` (already `pub(crate)` reachable).
- Renders mod-header rows (`render_header_row`) with rail-bg fill, dashed border, `🔒` lock button, `🔗 ▾/▸` chevron, mod-name + copy-suffix + count + version.
- Renders child rows (`render_child_row`) with 18px indent, `≡` drag handle, order-number column (reusing `weidu_line::lineno_column_width` pattern), WeiDU-styled text (delegating to `format_step3::format_step3_item` + `format_step3::weidu_colored_widget_text`), and compat/prompt pills (`render_compat_marker_pill` / `render_prompt_pill` shapes, calling `compat_colors` from `tree_compat_display_step2.rs` with `palette`).
- Detects drag-start and fires inline mutations (Q2 table) exactly as `list_rows_step3.rs` does.
- After the row loop, calls the four `service_drag_ops_step3` entry points in the same sequence as `list_step3.rs:run_drag_pipeline` (Q3).
- Returns a `RowRenderOutcome`-equivalent (or reuses that struct) so `apply_row_outcome` can be called identically.
- Called from `workspace_step3.rs` in place of `list_step3::render`; BIO's `list_step3::render` → `list_rows_step3::render_rows` remains untouched and continues to serve the legacy `BIO` binary.

**File inventory:**
- New: `src/ui/workspace/step3/step3_list_body.rs` (~300 lines: `render_header_row` + `render_child_row` + drag orchestration + pill rendering)
- Modified (orchestrator-owned): `src/ui/workspace/step3/workspace_step3.rs` — replace `list_step3::render(...)` call (line 65) with `step3_list_body::render(...)`. One line change in an orchestrator-owned file; no carve-out needed.
- `src/ui/workspace/step3/mod.rs` — additive `pub mod step3_list_body;` line.

**Risk register:**
1. **Drag-orchestration sequence drift** — the sibling's inline mutations (Q2 table) and service-call sequence (Q3) must mirror `list_rows_step3.rs` exactly. A missed step silently breaks drag behavior. Mitigation: the sequence is fully documented in Q3; the manual-test script (Q10) covers every drag scenario explicitly.
2. **`blocks::` helper visibility** — `blocks::visible_indices`, `blocks::block_indices`, `blocks::count_children_in_block`, `blocks::clone_parent_empty_block`, `blocks::repair_orphan_children`, `blocks::merge_adjacent_same_mod_blocks`, `blocks::prune_empty_parent_blocks`, `blocks::step3_item_key` are all in `src/ui/step3/blocks.rs`. Verify each is `pub(crate)` before the implementation run; if any is `pub(super)` only, a narrow visibility-widening (carve-out #7 pattern) is required — escalate as `PLAN GAP` at that point.
3. **`RowRenderOutcome` / `RowRenderContext` reuse** — the sibling may return its own equivalent struct or reuse `RowRenderOutcome` directly (it's `pub(crate)`). Either is fine; the implementation decides based on which shape fits the sibling's row loop.
4. **Version-string availability** — the wireframe shows `v{g.version}` on the mod-header row. `Step3ItemState` (`src/core/app/state/state_step3.rs`) must have a `version` or equivalent field. Verify before implementation; if absent, omit the version string (the wireframe's version display may be a static mock) — do not add a field to `Step3ItemState` (hard constraint).

**Constraint compliance:** the sibling is a new file in the orchestrator namespace. It calls BIO service functions read-only for behavior. The hard constraint (no reorder/undo/lock/state-struct changes) is enforced by the filesystem — the sibling cannot modify `list_rows_step3.rs` or any state struct.

**Cross-binary impact:** none. BIO's `list_rows_step3.rs` is untouched; the legacy `BIO` binary continues to call `content_step3::render` (which calls `list_step3::render` → `list_rows_step3::render_rows`). The orchestrator binary's Step 3 body switches to the sibling.

**Rollback cost:** edit one new file (`step3_list_body.rs`) + restore one line in `workspace_step3.rs`. Low.

#### Option B — Carve-out #11 in-place rewrite of `list_rows_step3.rs`

A new SPEC §1 carve-out #11 would authorize rewriting the render-function bodies in `list_rows_step3.rs` — specifically `render_parent_label` and `render_child_label` — while preserving all function signatures, all inline state mutations (Q2 table), and all service-call invocations unchanged.

**File inventory:**
- Modified (BIO source under carve-out #11): `src/ui/step3/list_rows_step3.rs` — bodies of `render_parent_label` (~37 lines) and `render_child_label` (~28 lines) rewritten; everything else (`handle_drag_start`, `handle_row_selection`, `update_drag_indices`, `update_drag_grab_geometry`, `update_locked_blocks`, `render_parent_toggle`, `render_parent_context_menu`, `render_child_context_menu`) verbatim.
- No new files; no mod.rs additions.

**Risk register:**
1. **Enforcement mechanism** — the boundary between "render-function body" and "inline mutation" is enforced by agent vigilance and reviewer inspection only. There is no filesystem separation. A carve-out that says "rewrite lines A-B but not lines C-D in the same file" is inherently fragile under agent drift. The Q2 inline-mutation sites are interspersed in `render_row` (`list_rows_step3.rs:98-112`) with the render calls — the separation is by code intent, not file structure.
2. **Cross-binary blast radius** — `list_rows_step3.rs` is shared. The legacy `BIO` binary's Step 3 view also changes visually. This may be desired (consistent look between BIO and orchestrator on this surface) or may violate the intent of the CRITICAL DIRECTIVE's "BIO behavior stays identical" rule. The CRITICAL DIRECTIVE states "no functionally modify" — a visual-only change inside a render function is arguable, but the safest interpretation is that the BIO binary should remain visually unaffected unless a carve-out explicitly authorizes cross-binary visual change on that file.
3. **Future iteration cost** — if the wireframe target changes, the implementer edits `list_rows_step3.rs` (BIO source), which requires a carve-out authorization each time.

**Constraint compliance:** the carve-out's authorized-edit-pattern section would enumerate the two function bodies; the inline mutations (Q2 table) are annotated as "verbatim — do not touch." Agent vigilance is the sole enforcement.

**Cross-binary impact:** yes. Both `BIO` and `infinity_orchestrator` binaries show the new visual. Whether that is desired is a user decision not yet made.

**Rollback cost:** re-edit `list_rows_step3.rs` to revert the two function bodies. Medium (touching BIO source triggers extra scrutiny).

### Q6 — Recommendation

**Recommend Option A** (net-new sibling at `src/ui/workspace/step3/step3_list_body.rs`).

Rationale:
1. **Constraint compliance is a filesystem property, not a vigilance property.** The hard constraint ("do not modify reordering logic") is unambiguous with Option A — the sibling file literally cannot mutate `list_rows_step3.rs`. With Option B the enforcement is by annotation and inspection in the same file.
2. **The inline-mutation surface is well-understood and low-drift.** The Q2 table shows eight sites; all are short (<10 lines each) and delegate to reachable service helpers. The drift risk of faithfully reimplementing them is low, and the Q3 sequence documentation makes the contract explicit.
3. **Cross-binary isolation.** The legacy `BIO` binary's Step 3 continues to render in its current form, unaffected. Option B changes it too — and that cross-binary visual change has not been authorized by the user.
4. **The sibling's row-render surface (~300 lines) is not disproportionate.** The concern that "sibling re-implementation would carry serious drift risk" (from the decision order) applies when the reimplemented behavior is complex business logic. Here the sibling reimplements visual layout, not logic; logic is delegated to the same service functions via `pub(crate)` calls.

The recommendation assumes the `blocks::*` helpers referenced in risk item 2 are `pub(crate)`. The implementation agent must verify this before writing code and escalate as `PLAN GAP` if any need a visibility-widening carve-out.

### Decisions locked during planning review (USER 2026-05-27)

1. **Recommend Option A — net-new sibling renderer.** Implementation proceeds against `src/ui/workspace/step3/step3_list_body.rs`.
2. **Parent-row suffix wording: "(split target)" stays.** The Q4 table's "(copy)" vs "(split target)" entry is closed — sibling renders `parent_row_title`'s output verbatim, no override. **No SPEC §1 carve-out is implicated by this decision.** Carve-outs govern *modifications to BIO source*; orchestrator-side renderers can freely output whatever string they choose. A carve-out would only be required if the wording change were applied *inside `parent_row_title`* (a BIO function), so both binaries adopt it. Keeping the orchestrator's sibling output aligned to `parent_row_title` makes both binaries display the same text and avoids cross-binary divergence.

### P8.T15 tasks

#### P8.T15.1 — Version-field pre-check and `blocks::` visibility audit

Before writing any render code, verify:
- `Step3ItemState` has a field carrying a version string (field name TBD — check `src/core/app/state/state_step3.rs`). If absent, the sibling omits the `v<version>` string from the mod-header row — do not add a new field to `Step3ItemState`.
- Every `blocks::*` function the sibling calls is `pub(crate)` (not `pub(super)`). List any that are not. If any require a visibility widening, escalate as `PLAN GAP` before proceeding.

**Acceptance:** a short pre-check report (inline in the implementation agent's report) citing file:line for each `blocks::*` function's visibility and the `Step3ItemState` version field finding.

#### P8.T15.2 — `step3_list_body.rs` sibling renderer

Implement `src/ui/workspace/step3/step3_list_body.rs` with:
- `pub(crate) fn render(ui: &mut egui::Ui, orchestrator: &mut OrchestratorApp, compat_markers: &HashMap<String, Step3CompatMarker>)` — the entry point called from `workspace_step3.rs`.
- Inner helpers (all `fn`, not `pub`): `render_header_row`, `render_child_row`, `render_drag_handle`, `render_lineno`, `render_row_pills`, plus the drag-orchestration shims that mirror the Q3 sequence.
- The scroll area, outer Box frame (redesign `border-strong` stroke + `shell-bg` fill + `REDESIGN_BORDER_RADIUS_U8` corner + `BOX_PADDING = 10.0`), and inner scroll (`ScrollArea::both`) match the wireframe.
- Header row: `rail-bg` fill + `1px dashed border-dashed-light` border + `🔒` lock button + `🔗 ▾/▸` chevron (egui `CollapsingState` + `🔗` prefix label) + `parent_row_title` output verbatim (mod-name + faint suffix as emitted — today "(split target)" / "(split target 2)" for placeholder rows) + `(N)` count + `v<version>` (omit if field absent per T15.1).
- Child row: 18px indent + `≡` drag handle (faint, 16px) + order-number column (`weidu_line::lineno_column_width` sizing, FiraCode, 11px, right-aligned, faint) + WeiDU-style text (via `format_step3::format_step3_item` + `weidu_colored_widget_text` with `palette`) + compat/prompt pills + `1px dashed border-dashed-light` bottom separator.
- Drag orchestration: exact Q3 sequence using the four `service_drag_ops_step3` entry points.
- Returns compat-popup / prompt-popup / uncheck / prompt-action requests for `apply_row_outcome`.

**Acceptance:** see Q10 below.

#### P8.T15.3 — Wire `workspace_step3.rs` to the sibling

Replace the `list_step3::render(...)` call at `workspace_step3.rs:65` with `step3_list_body::render(...)`. Add `pub mod step3_list_body;` to `src/ui/workspace/step3/mod.rs` (additive). Remove the `list_step3` import from `workspace_step3.rs` if it becomes unused.

**Acceptance:** `cargo build --bin infinity_orchestrator --release` no-op rebuild. `cargo build --bin BIO --release` unchanged.

### P8.T15 — Delivered (with iterations and follow-on carve-outs, 2026-05-27 → 2026-05-28)

Shipped on branch `feat/step3-list-body-sibling` across the original P8.T15 sibling renderer + ~7 iteration rounds of live-app feedback. Key deviations from the original plan, all preserving the original constraint set (no BIO source touched outside named carve-outs):

- **Header-bar visual restructure (wireframe-fidelity).** The original plan rendered a dashed border around the entire group (header + children). As shipped, the dashed/dotted border + a `redesign_rail_bg(palette)` background fill wrap **only the header bar** (per wireframe `screens.jsx:3083–3120`). Children render below without an enclosing border. Per-row dotted separators between children, skipped for the last child of each group. Final border style is **dotted** (filled circles, 7 px step, 0.7 px radius) at 50 % alpha of `theme_global::accent_path()` — fainter than the prior dashed iteration.
- **Header vertical centering** via asymmetric padding (`HEADER_BAR_VPAD_TOP = 6.0`, `HEADER_BAR_VPAD_BOT = 2.0`) to visually center the title text within the band.
- **Mod version label** rendered after the count in `redesign_text_faint(palette)` (light grey). Version sourced at render time via `crate::parser::weidu_version::parse_version` on the first child's `raw_line`; no `Step3ItemState` field added.
- **Lock glyph** uses bundled `FiraCodeNerdFont-Light.ttf` PUA glyphs (`U+F023` locked, `U+F09C` unlocked) via `FontFamily::Name("firacode_nerd".into())`. The earlier vector-paint approach (and an even earlier emoji approach that rendered as a smudge at 12 px) was dropped.
- **Chain icon and `≡` drag handle dropped.** User decision: lock + chevron is sufficient header iconography; the whole label rect is already draggable.
- **Floating overlay scrollbar** (`ScrollStyle::floating`) so the per-mod border can extend to the viewport edge without colliding with the bar.
- **Drag-target `Grab` cursor reverted.** Winit's Windows mapping of `CursorIcon::Grab` is `IDC_SIZEALL` (4-arrow compass), not a hand. Windows has no native open-hand grab cursor; the cursor change was removed pending a future custom-cursor effort.
- **Full-width drop-marker** on drag-over. Orchestrator-side `paint_insert_marker_full_width` replaces the `service_step3::drag_ops::draw_insert_marker` call; uses `ui.clip_rect()` minus `SCROLLBAR_RESERVE` so the line spans the full inner-container width regardless of which row is under the cursor. Sibling-implemented; no carve-out needed.

Two SPEC §1 carve-outs were added during this work:

- **#11 — Text-prompt jump-to-target Step-3 routing** (single-function body extension of `apply_text_prompt_jump` in `src/core/app/prompt_popup_nav.rs`). Fixed the bug where clicking a component in the Step-3 prompt-component popup didn't scroll the Step-3 list to that component.
- **#12 — Non-uniform-pitch-aware drag target in Step 3** (single-function carve-out for `service_drag_ops_step3.rs::update_drag_target_from_pointer`). Replaced the uniform-pitch floor-division math with iteration over `visible_rows`, because the sibling renderer's per-group inter-row spacing violated the BIO drag pipeline's uniform-pitch assumption (dragged rows jumped down by `accumulated_extra_spacing / row_pitch` slots on drag start).

Verification across all iterations: 590 lib tests passing, both binaries gate-fresh, crate-wide clippy pedantic + nursery zero warnings, BIO-source guard hits only the named carve-outs, DATA-LOSS sentinel byte-identical, render-gate PNGs (`target/render-gate/step3_polish_{dark,light}.png`) orchestrator-reviewed visually in both palettes, live-app user sign-off on the final state.

---

## P8.T16 — Step 4 review list wireframe polish (Item 2)

### Q7 — Step 4 wireframe fidelity gap

Comparing wireframe `OrderPanel` (`screens.jsx:3176–3225`) against `workspace_step4.rs` + `step4_review_list.rs` + `step4_save_row.rs`:

| Element | Wireframe | Current | Gap |
|---|---|---|---|
| Save button label (dual game) | `"Save weidu.log's"` (line 3183) | `"Save weidu.log's"` (`step4_save_row.rs:22`) | None |
| Count line copy | `"{N} components ready to install on {tab} · across {M} mods"` (line 3185) | Same format (`step4_save_row.rs:50-53`) | None |
| Tab strip (EET) | Game tabs row (line 3195-3197) | `render_game_tab_strip` (`workspace_step4.rs:77-83`) | None |
| Box chrome | `<Box style={{ padding: 12, ... }}>` (line 3199) | `rect_filled` + `rect_stroke` with `BOX_PADDING = 12.0` (`step4_review_list.rs:27-35`, `step4_review_list.rs:14`) | None — already matches |
| Line-number column | `minWidth: String(selected.length).length * 9 + 4, textAlign: right` (line 3205-3213) | `weidu_line::lineno_column_width` right-aligned FiraCode (`weidu_line.rs:17-20`, `step4_review_list.rs:53-54`) | None — already matches |
| WeiDU line coloring | Three-hue: path (amber), numbers (blue), comment (green) (via `WeiduLine`) | `weidu_line::build_weidu_job` three-hue (`weidu_line.rs:68-96`) | None — already matches |
| Line-number suffix | `{i + 1}` (no trailing `.`) (line 3216) | `n.to_string()` (no trailing `.`) (`weidu_line.rs:45`) | None |
| Empty state | `"No components selected on {tab}."` (line 3202) | `format!("No components selected on {active_tab}.")` (`step4_review_list.rs:47`) | None |
| `gap: 10` between items | `display: flex, alignItems: baseline, gap: 10` (line 3207) | `ui.spacing().item_spacing.x = 0.0` + explicit `add_space(LINENO_GAP_PX)` (`weidu_line.rs:32`, line 52) | `LINENO_GAP_PX = 10.0` already matches the `gap: 10`; alignment baseline vs. center is cosmetic |
| `display: flex, alignItems: baseline` | Items aligned by text baseline | `ui.horizontal(...)` (egui centers vertically by default) | Cosmetic difference: egui horizontal aligns center, wireframe aligns baseline. Acceptable approximation — no gap to close. |

**Conclusion: no meaningful visual gaps exist between the current Step 4 render and the wireframe.** The `step4_review_list.rs` implementation already matches the `OrderPanel` in every structural and visual dimension. The `weidu_line.rs` widget already handles line numbers, three-hue coloring, and faint FiraCode monospace. The `step4_save_row.rs` already has the correct button label and count-line copy.

### Q8 — Step 4 task scope

**No new task is needed for Step 4.** The existing implementation satisfies the wireframe. One minor polish point: `weidu_line.rs:68-70` uses hard-coded `Color32` literals for the path and numbers hues rather than `redesign_*` token calls. These are in the redesign-owned file (`src/ui/workspace/widgets/weidu_line.rs`), so they are in scope for future cleanup, but they are not a wireframe-fidelity gap — the colors match the wireframe's intent. If the palette-token sweep (P8.T1) defines `redesign_accent_path` and `redesign_accent_numbers`, the implementation agent for P8.T6 or P8.T1 should also update `weidu_line.rs` at that time. This is noted here so it is not overlooked — it is not a new P8.T16 task.

---

## Deferred backlog — not yet task-scoped; do not implement here

The entries below carry no `P8.T*` id yet. A future planning pass promotes each into a concrete task before any implementation.

- **(deferred from Phase-6 Fix-Run 2)** fork/import preview weidu lines = 3-hue. Colors-only; the shared `src/ui/install/preview_tabs.rs` renderer (Install + fork preview); Phase 8 reconciles SPEC §6.7's "Step 3 / Step 4 only" scope (extend to the preview weidu tabs, applied consistently incl. the Install preview) during the colour pass — rationale: see [`revision-log.md`](revision-log.md) 2026-05-17 Fix-Run-2 escalations RESOLVED entry.
- **(deferred from Phase-6 Fix-Run 2)** per-component prompt popup grows vertically. Root cause = one line in protected BIO `src/ui/step2/prompt/prompt_popup_step2.rs:31` (`ui.set_min_size(ui.available_size())` self-feedback). Phase 8 would introduce a NEW SPEC §1 carve-out for important-but-small BIO bug fixes (**pending user authorization** — not yet sanctioned); this is the first candidate (~1-line bounded-size change, Text branch) — rationale: see [`revision-log.md`](revision-log.md) 2026-05-17 Fix-Run-2 escalations RESOLVED entry.
- **(deferred from Phase-6 #2 user verification, 2026-05-18)** BIO grey pane-border recolor (carve-out #6). Reused BIO sub-renderers still paint pane/group borders in BIO's grey rather than the redesign `border-strong` token. Concrete instance: the redesign GameTab strip across Steps 2/3/4 reads as having a grey bar beneath it — root cause is the reused BIO content pane below the tab strip painting a grey top edge. Action: audit the reused-BIO render path for `theme_global` border accessors the orchestrator actually shows (covering specifically the Step-2/3/4 content-pane top/border accessor under the tab strip), add to the carve-out #6 table, swap value-only — rationale: see [`revision-log.md`](revision-log.md) 2026-05-18 Phase-6 verification plate CLEARED entry.
- **Window-narrow card-bottom misalignment.** Side-by-side cards/boxes (Home cards, Create starting-point boxes) fall out of vertical alignment when one column's content wraps to a different height. Cosmetic, width-dependent. Action: equalize row-of-cards heights (measure-tallest-then-set or shared min-height per row).
- **App-wide notifications UX revamp.** Transient confirmations / toasts auto-dismiss on a short fixed timer (~1.6–1.8s) with no dwell/hover-to-hold and no history. Action: a coherent app-wide notification model — longer/while-hovered dwell, optional manual dismiss, possibly a small recent-notifications surface — applied uniformly across Home / Workspace / Settings / Create / Install.

## Verification

1. `cargo build --bin infinity_orchestrator --release` succeeds.
2. `cargo build --bin BIO --release` continues to succeed; the legacy wizard is unaffected. (`update_app.rs` / `app_nav_ui.rs` / other legacy-only files unchanged; their `theme_global::*()` accessors still resolve to BIO's original palette values.)
3. Run the full Phase 5 / 6 / 7 verification checklists; everything still passes.
4. Visit each popup in the orchestrator (Compat, Prompt, Update Check, OAuth) — each one renders with redesign tokens. The title-bar collapse chevron works; collapsing leaves the title bar pinned. Light theme also produces correct values.
5. Open Step 2 in the workspace. Click components with compat issues. Verify: every compat pill (conflict, included, mismatch, order, warning) renders in `redesign_*(palette)` tones. The Details pane reads redesign tokens. The disabled-component dim text uses `redesign_text_disabled(palette)`.
6. Open Step 3 (the orchestrator's net-new C4 chrome from P6.T2d). Drag components. Verify in the **reused list rows**: per-row compat pills use redesign tokens; the PROMPT pill uses redesign tokens; lock icon flips between `redesign_warning(palette)` and `redesign_text_disabled(palette)`. (The aggregate tab conflict/prompt pills + Undo/Redo/Collapse/Expand are the net-new `step3_tab_row` redesign chrome — already correct from Phase 6, not a carve-out #6 / Phase-8 concern; `content_step3`'s old tab/badge row renders only in the legacy `BIO` binary.)
7. Start a Step 5 install. Verify: every console line tone (success / error / warning / debug / info / sent / sand / dim / default) reads redesign tokens; install-state banner (Preparing / Installing) reads redesign tokens; phase indicator (Idle / Preparing / Running / Waiting Input / Cancelling / Finished) reads redesign tokens; cancel-confirm warning reads redesign tokens; dev-mode RUST_LOG status line reads redesign tokens.
8. `git diff` each touched BIO file under the carve-out tables: only value substitutions inside existing conditional branches, signature additions of `palette: ThemePalette` on functions on the swap path, threaded `palette` args at call sites, and (where listed) `.collapsible(false)` → `.collapsible(true)` keyword flips. **No new branches, no removed branches, no logic mutations, no behavior changes.**
9. Switch theme light → dark → light: every orchestrator screen and the popups in the table recolor consistently. The carve-out #6 conditional sites recolor too (the hover / selected / disabled / conflict tones all read from the active palette).
10. Run an EET install: inspect the WeiDU command line — `-p` and `-n` paths land inside the destination as standard fixed-name folders. Single-game installs use `-g` similarly.
11. Run a Continue Partial Install: `-s` and `-c` are ON; no `prepare_target_dirs_before_install`.
12. Confirm `modlist-import-code.txt` appears in destination on every install start.
13. Hover every tree row / pill / button — the hover-overlay tint reads redesign palette (because egui's runtime visuals are updated by the orchestrator at frame start). The orchestrator-side widgets additionally show the 2px drop-shadow + 1px hover-lift per SPEC §12.3; the BIO-file pills retain egui's default hover (no drop-shadow upgrade — SPEC §12.3 doesn't require it for in-BIO pills).
14. Visual diff against `wireframe-preview/build.html` for every workspace surface (Step 2 tree + Details, Step 3 list + toolbar, Step 5 console + status + install row + dev header + cancel-confirm, every popup) — no regressions.

### Create-screen UI cleanup — deferred from Phase-6 verification (user, 2026-05-18)

Rationale + the Fix-Run-5/6 history these refinements sit on top of: see [`revision-log.md`](revision-log.md) 2026-05-18 entries.

1. Too much gap between box title and input box (e.g. modlist name ↔ input). Wireframe is cleaner.
2. Game input box still not the same height as modlist name.
3. Game input box text is not vertically center-aligned. Same with the "imported" text.
4. Destination folder box height still not the same as modlist name box.
5. Browse button height should also increase to match — all 4 input controls in this section must be the same height.
6. Box-equal-height (the two side-by-side "Choose one" boxes): misaligned on click-into-create until a window resize, then off by a pixel or two. Implement the **standard** equal-height technique (not another ad-hoc measured-max pass). **User directive (verbatim):** "Do not fixate on your way of fixing it. Think of what goes into implementing any standard UI design needing two side-by-side boxes that share the same height regardless of the text contents."

These are minor; verified via the egui_kittest render gate in Phase 8. The Fix-Run-6 changes (footer pin, right-margin, selected-box contrast, P1–P5) are NOT reverted — these are residual refinements on top.
