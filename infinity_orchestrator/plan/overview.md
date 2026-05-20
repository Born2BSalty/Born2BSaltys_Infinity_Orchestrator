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

| # | Title | Summary | Status / Doc |
|---|-------|---------|--------------|
| 1 | Library/binary split + theme tokens + fonts + shell modules + new binary entry | Apply the carve-out #3 structural split (`src/lib.rs`, slim `src/main.rs`, `src/bin/infinity_orchestrator.rs`). Add Poppins, dark-default theme tokens, sketchy border/shadow primitives, the shell chrome modules (titlebar + statusbar, not yet invoked), and a placeholder app stub in the new binary. | **SHIPPED** — [stub](phase-01-theme-and-shell.md) · [audit](archive/phase-01-theme-and-shell.md) |
| 2 | `OrchestratorApp` + left-rail navigation + destination routing | Add the `OrchestratorApp` struct (its own `eframe::App` impl, owns its own `WizardState`), wire the shell chrome around it, add the persistent left rail with 4 destinations + an unreachable `Workspace` destination (stub renderer only — no `WizardApp` hosting). Replace the binary's placeholder stub with the real `OrchestratorApp::run`. | **SHIPPED** — [stub](phase-02-nav-routing.md) · [audit](archive/phase-02-nav-routing.md) |
| 3 | Modlist registry + workspace state files | Add `modlists.json` model + store + corrupt-file terminal error state, and the per-modlist `modlists/<id>/workspace.json` model. Wire into the orchestrator's persistence cycle. Nothing renders these yet. | **SHIPPED** — [stub](phase-03-modlist-registry.md) · [audit](archive/phase-03-modlist-registry.md) |
| 4 | Settings screen (5 sub-tabs) + per-edit debounced path validation | Build the new Settings screen with file-folder tabs (General, Paths, Tools, Accounts, Advanced). Reads/writes `bio_settings.json` via the existing `bio::settings::SettingsStore` (instantiated independently by the orchestrator). GitHub OAuth popup invoked as a shared popup using BIO's existing public popup renderer. Validate-now button + per-edit debounced path validation events. | **SHIPPED** — [stub](phase-04-settings.md) · [audit](archive/phase-04-settings.md) |
| 5 | Home + Install Modlist (paste/preview/download stages) | Build Home (filter chips, modlist cards, Add-a-modlist Box, game-installs-detected block, first-launch empty state, delete confirm dialog). Build Install Modlist: paste + preview fully; downloading as a **chassis only** (live download/extract + per-install dirs + content-addressed staging = Phase 7 P7.T17 / SPEC §13.12a). Reuses BIO share-code parser. Install runtime stub at stage 4. | **SHIPPED** — [stub](phase-05-home-install-paste.md) · [audit](archive/phase-05-home-install-paste.md) |
| 6 | Create screen + Workspace shell (Steps 2-4) | Build Create's choose mode + setup Box + starting-point cards + Load Draft dialog. Build the `WorkspaceView` shell (title row, rename, fork badge, save-draft, progress bar, nav bar). Workspace **calls BIO's existing per-step `pub fn render`** for Steps 2-3 directly. **Step 4's body is replaced with an orchestrator-side renderer (C4)** that reuses BIO's public save action and Step 3 order data. The workspace state loader populates `WizardState` on open and extracts back on close / nav-away / debounce. | **SHIPPED** — [stub](phase-06-create-workspace-shell.md) · [audit](archive/phase-06-create-workspace-shell.md) |
| 7 | Step 5 install runtime wrapper + Reinstall + import-code auto-write + concurrency gate + rail-nav lock | Wrap the existing BIO Step 5 install runtime in the new pre-install / during-install / post-install chrome. Add success banner, Share import code dialog (post-install), Return to Home / Open install folder buttons, registry state transition on success, install concurrency policy gate, **rail-navigation lock during an in-flight install (C5)**. Wire Install Modlist stage 4 + Reinstall flow + modlist-import-code.txt auto-write. **P7.T17: per-install directory derivation (incl. the install-critical #2 `-u` / #3 `-p`/`-n` / #4 `-g` dirs) + content-addressed archive staging + import→auto-build pipeline drive that binds the Phase-5 §4.3 chassis live (SPEC §13.12a).** | **SHIPPED** — [stub](phase-07-install-runtime.md) · [audit](archive/phase-07-install-runtime.md) |
| 8 | Popup theme reskins, automatic flag policies, polish | Apply theme-token extraction refactors to the BIO Compat popup, Prompt popup, Update Check popup, GitHub OAuth popup (per CRITICAL DIRECTIVE's mild-refactor carve-out #1). Apply window-chrome flips (carve-out #2). Apply the **residual** §13.12 flag policies only: #6 (`DestChoice` → prepare/backup mapping) + #7 (hardcoded `-autolog`/`-logapp`/`-log-extern`) + Settings-surface removal. **#1/#5 are Phase 7 P7.T16; #2/#3/#4 install-critical per-install dirs are Phase 7 P7.T17 (an install can't run without them — SPEC §13.12a).** Wire toast notifications, hover affordances, copy-to-clipboard polish. Final smoke pass. | **LIVE** — [phase-08-popup-reskins-polish.md](phase-08-popup-reskins-polish.md) |

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


## Revision log → see [`revision-log.md`](revision-log.md).
