# Phase 8 — Full visual-fidelity reskin sweep, automatic flag policies, polish

## Summary

Phase 8 now covers the **full visual-fidelity sweep** across every BIO source file the orchestrator embeds — Step 2 tree + Details panel, Step 3 reorder list + toolbar pills, Step 5 console + status row + install row + menus + cancel-confirm, and the popup group (Compat, Prompt, Update Check, GitHub OAuth). The aggressive pruning from the prior Phase 8 draft is **reversed** under the SPEC §1 CRITICAL DIRECTIVE's newly-authorized carve-out #6 (state-aware theme-token reads). Every BIO file that renders into the orchestrator's Workspace surfaces is now in scope; each is annotated with the carve-out(s) it falls under (#1 pure value substitution, #2 window-chrome flip, or #6 state-aware token reads) and the specific conditional sites whose inline `theme_global::*()` accessors swap for `redesign_*(palette)` accessors.

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

| Path | Purpose | Depends on |
|------|---------|-----------|
| ~~`src/install_runtime/flag_policies_eet.rs`~~ | **SUPERSEDED — moved to Phase 7 P7.T17** (`install_runtime/per_install_dirs.rs`). EET `-p`/`-n` phase-dir creation is install-critical (SPEC §13.12a); not a Phase-8 file. | — |
| ~~`src/install_runtime/flag_policies_single_game.rs`~~ | **SUPERSEDED — moved to Phase 7 P7.T17** (`install_runtime/per_install_dirs.rs`). Single-game `-g` dir is install-critical. | — |
| ~~`src/install_runtime/flag_policies_log.rs`~~ | **SUPERSEDED — moved to Phase 7 P7.T17** (`install_runtime/per_install_dirs.rs`). `-u` `weidu_component_logs/` dir is install-critical. | — |
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
| ~~`src/ui/step3/content_step3.rs`~~ | ~~#6~~ — **DROPPED (Phase-6 P6.T2d superseded this)** | The orchestrator no longer calls `content_step3::render` / `render_toolbar` / `page_step3::render` — Step-3's top chrome is **net-new redesign code** delivered in Phase-6 P6.T2d (`src/ui/workspace/step3/`, the C4 treatment, the Step-2 precedent). `content_step3.rs`'s tab/badge row + compat-rules-error label render only in the legacy `BIO` binary now (which is not theme-reskinned — we don't lint/gate/restyle BIO's own binary). There is no orchestrator code path through `content_step3.rs`, so its former carve-out #6 swap would recolor nothing the orchestrator shows. **Step-3's carve-out #6 surface reduces to the reused `list_rows_step3.rs` row only** (the body `workspace_step3` wraps via `list_step3::render`). Left as-is in BIO source. |
| `src/ui/step2/toolbar/toolbar_compat_step2.rs` | #6 | `draw_active_tab_issue_badge` lines 137-142: `compat_colors(kind_key).map(...).unwrap_or((theme_global::text_muted(), ui.visuals().widgets.inactive.bg_fill))` — the `unwrap_or` fallback branch carries one `theme_global::*()` accessor that swaps for `redesign_text_muted(palette)`. The `compat_colors` call itself receives `palette` (per the `tree_compat_display_step2.rs` row). Function gains `palette: ThemePalette` arg; threaded from `frame_step2.rs` toolbar invocation. |
| `src/ui/step5/content/content_install_row_step5.rs` | #6 | `render_install_row` lines 32-44: `if state.step5.prep_running { theme::accent_path() } else if state.step5.install_running { theme::accent_path() }` — two-branch state-conditional, both accessors swap for `redesign_accent_path(palette)`. (The two branches independently read the same color today; the swap preserves that.) Line 175: unconditional `theme::error()` inside the share-error sub-popup. Function gains `palette: ThemePalette` arg; threaded from `content_step5.rs::render` (line 35). |
| `src/ui/step5/content/content_cancel_step5.rs` | #6 + #2 | Line 19: `.collapsible(false)` → `.collapsible(true)` on the confirm-cancel egui::Window. Line 34: `theme::warning()` inside `if state.step5.cancel_force_checked` branch swaps for `redesign_pill_warn(palette)`. Function gains `palette: ThemePalette` arg; threaded from `content_install_row_step5.rs::render_install_row` (line 157). |
| `src/ui/step5/content/content_dev_header_step5.rs` | #6 | `render_dev_header` lines 28-32: `let color = if has_rust_log { theme::success() } else { theme::accent_path() }` — two-arm conditional, each accessor swaps for redesign counterparts. Function gains `palette: ThemePalette` arg; threaded from `content_step5.rs::render` (line 31). |
| `src/ui/step5/status/status_phase_step5.rs` | #6 | `compute_phase` lines 13-51: cascading `if/return` blocks each constructing `PhaseInfo { color: theme::* }` — six accessors total (`warning`, `accent_path`, `status_running`, `status_preparing`, `text_muted`, `status_idle`). Each accessor inside its branch swaps for the matching `redesign_*(palette)` accessor; the conditional cascade is preserved. `render_phase` lines 80-86: `let status_color = if status_text.starts_with("Install start failed:") || status_text.contains("os error") { theme::error() } else { theme::text_muted() }` — two-arm conditional, each accessor swaps. Both functions gain `palette: ThemePalette` arg; threaded from `status_bar_step5.rs::render_status_and_input` (lines 69, 81). |
| `src/ui/step5/status/status_console_step5.rs` | #6 | `token_color` lines 46-85: pre-resolves eight `theme_global::*` accessors as locals (`default`, `red`, `debug_blue`, `sent_blue`, `info_green`, `amber`, `sand`, `dim`), then dispatches via an `if n == ... return ...` cascade. The eight local accessor calls (lines 49-56) swap for `redesign_terminal_*(palette)` counterparts; the cascade is preserved exactly. `render_styled_line` lines 87-112: pre-resolves `theme_global::success()` at line 90; inner `let color = if success_line && (n == "SUCCESSFULLY" || n == "INSTALLED") { success_green } else { token_color(&token) }` is the conditional — `success_green` already a local resolved at line 90. Single accessor swap on line 90 (`success() → redesign_success(palette)`); structure preserved. Function `render_console_panel` (line 114) also gains `palette: ThemePalette` arg; threaded from `status_bar_step5.rs::render_console` (line 17). |

### BIO files audited and confirmed out of carve-out scope (no swap needed)

These files were on the candidate list but contain no `theme_global::*()` color calls and no inline `egui::Color32` literals at their render sites — there is nothing for carve-out #1 or #6 to substitute. They render either pure layout/structure (orchestration shims, scroll panels, table grids) or pure data mappers.

| File | Why out of scope |
|------|-------------------|
| `src/ui/step2/tree/tree_render_step2.rs` | Pure orchestration of parent + component row rendering; no inline color reads. |
| `src/ui/step2/details/details_meta_step2.rs` | Pure data mapping (`SelectedDetailsData → Step2Details`); no rendering. |
| `src/ui/step2/update_check/update_check_popup_report_step2.rs` | Pure text builder (assembles report string for clipboard); no UI rendering. |
| `src/ui/step3/list_step3.rs` | Pure orchestration wrapper around `list_rows_step3.rs`; no inline color reads. |
| `src/ui/step5/content/content_step5.rs` | Pure orchestration shim that calls `render_dev_header / top_panels::render / render_install_row / status_bar::render_console / status_bar::render_status_and_input / prompt_answers::render_window`; no inline colors of its own. (Each callee that has color sites is covered above; this file just gains a `palette` arg and threads it.) |
| `src/ui/step5/status/status_input_row_step5.rs` | Pure input-field rendering with no color overrides; no inline color reads. |
| `src/ui/step5/status/status_bar_step5.rs` | Pure orchestration of console + phase + input rows; no inline colors of its own. (Each callee is covered; this file gains a `palette` arg and threads it.) |
| `src/ui/step5/menus_step5.rs` | Menu buttons with default egui styling; no inline color reads. |
| `src/ui/step5/prompts/prompt_answers_step5.rs` | Pure window-open/close + dispatch to top bar + table; no inline colors. (The window's `.collapsible` is already default — no flip needed.) |
| `src/ui/step5/prompts/prompt_answers_table_step5.rs` | Grid layout with `ui.strong` headers; no inline color reads. |
| `src/ui/step5/prompts/prompt_answers_rows_step5.rs` | Standard form widgets (checkbox / text edit / button / label); no inline color reads. |
| `src/ui/step5/prompts/prompt_answers_top_bar_step5.rs` | Three buttons (Capture / Import JSON / Export JSON); no inline color reads. |
| `src/ui/step5/top_panels_step5.rs` | Upper console panel container; no inline color reads. |
| `src/ui/app_nav_ui.rs` | No rendering at all — only action handlers (`handle_reset / handle_back / handle_next / handle_exit`). The legacy nav button render lives in `src/ui/frame/update_app.rs` (`render_back_button` / `render_exit_button` lines 49-105) — that file is part of the legacy `BIO` binary's nav layer and is **not invoked by `OrchestratorApp`** (the orchestrator builds its own `workspace_nav_bar.rs` in Phase 6, a net-new file already using redesign tokens). Per the CRITICAL DIRECTIVE's scope, the legacy `BIO` binary's nav stays BIO-default; carve-out #6 does not extend to surfaces outside the orchestrator's render path. |

### Files removed from this revision's scope vs. prior pruned Phase 8

The previous (pruned) Phase 8 draft removed `tree_render_step2.rs` / `tree_component_row_step2.rs` / `tree_parent_step2.rs` / `details_pane_step2.rs` / `details_paths_step2.rs` / `details_meta_step2.rs` / `list_step3.rs` / `list_rows_step3.rs` / `content_step3.rs` / `page_step4.rs` / Step 5 status row / install row / menus / prompts. **This revision re-adds the files that actually carry color sites (per the per-file audit above) under carve-out #6 authorization.** The handful that contain no color sites — `tree_render_step2.rs`, `details_meta_step2.rs`, `list_step3.rs`, `content_step5.rs`, `status_input_row_step5.rs`, `status_bar_step5.rs`, `menus_step5.rs`, the four `prompts/*` files, `top_panels_step5.rs`, `app_nav_ui.rs` — remain out of scope (cataloged in the "audited and confirmed out of carve-out scope" table above). `page_step4.rs` is replaced wholesale by `src/ui/workspace/step4/workspace_step4.rs` per the C4 resolution (see `overview.md` revision log) and is not part of Phase 8. **`content_step3.rs` is likewise dropped** — Phase-6 P6.T2d replaced Step-3's top chrome wholesale with net-new redesign code (`src/ui/workspace/step3/`, the C4 treatment); the orchestrator no longer calls `content_step3::render` / `render_toolbar`, so its carve-out #6 row above is struck and Step-3's carve-out #6 surface reduces to the reused `list_rows_step3.rs` row only (P8.T5).

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

## Open questions / risks

- **Token extraction scope creep.** During extraction it's tempting to also rename a function, move a constant, simplify a branch, add a helper, or "clean up" the conditional structure. **All forbidden** by carve-outs #1 and #6. If a token extraction surfaces a clear bug or improvement opportunity, file an issue; do not change behavior in this phase. The bar for each touched BIO file is: the `git diff` shows only value substitutions inside existing branches, signature additions of `palette: ThemePalette`, and (where applicable) one `.collapsible()` keyword flip — nothing else. **No new branches, no removed branches, no reordered branches, no logic mutations.**
- **Light theme coverage.** Some BIO theme_global accessors don't have a SPEC §12.1 light counterpart (the SPEC focuses on shell-bg / accent / text tones; compat fill tones aren't enumerated). Where the SPEC is silent, the planner derives a Light-theme value by inverting the Dark-theme value's luminance, matching the wireframe's `index.html` Light palette where applicable. New `redesign_*` accessors live in `redesign_tokens.rs` / `theme_global.rs`.
- **Embedded terminal coloring.** The `EmbeddedTerminal` widget (`third_party/egui_term`) has its own internal color handling. Carve-out #6 does not reach inside the terminal widget. Acceptable scope for Phase 8: skin the surrounding Console Box and the line-tone classifier in `status_console_step5.rs` (P8.T6); the terminal's internal cell colors stay BIO-default. SPEC §9 doesn't require the terminal to recolor.
- **`.collapsible(true)` interactions with focus traps.** Today's BIO popups use `.collapsible(false)` because they behave like modal dialogs. Flipping to `.collapsible(true)` should be safe (collapse only affects body visibility, not modality), but verify no popup state breaks when collapsed mid-operation (e.g., collapsing the Update Check popup while a check is running). Test before merging each flip.
- **Per-frame palette propagation.** Phase 1 P1.T2's "no global theme value" rule means every render function on the carve-out path gains a `palette: ThemePalette` parameter. This is a wide-but-shallow signature change — many files, one new arg each. The orchestrator's frame entry (`OrchestratorApp::update`) reads `self.theme_palette` and threads it through `workspace_step_router::render(ui, self, palette)` → `page_stepN::render(ui, &mut state, dev_mode, exe_fingerprint, palette)` → all sub-renderers. The `palette: ThemePalette` add is the only signature change per BIO function; otherwise functions stay byte-identical.
- **Carve-out #6 cliff edge.** A few sites read `ui.visuals().widgets.*` or `ui.visuals().text_color()` instead of a `theme_global::*` accessor (e.g., `tree_component_row_step2.rs::render_filter_row` lines 195-203 use `visuals.widgets.active.bg_fill` / `bg_stroke`). These are **already palette-driven** because Phase 1 P1.T2's per-frame palette application writes the redesign palette into `egui::Context::style_mut()` at the start of every frame. So they automatically pick up redesign colors without any source change. The carve-out #6 scope is strictly about explicit `theme_global::*()` accessor calls — anything reading egui's runtime style is already covered.

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
