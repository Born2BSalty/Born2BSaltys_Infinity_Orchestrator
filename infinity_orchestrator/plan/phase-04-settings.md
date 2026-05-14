# Phase 4 — Settings screen (5 sub-tabs) + per-edit debounced path validation

## Summary

Build the new top-level Settings screen with five file-folder tabs (General, Paths, Tools, Accounts, Advanced). Each tab reads from and writes to the existing `bio_settings.json` via the existing `bio::settings::SettingsStore` (the same store `OrchestratorApp` already constructed in Phase 2 via `app_bootstrap_init::initialize`). Settings persist immediately (no Save/Cancel buttons). The GitHub OAuth flow is invoked from the Accounts tab by calling BIO's existing public OAuth orchestration functions and rendering BIO's existing public popup — no BIO modification. Path validation runs on edit (with per-edit debounce per H11) and on app start, producing validation events that update the left-rail status line + the Home `game installs detected` block (rendered in Phase 5). There is no explicit "Validate now" button — auto-validation is the only mechanism.

## What ships after this phase

- `cargo build --bin infinity_orchestrator --release` succeeds.
- Clicking Settings in the left rail opens the redesigned Settings screen.
- Five file-folder tabs at the top, the active tab visually merged with a single Box below.
- **General** sub-tab: NameRow + 2-col grid of Theme (segmented light/dark), Language (ComboBox), Validate-all-paths-on-startup (Toggle), Diagnostic mode (Toggle). Theme toggle writes `OrchestratorApp::theme_palette = ThemePalette::Light/Dark` and persists the choice into `RedesignSettings`; the next frame's `OrchestratorApp::update` applies it.
- **Paths** sub-tab: PathRows for BGEE / BG2EE / IWDEE game folders + Mods archive / Mods backup / Temp working folders. `browse...` opens an `rfd` folder picker. Per-keystroke edits trigger a debounced re-validation (~200ms idle window). Each row is two lines tall: top line = label + input + browse button; bottom line = inline status text (specific reason for Warning/Error, blank otherwise). While a debounce is pending the bottom line reads `checking…`. There is no aggregate summary, no Validate-now button.
- **Tools** sub-tab: PathRows for `weidu` / `mod_installer` / `7z` / `git` binaries with detected-version hints.
- **Accounts** sub-tab: three cards (GitHub, Nexus Mods, Mega). GitHub `connect` opens BIO's existing OAuth popup via its public renderer (with the C2-audit fallback path — see Open Questions). Nexus / Mega `connect` shows a "not yet implemented" hint.
- **Advanced** sub-tab: 2-col grid of value fields (Timing & limits) + toggles (Install behavior, WeiDU command-line flags) per SPEC §11.5.
- Every edit persists to `bio_settings.json` immediately (debounced by an orchestrator-owned settings persistence cycle that mirrors BIO's existing pattern).

## What's still missing

- The `game installs detected` block on Home — Phase 5 (it reads the same validation events).
- The Tweaks panel — explicitly dropped in production (SPEC §14.2).
- Light/dark token coverage: the theme toggle works but some BIO popups may still render in their original BIO styling until their theme-token extractions land in Phase 8.
- Language ComboBox is a **visual stub only** for v1 alpha — no i18n layer exists in today's BIO. Selecting a non-English language is silently persisted but does not change the UI text. Building the i18n layer is out of scope for v1.

## Dependencies

- Phase 1 (theme tokens, fonts, redesign widgets, carve-out #3 split).
- Phase 2 (`OrchestratorApp`, page_router, settings stub destination, `SettingsStore` already loaded via `app_bootstrap_init::initialize`).

## File inventory

### New files

| Path | Purpose | Depends on |
|------|---------|-----------|
| `src/ui/settings/mod.rs` | `pub mod page_settings; pub mod state_settings; pub mod tab_general; pub mod tab_paths; pub mod tab_tools; pub mod tab_accounts; pub mod tab_advanced; pub mod widgets; pub mod validate_now; pub mod validate_debounce; pub mod oauth_glue;` | — |
| `src/ui/settings/page_settings.rs` | `pub fn render(ui, orchestrator: &mut OrchestratorApp, ctx)` — the screen entry. Renders the file-folder tab strip at the top, dispatches to the active tab renderer. | tab_* modules |
| `src/ui/settings/state_settings.rs` | `pub struct SettingsScreenState { active_tab: SettingsTab, name_row_editing: bool, name_row_temp: String, validate_now_in_flight: bool, oauth_popup_open: bool, path_edit_debounce: HashMap<&'static str, Instant> }` + `enum SettingsTab { General, Paths, Tools, Accounts, Advanced }`. Tab persists in memory across screen visits within a session. Lives on `OrchestratorApp::settings_screen_state`. | — |
| `src/ui/settings/tab_general.rs` | Renders NameRow + 2-col grid of theme/language/validate-on-start/diagnostic-mode. | widgets, settings store, `ThemePalette` |
| `src/ui/settings/tab_paths.rs` | Renders Game-sources (BGEE / BG2EE / IWDEE) + Working-folders (Mods archive / Mods backup / Temp). Each row binds to a `Step1Settings::*_folder` field via `&mut` reference. On edit, kicks off the per-edit debounce cycle (see `validate_debounce.rs`). No "Validate now" button, no bottom aggregate summary — per-row inline status carries everything. | widgets, settings store, `validate_debounce` |
| `src/ui/settings/tab_tools.rs` | Renders 4 PathRows for binaries. | widgets, settings store |
| `src/ui/settings/tab_accounts.rs` | Renders 3 service cards. GitHub card's `connect` button triggers `oauth_glue::start_github_flow(orchestrator)`. | `oauth_glue` |
| `src/ui/settings/tab_advanced.rs` | Renders 2-col grid of ValueRows and ToggleRows per SPEC §11.5. | widgets, settings store |
| `src/ui/settings/widgets/tab_strip.rs` | `pub fn render(ui, current: &mut SettingsTab, body: impl FnOnce(&mut Ui))` — file-folder tab pattern that merges visually with the box below. Reused in Phase 6's workspace progress bar (different content, same pattern). | redesign theme tokens |
| `src/ui/settings/widgets/path_row.rs` | `pub fn render(ui, label, mono_value: &mut String, hint: Option<&str>, on_browse: impl FnOnce(), on_change: impl FnOnce())` — when the text field changes, the `on_change` callback is invoked so the parent can kick off per-edit debounce. | redesign theme tokens, rfd folder picker |
| `src/ui/settings/widgets/value_row.rs` | `pub fn render(ui, label, value: &mut String, placeholder, hint)` for absorb-the-gate fields per SPEC §11.5. | redesign theme tokens |
| `src/ui/settings/widgets/toggle_row.rs` | `pub fn render(ui, label, on: &mut bool, hint)` | redesign theme tokens |
| `src/ui/settings/widgets/segmented_toggle.rs` | For Theme (light / dark). | redesign theme tokens |
| `src/ui/settings/widgets/name_row.rs` | Edit-in-place name field with display + edit modes. | redesign theme tokens |
| `src/ui/settings/widgets/account_card.rs` | The service card chassis (avatar + service name + Pill + action button). | redesign theme tokens |
| `src/ui/settings/validate_now.rs` | `pub fn run_now(settings: &Step1Settings) -> ValidationReport` + `pub fn run_for_field(...)` — synchronous one-shot path validation. Applies role-specific rules: game folders require a `chitin.key` + `lang/` marker (mirroring BIO's `state_validation_fs::check_game_dir`, mechanical clone since that helper is `pub(super)`); working folders require the path to NOT look like a game install. Each field's result is a `PathStatus { Empty, Ok { detail }, Warning { reason }, Error { reason } }`. | BIO `state_validation_*` (read-only consumer; no carve-out needed — clones the marker check) |
| `src/ui/settings/validate_debounce.rs` | `pub fn tick(orchestrator: &mut OrchestratorApp, now: Instant)` — called from `OrchestratorApp::update` once per frame. When any field's debounce window (`DEBOUNCE_MS = 200`) has elapsed, runs `validate_now::run_now` once and refreshes both the per-field map AND the rail-status `step1_path_check`. State stored in `SettingsScreenState::path_edit_debounce`. The orchestrator's update loop calls `ctx.request_repaint_after(next_debounce_due_in)` so the tick fires even without intervening user input (egui paints lazily). | `validate_now` |
| `src/ui/settings/oauth_glue.rs` | Glue functions that drive the OAuth device flow from the orchestrator side. `pub fn start_github_flow(orchestrator: &mut OrchestratorApp)` calls the same BIO public flow-runner functions that `WizardApp::handle_step1_action::StartGithubAuth` calls — but **does not call `handle_step1_action` itself** because the C2 audit found `handle_step1_action` mutates `WizardApp.step1_github_auth_rx` (a channel receiver, not in `state`). The glue function owns its own `OrchestratorApp.github_auth_rx` field. `pub fn render_github_popup_if_open(orchestrator: &mut OrchestratorApp, ctx)` invokes `bio::ui::step1::github_auth_popup_step1::render(ctx, &mut orchestrator.wizard_state, &mut step1_action_sink)` and dispatches any returned action via the same public flow-runner functions. **Both functions use only BIO's existing public API (`pub fn` and `pub(crate) fn` reachable same-crate); no BIO source is modified.** | BIO `app_step1_github_oauth` (read-only), BIO `github_auth_popup_step1` (read-only) |
| `src/settings/redesign_fields.rs` | Net-new serde struct `RedesignSettings` for fields the redesign adds that don't exist in `Step1Settings`: `user_name` (for the share-code author field — SPEC §11.1), `theme_palette` (`Light \| Dark`), `language: UiLanguage`, `diagnostic_mode: bool` (default `false`; OR'd with the CLI `-d` flag at app launch per M12 — see `tab_general.rs` and P1.T8). Persisted in a sibling file `bio_redesign_settings.json` managed by a new orchestrator-side store. See P4.T10 — the orchestrator does **not** extend BIO's `AppSettings`. | serde |
| `src/settings/redesign_store.rs` | `pub struct RedesignSettingsStore { path }` with `load` / `save` mirroring `SettingsStore`. Persists `bio_redesign_settings.json` in the same config dir. | `redesign_fields`, platform_defaults |

### BIO files read from / consumed (no modifications)

- `src/settings/model.rs::Step1Settings` — All path / tool / advanced fields are read and written. The `tab_paths`, `tab_tools`, `tab_advanced` renderers bind directly to existing `Step1Settings` fields. **No field is renamed**; the redesign reads them with new labels. No struct modification.
- `src/settings/store.rs::SettingsStore` — Reused as-is. `OrchestratorApp` already owns one (Phase 2).
- `src/core/app/state/state_validation*.rs` — Read by `validate_now.rs` and `validate_debounce.rs`. No modification.
- `src/core/platform_defaults.rs` — Read for default paths shown as hints when a field is empty.
- `src/ui/step1/github_auth_popup_step1.rs` — The existing OAuth popup renderer. **Called by the orchestrator's `oauth_glue::render_github_popup_if_open` via its existing public function**. Its body, signatures, and behavior stay identical. Verify visibility per the Open Questions section.
- `src/core/app/app_step1_github_oauth.rs` — The existing OAuth flow runner. **Called via its existing public functions** (`start_github_oauth_device_flow`, `clear_github_oauth_token`, `load_github_login_from_stored_token`). All are `pub fn` per source inspection.

### C2 audit table — Phase 4 functions

Per SPEC §1 carve-out #4 (WizardApp → WizardState refactor), audit each `&mut WizardApp`-taking function that Phase 4 needs and document whether it qualifies for the carve-out.

| Function | Location | C2 audit result | Phase 4 plan |
|----------|----------|-----------------|--------------|
| `WizardApp::handle_step1_action` | `src/ui/app_methods.rs:9` | **Stays as `&mut WizardApp`** — body mutates `self.step1_github_auth_rx` (per `src/ui/app_methods.rs:37,49`), which is a channel receiver field on `WizardApp`, not part of `WizardState`. Mutation surface includes `state.github_auth_*` + the receiver field. | Orchestrator does **not** call `handle_step1_action`. Instead, `oauth_glue::start_github_flow` directly calls the underlying `bio::app::app_step1_github_oauth::start_github_oauth_device_flow()` (`pub fn`) and owns its own receiver field (`OrchestratorApp::github_auth_rx`). The orchestrator replicates the dispatch surface of `handle_step1_action`'s three arms (`ConnectGitHub`/`ReconnectGitHub`, `DisconnectGitHub`, `PathsChanged`) inline in `oauth_glue.rs` using the same `bio::app::*` public functions. |

No BIO functions are refactored under carve-out #4 in Phase 4. The carve-out remains available but is not invoked here.

### BIO files needing allowed mild refactor

**None.**

The previous plan's "Phase 4 extends `AppSettings` with a `redesign: RedesignSettings` field" is **removed**. Per the CRITICAL DIRECTIVE, no new fields on BIO structs — even with `#[serde(default)]` — are allowed. Redesign-specific settings live in a sibling file `bio_redesign_settings.json` managed by a new orchestrator-side store (`src/settings/redesign_store.rs`). This adds one new file and zero edits to BIO source.

If the GitHub OAuth popup renderer or flow-runner functions turn out to be `pub(super)` / `pub(crate)` rather than `pub`, **same-crate orchestrator reachability via the Phase 1 carve-out #3 split makes both visibilities accessible** without flipping anything. Per SPEC §1 decision order: if a required entry turns out to be `pub(super)`-without-`pub(crate)`-fallback, the work is **not** blocked — the orchestrator's `oauth_glue.rs` already exists as a sibling for the dispatch coupling (`WizardApp::handle_step1_action` is disqualified from carve-out #4 by the C2 audit), and the same sibling extends to drive the device flow against BIO's lower-level public surface (the `ureq` HTTP helpers in `bio::app::app_step1_github_oauth`, the `keyring` storage helpers). Quick grep before P4.T5 confirms visibility; the sibling path is the fallback in either case.

## Implementation tasks

### P4.T1 — `SettingsScreen` shell and tab strip

- **What:** Implement `src/ui/settings/page_settings.rs::render`. The top renders a file-folder tab strip (`tab_strip::render`) with five tabs in fixed order: General / Paths / Tools / Accounts / Advanced. Below the strip, a single Box fills the remaining vertical space. The active tab's body renders inside the Box.
- **Where:** New file.
- **Acceptance:** Clicking each tab activates it; the body content swaps. Tab selection persists in `SettingsScreenState::active_tab`.
- **SPEC:** §11 (intro paragraph).

### P4.T2 — `tab_general` renderer

- **What:** Render NameRow at the top, then 2-col grid: Theme (segmented light/dark), Language (`ComboBox` of UI languages per §11.1), Validate-on-startup (Toggle, default on), Diagnostic mode (Toggle, default off). Each setting has its hint string verbatim from the spec. The Diagnostic mode toggle persists to `RedesignSettings::diagnostic_mode` and is OR'd with the CLI `-d` flag at app launch (see P1.T8); toggling at runtime updates `OrchestratorApp::dev_mode` on the next frame with no restart required.
- **Where:** New file.
- **Acceptance:** Switching theme via the segmented control writes `OrchestratorApp::theme_palette` and the matching `RedesignSettings::theme_palette`; the app's colors update on the very next frame (no atomic / no global). The language ComboBox lists English (default), German, French, Spanish, Italian, Polish, Portuguese, Czech, Turkish, Ukrainian. Toggling Diagnostic mode flips `OrchestratorApp::dev_mode` to the OR of the new toggle value and the original CLI flag value; the change is visible the next frame (dev-mode-only buttons appear/disappear without restart).
- **SPEC:** §11.1.

### P4.T3 — `tab_paths` renderer

- **What:** Render two labeled sections per SPEC §11.2: "Game sources" (BGEE, BG2EE, IWDEE game folders) and "Working folders" (Mods archive, Mods backup, Temp). Each row is a `path_row` with label + mono value field + per-row inline status indicator + `browse...` button. No "Validate now" button, no bottom aggregate summary.
- **Where:** New file.
- **Acceptance:** Each row binds to the corresponding `Step1Settings` field (`bgee_game_folder`, `bg2ee_game_folder`, `eet_bgee_game_folder` as the IWDEE alpha-stub binding, `mods_archive_folder`, `mods_backup_folder`, `mods_folder` for Temp). `browse...` opens an `rfd::FileDialog::new().pick_folder()`. Per-keystroke edits update the field's last-dirty-at timestamp in `SettingsScreenState::path_edit_debounce`; the debounce cycle (P4.T11b) picks them up and produces an updated `PathStatus` per field. The row's input border tints subtle success/warn/danger based on the status; the right hint shows the specific reason text from the `PathStatus::Warning { reason }` / `Error { reason }` variant.
- **`game_install` is per-modlist, not per-app.** When the orchestrator's settings-persistence cycle writes `Step1Settings` back to `bio_settings.json`, the `game_install` field is **excluded from the write** — it lives in the orchestrator's in-memory `wizard_state.step1.game_install` as a per-modlist value (loaded by `populate_wizard_state_from_workspace` from the modlist's `entry.game`). Persisting it to the global settings file would conflate per-modlist state with global state and break per-modlist game choice. Implementation: the persistence cycle's `should_save_settings` comparator ignores the `game_install` field; or equivalently, snapshot the original `game_install` on workspace open and restore it before each settings write.
- **No "Tools folder" row on this tab.** A "tools" PathRow appears in the wireframe but it conflates with the dedicated Tools sub-tab (which already owns weidu/mod_installer binary paths). The Paths tab is scoped to game sources + working folders only.
- **SPEC:** §11.2, Appendix A.1 (`Test Paths` replaced by inline auto-validation).

### P4.T4 — `tab_tools` renderer

- **What:** Two row variants per SPEC §11.3:
  - **Writable PathRow** for `WeiDU binary` and `Mod installer` — bound to `Step1Settings::{weidu,mod_installer}_binary`. The row uses the same two-line layout as Paths (label + input + browse + status line). `validate_now::check_binary` runs `resolve_on_path` for bare names so the row tells the truth: bare name not on `$PATH` → `Error { reason: "not on $PATH — install or specify the full path" }` (red border); bare name on `$PATH` → `Ok { detail: Some(resolved absolute path) }` so the user sees which binary will actually run; absolute path → `is_file` check.
  - **Detection-only row** for `7-Zip executable` and `Git executable` — these have no `Step1Settings` backing field (BIO uses system installs only), so the row is a label + status line, no input or browse. Status comes from `OrchestratorApp::tool_version_cache.{sevenzip,git}_path`, populated once in `OrchestratorApp::new` by `validate_now::resolve_on_path("7z")` / `("git")`. Found → `found at <path>` (success-soft); not found → `not installed — <purpose>` (warning-soft).
- **Where:** New file `src/ui/settings/tab_tools.rs`; helper widget `detection_row` lives inline in that file. `validate_now::resolve_on_path` is `pub fn` in `src/ui/settings/validate_now.rs` (also used by `check_binary`).
- **Acceptance:** Open Tools with no `weidu` / `mod_installer` on PATH → both rows show red `× not on $PATH` text below the input. Install `weidu` system-wide → row flips to `ok · <resolved path>` after the debounce settles. 7z / git rows show their actual install status from startup detection (e.g., `found at /usr/local/bin/7z`).
- **SPEC:** §11.3.

### P4.T5 — `tab_accounts` renderer

- **What:** Three cards (GitHub, Nexus Mods, Mega) rendered via `account_card::render` (the service card chassis in `widgets/account_card.rs`). Per SPEC §11.4 each card is a single horizontal row inside a redesign Box: 36×36 shell-bg avatar with sketchy border + 2×2 drop shadow + initials in `poppins_bold`, service name, optional "as @user" faint hand-style label (connected only), then a right-anchored cluster of a small pill (no border) + small Btn.
  - **GitHub** — fully wired. Pill says `connected` (info tone, with "as @user" label sitting earlier in the row) or `not connected` (neutral tone). Btn label flips between `disconnect` (non-primary) and `connect` (primary). `connect` calls `oauth_glue::start_github_flow`; `disconnect` calls `oauth_glue::disconnect_github`.
  - **Nexus Mods / Mega** — rendered with `disabled: true` on the Btn. The button is greyed at 50% alpha, clicks are suppressed by the underlying `redesign_btn` sense, and the row paints a `coming soon` tooltip on hover. **Deviation from the wireframe**, which renders both as live `connect` buttons; we prefer visibly-disabled affordances over buttons that produce "not yet implemented" hints when clicked.
- **Where:** New file.
- **Acceptance:** GitHub `connect` triggers the OAuth device flow popup (rendered via `oauth_glue::render_github_popup_if_open` from `OrchestratorApp::update`, see P4.T9); on completion the card flips to a connected pill + "as @user" label + `disconnect` button. Nexus / Mega buttons render greyed; clicking does nothing; hovering shows the tooltip.
  **Before implementation:** run `grep -n "^pub fn\|^pub(crate) fn\|^pub(super) fn" src/ui/step1/github_auth_popup_step1.rs src/core/app/app_step1_github_oauth.rs` to confirm the entry points are reachable. If everything is `pub` / `pub(crate)`, the orchestrator's `oauth_glue.rs` sibling calls the popup renderer and flow-runner directly. If a required entry turns out to be `pub(super)`-without-fallback, `oauth_glue.rs` instead drives the device flow against BIO's lower-level public HTTP / token-storage helpers (no carve-out needed; the sibling extends to whatever public surface is reachable).
- **SPEC:** §11.4, §13.2, Appendix A.2.

### P4.T6 — `tab_advanced` renderer

- **What:** 2-col grid: left column "Timing & limits" with ValueRows per SPEC §11.5 (Custom scan depth, Mod install timeout, Mod install timeout per mod, Auto-answer initial delay, Auto-answer post-send delay, Tick (dev), Prompt context lookback). Right column "Install behavior" with ToggleRows (Sound cue, Download missing, Casefold) and "WeiDU command-line flags" subsection with ToggleRows (`-a`, `-x`, `-o`).
- **Where:** New file.
- **Acceptance:** Each ValueRow follows the absorb-the-gate pattern: an empty field means "use BIO default"; a filled field means "override". Internally, the renderer maps an empty input to setting both `<field>_enabled = false` and `<field> = default_value` in `Step1Settings`; a filled input maps to `<field>_enabled = true` and `<field> = parsed_value`. This preserves BIO's two-field representation without exposing the gate to the user.
- **Backcompat:** The `<field>_enabled` booleans stay in `Step1Settings` for backward compatibility with users' existing `bio_settings.json`. They are derived from the new UI rather than user-controlled. Do not remove these fields from BIO source — that would be a CRITICAL DIRECTIVE violation.
- **SPEC:** §11.5, Appendix A.15.

### P4.T7 — `validate_now::run_now`

- **What:** Synchronous role-aware path validation. Inputs: a `&Step1Settings`. Output: a `ValidationReport` mapping each path field to a `PathStatus { Empty, Ok { detail: Option<String> }, Warning { reason: String }, Error { reason: String } }`. Each field is checked against rules that match what the path is actually used for:
  - **Game folders** (BGEE / BG2EE / IWDEE bindings): `Ok` when the path exists, is a directory, and contains both `chitin.key` and a `lang/` subfolder. `Warning` when the path exists but those markers are missing. `Error` when the path is set but doesn't exist or is a file. The `chitin.key` + `lang/` check is a mechanical clone of BIO's `state_validation_fs::check_game_dir` logic (~5 lines) — that helper is `pub(super)` so the orchestrator re-implements it inline. No BIO modification.
  - **Working folders** (Mods archive / Mods backup / Temp): `Ok` when the path exists and is a directory that does NOT contain `chitin.key`. `Warning` when the path is set but doesn't exist (will be auto-created on first install) OR exists with a `chitin.key` (user likely picked their game folder by mistake). `Error` when the path is set but is a file.
- **Where:** New file.
- **Acceptance:** Calling `run_now` with a real BGEE folder returns `Ok` for the BGEE field; pointing it at a non-game folder returns `Warning { reason: "no chitin.key/lang — not a recognizable Infinity Engine install" }`. Calling with a Mods archive pointing at a game folder returns `Warning { reason: "looks like a game install — pick an empty folder" }`. Callable from `validate_debounce::tick` and from Phase 5's Home app-start validation.
- **SPEC:** §11.2.

### P4.T8 — Wire Settings into `page_router`

- **What:** In `src/ui/orchestrator/page_router.rs`, replace the `NavDestination::Settings` arm's call to `stubs::render_settings_stub` with `bio::ui::settings::page_settings::render(ui, orchestrator, ctx)`.
- **Where:** Edit `src/ui/orchestrator/page_router.rs` (Phase 2 new file — editable).
- **Acceptance:** Clicking Settings in the rail opens the real screen (not the stub).
- **SPEC:** §11.

### P4.T9 — Wire OAuth popup rendering into `OrchestratorApp::update`

- **What:** In `src/ui/orchestrator/orchestrator_app.rs::update`, after the destination dispatch, call `bio::ui::settings::oauth_glue::render_github_popup_if_open(self, ctx)`. The glue function checks `self.wizard_state.github_auth_popup_open` and, if true, calls `bio::ui::step1::github_auth_popup_step1::render(ctx, &mut self.wizard_state, &mut step1_action_sink)` exactly as BIO's `bio::ui::app::update_loop::run` (the real path per H3) does. **No changes to the popup renderer file itself.** Any `Step1Action` returned (e.g., user clicked "Cancel" in the popup) is dispatched via BIO's existing public flow-runner functions — call them directly from the glue function, not through `WizardApp` (which the orchestrator does not host, and whose `handle_step1_action` was disqualified by the C2 audit).
- **Where:** Edit `src/ui/orchestrator/orchestrator_app.rs`. Implement the glue in `src/ui/settings/oauth_glue.rs`.
- **Acceptance:** From Accounts → GitHub `connect`, the OAuth popup opens. The flow proceeds (device-code URL + user code displayed). On completion the card flips to `connected as <name>`.
- **SPEC:** §11.4, §13.2.

### P4.T10 — Persist `RedesignSettings` in a sibling file

- **What:** Create `src/settings/redesign_fields.rs` (the `RedesignSettings` struct) and `src/settings/redesign_store.rs` (the store). `OrchestratorApp::new` constructs the store, calls `load()`, and stores the result. The General sub-tab's NameRow, theme picker, and language ComboBox bind to fields on this struct. An orchestrator-owned debounce cycle (mirroring the registry's `RegistryPersistenceCycle` shape from Phase 3) writes `bio_redesign_settings.json` on dirty.
- **Where:** New files in `src/settings/`. The existing `bio::settings::model::AppSettings` is **not** extended (per the CRITICAL DIRECTIVE).
- **Before implementation:** grep `src/settings/model.rs::AppSettings` and `src/core/app/modlist_share.rs` for any existing `author` / `user_name` field. If one exists, use it rather than creating a new field. If not (the expected case), the new `RedesignSettings::user_name` is the sole source for the share-code `author` field.
- **Acceptance:** Editing the user name in NameRow persists across restarts. Switching theme persists. The file `bio_redesign_settings.json` appears in the platform config dir.
- **SPEC:** §11.1.

### P4.T11 — Path validation status events feed left-rail summary

- **What:** Whenever the debounced auto-validation produces a fresh report (any field's debounce window elapsing — see P4.T11b), update `OrchestratorApp::wizard_state.step1_path_check` (the field BIO's `nav_status::compute_path_validation_summary` already reads) AND the aggregate `path_validation_results.issue_count` on `SettingsScreenState`. The left rail's bottom status line reads from `step1_path_check`; the Paths tab's per-row hints read from `path_validation_results.fields`.
- **Where:** Edit `src/ui/settings/validate_debounce.rs::tick` to call `validate_now::run_now` and `state_validation::run_path_check` after any debounced field becomes due, so both surfaces stay synchronized in one pass.
- **Acceptance:** Editing a path field to a non-existent path, then pausing for 200ms, updates the rail status AND the per-row hint AND the issue count without any explicit user action. Restoring the path clears all three after another 200ms pause.
- **SPEC:** §2.1, §11.2.

### P4.T11b — Per-edit debounced path validation (H11)

- **What:** Build `src/ui/settings/validate_debounce.rs::tick(orchestrator: &mut OrchestratorApp, now: Instant)`. Called once per frame from `OrchestratorApp::update`. When any field's debounce window (`DEBOUNCE_MS = 200`) has elapsed, runs `validate_now::run_now` once (cheap at this scale — ~10 fields) and refreshes both the per-field map and the rail-status `step1_path_check`. Clears the elapsed entries from `path_edit_debounce`. State stored in `SettingsScreenState::path_edit_debounce`.

  **Lazy-paint hazard.** egui's `eframe::App::update` is only invoked on user input by default. If the tick fires once after typing settles but nothing else queues a frame, the validation logic literally never runs until the user moves the mouse — making the perceived validation lag balloon to seconds. The orchestrator's `update` calls `ctx.request_repaint_after(next_debounce_due_in)` immediately after `validate_debounce::tick` so the next frame fires exactly when the soonest pending debounce window will elapse. Without this, typing → wait felt like ~5s in practice.

  **Pending indicator.** While a field's debounce timer is still ticking, `tab_paths.rs` renders `checking…` in that row's status slot (tone Neutral, so the border doesn't flash a stale state). Once the tick fires, the actual `PathStatus` replaces it.

  Edit hook: `tab_paths.rs` calls `validate_debounce::mark_dirty(orchestrator, "bgee_game_folder")` (etc.) on every `path_row` `on_change` callback.

  Per-keystroke typing does not fire a validation per keystroke; only when the user pauses for ≥200ms does the validation run.
- **Where:** New file + edits to `tab_paths.rs` to mark fields dirty on change + edit `OrchestratorApp::update` to invoke `tick` and queue the timed repaint.
- **Acceptance:** Typing a path one character at a time, with no pause: only one validation runs (after the final keystroke + 200ms idle). The `checking…` indicator is visible during that window. Validation results appear in the row's status slot and in the left-rail status line ~200ms after the last keystroke, even if the user doesn't touch the mouse afterwards.
- **SPEC:** §11.2 (path validation timing), Open-question resolution per H11.

## Open questions / risks

- **WeiDU detected version.** BIO's existing Step 1 page displays the WeiDU version inline. Confirm which BIO function returns it (likely `src/core/install/weidu_exec.rs` runs `weidu --help` and parses the banner) and that it's `pub fn` or same-crate-reachable `pub(crate) fn`. Reuse in `tab_tools` and in the left-rail status line.
- **Removed Settings (§11.6).** The redesign explicitly drops `Enable WeiDU logging options (-u) master toggle` and `Advanced mode toggle`. The underlying `Step1Settings::weidu_log_mode_enabled` field stays for backwards compat but is never read by the new Settings screen; the install runner derives the value from §13.12 policy #2.
- **OAuth popup visibility verification.** Before P4.T9 begins, verify that `bio::ui::step1::github_auth_popup_step1::render` and the relevant BIO OAuth flow-runner functions (`bio::app::app_step1_github_oauth::*`) are `pub fn` or `pub(crate) fn` (same-crate reachable per the Phase 1 split). The CRITICAL DIRECTIVE forbids flipping visibility on these; the SPEC §1 decision order's fallback is the sibling in `oauth_glue.rs` (already planned). If any required entry is `pub(super)` or private, the sibling drives the device flow against BIO's lower-level public surfaces (the `ureq` HTTP helpers in `bio::app::app_step1_github_oauth` if those are `pub`, the `keyring` storage helpers if those are `pub`). Document the chosen path in `oauth_glue.rs`. The OAuth surface qualifies as **complex** (channel receivers, multi-step device flow, settings-store coordination), so the sibling — not a duplicate full re-implementation — is the right answer; new behavior in the sibling only reaches for BIO's existing public helpers.
- **OAuth dispatch surface (C2 audit outcome).** The C2 audit found that `WizardApp::handle_step1_action` mutates `self.step1_github_auth_rx` (a channel receiver, not in `state`). The function does **not** qualify for carve-out #4 refactor. The orchestrator's `oauth_glue.rs` replicates the dispatch logic inline, owning its own `github_auth_rx` field and calling the underlying `bio::app::app_step1_github_oauth::start_github_oauth_device_flow` (`pub fn`) directly. Read `bio::ui::app::update_loop::run` (the real H3-corrected path) and `bio::ui::app::WizardApp::handle_step1_action` (read-only) for the dispatch map; replicate in `oauth_glue.rs`.
- **Diagnostic mode runtime toggle (per M12).** Toggling Diagnostic mode in Settings → General updates `OrchestratorApp::dev_mode` on the next frame (no restart required). On app launch, the toggle's persisted value is OR'd with the CLI `-d` flag (`dev_mode = cli_flag || redesign_settings.diagnostic_mode`). The CLI flag and the Settings toggle are independent enables — either one is sufficient to activate dev mode.

## Verification

1. `cargo build --bin infinity_orchestrator --release` succeeds.
2. Launch the app, click Settings: the screen renders with five tabs.
3. Switch tabs: each renders without panics.
4. Theme tab: toggle light/dark, observe the entire app re-color.
5. Paths tab: edit a path field one character at a time. Verify validation does NOT run per keystroke (no row-hint flicker during typing). Pause 200ms after the last keystroke: row hint updates with the validation result.
6. Paths tab: click `Validate now`, observe the row's hint update + the left-rail status line update.
7. Accounts tab: click GitHub `connect`, the OAuth popup opens. Completing the flow flips the card to `connected as <name>`.
8. Restart the app: the user name (NameRow), theme, language, and all path values persist.
9. `cargo build --bin BIO --release` continues to succeed; the legacy wizard is unaffected.
