# Infinity Orchestrator — Implementation Plan Overview

## Project summary

Infinity Orchestrator is the redesign of the existing BIO crate (`Cargo.toml` `name = "bio"`, binary `BIO`) into a multi-modlist workspace app. The redesign reframes BIO from a single linear 5-step wizard into four top-level destinations (Home, Install, Create, Settings) with a workspace view that wraps Steps 2-5 for each modlist. The existing install pipeline, WeiDU integration, compat engine, share-code format, scan worker, embedded terminal, mod-source manifests, prompt-eval, and CLI subcommands are preserved verbatim. The redesign adds a new visual language (Poppins + FiraCode Nerd, dark teal-on-slate by default, sketchy borders + drop shadows), a modlist registry (`modlists.json`), per-modlist workspace state files, and a re-skinned chrome. See `infinity_orchestrator/SPEC.md` §1 for the full vision and the CRITICAL DIRECTIVE that gates how existing BIO code may be touched.

## Architecture: library/binary split, two binaries, no hosting

The orchestrator is a **standalone `eframe::App` (`OrchestratorApp`) shipped from a new binary entry (`infinity_orchestrator`)** that lives **inside the same crate** as the existing `BIO` binary. The crate is restructured from its current single-binary layout into a `lib + 2 bins` layout. Both binaries link the library; the library exposes the entire BIO module tree as `bio::*`. This restructure is authorized by SPEC §1 CRITICAL DIRECTIVE carve-out #3.

| Binary | Entry file | Entry function | Behavior |
|--------|------------|----------------|----------|
| `BIO` (existing) | `src/main.rs` (becomes a 5-15 line shim) | `bio::ui::run(dev_mode)` (via `crate::app::dispatch::run` for non-GUI subcommands) | Legacy linear wizard, behavior unchanged. |
| `infinity_orchestrator` (new) | `src/bin/infinity_orchestrator.rs` | new — constructs and runs `bio::ui::orchestrator::OrchestratorApp` | The redesign. |

### Library/binary split (carve-out #3)

The CRITICAL DIRECTIVE's structural-split carve-out (SPEC §1, carve-out #3) authorizes the following **mechanical, behavior-preserving** restructure:

- Add **`src/lib.rs`** declaring the entire module tree that `src/main.rs` declares today. The declarations move verbatim:

  ```rust
  // src/lib.rs (new)
  #[path = "core/app/mod.rs"]
  pub mod app;
  #[path = "core/cli/mod.rs"]
  pub mod cli;
  #[path = "core/config/mod.rs"]
  pub mod config;
  #[path = "core/install/mod.rs"]
  pub mod install;
  #[path = "core/logging/mod.rs"]
  pub mod logging;
  #[path = "core/mods/mod.rs"]
  pub mod mods;
  #[path = "core/parser/mod.rs"]
  pub mod parser;
  #[path = "core/platform_defaults.rs"]
  pub mod platform_defaults;
  pub mod settings;
  pub mod ui;
  ```

  Visibility is `pub mod` for every module so both binaries can reach the library's public surface. Anything inside that was `pub(crate)` remains reachable: the orchestrator binary's code lives **inside the library crate** (as `bio::ui::orchestrator::*` — see below), so `pub(crate)` items stay accessible.

- **`src/main.rs` becomes a thin shim** (~12 lines) that hosts the CLI dispatch logic verbatim from today's `main.rs` but no longer declares any modules. Its body in full:

  ```rust
  // src/main.rs (after split)
  #![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
  use anyhow::Result;
  use clap::Parser;
  use bio::cli::args::{Cli, Command};
  use bio::config::options;
  use bio::logging::setup;

  fn main() -> Result<()> {
      let mut cli = Cli::parse();
      if cli.command.is_none() && cli.help.is_none() && cli.version.is_none() {
          cli.command = Some(Command::Gui);
      }
      setup::init(&cli.log_level)?;
      if let Some(command) = options::from_cli(&cli) {
          match &command {
              options::AppCommandConfig::Gui { dev_mode } => bio::ui::run(*dev_mode)?,
              _ => bio::app::dispatch::run(&command)?,
          }
      }
      Ok(())
  }
  ```

  Behavior bit-for-bit identical to today's `src/main.rs`. The only edits are the deletion of the `#[path] mod ...` block (now in `lib.rs`) and the `crate::` → `bio::` path rewrite on the four `use` lines and the two dispatch call sites.

- **`src/bin/infinity_orchestrator.rs`** is the new binary. It also depends on the library and calls into `bio::ui::orchestrator::OrchestratorApp::new(dev_mode)`. The orchestrator's module tree lives **inside the library** at `src/ui/orchestrator/...` so it is `bio::ui::orchestrator::*`, NOT a separate binary's local module tree. This is critical: orchestrator code and BIO code are in the same crate, so every `pub(crate)` BIO item is reachable from orchestrator code without flipping visibility.

- **`Cargo.toml`** gains a `[lib]` section and two `[[bin]]` entries (build configuration, not source):

  ```toml
  [lib]
  name = "bio"
  path = "src/lib.rs"

  [[bin]]
  name = "BIO"
  path = "src/main.rs"

  [[bin]]
  name = "infinity_orchestrator"
  path = "src/bin/infinity_orchestrator.rs"
  ```

  The `[lib]` block is new. The first `[[bin]]` already exists in today's `Cargo.toml`; the second is added.

Verification: after the split, the following identifiers must be reachable from outside `src/main.rs` (i.e., from the new binary and from orchestrator code under `src/ui/orchestrator/`):

- `bio::cli::args::Cli` (currently `pub struct Cli` per `src/core/cli/args.rs`).
- `bio::app::dispatch::run` (currently `pub fn run` per `src/core/app/dispatch.rs`).
- `bio::config::options::from_cli` and `AppCommandConfig`.
- `bio::logging::setup::init`.
- `bio::ui::run` (currently `pub use run::run` per `src/ui/mod.rs`).

All five are already `pub` in today's source — the split exposes them under `bio::*` instead of `crate::*` when used from the new binary, but no visibility flip is required.

No logic changes anywhere. `main.rs` is reduced to a 12-line shim; `lib.rs` is a new file that declares the module tree. Existing behavior is preserved bit-for-bit. Carve-out #3 authorizes this.

### What the orchestrator owns

`OrchestratorApp` is its own `eframe::App` implementation, defined in `src/ui/orchestrator/orchestrator_app.rs` (inside the library crate). It owns:

- Its **own `bio::app::state::WizardState` instance** (one shared instance, swapped between modlists by the workspace-state loader — see below). `WizardState` is `pub` per BIO's existing API (`src/core/app/state/state_wizard.rs:7` declares `pub struct WizardState`; all step substate fields are `pub`); the orchestrator constructs it directly.
- Its **own `SettingsStore`** for `bio_settings.json` (BIO's settings store is `pub` and reusable).
- Its **own background-thread receivers** for the scan worker, GitHub OAuth flow, update check/download/extract pipeline, and install runtime. These channels are wired the same way `WizardApp` wires them today — by calling the same `bio::app::app_step*_*` orchestration functions that BIO already exposes (now reachable as `pub(crate)` from same-crate orchestrator code).
- The new **modlist registry** (`modlists.json`) and **per-modlist workspace state** files (`modlists/<id>/workspace.json`).
- The new **destination router** (Home / Install / Create / Settings / Workspace).
- The new **theme tokens**, **shell chrome** (titlebar + statusbar), and **left rail**.

### How the orchestrator renders Steps 2-5 inside the workspace

The orchestrator's `Workspace` destination calls BIO's **existing per-step renderers** directly:

```rust
// pseudocode inside workspace_step_router.rs
match step {
    WorkspaceStep::Step2 => bio::ui::step2::page_step2::render(ui, &mut self.wizard_state, self.dev_mode, &self.exe_fingerprint),
    WorkspaceStep::Step3 => bio::ui::step3::page_step3::render(ui, &mut self.wizard_state, self.dev_mode, &self.exe_fingerprint),
    WorkspaceStep::Step4 => workspace_step4::render(ui, self), // Phase 6's new chrome — see C4
    WorkspaceStep::Step5 => bio::ui::workspace::step5::render(ui, &mut self.wizard_state, ...), // Phase 7 wraps with new chrome
}
```

Each `page_stepN::render` has a step-specific signature (verified in source):

- **Step 2:** `pub fn render(ui, &mut WizardState, dev_mode, exe_fingerprint) -> Option<Step2Action>` (`src/ui/step2/page_step2.rs:10`).
- **Step 3:** `pub fn render(ui, &mut WizardState, dev_mode, exe_fingerprint)` returning `()` — no action enum; the page mutates `WizardState` directly (`src/ui/step3/page_step3.rs:7`; see H2 below).
- **Step 4:** `pub fn render(ui, &mut WizardState, dev_mode, exe_fingerprint) -> Option<Step4Action>` (`src/ui/step4/page_step4.rs:8`). **Note:** per Phase 6 P6.T2b (C4), the orchestrator's workspace step router replaces this body with an orchestrator-side renderer; the orchestrator does **not** call `page_step4::render`. The signature is documented here only for the legacy `BIO` binary's path.
- **Step 5:** `pub fn render(ui, &mut WizardState, &mut Step5ConsoleViewState, Option<&mut EmbeddedTerminal>, Option<&str>, dev_mode, exe_fingerprint) -> Option<Step5Action>` — verified in source at `src/ui/step5/page_step5.rs`.

The returned `StepNAction` (where present) is dispatched to the same `bio::app::app_step*_*` orchestration functions BIO's `WizardApp` dispatches to.

The orchestrator handles each step page's returned action by calling the same dispatch functions BIO already exposes (e.g., `bio::app::app_step2_router::handle_step2_action`, which is `pub(crate)` per `src/core/app/app_step2_router.rs:6` — reachable from same-crate orchestrator code). The orchestrator does not call `WizardApp::handle_stepN_action` directly — those are `pub(super)` methods on `WizardApp` defined in `src/ui/app_methods.rs`. Instead, it calls the underlying `bio::app::app_step*_*` functions that those handlers wrap. The dispatch surface is BIO's `bio::app::*` API plus the `pub(crate)` step-action handlers — identical to what `bio::ui::app::update_loop::run` uses (the BIO update loop's real path; see H3 below).

### Workspace state loader (replaces the "orchestrator bridge")

The previous plan's "orchestrator bridge" concept (which translated between a per-modlist registry and a foreign `WizardApp.state`) is **removed**. In its place is a simpler **workspace state loader**:

- **On workspace open** (entering `NavDestination::Workspace { modlist_id }`): the loader reads the registry entry by id, reads `modlists/<id>/workspace.json` (per-modlist `ModlistWorkspaceState`), and **populates the orchestrator's own `WizardState`** with that data — Step 2 selections, Step 3 order, Step 4 review state, prompt overrides. Step 1 fields are also populated: `game_install` is set from the modlist's stored game; path / tools fields come from `bio_settings.json`. **No BIO files touched** — `WizardState` is `pub`, all its fields are `pub`; the loader writes directly.
- **On workspace close, nav-away, or debounce tick**: the loader extracts the current state out of `WizardState` and writes it to `modlists/<id>/workspace.json` (and updates the registry's meta counts).
- **`game_install` write is per-modlist, not global.** Because `WizardState` lives inside `OrchestratorApp` (not in `bio_settings.json`), setting `state.step1.game_install = entry.game` only mutates the orchestrator's in-memory copy. The settings store on disk is untouched.

This pattern matches today's BIO: `WizardApp` populates its own `WizardState` from `bio_settings.json` on startup and reads/writes it directly across frames. The orchestrator does the same thing — it just swaps the source from "the one global settings file" to "the per-modlist workspace file".

### What this means for the CRITICAL DIRECTIVE

The orchestrator only ever:

1. **Constructs** `WizardState` (which is `pub`).
2. **Reads and writes** `pub` fields on it.
3. **Calls** existing `pub fn` and `pub(crate) fn` entry points in `bio::app::*` and `bio::ui::stepN::page_stepN` (same-crate reachability via carve-out #3).
4. **Owns** background-thread receivers identically to how BIO already does it.

The six authorized carve-outs from SPEC §1 are:

1. **Theme-token extraction** (Phase 8) — pure value substitution of inline color/padding literals.
2. **Window-chrome config flips** (`.collapsible(false)` → `.collapsible(true)`) (Phase 8).
3. **Library/binary structural split** (Phase 1; this overview's section above).
4. **WizardApp → WizardState signature refactor** (Phase 4 / Phase 6 / Phase 7 / where it falls out — tightly scoped per the C2 audit).
5. **Schema-additive serde field additions** — new optional `#[serde(default = ...)]` fields on existing BIO serde structs when the default preserves today's behavior. Used for share-code `allow_auto_install` (Phase 5 / SPEC §13.3) and any future per-modlist serde additions.
6. **State-aware theme-token reads** (Phase 8) — for BIO source files that render redesign-relevant UI surfaces, inline `Color32` literals (or `theme_global::*()` accessors that resolve to them) may be swapped for `redesign_*(palette)` accessors **even when the literal sits inside conditional logic** (hover, selected, conflict tone, dev-mode banner, install-running state, etc.). The conditional structure of the function must be preserved exactly — no new branches, no removed branches, no reordered branches, no logic mutations. Only the color expressions inside each branch change. Each touched function gains a `palette: ThemePalette` parameter; call sites thread it through. Per-file scope documentation is required in Phase 8's file inventory.

Anywhere a fix would need more than these six carve-outs, the plan routes the work per SPEC §1's decision order: direct reuse first; sibling under `src/ui/orchestrator/...` as the default fallback for simple workflows; escalate for a new carve-out only when the workflow is complex and a sibling would carry serious drift risk. No `pub(super)` flips on BIO source (other than what the structural split implies), no new methods on BIO impls.

### New binary entry

```toml
# Cargo.toml addition (build configuration, not BIO source)
[lib]
name = "bio"
path = "src/lib.rs"

[[bin]]
name = "infinity_orchestrator"
path = "src/bin/infinity_orchestrator.rs"
```

The existing `[[bin]] name = "BIO" path = "src/main.rs"` block stays in place. The new binary's `main` constructs and runs `OrchestratorApp` via `eframe::run_native`. Development command: `cargo run --bin infinity_orchestrator`. Release artifact: `target/release/infinity_orchestrator` (or `.exe` on Windows).

## Phasing philosophy

Every phase in this plan must leave the **new binary** in an **alpha-shippable** state. After merging phase N:

- `cargo build --bin infinity_orchestrator --release` succeeds.
- `target/release/infinity_orchestrator` (or `.exe`) launches and runs without panics in the codepaths exposed at that phase.
- The user can navigate around the surfaces this phase introduces. Features that depend on later phases may be stubbed (button visible, click is a no-op with a clear "Coming in phase X" hint, or button disabled) but the app must not crash.
- The legacy `BIO` binary continues to compile and run as today. After Phase 1's structural split, `src/main.rs` is a thin shim that calls `bio::ui::run` (legacy path) — behavior is bit-for-bit identical to today.

Phases progress **foundation → shell → data → screens → runtime → polish**. No phase introduces a backwards-incompatible change to a data file or public API touched by a previous phase. When a phase needs to extend a struct from a prior phase, it does so additively (new fields default to existing behavior).

## Phase table

| # | Title | Summary | Doc |
|---|-------|---------|-----|
| 1 | Library/binary split + theme tokens + fonts + shell modules + new binary entry | Apply the carve-out #3 structural split (`src/lib.rs`, slim `src/main.rs`, `src/bin/infinity_orchestrator.rs`). Add Poppins, dark-default theme tokens, sketchy border/shadow primitives, the shell chrome modules (titlebar + statusbar, not yet invoked), and a placeholder app stub in the new binary. | [phase-01-theme-and-shell.md](phase-01-theme-and-shell.md) |
| 2 | `OrchestratorApp` + left-rail navigation + destination routing | Add the `OrchestratorApp` struct (its own `eframe::App` impl, owns its own `WizardState`), wire the shell chrome around it, add the persistent left rail with 4 destinations + an unreachable `Workspace` destination (stub renderer only — no `WizardApp` hosting). Replace the binary's placeholder stub with the real `OrchestratorApp::run`. | [phase-02-nav-routing.md](phase-02-nav-routing.md) |
| 3 | Modlist registry + workspace state files | Add `modlists.json` model + store + corrupt-file terminal error state, and the per-modlist `modlists/<id>/workspace.json` model. Wire into the orchestrator's persistence cycle. Nothing renders these yet. | [phase-03-modlist-registry.md](phase-03-modlist-registry.md) |
| 4 | Settings screen (5 sub-tabs) + per-edit debounced path validation | Build the new Settings screen with file-folder tabs (General, Paths, Tools, Accounts, Advanced). Reads/writes `bio_settings.json` via the existing `bio::settings::SettingsStore` (instantiated independently by the orchestrator). GitHub OAuth popup invoked as a shared popup using BIO's existing public popup renderer. Validate-now button + per-edit debounced path validation events. | [phase-04-settings.md](phase-04-settings.md) |
| 5 | Home + Install Modlist (paste/preview/download stages) | Build Home (filter chips, modlist cards, Add-a-modlist Box, game-installs-detected block, first-launch empty state, delete confirm dialog). Build Install Modlist: paste + preview fully; downloading as a **chassis only** (live download/extract + per-install dirs + content-addressed staging = Phase 7 P7.T17 / SPEC §13.12a). Reuses BIO share-code parser. Install runtime stub at stage 4. | [phase-05-home-install-paste.md](phase-05-home-install-paste.md) |
| 6 | Create screen + Workspace shell (Steps 2-4) | Build Create's choose mode + setup Box + starting-point cards + Load Draft dialog. Build the `WorkspaceView` shell (title row, rename, fork badge, save-draft, progress bar, nav bar). Workspace **calls BIO's existing per-step `pub fn render`** for Steps 2-3 directly. **Step 4's body is replaced with an orchestrator-side renderer (C4)** that reuses BIO's public save action and Step 3 order data. The workspace state loader populates `WizardState` on open and extracts back on close / nav-away / debounce. | [phase-06-create-workspace-shell.md](phase-06-create-workspace-shell.md) |
| 7 | Step 5 install runtime wrapper + Reinstall + import-code auto-write + concurrency gate + rail-nav lock | Wrap the existing BIO Step 5 install runtime in the new pre-install / during-install / post-install chrome. Add success banner, Share import code dialog (post-install), Return to Home / Open install folder buttons, registry state transition on success, install concurrency policy gate, **rail-navigation lock during an in-flight install (C5)**. Wire Install Modlist stage 4 + Reinstall flow + modlist-import-code.txt auto-write. **P7.T17: per-install directory derivation (incl. the install-critical #2 `-u` / #3 `-p`/`-n` / #4 `-g` dirs) + content-addressed archive staging + import→auto-build pipeline drive that binds the Phase-5 §4.3 chassis live (SPEC §13.12a).** | [phase-07-install-runtime.md](phase-07-install-runtime.md) |
| 8 | Popup theme reskins, automatic flag policies, polish | Apply theme-token extraction refactors to the BIO Compat popup, Prompt popup, Update Check popup, GitHub OAuth popup (per CRITICAL DIRECTIVE's mild-refactor carve-out #1). Apply window-chrome flips (carve-out #2). Apply the **residual** §13.12 flag policies only: #6 (`DestChoice` → prepare/backup mapping) + #7 (hardcoded `-autolog`/`-logapp`/`-log-extern`) + Settings-surface removal. **#1/#5 are Phase 7 P7.T16; #2/#3/#4 install-critical per-install dirs are Phase 7 P7.T17 (an install can't run without them — SPEC §13.12a).** Wire toast notifications, hover affordances, copy-to-clipboard polish. Final smoke pass. | [phase-08-popup-reskins-polish.md](phase-08-popup-reskins-polish.md) |

## Cross-phase dependency note

```
Phase 1 (split + theme + shell + binary entry) ──┬──> Phase 2 (OrchestratorApp + nav routing) ──┬──> Phase 4 (Settings)
                                                 │                                              │
                                                 │                                              ├──> Phase 5 (Home + Install paste)
                                                 │                                              │            ↑
                                                 │                                              │            │
                                                 ├──> Phase 3 (registry) ──────────────────────┴────────────┴──> Phase 6 (Create + Workspace)
                                                 │                                                                          │
                                                 │                                                                          ↓
                                                 │                                                                Phase 7 (Step 5 runtime)
                                                 │                                                                          │
                                                 └────────────────────────────────────────────────────────────────────────> Phase 8 (polish)
```

- Phase 1 includes the carve-out #3 structural split; everything below builds on the library/binary layout.
- Phase 3's registry types are referenced by Phases 5, 6, 7.
- Phase 5's Install paste/preview reuses helpers (`modlist_share::preview_modlist_share_code`) referenced again in Phase 6's fork-paste flow.
- Phase 6's Workspace shell calls BIO Step 2/3 page renderers directly; Step 4's body is rewritten orchestrator-side (C4); Phase 7 wraps Step 5 with new chrome that calls BIO's `page_step5::render`.
- Phase 8's popup reskins are safe to do earlier but are batched at the end to avoid touching BIO files repeatedly across phases.

## How to use this plan

Each phase doc has the following sections — read them in order:

1. **Summary** — one paragraph framing the phase.
2. **What ships after this phase** — concrete observable behavior. Use this as the demo script.
3. **What's still missing** — features deferred to later phases. Helpful for stub design.
4. **Dependencies** — phases that must be merged first.
5. **File inventory** — three sub-lists:
   - *New files* — absolute paths and purpose. **Default to creating these; do not patch BIO files.**
   - *BIO files read from / consumed* — files the new code imports. **Do not modify them.**
   - *BIO files needing allowed mild refactor* — the rare exceptions, each with a one-line justification for which CRITICAL DIRECTIVE carve-out applies (#1 theme-token extraction, #2 `.collapsible()` window-chrome flip, #3 structural split, #4 WizardApp → WizardState refactor). If a task is not justified by one of those four carve-outs, it does not belong here — extract the behavior to a net-new component instead.
6. **Implementation tasks** — numbered list of `P<phase>.T<task>` units. Each task names a file, a location (line range, function name, or "create new file"), an acceptance criterion, and the SPEC §number it traces to.
7. **Open questions / risks** — escalate before implementing.
8. **Verification** — `cargo build --bin infinity_orchestrator` plus manual smoke checks.

### Acceptance criteria

Acceptance criteria are **observable, testable** statements:

- Good: "Clicking the Home left-rail item navigates to the HomeScreen, which renders a `ScreenTitle` reading 'Welcome back, adventurer'."
- Bad: "Home navigation works."

When a criterion involves an existing BIO behavior, name the BIO function or struct that produces it (e.g., "Settings → Paths reads `Step1Settings::bgee_game_folder` from `bio::settings::model::Step1Settings`").

### CRITICAL DIRECTIVE reminders

Re-read SPEC §1 before each phase. The six authorized carve-outs are:

1. **Theme-token extraction.** Replace unconditional inline `egui::Color32::from_rgb(...)` literals (or unconditional `theme_global::*()` accessor calls that resolve to such literals) and inline padding `f32` values with reads from the redesign token surface (`src/ui/shared/redesign_tokens.rs::redesign_*(palette)` accessors). Behavior unchanged; conditional structure (if any) unchanged.
2. **Window-chrome config flips.** One-line changes to `egui::Window` builder calls — specifically `.collapsible(false)` → `.collapsible(true)`. The body, signatures, and behavior of the popup stay identical.
3. **Library/binary structural split.** The carve-out described in detail in the Architecture section above. Mechanical; no logic changes; behavior preserved bit-for-bit.
4. **WizardApp → WizardState signature refactor.** BIO functions whose body only mutates `app.state` may be refactored to take `&mut WizardState` instead of `&mut WizardApp`. Body unchanged; existing call sites inside `WizardApp` update to pass `&mut self.state`. Per-function audit required (see Phase 4's C2 audit table).
5. **Schema-additive serde field additions.** New optional `#[serde(default = ...)]` fields on existing BIO serde structs when the default preserves today's behavior. Used for share-code v1 → v1+ additive evolution.
6. **State-aware theme-token reads.** For BIO source files that render redesign-relevant UI surfaces (Step 2 tree + Details, Step 3 reorder list + toolbar, Step 5 console + status + install row + dev header + cancel-confirm, popup group), inline `Color32` literals or `theme_global::*()` accessor calls may be swapped for `redesign_*(palette)` accessors **even when the literal sits inside conditional logic that decides between colors based on state** (hover, selected, conflict tone, dev-mode banner, install-running, prep-running, locked, disabled, kind, etc.). The conditional structure of the function must be preserved exactly — no new branches, no removed branches, no reordered branches, no logic mutations, no behavior changes. Only the color expressions inside each branch change, and only to swap `theme_global::*()` / inline `Color32::from_rgb(...)` calls for `redesign_*(palette)` calls. The function gains a `palette: ThemePalette` parameter; call sites thread it through. Per-file scope documentation required (see Phase 8 file inventory).

**All other behavior changes** belong in **new** files (e.g., `src/ui/orchestrator/orchestrator_app.rs`, `src/ui/home/page_home.rs`, `src/ui/workspace/workspace_view.rs`, `src/registry/modlists_store.rs`). No `pub(super)` flips (beyond what the structural split implies), no new methods on BIO impls.

If a phase doc reads as though it asks for a behavior change inside an existing BIO file (anything other than the six carve-outs above), stop and follow SPEC §1's decision order: prefer direct reuse of any reachable `bio::*` public API; if the workflow is **simple** (state mutation, dialog wrapper, format helper, single-screen render) and direct reuse isn't a clean fit, default to building a sibling under `src/ui/orchestrator/...`; only escalate for a new carve-out if the workflow is **complex** (install-pipeline coordination, share-code interop, settings-store coordination across screens) and a sibling re-implementation would carry serious drift risk.

### Wireframe references

Visual reference is `infinity_orchestrator/wireframe-preview/build.html` (preview HTML) and the source files:

- `infinity_orchestrator/wireframe-preview/index.html` — CSS tokens (color hex values, spacing).
- `infinity_orchestrator/wireframe-preview/app.jsx` — top-level shell + routing skeleton.
- `infinity_orchestrator/wireframe-preview/screens.jsx` — every named screen and popup component (e.g., `HomeScreen`, `WorkspaceView`, `InstallScreen`, `CreateScreen`, `SettingsScreen`, `SharePasteCodeDialog`, `LoadDraftDialog`).

The `tweaks-panel.jsx` file is wireframe-iteration-only — its contents do not ship (SPEC §14.2).

## Revision log

### 2026-05-12 — C1 and C4 resolution pass

**Scope:** Architectural fixes for adversarial-review findings C1 (hosted-WizardApp visibility violation) and C4 (`main.rs` modification disallowed). **Current status of all review findings: Critical — C1-C5 all resolved. High — H1-H11 all resolved. Medium/Low — addressed via bulk-fix passes (most recent: 2026-05-13); remaining items are doc-cleanup tier.**

> **Staleness clarification (per L10).** Earlier revision logs marked C1/C3 as resolved before the actual fixes landed; the architecture pivot completed only in the latest revision pass (the standalone-orchestrator + lib+bin split + C3's triple condition). The plan's current state reflects fully-applied resolutions — read the C1/C2/C3/C4/C5 + H-tier collateral pass entry below for the file-level scope, and the 2026-05-12 plan-file finalization pass entry for the per-file edits that actually landed.

**Changes:**

- Architecture section added at the top of this overview describing the standalone-orchestrator pattern, the workspace state loader, and the new binary entry.
- `OrchestratorApp` is now standalone — it does not host, embed, wrap, or compose `WizardApp`.
- `OrchestratorApp` owns its own `WizardState` — confirmed `pub` in `src/core/app/state/state_wizard.rs:7` with all step substate fields `pub`.
- The "orchestrator bridge" concept was removed.
- Per-modlist game choice is local — setting `wizard_state.step1.game_install` from the modlist entry mutates only the orchestrator's in-memory `WizardState`, not `bio_settings.json`.
- `src/main.rs` was no longer modified in any phase (the previous-pass solution; superseded by the 2026-05-12 second pass below).
- Phase 1, 2, 3, 4, 6, 7 docs revised in place to reflect the new architecture.
- Verification commands updated across phases from `cargo build --release` to `cargo build --bin infinity_orchestrator --release` (the relevant binary).

### 2026-05-12 — C1/C2/C3/C4/C5 + H-tier collateral pass (this revision)

**Scope:** Address the five remaining critical findings from the adversarial review (C1 visibility/access, C2 WizardApp→WizardState audit, C3 errors_detected fiction, C4 Step 4 body replacement, C5 rail-nav lock) plus the H-tier collateral fixes that fall out naturally. The user approved all resolution paths.

**Carve-outs the SPEC now authorizes (SPEC §1 CRITICAL DIRECTIVE, expanded):**

1. **#1 Theme-token extraction** (pre-existing).
2. **#2 Window-chrome config flips** (pre-existing).
3. **#3 Library/binary structural split** (NEW — authorizes the `src/lib.rs` + slim `src/main.rs` + `src/bin/infinity_orchestrator.rs` restructure).
4. **#4 WizardApp → WizardState signature refactor** (NEW — authorizes refactoring BIO functions whose mutation surface is exclusively `app.state` to take `&mut WizardState` directly).

**Changes applied in this revision:**

- **C1 — Library/binary split** (Phase 1).
  - `src/lib.rs` declares the entire BIO module tree as `pub mod app; pub mod ui; pub mod settings; pub mod logging; pub mod cli; pub mod config; pub mod install; pub mod mods; pub mod parser; pub mod platform_defaults;` (mirroring today's `src/main.rs` `mod` block but with `pub` visibility).
  - `src/main.rs` becomes a 12-line shim that calls `bio::ui::run` / `bio::app::dispatch::run`. CLI parsing logic is preserved verbatim; only the `crate::` → `bio::` path rewrite changes.
  - `src/bin/infinity_orchestrator.rs` calls into `bio::ui::orchestrator::OrchestratorApp::new(...)`.
  - `Cargo.toml` gains a `[lib] name = "bio"` block and the `[[bin]] name = "infinity_orchestrator"` block. The existing `[[bin]] name = "BIO"` block stays.
  - Result: orchestrator code lives inside the library crate at `bio::ui::orchestrator::*`, so `pub(crate)` BIO items remain reachable.

- **C2 — WizardApp → WizardState signature refactor audit** (Phase 4, Phase 6, Phase 7).
  - The audit table (with verification commands and outcome per function) lives in each relevant phase. Summary:
    - `src/ui/app_step2_log.rs::apply_weidu_log_selection(app: &mut WizardApp, bgee: bool)` — **Stays as `&mut WizardApp`**. Body calls `app.save_settings_best_effort()` (a `WizardApp` method), so the function touches more than `app.state`. The orchestrator builds a net-new sibling that calls the underlying `bio::app::app_step2_log::apply_weidu_log_selection_from_path(&mut state, bgee, log_path)` (`pub fn`) directly without going through `WizardApp`.
    - `src/ui/app_methods.rs::handle_step1_action` — **Stays as `&mut WizardApp`**. Body mutates `self.step1_github_auth_rx` (a channel receiver, not in `state`). Orchestrator builds a net-new equivalent in `src/ui/settings/oauth_glue.rs` that owns its own receiver field on `OrchestratorApp`.
    - `src/ui/app_methods.rs::handle_step2_action` — **Stays as `&mut WizardApp`**. Body calls `super::step2_router::handle_step2_action(self, action)` which itself takes `&mut WizardApp` because it mutates `app.step2_scan_rx`, `step2_cancel`, `step2_progress_queue`, etc. (channel receivers). Orchestrator dispatches actions through its own equivalent that calls `bio::app::app_step2_router::handle_step2_action(&mut state, &mut scan_rx, &mut cancel, ..., action)` (the underlying `pub(crate) fn` per `src/core/app/app_step2_router.rs:6`) directly — `bio::app::app_step2_router::handle_step2_action` already takes only state + per-receiver `&mut` args, no `WizardApp`.
    - `src/ui/app_methods.rs::handle_step4_action` — **Refactor authorized but unnecessary**. Body calls `super::step4_flow::handle_step4_action(&mut self.state, action)`, which is just a re-export of `bio::app::app_step4_flow::handle_step4_action(state: &mut WizardState, action: Step4Action)` — that public `pub(crate) fn` per `src/core/app/app_step4_flow.rs:8` already takes `&mut WizardState`. The orchestrator calls it directly. No carve-out #4 refactor needed.
  - **Net effect:** no BIO functions are refactored under carve-out #4 in this pass. The audit found that every candidate either (a) genuinely needs `WizardApp` fields beyond `state` (channels), or (b) already takes `&mut WizardState` at the underlying `bio::app::*` layer and can be called directly. The carve-out remains available for future cases.

- **C3 — Replace `errors_detected` with real BIO success-detection fields** (Phase 7).
  - **Verified via `src/core/app/state/state_step5.rs:12-66`:** `Step5State` has no `errors_detected` field. The real success-detection fields are:
    - `pub last_install_failed: bool` (set in `src/core/app/step5_runtime_status.rs:72` from `term.likely_failure_visible()` on exit).
    - `pub last_exit_code: Option<i32>` (set in `src/core/app/step5_runtime_status.rs:73`).
    - `pub install_running: bool` (toggled false on exit at `src/core/app/step5_runtime_status.rs:74`).
    - `pub has_run_once: bool`.
  - **BIO's canonical "clean install" check** (per `src/ui/step5/content/content_install_row_step5.rs:115-117`): `state.step5.has_run_once && !state.step5.install_running && state.step5.last_exit_code == Some(0)`. This is the gate BIO uses for its own "Export Modlist…" button.
  - **Replacement everywhere the plan said `errors_detected`:** the condition becomes `state.step5.install_running == false && state.step5.last_exit_code == Some(0) && state.step5.last_install_failed == false`. The phrase "clean exit" in the plan maps to this triple.
  - Updated tasks in Phase 7: P7.T4 (success-banner gate), P7.T6 (`flip_to_installed` trigger), P7.T9 (concurrency gate's `install_in_progress` check — uses `install_running`), and the post-install action row visibility (P7.T5).

- **C4 — Replace BIO's Step 4 body with an orchestrator-side renderer** (Phase 6, P6.T2b rewritten).
  - The orchestrator's Step 4 wrapper (`src/ui/workspace/step4/workspace_step4.rs`) renders the entire Step 4 panel in new code: top action row (Save button + count), tab row (game tabs), and the line-numbered monospace review list per SPEC §8.1.
  - **The Save button calls BIO's existing public save logic.** Verified: `bio::app::step4_weidu_log_export::auto_save_step4_weidu_logs(state: &mut WizardState) -> Result<(), String>` is `pub(crate) fn` per `src/core/app/step4_weidu_log_export.rs:50` and takes `&mut WizardState` directly. The orchestrator calls it without any C2-style refactor. (BIO's `bio::app::app_step4_flow::handle_step4_action` wraps it but the orchestrator reaches the underlying function directly — both are reachable from same-crate orchestrator code per the C1 split.)
  - BIO's `bio::ui::step4::page_step4::render` is **not called** from the workspace step router for v1 alpha. (Today's BIO continues to invoke it from `WizardApp`'s update loop; the orchestrator just doesn't.)
  - The line-numbered monospace list reads from `wizard_state.step3.bgee_items` / `wizard_state.step3.bg2ee_items` — verified `pub` fields per `src/core/app/state/state_step3.rs:23-24`, each a `Vec<Step3ItemState>`. The wrapper uses the same three-color WeiDU-line syntax per SPEC §6.7 — implemented in `src/ui/workspace/widgets/weidu_line.rs`.
  - Exact-log mode: when `install_mode == install_exactly_from_weidu_logs`, the wrapper renders the read-only viewer + Check Mod List button per SPEC §8.2 / A.7. The Check Mod List button triggers `bio::app::app_step4_flow::handle_step4_action(state, Step4Action::CheckMissingMods)` — `pub(crate) fn`, reachable same-crate.
  - SPEC §8.1 / §8.2 BIO-fidelity callouts for the save action stay (the save logic is reused via the public `auto_save_step4_weidu_logs`); the rendering surface is **net-new**, not BIO-fidelity. Phase 6's P6.T2b reflects this.

- **C5 — Rail-navigation lock during in-flight install** (Phase 7, new P7.T9b).
  - When `install_runtime::install_concurrency::install_in_progress(orchestrator)` returns `Some(RunningInstall { modlist_id, .. })`, every rail item except the currently-active modlist's workspace is disabled with the SPEC §13.15 tooltip.
  - Home / Install / Create / Settings rail buttons all show the tooltip and are non-clickable during an install.
  - The user can stay in the running install's workspace and watch it; they cannot navigate away. To navigate away they must cancel the install first.
  - **Workspace state loader is never invoked while an install is running** — eliminating the swap-mid-install corruption risk.
  - Phase 7's Open Questions are updated to remove the C5 risk language. The `registry_snapshot` mitigation from H8 is **dropped** (it was protecting the wrong thing — the rail-nav lock makes it moot).

**H-tier collateral fixes applied:**

- **H2** — Step 3's `page_step3::render` returns `()`, not `Option<Step3Action>`. Verified at `src/ui/step3/page_step3.rs:7-9`. The plan's `step_action_dispatch.rs` and Phase 6's P6.T2 are updated to skip the Step 3 dispatch arm (the page handles its actions internally; no action enum bubbles up).
- **H3** — `crate::ui::app_update_loop` is **not** a public module. The real path is `bio::ui::app::update_loop`, which is a `mod` (private) inside `src/ui/app.rs` (declared at `src/ui/app.rs:31-32` via `#[path = "app_update_loop.rs"] mod update_loop;`). Plan references that previously read "see `app_update_loop::run` for the dispatch surface" now correctly cite `bio::ui::app::update_loop::run` and clarify it as a **read-only reference path** — the orchestrator never calls it.
- **H4** — Persistence-on-exit uses `eframe::App::on_exit` as the primary hook + `Drop for OrchestratorApp` as the fallback. Documented in Phase 3 P3.T7 and Phase 6 P6.T15.
- **H5** — The orchestrator's `OrchestratorApp::new(dev_mode)` calls `bio::app::app_bootstrap_init::initialize(dev_mode)` directly (now reachable via lib split + same-crate orchestrator). `initialize` is `pub(crate) fn` per `src/core/app/app_bootstrap_init.rs:17`. No "copy the relevant logic" fallback needed. Documented in Phase 2 P2.T3.
- **H7** — `install_redesign_fonts` builds a complete `FontDefinitions` from scratch (no additive composition on top of `configure_typography`'s output). Phase 1 P1.T1 updated: the new function calls `egui::FontDefinitions::default()`, registers `poppins_light` / `poppins_medium` / `poppins_bold` / `firacode_nerd`, and replaces the proportional + monospace families with these new ones. `configure_typography` then runs on the already-installed font definitions (it only mutates `Style`, not `FontDefinitions`). The two functions compose correctly with this ordering: call `install_redesign_fonts` first, then `configure_typography`.
- **H8** — The Phase 7 `registry_snapshot` mitigation is **dropped**. C5's rail-nav lock makes mid-install registry corruption moot (the user cannot navigate the orchestrator while an install runs; external mutation to `modlists.json` is out-of-scope for v1 alpha). Phase 7 P7.T3 simplified.
- **H9** — Step 5's post-install buttons (`Return to Home`, `Open install folder`) are rendered **immediately above** BIO's embedded Step 5 panel, not below. Per SPEC §9.2 "next to the disabled Install button." BIO's `page_step5::render` puts the Install button at the top of its panel; the orchestrator's chrome row above the panel sits visually adjacent to it. Phase 7 P7.T5 updated. The alternative (replacing BIO's Step 5 body wholesale per C4) was rejected as too heavy for v1 alpha.
- **H10** — Wireframe label is "Clear contents", not "Replace contents". Phase 5 P5.T9 updated.
- **H11** — Per-edit debounced path validation is a real Phase 4 task (P4.T11b), not just an Open Question. Debounce window 200ms idle per keystroke (authoritative; matches SPEC §11.2 and `validate_debounce::DEBOUNCE_MS`).

**What remains open (not addressed in this pass):**

- M/L items from the adversarial review (a later polish pass).
- Real game launcher for the Home `play` button (out of scope for v1 alpha; opens install folder as fallback).
- Real Nexus Mods / Mega OAuth integration (out of scope for v1 alpha; stubbed).
- The Explore tab (SPEC v2 track, not v1 alpha).
- ~~Step 2 / Step 3 / Step 4 details panel and tree theme-token extraction (Phase 8 carve-out audit; current Phase 8 scope only covers popups and Step 5 console line tones).~~ **Now covered.** Phase 8 has been rewritten to cover the full visual-fidelity sweep under carve-out #6 (state-aware theme-token reads): Step 2 tree + Details, Step 3 reorder list + toolbar, Step 5 console + status + install row + dev header + cancel-confirm, plus the popup group already in pre-revision scope. See the 2026-05-13 revision log entry below.

### 2026-05-12 — Plan-file finalization pass (this writing)

**Scope:** Finalize the C1/C2/C3/C4/C5 + H-tier revision in the plan files themselves. The previous entry above documented intent; this entry records the file-level edits.

**Per-file changes:**

- **`phase-07-install-runtime.md`** (largest edit set):
  - Header title gains "+ rail-nav lock".
  - Summary block adds a new "Clean-exit definition (per C3, replaces every `errors_detected` reference)" paragraph that cites `state_step5.rs:12-66`, `step5_runtime_status.rs:72-74`, and `content_install_row_step5.rs:115-117`. The C3 triple — `state.step5.install_running == false && state.step5.last_exit_code == Some(0) && state.step5.last_install_failed == false` — replaces every `errors_detected == 0` occurrence in the phase doc.
  - "What ships" gains a C5 rail-lock bullet.
  - File inventory adds `src/install_runtime/rail_lock_reason.rs` (new file for the `RailLockReason` enum).
  - `src/install_runtime/install_concurrency.rs` description updated to note it powers both P7.T9 (per-button gate) and P7.T9b (rail-nav lock).
  - `success_banner.rs` and `post_install_actions.rs` descriptions updated to cite the C3 triple and H9 positioning (above BIO's panel, not below).
  - P7.T2 task body rewritten: the post-install action row renders above the embedded panel (per H9), immediately below the success banner row; both chrome rows sit above the BIO panel.
  - P7.T3 task body: the `registry_snapshot` step is replaced with a "No `registry_snapshot` taken (per H8: dropped)" explanatory bullet.
  - P7.T4 (success banner) and P7.T6 (registry transition) gate conditions rewritten using the C3 triple.
  - P7.T5 (post-install action row) repositioned above BIO's panel per H9; the rationale (SPEC §9.2 "next to the disabled Install button") is documented inline.
  - **New P7.T9b** added: the C5 rail-navigation lock. Disables every left-rail item when `install_in_progress(...)` returns `Some`. Eliminates the workspace state loader's swap-mid-install path.
  - Every `crate::ui::*` / `crate::app::*` reference in the phase doc rewritten to `bio::ui::*` / `bio::app::*` (since the orchestrator code uses the library-crate path).
  - `crate::ui::app_update_loop::run` corrected to `bio::ui::app::update_loop::run` with the H3 read-only-reference clarification.
  - Open Questions section: the `registry_snapshot` mitigation paragraph rewritten to explain why it was dropped (per H8). New "C5 risk closure" paragraph confirms the swap-mid-install risk is eliminated by the rail lock.
  - Verification section: numbered steps updated to verify the C3 triple, the C5 rail lock (item #3 and #8), the H9 positioning (item #4), and a new item #11 explicitly verifies the failure path (no success banner, no transition).

- **`overview.md`** (this file):
  - This finalization log entry added.

- **`phase-01-theme-and-shell.md`**, **`phase-02-nav-routing.md`**, **`phase-03-modlist-registry.md`**, **`phase-04-settings.md`**, **`phase-05-home-install-paste.md`**, **`phase-06-create-workspace-shell.md`**, **`phase-08-popup-reskins-polish.md`**: no further edits in this pass — these were already aligned with the intent of the earlier 2026-05-12 revision pass (each carries the H-tier note appropriate to its scope).

**Verification of the BIO source citations:**

- `Step5State` field check (`src/core/app/state/state_step5.rs:12-66`): `last_install_failed`, `last_exit_code`, `install_running`, `has_run_once` are all `pub`. No `errors_detected` exists.
- `step5_runtime_status::process_exit_event` (`src/core/app/step5_runtime_status.rs:59-111`): sets `last_install_failed = term.likely_failure_visible()` at line 72; `last_exit_code = finished_exit` at line 73; `install_running = false` at line 74. Confirms the C3 triple is the right gate.
- `content_install_row_step5.rs:115-117`: confirms BIO's own clean-exit gate uses `has_run_once && !install_running && last_exit_code == Some(0)` — the C3 triple is equivalent (the `!last_install_failed` check tightens it; BIO's existing gate relies on `last_exit_code == Some(0)` alone, which is fine because a non-zero exit always sets `last_install_failed = true` via `likely_failure_visible`).
- `step4_weidu_log_export::auto_save_step4_weidu_logs` (`src/core/app/step4_weidu_log_export.rs:50`): `pub(crate) fn auto_save_step4_weidu_logs(state: &mut WizardState) -> Result<(), String>`. Confirms C4's "Save calls BIO's existing public save logic" — orchestrator calls it directly via the carve-out #3 same-crate path.
- `app_step4_flow::handle_step4_action` (`src/core/app/app_step4_flow.rs:8`): `pub(crate) fn handle_step4_action(state: &mut WizardState, action: Step4Action)`. Confirms C4's `CheckMissingMods` dispatch works without a carve-out #4 refactor.
- `app_step2_router::handle_step2_action` (`src/core/app/app_step2_router.rs:6`): `pub(crate) fn handle_step2_action(state: &mut WizardState, scan_rx: &mut ..., cancel: &mut ..., ..., action: Step2Action)`. Confirms C2's audit-table conclusion: orchestrator calls this directly with its own receivers; no `&mut WizardApp` refactor needed.
- `app_step2_log::apply_weidu_log_selection` (`src/ui/app_step2_log.rs:10`): body calls `app.save_settings_best_effort()` at line 31, confirming the C2 audit's "stays as `&mut WizardApp`" verdict — orchestrator builds a net-new sibling instead.
- `state_step3.rs:23-24`: `pub bgee_items: Vec<Step3ItemState>`, `pub bg2ee_items: Vec<Step3ItemState>` — the canonical Step 3 order fields used by C4's `step4_review_list`.

**Net effect:** every plan file is now internally consistent with the SPEC §1 four-carve-out directive. No reviewer-flagged stale reference (`errors_detected`, `registry_snapshot`, `crate::ui::app_update_loop`, etc.) remains in the implementation tasks.

### 2026-05-13 — Carve-out #6 authorization + Phase 8 full-fidelity sweep pass

**Scope:** Reverse the prior Phase 8 pruning that deferred Step 2 / Step 3 / Step 5 sub-renderer reskins to a post-v1-alpha release. The user authorized a new carve-out #6 (state-aware theme-token reads) in SPEC §1, which permits inline color literal / `theme_global::*()` accessor swaps **even when the call sits inside conditional logic that picks between colors based on state** — provided the conditional structure of the function is preserved exactly. Phase 8's file inventory is rewritten to cover every BIO file the orchestrator embeds that carries color sites.

**Carve-outs the SPEC now authorizes (SPEC §1 CRITICAL DIRECTIVE, expanded to six):**

1. **#1 Theme-token extraction** (pre-existing) — unconditional value substitution.
2. **#2 Window-chrome config flips** (pre-existing).
3. **#3 Library/binary structural split** (pre-existing).
4. **#4 WizardApp → WizardState signature refactor** (pre-existing).
5. **#5 Schema-additive serde field additions** (pre-existing).
6. **#6 State-aware theme-token reads** (NEW — authorizes inline color swaps inside hover/selected/conflict/dev-mode/install-state/etc. conditionals, with the conditional structure preserved exactly. Each touched function gains a `palette: ThemePalette` parameter; call sites thread it through).

**Changes applied in this revision:**

- **Phase 8** rewritten in full. The pruned table that previously listed only the popup group + Step 5 console line tones is replaced by:
  - A carve-out #1 + #2 table covering the popup group (`compat_window`, `compat_popup`, `prompt_popup`, `update_check_popup`, `update_check_popup_lists`, `update_check_popup_source_editor`, `github_auth_popup`) plus unconditional accent-color sites in `format_step2.rs`, `format_step3.rs`, `tree_parent_step2.rs` (PROMPT pill).
  - A carve-out #6 table covering the conditional sites in `compat_popup_step2.rs`, `tree_compat_display_step2.rs`, `tree_component_row_step2.rs`, `tree_header_marker_step2.rs`, `details_pane_step2.rs`, `details_paths_step2.rs`, `details_selection_step2.rs`, `list_rows_step3.rs`, `content_step3.rs`, `toolbar_compat_step2.rs`, `content_install_row_step5.rs`, `content_cancel_step5.rs`, `content_dev_header_step5.rs`, `status_phase_step5.rs`, `status_console_step5.rs`.
  - An audited-and-confirmed-out-of-scope table covering files on the candidate list that contain no color sites (`tree_render_step2.rs`, `details_meta_step2.rs`, `update_check_popup_report_step2.rs`, `list_step3.rs`, `content_step5.rs`, `status_input_row_step5.rs`, `status_bar_step5.rs`, `menus_step5.rs`, the four `prompts/*` files, `top_panels_step5.rs`, `app_nav_ui.rs`).
- **Per-file line-number citations** added for every conditional site that gets the swap, with explicit note on the structure that must be preserved.
- **Implementation tasks split into per-cluster batches** (P8.T2 = popup window-chrome flips; P8.T3 = unconditional swaps; P8.T4 = Step 2 tree + Details; P8.T5 = Step 3; P8.T6 = Step 5 sub-renderers). Each task explicitly notes the diff bar: signature changes adding `palette`, threaded call sites, accessor swaps inside existing branches, nothing else.
- **What ships after this phase** rewritten to enumerate every workspace surface that now matches the wireframe palette in v1 alpha. The deferred-to-post-alpha language is removed.
- **What's still missing** trimmed: the "Step 2 / 3 / 4 visual reskin deferred" entry is deleted (carve-out #6 covers it). The remaining items (Explore tab, asset packaging UI, real Nexus/Mega integration, real game launcher) are unchanged.
- **Overview revision log entry** (this entry) added. The "What remains open" entry from the prior log that previously read "Step 2 / Step 3 / Step 4 details panel and tree theme-token extraction" is struck through and replaced with a "now covered" note pointing here.
- **CRITICAL DIRECTIVE reminders** updated to enumerate all six carve-outs (the four pre-existing plus #5 schema-additive and #6 state-aware).
- **The "four authorized carve-outs from SPEC §1"** paragraph in the Architecture section (overview line ~155) updated to "six authorized carve-outs" with #5 and #6 added.

**Per-file changes:**

- **`phase-08-popup-reskins-polish.md`**: rewritten in full per the above.
- **`overview.md`** (this file): carve-out list updated to six; the prior-revision "What remains open" entry struck through with redirect; this revision log entry added.
- **No other phase docs touched.** Phase 1's `redesign_tokens.rs` accessor surface is extended in Phase 8 P8.T1 (additive — no Phase 1 doc edit needed; the additive nature is in scope per "Phase 1 new files are editable as Phase-1 new files" rule).

**Audit-driven scope deltas vs. user's initial candidate list:**

- **Added** (not in user's list, but found during audit): `tree_compat_display_step2.rs` (the lookup table that returns `(text_color, fill_color, label)` for compat kinds — touched by every tree/list pill caller); `tree_header_marker_step2.rs` (the package-marker glyph color); `details_selection_step2.rs` (Checked / State / Reason rows); `toolbar_compat_step2.rs` (active-tab issue badge fallback); `format_step2.rs` + `format_step3.rs` (WeiDU-line tri-color rendering); `content_dev_header_step5.rs` (dev-mode RUST_LOG status line); `content_cancel_step5.rs` (cancel-confirm dialog + warning).
- **Removed from user's list as audited-no-color-sites**: `tree_render_step2.rs`, `details_meta_step2.rs`, `list_step3.rs`, `content_step5.rs`, `status_input_row_step5.rs`, `status_bar_step5.rs`, `menus_step5.rs`, the four `prompts/*` files, `app_nav_ui.rs`. These files exist and render redesign-relevant chrome but have nothing to swap (they delegate to sub-renderers that themselves are in scope, or they render pure structure). They get a `palette: ThemePalette` arg added so they can thread it through to their callees, but no color accessor swaps.
- **`app_nav_ui.rs` specifically out of scope**: the file is action-handler-only (no rendering); the legacy nav button render lives in `src/ui/frame/update_app.rs`, which is part of the legacy `BIO` binary and is **not invoked by `OrchestratorApp`**. The orchestrator builds its own `workspace_nav_bar.rs` in Phase 6 (a net-new file using redesign tokens directly). Per the CRITICAL DIRECTIVE, the legacy `BIO` binary's nav stays BIO-default — carve-out #6 does not extend to surfaces outside the orchestrator's render path.

### 2026-05-15 — Share-code provenance trio (`name` / `author` / `forked_from`) + ForkInfoPopup

**Scope.** The user (final authority) directed that every share code carry, *packed inside the code* (not as a text prefix on the string), the modlist's display **name**, the **author** who generated it, and a **`forked_from`** lineage so original creators stay credited through any number of forks. This is a deliberate spec change, authorized by the user, and supersedes the earlier open question "the share payload has no name/author" (finding C) — it is now structurally solved rather than papered over with fallback copy.

**Design decided (and why):**

- **Generation = net-new orchestrator `pack_meta` sibling; BIO's generator is *not* modified.** `pack_meta` calls `bio::app::modlist_share::export_modlist_share_code` for the canonical payload, then runs a standard zlib + base64url + JSON envelope round-trip that inserts the four keys (`allow_auto_install` + the provenance trio) as siblings into the decoded object and re-emits the same `BIO-MODLIST-V1:` string. It composes — never patches — BIO (CRITICAL DIRECTIVE option 2 / "small format helper" sibling), uses only existing deps (`flate2`, `serde_json`) + a ~20-line standard base64url codec, and passes the payload through as an opaque `serde_json::Value` (zero schema coupling, zero drift). A wrapper-object alternative (BIO code as a field of a new struct) was rejected: it would break BIO's own `preview_modlist_share_code` and force a parallel orchestrator decoder/preview.
- **Consume = carve-out #5, extended from one field to four.** `ModlistSharePayload` gains `#[serde(default)]` `name: Option<String>`, `author: Option<String>`, `forked_from: Vec<ForkAncestor>` (alongside the existing `allow_auto_install`); `ModlistSharePreview` gains the symmetric fields; `share_preview()` propagates them (the "paired in-memory counterpart" for this case). Defaults (`true`/`None`/empty) keep every pre-existing and third-party code bit-for-bit identical to today.
- **`ForkAncestor { name: String, author: String }`**, `forked_from` ordered oldest→newest. **Append-only lineage rule:** on Create → Import-and-modify, the child entry's `forked_from = parent.forked_from ++ [{parent.name, parent.author}]`; a modlist's own identity never appears in its own chain; earlier entries are never rewritten ⇒ the credit invariant.
- **Sourcing:** `author` ← `RedesignSettings.user_name` (SPEC §11.1, already shipped Phase 4; empty ⇒ omitted); `name` ← registry `ModlistEntry.name`; `forked_from` ← computed at fork-import commit (Phase 6).
- **Rendering:** packed `name`/`author` drive the Install + fork preview title/subline (honest generic fallback `Shared modlist` / author-less when absent — subject to wireframe iteration); the lineage is shown in a new **§10.9 ForkInfoPopup**, wired from the workspace header `⑂ view fork details` (§2.2) and a preview `⑂ fork info` affordance (§4.2/§5.3).

**Latent defect fixed by this revision.** The prior plan's `allow_auto_install` strategy (P7.T3/P7.T6: "re-decode the payload, flip the bit, re-encode") was **not implementable as written** — BIO's `base64url_*` / `zlib_*` / `decode_share_payload` envelope helpers are *private* and unreachable from orchestrator code (only `export_/preview_/import_modlist_share_code` are `pub(crate)`). `pack_meta` is now the single canonical generate-side envelope for *all four* keys, resolving this for `allow_auto_install` too.

**SPEC changes applied (this revision):** §1 (carve-out #5 "Modlist-share provenance application" paragraph — exact authorized field set + pack_meta-is-not-a-BIO-mod + per-field rationale); §13.3 (new "Provenance" + "Generation mechanism (`pack_meta`)" subsections; payload intro + carve-out rationale updated to four keys); §4.2 + §5.3 (preview title/subline from packed name/author with honest fallback + `⑂ fork info`; fork-preview shows incoming parent provenance; lineage committed at import not display); §10.9 (new ForkInfoPopup); §2.2 (`⑂ view fork details` → ForkInfoPopup, shown only when chain non-empty); §15 (new `bio_redesign_settings.json` row; registry row notes `author`/`forked_from`); §11.1 (name→author line cross-refs `pack_meta` + empty-name fallback).

**Plan impact (applied in the plan-cascade pass):**

- **Phase 5 Run 4** carve-out scope grows from 1 → 4 schema-additive fields on `ModlistSharePayload`/`ModlistSharePreview` + 4 `share_preview()` propagation lines (still consume-only, still pure carve-out #5, still zero BIO behavior change). P5.T10/T11 updated to read the trio, drive title/subline from packed name/author with the fallback, and add the `⑂ fork info` affordance; the wireframe is the canonical reference for the popup + fallback copy.
- **Phase 6** gains: the fork-import lineage append, the `pack_meta`-based draft/share generation, ForkInfoPopup wiring from the workspace header and fork-preview.
- **Phase 7** P7.T3 / P7.T6 rewritten to generate via `pack_meta` (install-start write, `flip_to_installed` regeneration, Share dialog source) instead of the non-implementable private-primitive approach.
- The legacy BIO Step 5 "Export Modlist…" path stays untouched and field-less by design (consumed via defaults).

**Status:** SPEC + this log updated. Wireframe (canonical UI reference) and the plan-phase cascade (P5.R4 onward + HANDOFF) are updated in the same change set so nothing drifts.

### 2026-05-16 — Directory architecture + content-addressed archives; Run-5 resolution; implementer-brief process fix

**Scope.** Phase 5 Run 5 (Install Downloading stage) surfaced a `SPEC CONFLICT`/`PLAN GAP`: the plan assumed a simple "subscribe to the download channel like `WizardApp`" but BIO produces a download list only as a byproduct of the multi-step `import_modlist_share_code` → saved-log/auto-build pipeline, which terminates in the install runtime. Independent investigation **corrected two wrong premises** in the escalation: it is **not** "BIO can't" (the pipeline exists and is reachable `pub(crate)`) and **not** "the Install screen lacks game paths" (paths are global in Settings → Paths §11.2 and reach the orchestrator's `WizardState` via the established settings-sync — the Install screen must not collect them). The real gap was unwritten spec, now closed.

**Decided (user, final authority):**

- **Five-folder directory model.** Global, Settings-defined ([§11.2](#)): **Mods archive** (all downloads for all modlists, always), **Mods backup**, **Game sources**. Per-install, derived inside the destination: the **Mods extract/stage/scan folder** (removed on clean success), and the **game-install clone dirs** (already specified by §13.12 #3/#4). Most of this (#2/#4/#5 + the clone dirs) was **already specced** in §11.2 + §13.12 #3/#4 — the genuinely new spec content is the per-install Mods folder, the content-addressed archive scheme, and the reuse contract.
- **#5 is not a removal of BIO behavior.** The redesign simply never surfaces BIO's install-into-a-clean-dir-without-cloning path; clone is forced for every install. BIO's capability is unchanged (consistent with §13.12 #3/#4 "always ON … not user-configurable").
- **Content-addressed archives (#1) = net-new orchestrator staging layer wrapping BIO's `app_step2_update_download`/`app_step2_update_extract` — zero BIO modification.** Hash-on-write; same name+hash ⇒ cross-modlist dedupe; same name, different hash ⇒ coexist under a hash-encoded name; modlist lock records the resolved hash; extract selects the matching archive per install.
- **Run-5 resolution:** Phase 5 ships the §4.3 Downloading **chassis** (built, navigable, forward-compatible, empty grid — verified, 187 lib tests, zero BIO source touched). Live wiring (drive the import→auto-build→download/extract pipeline + per-install dirs + content-addressed staging) lands in **Phase 7** with the install runtime, since the pipeline terminates there.

**Process fix (so the phase-local miss does not recur).** The Run-5 implementer reasoned within its phase-local brief (phase-05 + the BIO download engine) and concluded "Install lacks paths," not knowing Phase-4 Settings → Paths supplies them globally + `sync_paths_from_settings` feeds them. The orchestrator then verified the hard technical claim but under-scrutinized that premise. Two standing corrections, applied to `.claude/orchestrator-handoff.md`: (1) every implementer brief now carries an **"Already built / cross-phase context"** block (Settings→Paths globals via `Step1Settings`; `Step1Settings`↔`Step1State` convert; `sync_paths_from_settings`; orchestrator owns `WizardState`; registry; carve-out #5 fields); (2) on verify, the orchestrator must sanity-check an escalation's **premises** against the HANDOFF status table / prior-phase deliverables, not just its novel technical claim.

**Artifacts changed (this revision):** SPEC §13.12a (new — directory architecture + content-addressed archives + pipeline-reuse contract) + §4.3 (Phasing note); plan phase-07 P7.T17 (new — per-install dirs + staging layer + pipeline drive + binds the §4.3 chassis live); plan phase-05 P5.T12 (chassis/Phase-7 split) + Open-questions + Verification; HANDOFF (status + caveat + lessons-learned); `.claude/orchestrator-handoff.md` (Run-5 done + the standing cross-phase-context block + the premise-check verify rule); this log.

**Status:** Phase 5 complete after Run 5's chassis commits. Phase 6 (Create + Workspace shell) is the next dispatch.

### 2026-05-16 — Run-5 QA fixes: destination required (SPEC §4.1) + warning Light-mode legibility

**Scope.** QA of Run 5 surfaced two issues; both fixed (redesign-only, no BIO touch):

1. **Destination was not required.** SPEC §4.1 only *described* the FolderInput; it never required it, and `stage_paste.rs` gated the footer primary on the share code alone — you could click `Preview →` / `Continue Install →` with no destination. Per the user (final authority): **§4.1 amended** — a valid destination is required in every mode; the primary stays disabled until `destination` resolves to a **real existing directory** (typed garbage / empty does not satisfy it; `browse` does), with an actionable hint while blocked. Enforced in `stage_paste.rs`; P5.T9 acceptance updated.
2. **`DestinationNotEmptyWarning` unreadable in Light mode.** `WARN_FILL` was a *premultiplied* (baked-dark) constant → an opaque dark box on the Light parchment. Fixed to un-premultiplied amber composited source-over the theme background (pale wash on Light, subtle on Dark); `const`→`fn` because `from_rgba_unmultiplied` is not `const fn` in this egui version. **Follow-up (same day, two iterations):** QA showed the amber fill composites to a strong **dark olive** in both modes. `text-muted` grey was unreadable on it; a dark-ink (`#1a2638`) attempt then looked muddy (the §12.2 dark-on-tone rule is for the *light* pill tones, not this dark composited surface). Final: **white ink** (header solid, secondary line ~80% white) — crisp in both themes. Deliberate deviation from the wireframe's `var(--text-muted)`, recorded in SPEC §4.1. (Open, not fixed: the fill reads darker than the wireframe's faint `rgba(…,0.18)` wash — a separate change if the lighter look is wanted.)

Verified: orchestrator + BIO build clean, 187/187 tests, scoped rustfmt clean, BIO-source guard empty. SPEC §4.1 + plan P5.T9 + HANDOFF + orchestrator-handoff synced.

**Status:** Phase 5 still complete; these are Run-5 QA follow-ups. Phase 6 next.

### 2026-05-16 — Phase 6 sliced (4 runs); Run 1 (workspace spine); M2 path-sync → open-only

**Phase 6 sliced** into 4 user-approved runs: R1 workspace spine; R2 Step-4 C4 renderer + header/rename + save-draft + game-tabs; R3 Create + Load Draft; R4 fork sub-flow + dirty-bit persistence + nav-away flush. Each has a manual-test breakpoint; the orchestrator independently verifies + commits per run.

**Run 1 shipped + orchestrator-verified** (commit `150593f`): real workspace state loader — Step 2 selection + Step 3 order reconstruction via BIO's own `step3_sync::build_step3_items` (read-only reuse), **not** the PR#11 `game_install`-only hollow failure (proven by a reversed-order + an anti-bleed unit test); step router + Step-2 24-variant dispatch (catch-all so no variant drops); `NavDestination::Workspace{Some(id)}` made real; progress/nav-bar chrome; Home-`resume` route. `OrchestratorApp` gains the **6** Step-2 channel receivers + `workspace_state_dirty`/`mark_workspace_dirty` (consumer = R4); `ModlistEntry` gains additive `#[serde(default)]` `author`/`forked_from` (reuses the Phase-5 `ForkAncestor`). 199/199 tests; BIO-source guard empty (sole hit = the carve-out #3 companion-provision `+pub mod workspace;`).

**M2 amended — `sync_paths_from_settings` is open-only, not per-frame** (user decision, this revision). The original M2 mitigation specified a per-frame `Step1Settings`→`wizard_state.step1` mirror. The codebase has **no in-memory `Step1Settings` accessor**, and the orchestrator's Settings → Paths tab edits the *same in-memory* `wizard_state.step1` the workspace renders from — so Settings edits propagate to the active workspace **by construction**, making a per-frame sync (and its per-frame `settings_store.load()` disk read) unnecessary. Resolution: the sync is a one-time defensive re-assert on workspace open (inside `populate`). Written as the settled design; `workspace_state_loader.rs` + `page_router.rs` + phase-06 P6.T1/P6.T12 + HANDOFF synced. Surfaced by the Run-1 implementer as a `PLAN GAP`; the orchestrator independently verified the premise **TRUE** before relaying it (contrast Run 5's false premise — premise-checking the agent, not just relaying, is the orchestrator's job).

### 2026-05-16 — Run-1 manual test: Step-2 C4 treatment (SPEC CONFLICT resolved) + mods-folder framing fix

**Run-1 manual test (user) surfaced the issues unit tests + static review can't.** Spine (loader/dispatch/nav/routing) is sound; the failures trace to scoping, not broken logic.

**`SPEC CONFLICT` — plan said "reuse BIO Step 2 verbatim", wireframe says otherwise (resolved).** `phase-06 P6.T2` asserted "the wireframe content of Step 2 is unchanged from today's BIO" + prescribed reuse with no wrapper, deferring fidelity to Phase-8 carve-out #6 (colour-only). The wireframe (canonical UI authority, wins over plan prose) shows a structurally different Step 2: full-width `flex` search, a different toolbar button set, **no** "Restart App With Diagnostics" (it's a top-bar `Diagnostics` button), Details pane hidden-by-default (SPEC §6). Colour-only carve-out #6 cannot restructure BIO's `frame_step2`, and the CRITICAL DIRECTIVE forbids editing it ⇒ unreachable as planned. **User decision: the Step-2 C4 treatment** — a net-new orchestrator chrome wrapper (`workspace_step2::render`, P6.T2c, parallel to P6.T2b for Step 4) that reuses **only** BIO's `list_pane_step2::render_list_pane` (tree) + `details_pane_step2::render_pane` sub-renderers (verified cleanly callable — `frame_step2.rs` already decomposes into public rect-parameterized sub-renderers), no carve-out needed. Plan P6.T2/P6.T2c rewritten as if always intended; Phase-8 Step-2 scope shrunk to the tree-row/detail colour swaps + compat/prompt popups only.

**Mods-folder framing corrected to match SPEC §13.12a.** The Run-1 brief + an orchestrator off-hand answer said "Settings → Paths supplies the global mods folder Step 2 scans" — **wrong, and it contradicted §13.12a**. There is no global "mods" folder: Settings supplies game-source / Mods-*archive* (content-addressed downloads) / backup / tool paths; the **scannable mods folder is per-install, extracted at prep time (post-download, pre-Step-2) by the Phase-7 P7.T17 pipeline**. Consequence: a fresh/seeded modlist legitimately has nothing to scan pre-Phase-7. Loose "Settings supplies the scan mods folder" wording is corrected in the plan/HANDOFF/orchestrator-handoff cross-phase blocks so no future brief repeats it.

**Run-1 functional defects (the Run-1 follow-up / "Run 1b" covers them):** (1) scan hangs / Cancel stuck — `OrchestratorApp::update` never calls `app_update_cycle::poll_before_render`, so the 6 Step-2 receivers are never drained; the poll was mis-deferred to Run 4 but is required for Step 2 to function — the orchestrator's Run-1 breakpoint over-asserted "scan works" without scoping the poll (orchestrator's brief miss, owned). (2) the embedded panel min-height bleeds into the workspace nav-bar footer — fixed by the orchestrator owning the layout rects in P6.T2c. (3) the `workspace_progress_bar.rs` drop shadow misaligns segment text (net-new Run-1 code polish). **Dev-only scan affordance** (behind `-d`, never shipped) added to P6.T2c so the Step-2 chrome+tree is visually testable before Phase 7. Run 1 spine stays committed (`150593f`); the follow-up delivers the Step-2 C4 wrapper + poll + (2)/(3); Run 2 gated on it.

### 2026-05-16 — Run 1b complete (Step-2 C4 chrome) + Show-Details `✓`-prefix resolution

**Run 1b shipped + orchestrator-verified** (commit `832845d`). The Step-2 C4 chrome wrapper (`workspace_step2` + `step2_tab_row` + `step2_search` + `step2_dev_scan`) matches the canonical wireframe (`screens.jsx:2786-2880`): title, full-width flex search + Rescan, redesign GameTabs + Select-via-Log + Updates + clickable compat/prompt pills + count + Kebab. Independently verified: zero `render_controls`/`render_tabs`/`render_header`/`page_step2`/`frame_step2` call sites (grep); only BIO tree/details/popup sub-renderers + the **same `pub(crate)` `toolbar_actions_step2::*` helpers `render_controls` itself calls** are reused read-only (directive decision-order 1 — reuse, not reimplement; zero BIO edits); `poll_step2_channels` drains the 5 Step-2 poll fns before render (fixes scan-hang/Cancel-stuck — `poll_before_render` is monolithic + needs Phase-7 Step-5 args, so the narrower calls are used = the H3 Step-2 behavior with zero divergence); #4 bounded-rect layout; #9 progress-bar; dev-only scan affordance. 199 tests, both binaries clean, BIO-source guard empty. Resolves Run-1 manual-test feedback #2–#9.

**`SPEC CONFLICT` — Show-Details Kebab `✓ ` prefix (resolved).** The wireframe pinned the item verbatim as `✓ Show Details panel` (✓ = open). The shared `kebab` widget renders labels Poppins-only (no `firacode_nerd` fallback), and the cmap-verified Poppins subset tofus `✓` U+2713; `kebab.rs` is a Phase-5 shared widget out of Run-1b's editable scope. The implementer correctly escalated (functional ASCII fallback shipped, **no docs silently amended**). The orchestrator premise-checked the claim **TRUE** against the established symbol-glyph caveat + the `workspace_progress_bar.rs` precedent (not a Run-5-style false premise). **User decision: ratify the `Show Details panel` / `Hide Details panel` verb form** — the verb conveys state at least as clearly as a `✓` prefix and is glyph-safe in the Poppins subset. Resolution is doc-only (the code already shipped the verb form): wireframe `screens.jsx:2844` + SPEC §6.4 Kebab line amended as the settled intent (brief inline "intentional, not the former `✓ ` prefix" marker per the project's recorded-deviation pattern; rationale here). SPEC §6.8 / §6 "Details hidden by default" unchanged (still true).

### 2026-05-16 — Run 1b implemented: Step-2 C4 chrome wrapper + poll + #4/#9 fixes + dev affordance

**Run 1b delivered (implementer; orchestrator verifies + commits).** P6.T2c shipped as a net-new `src/ui/workspace/step2/` tree (`mod` + `workspace_step2` + `step2_search` + `step2_dev_scan`). The Step-2 C4 wrapper composes net-new redesign chrome — title `Mods / Components`, per-step hint, **full-width `flex` search** writing the same `state.step2.search_query` BIO's `content_step2::render_header` writes, a scan-status footer (mirrors `frame_step2.rs:163-166`), the Details-pane toggle defaulting **hidden** (SPEC §6) with wireframe `onSelect → setDetailsOpen(true)` auto-open — and reuses **only** BIO's sub-renderers read-only: `content_step2::render_controls` (the SPEC §6 toolbar set, **no** Restart-Diagnostics — that lives in the un-called `render_header`), `content_step2::render_tabs` (BGEE/BG2EE tabs + Select-via-Log + Updates + compat/prompt badges), `list_pane_step2::render_list_pane`, `details_pane_step2::render_pane`, `compat_window_step2`/`prompt_popup_step2`/`update_check_popup_step2`. `normalize_active_tab` is called first, exactly as `frame_step2`. **The #4 fix** is the orchestrator owning all layout rects from the *bounded* `available_rect_before_wrap()` (never BIO's forced 620-min `allocate_exact_size`), so the panel can't bleed past the nav bar `workspace_view` reserves. The router's Step-2 arm now calls `workspace_step2::render`. **Reuse-not-reimplement justification:** reusing `render_controls`/`render_tabs` is directive decision-order step 1 (public sub-renderers) and yields the exact wireframe button/tab set with zero behavior drift vs. re-deriving BIO's enable/disable + badge logic; only `render_header` is replaced (it carries the three things the SPEC-CONFLICT was about: capped search, Restart-Diagnostics, no details-default).

**Poll (defect #2/#3 of the Run-1 list — scan-hang/Cancel-stuck).** `poll_before_render` is **monolithic** (also requires the Step-5 runtime args + unconditionally polls Step 5 — unsatisfiable pre-Phase-7), so `OrchestratorApp::poll_step2_channels` calls the **same narrower `bio::app::app_step2_*` `pub(crate) fn`s `poll_before_render` calls for its Step-2 portion** (`app_update_cycle.rs:38-64`), in order, with the orchestrator's owned receivers, **before** the render, plus a 16 ms repaint while a Step-2 task is in flight (the Step-2 subset of `needs_repaint`). The historically-missed `step2_update_extract_rx` `#[allow(dead_code)]` is now removed (the poll reads it). **The progress-bar #9 fix:** kicker/label/✓ now share the segment's true vertical center via `painter.text(.., Align2::*_CENTER, ..)` (precedent `left_rail.rs`) instead of per-run `pos.y - galley.size().y/2.0` (which mismatched optical lines across the 10px/14px fonts and read as misaligned under the wireframe 3×3 shadow — shadow kept, matches `screens.jsx::WorkspaceProgressBar`). **Dev affordance:** behind `dev_mode` only, an rfd folder picker that sets `wizard_state.step1.mods_folder` then dispatches `Step2Action::StartScan` through the existing `dispatch_step2` path (no BIO touch, no scan reimplementation); absent in normal mode (pre-Phase-7 there is no per-install extracted-mods folder — SPEC §13.12a).

**Verification:** both binaries build clean (only the 2 pre-existing `PromptInfo` warnings); `cargo test --lib` 199/199 (Run-1 baseline held — additions behavior-neutral); BIO-source guard empty (zero protected paths — the 5 modified files are the authorized editable redesign set + the additive `pub mod step2;` / net-new `WorkspaceStep2State`; 4 new files all under `src/ui/workspace/step2/`). SPEC/plan/HANDOFF synced in this change set (P6.T2c poll bullet rewritten to the monolithic-→-narrower resolution; HANDOFF Phase-6 Step-2 line + this log entry). Run 2 unblocked.

### 2026-05-16 — Run 1c: Run-1b manual-test defect fixes (6) + 2 recorded deviations

**Run 1c delivered (implementer; orchestrator verifies + commits).** The Run-1b manual test surfaced 6 Step-2 chrome defects + 2 capability gaps; all addressed in the 4 authorized editable redesign files only (`workspace_step2.rs`, `step2_tab_row.rs`, `step2_search.rs`, `workspace_view.rs`), zero BIO edits, Run-1b's no-`render_controls`/`render_tabs`/`render_header`/`page_step2`/`frame_step2` boundary preserved. **(#1)** BIO's `details_pane_step2::render_pane` does not clip and let its inner `ScrollArea` grow the parent UI, pushing the workspace nav bar's `Next →` off-screen — both reused BIO sub-renderers (tree + details) now run inside a hard-clipped, fixed-size child UI (`clipped_pane`: child clip rect set to the pane rect; parent placer advanced by exactly the bounded rect, never by BIO's overgrown internal min-rect). The BIO pane may be visually clipped (its internal restyle is Phase 8) but can never break its box or push the nav bar. **(#3)** The `GameTab` was stroked on all 4 sides (a closed button box); it now strokes top/left/right only with rounded-top corners and the active tab's shell-bg fill overlapping the tree pane's top border by 1.5px (the wireframe `GameTab`/Settings-tab `marginBottom: -1.5px`, `screens.jsx:1609-1637`/`485-503`) so the tab merges into the pane. **(#5)** Step 3 was never re-synced from Step 2 on the nav transition (it ran only on workspace-open in `workspace_state_loader`); the orchestrator's Step2→Step3 forward nav edge now mirrors BIO's exact trigger (see the dedicated note below). **(#6)** Removed the duplicate per-step sub-hint in `workspace_step2.rs` (the workspace shell already renders the per-step hint under the progress bar; the wireframe `SourcesPanel` has only the title) and reclaimed its rect. **(#4 "details shows the right details")** was the user confirming the data wiring is correct — no change. **Noted-minor:** the dev-only scan button is gated on `dev_mode` only (it points BIO's scan at a chosen folder regardless of the absent production target — it must not inherit a production `can_scan` gate).

**#5 — the mirrored BIO Step2→Step3 sync call site + semantics.** BIO's `WizardApp` Next handler `bio::app::app_nav_actions::advance_after_next` (`app_nav_actions.rs:131-156`) asks `bio::app::app_nav::decide_next_action` (`app_nav.rs:85-114`); when leaving Step 2 and the Step-2 selection changed since the last sync (or Step 3 has no real items) it returns `NextAction::SyncStep3AndAdvance { signature }`, on which `advance_after_next` runs exactly `app_step3_sync_flow::sync_step3_from_step2(state); state.set_last_step2_sync_signature(signature)`. The orchestrator's `workspace_view.rs` Step2→Step3 forward edge now replicates **that arm verbatim**, calling BIO's own `pub(crate)` `decide_next_action` (the change-detection signature is BIO's own, carried in the enum payload — zero logic copied) + BIO's own `pub(crate)` `sync_step3_from_step2` (no BIO edit, no sync reimplementation). `wizard_state.current_step` is temporarily set to BIO's Step-2 index `1` only so `decide_next_action` evaluates the right branch, then restored (it is a pure `&WizardState` read — sound, no residue). **Clobber protection (Step-3 reorder preserved)** is BIO's own design, inherited by mirroring: if only Step 3 was reordered and Step-2 selection is unchanged, `decide_next_action` returns a non-sync variant (signature unchanged + Step 3 has real items) ⇒ `sync_step3_from_step2` is **not called** ⇒ Step-3 order untouched; if Step-2 selection changed, `reconcile_step3_items` (`app_step3_sync_flow.rs:32-77`) preserves the relative order of still-selected items and appends only newly-selected ones.

**#2 — production "Rescan Mods Folder" is inert pre-Phase-7 (settled intent, SPEC §13.12a).** BIO's scan worker reads `step1.mods_folder` (`bio::app::step2::scan::worker.rs:75`); the redesign has no global mods folder (Settings → Paths is game-source / Mods-archive / backup / tool paths only) — the real scan target is the per-install extracted-mods folder produced by the Phase-7 P7.T17 pipeline (§13.12a). Pre-Phase-7 there is no valid target, so the production Rescan button is **disabled with an explanatory tooltip** ("Available after install prep (Phase 7) …"), the **same accepted §13.12a Phase-7 deferral pattern as the §4.3 Downloading chassis** — a deliberate "lights up in Phase 7" stub, not a path-mapping hack. The dev-only scan affordance is the functional scan path until P7.T17 binds the live pipeline.

**#7 — "Cancel Scan" added (intentional wireframe omission, settled intent).** The wireframe (`screens.jsx:2786-2880`) has no Cancel control. While `state.step2.is_scanning`, the Rescan button is replaced **in place** by **"Cancel Scan"** emitting `Step2Action::CancelScan` (already handled by `dispatch_step2` → BIO's `app_step2_router::handle_step2_action`). A necessary capability — a scan can be long-running or pointed at the wrong target — so it is added as a deliberate, settled wireframe-omission addition (the project's recorded-deviation pattern), not a wireframe deviation to re-litigate.

**Verification:** orchestrator + BIO binaries build clean (only the 2 pre-existing `PromptInfo` warnings); `cargo test --lib` 199/199 (Run-1b baseline held — UI-only changes, behavior-neutral to lib tests); BIO-source guard empty (the 4 modified files are the authorized editable redesign set). The Run-1b "what's good" set (functional tree, details data, Kebab, dev-scan, pills/popups, updates, progress bar, Step-2 C4 chrome) is preserved — no forbidden BIO renderer call sites, no regression to those surfaces.

### 2026-05-16 — Run 1d: Run-1c re-test fixes (3) — tab geometry, rescan-reconcile (SPEC §6.3, net-new), Previous→Home

**Run 1d delivered (implementer; orchestrator verifies + commits).** The Run-1c re-test surfaced 3 items; all addressed in the authorized editable redesign tree (`step2_tab_row.rs`, `step2_search.rs`, `step2_dev_scan.rs`, `workspace_step2.rs`, `workspace_view.rs`, `workspace_nav_bar.rs`, `state_workspace.rs`, `orchestrator_app.rs`) + one net-new file (`src/ui/workspace/step2/step2_rescan_reconcile.rs`), zero BIO edits, the Run-1b/1c no-`render_controls`/`render_tabs`/`render_header`/`page_step2`/`frame_step2` boundary preserved.

**#1 — Step-2 `GameTab` corner/bottom geometry.** Run-1c hand-rolled the tab corners via `paint_corner_arc(.., 180.0/270.0, ..)` (wrong arc orientation) and stroked the top/left/right edges as separate `line_segment`s, which left a visible bottom segment and read as a boxed button. `paint_corner_arc` is **deleted**; the box now uses **egui's native per-corner rounding** — a single rounded-top `RectShape` for the fill (`rect_filled`) and one `rect_stroke` for the 1.5px border, both with `CornerRadius { nw:R, ne:R, sw:0, se:0 }`, so the two top corners are correct by construction. The wireframe `borderBottom: active ? shell-bg : border-strong` (`screens.jsx:1625`) is honoured: the box extends 1.5px past its bottom into the seam (the `marginBottom: -1.5px`), the **active** tab over-paints its bottom edge in shell-bg so — with the shell-bg fill overlap — it masks the tree pane's top border and the tab "flows into" the pane; the **idle** tab keeps the border-strong bottom, which coincides with the pane's own top border (a single line, no double rule). Reads as a tab in both states (single-game modlists too).

**#2 — Rescan reconciles, never wipes (SPEC §6.3 — user-directed, net-new orchestrator logic).** SPEC §6.3 was amended (by the orchestrator, this change set) so Rescan is **non-destructive**: it re-scans, then **re-applies the current selection onto the freshly-scanned mod list** (matched by `tp2`+component id, preserving `selected_order`), **dropping only selections whose mod/component is no longer present**, and surfaces _"N component(s) dropped — M mod(s) no longer present"_ in the scan-status footer; no confirmation dialog. Premise-checked: BIO has **no** reusable rescan-preserves-selection mechanism — BIO's `Step2ScanEvent::Finished` handler (`app_step2_scan_events.rs:73-117`) unconditionally replaces the mod vectors and resets `selected`/`next_selection_order`. So this is **net-new orchestrator logic** (not a BIO edit, not a scan reimplementation). **The async-on-completion design (the main hazard):** BIO's scan is a worker thread drained by `OrchestratorApp::poll_step2_channels`, so the reconcile cannot run synchronously at trigger time (the scan hasn't run). `step2_rescan_reconcile::snapshot_current_selection` runs at trigger time (in `step2_dev_scan::pick_folder_and_scan`, *before* `StartScan` is dispatched — the dev scan is the functional rescan path pre-Phase-7; the production Rescan stays inert per §13.12a) and stashes `(tp2.to_ascii_uppercase(), component_id, selected_order)` for every checked component on both tabs into orchestrator-owned `workspace_view.step2.rescan_snapshot`. `reconcile_on_scan_complete` runs from `OrchestratorApp::update` **immediately after** `poll_step2_channels` (the fresh mods have landed), no-op unless `is_scanning` just transitioned `true → false`, the scan finished **successfully** (`last_scan_report.is_some()` — distinguishes `Finished` from `Canceled`/`Failed`/`Disconnected`, on which BIO does **not** replace the mods so there is nothing to reconcile), and a snapshot is pending; then it re-marks `checked`+`selected_order` on the matching scanned components, recomputes per-mod tri-state + `next_selection_order` exactly as `workspace_state_loader` does on workspace open (BIO exposes no public per-mod tri-state recompute — these mirror the loader's own private `recompute_mod_checked`/`max_selected_order`, the established precedent for these fields), and computes the dropped-components / distinct-missing-mods figures for the footer warning (warn-toned, SPEC §6.3 wording verbatim).

**#3 — first-step `← Previous` → Home (SPEC §2.2 — affordance-forward, settled intent).** SPEC §2.2 / the wireframe (`screens.jsx:3372`) / plan P6.T4 were amended (by the orchestrator, this change set) so first-step `← Previous` returns to **Home** (the user reached the workspace via a Home `resume`/`open`, so first-step Previous closes that loop rather than being a dead control). `workspace_nav_bar.rs`: removed the `is_first` disable — `← Previous` is **enabled on the first step** and force-disabled **only** by `disable_prev` (the Phase-7 install-running / post-install gate; `false` until Phase 7). `workspace_view.rs` nav handler: on `prev_clicked` when `current.prev()` is `None` (the first-step case — Step 2) it sets `orchestrator.nav = NavDestination::Home` (mirrors Run-1's Home→Workspace resume route in reverse; the loaded workspace state stays intact so a later resume re-opens it). The step indicator/label text is unchanged.

**Verification:** orchestrator + BIO binaries build clean (only the 2 pre-existing `PromptInfo` warnings); `cargo test --lib` 203/203 (Run-1c baseline 199 held + 4 new `step2_rescan_reconcile` unit tests — capture, re-apply/drop accounting, tri-state recompute, max-order); BIO-source guard empty (every `.rs` change under `src/ui/{orchestrator,workspace}/`, the net-new redesign trees + the one new `step2_rescan_reconcile.rs`). The Run-1b/1c "what's good" set (functional tree, details data, Kebab, dev-scan, pills/popups, updates, progress bar, the #1–#9/✓ fixes, Step-2 C4 chrome) is preserved — no forbidden BIO renderer call sites, reused BIO sub-renderer call sites unchanged.

### 2026-05-16 — Run 1e: Select-via-WeiDU-Log SPEC §6.10 danger-confirm gate + non-destructive cancel

**Run 1e delivered (implementer; orchestrator verifies + commits).** Two related Step-2 "Select \<Tab\> via WeiDU Log" fixes — **implementing what SPEC §6.10 + the wireframe `askWeiduImport` already mandate** (a Run-1d gap, not a spec change) plus a data-loss bug. No SPEC/wireframe amendment. All changes in the authorized redesign tree (`state_workspace.rs`, `step2/mod.rs`, `step2/step2_tab_row.rs`, `step2/workspace_step2.rs`, `step2_log_glue.rs`) + one net-new file (`src/ui/workspace/step2/step2_log_confirm.rs`); zero BIO edits; the Run-1b→1d no-`render_controls`/`render_tabs`/`render_header`/`page_step2`/`frame_step2` boundary preserved.

**#1 — Missing destructive `ConfirmDialog` (SPEC §6.10 + wireframe, settled intent).** SPEC §6.10 requires Step 2's only `ConfirmDialog` to gate Select-via-WeiDU-Log (it replaces *every* selection on the tab — destructive); the wireframe `askWeiduImport` (`screens.jsx:2778-2784`) is the flow: button → danger confirm → on confirm → file picker → apply; cancel → abort. Run 1d shipped the tab-row button wired straight to the picker with **no confirm**. Closed by reusing the **existing shared `ConfirmDialog`** widget (`widgets/dialogs/confirm_dialog.rs` — the same one Home Delete/Reinstall use; no new dialog widget) gated on an orchestrator-owned `WorkspaceStep2State::pending_weidu_log_confirm: Option<bool>` (`Some(true)` = BGEE, `Some(false)` = BG2EE). `step2_tab_row` arms it instead of emitting the action; `workspace_step2::render` renders the danger confirm alongside the other Step-2 popups (section 6b); on **Confirm** it emits `Step2Action::Select{Bgee,Bg2ee}ViaLog` (the *unchanged* `step_action_dispatch::dispatch_step2` → `step2_log_glue` picker+apply path runs after confirm) and clears the gate; on **Cancel/dismiss** it just clears the gate — nothing changes. Copy is **verbatim** from the wireframe (title `Replace <TAB> selections from a WeiDU log?`, the full overwrite-warning body, primary label `Pick a weidu.log...`, danger-styled, plus Cancel); the strings + descriptor live in the net-new `step2_log_confirm` (the Home `confirm_delete` precedent — owns only the wireframe-verbatim text).

**#2 — Picker-cancel applied the default log (DATA LOSS).** `step2_log_glue.rs` previously did `.or_else(|| resolve_{bgee,bg2}_weidu_log_path(..))` on a cancelled picker and then `apply_weidu_log_selection_from_path(.., default)` — destructively wiping the user's current Step-2 selection on cancel. The `.or_else(... resolve default ...)` fallback is **removed**: if the picker returns `None` the sibling returns early — applies nothing, mutates nothing, preserves the selection. `apply_weidu_log_selection_from_path` is now only ever called with a user-picked path. The now-wrong "Mirror BIO" comment is replaced with the redesign behavior (a confirmed destructive opt-in; picker-cancel ⇒ abort, edits preserved — the SPEC §6.10 / wireframe model). Net effect: **either cancel point (the dialog *or* the file picker) is a no-op that preserves the current selection.**

**Verification:** orchestrator library + binary compile clean (only the 2 pre-existing `PromptInfo` warnings; the only error is the Windows running-exe linker lock `os error 5` — environmental, reported, not worked around); BIO binary builds clean; `cargo test --lib` 206/206 (Run-1d baseline 203 held + 3 new `step2_log_confirm` tests — target-tab title, wireframe-verbatim body, danger descriptor/label). BIO-source guard empty (every change under `src/ui/workspace/`, net-new redesign trees + the one new `step2_log_confirm.rs`; `apply_weidu_log_selection_from_path` / `resolve_*_weidu_log_path` stay read-only `pub(crate)` calls). The no-forbidden-BIO-render-call boundary holds (the only `render_controls`/`render_tabs`/`render_header`/`page_step2`/`frame_step2` matches in `src/ui/workspace` are comments, not call sites). The Run-1b→1d "what's good" set (tree/details/Kebab/dev-scan/pills/popups/updates/progress-bar/rescan-reconcile/Previous→Home/tab geometry) is preserved.

### 2026-05-16 — Run 2: Step-4 C4 renderer + workspace header/rename + save-draft + game tabs

**Phase 6 Run 2 shipped + orchestrator-verified** (commit `6df87f9`; implements the already-specced P6.T2b/T5/T6/T10 — no spec change). **P6.T2b** — net-new `src/ui/workspace/step4/*` + `widgets/weidu_line.rs`: Save row (`Save weidu.log's`/`Save weidu.log` + "_N_ components ready to install on _<tab>_ · across _M_ mods"), EET game-tab strip (net-new redesign `GameTab`, single-game skips it), three-colour line-numbered review list (`<tp2>`→accent-deep, `<id>`→text-muted, `<label>`→text-primary, line-no→text-faint per SPEC §6.7, BIO's `diagnostics::format_step4_item` for the text) or exact-log read-only viewer + `Check Mod List`. Save → `Step4Action::SaveWeiduLog` via the router's existing `dispatch_step4` → BIO's `pub(crate)` `handle_step4_action`→`auto_save_step4_weidu_logs` (reused **read-only**, visibilities verified). **C4 boundary: BIO's `page_step4::render` is provably NOT called** (grep-verified — same boundary as Step-2; the legacy `BIO` binary still renders its own `page_step4`). **P6.T5** — `workspace_header.rs` (replaces Run-1's minimal placeholder) + new `operations_rename.rs`: `Editing <name>` + vector-painted `✎` (U+270E cmap-absent — symbol-glyph rule) inline rename (Enter/Esc) → `rename_modlist` which is **registry-entry-only, zero filesystem ops** (SPEC §2.2 — `destination_folder`/`workspace_file_relpath`/install folder never touched; explicit `rename_never_touches_destination_or_workspace_path` test; debounced via the registry persistence cycle, not `workspace_state_dirty`); fork badge + vector `⑂ view fork details` → the **reused Phase-5 `ForkInfoPopup`** (no new widget); `fork_meta` populated from the registry entry in `page_router::render_workspace` via the additive `fork_meta_from_entry` (composes with prior/user page_router state). **P6.T6** — save-draft is the **first caller** of Run-1's `extract_workspace_state_from_wizard` → immediate synchronous `WorkspaceStore::save` + persistence-baseline sync (Run-4's debounced cycle unaffected) + `✓ saved!` flash. **P6.T10** — EET dual / single-game (incl. IWDEE→BGEE bucket) tab branching off the loader-set `game_install`. Orchestrator-independent verification: BIO-source guard empty (the agent's unintended `store.rs` rustfmt-via-mod-tree leak caught + reverted, confirmed not staged); zero forbidden BIO-render call sites incl. `page_step4::render`; `operations_rename` SPEC §2.2 registry-only premise read + verified; `cargo test --lib` 226/0 (206 +20 substantive); Step-2/1b-1e 44/44 not regressed; both binaries link clean. 5 judgment calls (all sound/flagged, none an overreach): `✎` vector per cmap; additive `fork_info_open` mirroring `share_paste_open`; `fork_meta` in `page_router` (the Run-1 workspace-open block — right integration point); net-new `GameTab` consistent with `step2_tab_row` (avoids the Phase-4 `tab_strip` double-box; the wireframe Step-4 tab IS the same `GameTab`); save-error as an inline pill from BIO's `step5.last_status_text` (BIO's save-error modal is `WizardApp`-internal/unreachable — the C4 rationale).
