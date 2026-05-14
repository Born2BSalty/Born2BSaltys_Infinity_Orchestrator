# Phase 2 — `OrchestratorApp` + left-rail navigation + destination routing

## Summary

Introduce `OrchestratorApp`, the standalone `eframe::App` impl that powers the new binary. `OrchestratorApp` lives **inside the library crate** at `bio::ui::orchestrator::orchestrator_app::OrchestratorApp` (per the Phase 1 carve-out #3 split). It owns its own `WizardState` instance, its own `SettingsStore`, its own background-thread receivers (scan / OAuth / update / install), and its own destination router (Home / Install / Create / Settings / Workspace). It is wholly independent of `WizardApp` — the two coexist as separate `eframe::App` impls reachable from separate binaries.

Phase 2 wires the shell chrome (titlebar + statusbar from Phase 1) around `OrchestratorApp`, renders the persistent left rail with brand mark + 4 nav items + status dot, and dispatches to per-destination page renderers. The four primary destinations are stub pages in this phase; the `Workspace` destination is also a stub (renders "Workspace — Phase 6"). **No hosting of `WizardApp` anywhere.**

## What ships after this phase

- `cargo build --bin infinity_orchestrator --release` succeeds; the binary launches.
- The app opens with the new outer shell: titlebar, left rail (200px wide, labels mode), main pane, statusbar.
- Left rail shows the brand mark (`∞` glyph in a teal-accent rounded square), wordmark "INFINITY / ORCHESTRATOR", four nav items (Home / Install / Create / Settings), and a bottom status dot reading `weidu v<detected> · all paths ok` (green) or `× paths missing` (red).
- Clicking each of the four nav items navigates between four stub pages. Each stub shows a `ScreenTitle` ("Home", "Install Modlist", "Create / edit modlist", "Settings") plus a faint line "Coming in Phase N — see SPEC §<ref>". Settings stub mentions Phase 4; Home stub Phase 5; Install Phase 5; Create Phase 6.
- A fifth destination `Workspace` exists but is only reachable from a dev-mode button on the Home stub (`Open workspace stub (dev)`) — it renders "Workspace — Phase 6" as a placeholder. **It does not host `WizardApp`.** In Phase 6 the orchestrator's Workspace destination calls BIO's per-step page renderers directly with the orchestrator's own `WizardState`.
- Navigating between destinations does not crash. Returning to a destination restores its scroll position (within reason — full state restoration is a Phase 6 concern).
- `cargo build --bin BIO --release` continues to compile and launch the existing wizard unchanged (the Phase 1 carve-out #3 shim already ensures this).

## What's still missing

- Real Home content (Phase 5).
- Real Install / Create / Settings content (Phases 4-5).
- The Workspace shell with progress bar and Steps 2-5 (Phase 6); the Workspace destination is just a stub label in Phase 2.
- Modlist count + jobs running in the statusbar (Phase 3 / Phase 7).
- Light/dark theme switcher (Phase 4 wires the toggle in Settings).

## Dependencies

- Phase 1 (carve-out #3 split, theme tokens, fonts, shell modules, new binary entry).

## File inventory

### New files

| Path | Purpose | Depends on |
|------|---------|-----------|
| `src/ui/orchestrator/mod.rs` | `pub mod orchestrator_app; pub mod nav_destination; pub mod nav_rail; pub mod page_router; pub mod stubs; pub mod widgets; pub mod nav_status;`. Registered in `src/ui/mod.rs` via `pub mod orchestrator;` so the path is `bio::ui::orchestrator::*`. | — |
| `src/ui/orchestrator/orchestrator_app.rs` | The new `pub struct OrchestratorApp` that implements `eframe::App`. Owns: `nav: NavDestination`, `wizard_state: bio::app::state::WizardState`, `settings_store: bio::settings::SettingsStore`, `dev_mode: bool`, `exe_fingerprint: String`, plus the background-thread receivers needed by the orchestrator's flows (Phases 4-7 extend these as new flows come online). | Phase 1 shell + theme + fonts |
| `src/ui/orchestrator/nav_destination.rs` | `pub enum NavDestination { Home, Install, Create, Settings, Workspace { modlist_id: Option<String> } }` (Phase 2 leaves `modlist_id` `None`; Phase 6 wires the real id). `impl NavDestination { pub fn label(), pub fn icon(), pub fn rail_items() }`. Implements `Clone + Eq + Hash`. | — |
| `src/ui/orchestrator/nav_rail.rs` | `pub fn render(ui, current: &mut NavDestination, dev_mode: bool, validation_summary: &PathValidationSummary, rail_locked: Option<&RailLockReason>)` — renders the 200px-wide left rail per SPEC §2.1: brand mark + wordmark, 4 nav items as `egui::Button` widgets, bottom status dot + summary line. The `rail_locked` arg is `None` in Phase 2 (Phase 7's C5 rail-nav lock wires it). | redesign theme tokens |
| `src/ui/orchestrator/page_router.rs` | `pub fn render(orchestrator: &mut OrchestratorApp, ctx: &egui::Context, ui: &mut egui::Ui)` — `match` on `orchestrator.nav`; dispatches each destination to its renderer (or stub). The `Workspace` arm in Phase 2 renders the placeholder; Phase 6 replaces it with the real `workspace_view::render` that calls BIO's per-step page renderers (and the orchestrator-side Step 4 wrapper per C4). | stubs, theme tokens |
| `src/ui/orchestrator/stubs.rs` | Five stub renderers `render_home_stub`, `render_install_stub`, `render_create_stub`, `render_settings_stub`, `render_workspace_stub`. Each renders a `ScreenTitle` + one faint line referencing the phase that builds it out. The Home stub additionally renders a temporary `Open workspace stub (dev)` button (gated by `dev_mode`) that flips `nav` to `Workspace { modlist_id: None }`. | redesign theme tokens |
| `src/ui/orchestrator/widgets/mod.rs` | Aggregator for shared redesign widgets. Re-exports `screen_title`, `btn`, `r_box`, `label` (current Phase 2 contents). Later phases add `pill`, `kebab`, `chip`, `tab_strip`, `confirm_dialog`, `toast`, `clipboard`. | — |
| `src/ui/orchestrator/widgets/screen_title.rs` | `pub fn render(ui, title: &str, sub: Option<&str>)` — the redesign's `ScreenTitle` primitive: 22px Poppins 500 title, 13px text-muted subtitle, 20px bottom margin. Mirrors `screens.jsx::ScreenTitle` (line 191-218). | redesign theme tokens |
| `src/ui/orchestrator/widgets/btn.rs` | `pub fn redesign_btn(ui, label, opts: BtnOpts) -> egui::Response` — sketchy border, primary/secondary fill, optional small variant, optional disabled. Mirrors `screens.jsx::Btn` (line 22-43). | redesign theme tokens |
| `src/ui/orchestrator/widgets/r_box.rs` | `pub fn redesign_box(ui, label: Option<&str>, body: impl FnOnce(&mut Ui))` — sketchy chassis with optional corner label, 1.5px border, `shell_bg`, padding. Mirrors `screens.jsx::Box` (line 11-20). | redesign theme tokens |
| `src/ui/orchestrator/widgets/label.rs` | `pub fn redesign_label(ui, text)` and `pub fn redesign_label_hand(ui, text)` — mirrors `screens.jsx::Label` (line 177-189) two variants: regular Poppins 13px and "hand style" Poppins 14px in `accent-deep`. | redesign theme tokens |
| `src/ui/orchestrator/nav_status.rs` | `pub fn compute_path_validation_summary(state: &WizardState, settings: &Step1Settings) -> PathValidationSummary` — read-only formatter that returns `(StatusKind, String)` for the rail's bottom segment, e.g., `(Ok, "weidu v249 · all paths ok")` or `(Err(n), "× <n> path issues")`. Delegates to BIO's existing `bio::app::state::state_validation*` functions. | BIO `state_validation_*` (read-only) |

### BIO files read from / consumed (no modifications)

- `bio::app::state::WizardState` — `OrchestratorApp` constructs one of these directly (the struct is `pub`; all its fields are `pub` per `src/core/app/state/state_wizard.rs:7`). No modification.
- `bio::app::state::Step1State` — `WizardState::step1` is populated by calling `bio::app::app_bootstrap_init::initialize(dev_mode)` directly (per H5). `initialize` is `pub(crate) fn` per `src/core/app/app_bootstrap_init.rs:17`. Reachable from same-crate orchestrator code per the Phase 1 split.
- `bio::settings::SettingsStore` — Returned via `AppBootstrap.settings_store` from `initialize()`. No modification.
- `bio::app::state::state_validation*` — read by `nav_status::compute_path_validation_summary`. No modification.
- `bio::ui::frame::frame_window::native_options` — read as a reference for window sizing / icon when constructing the orchestrator's `eframe::NativeOptions`. May be called directly if its signature permits (per `pub`/`pub(crate)` visibility check), or replicated inline if it depends on internal state.

### BIO files needing allowed mild refactor

**None.** Phase 2 introduces zero new edits to BIO source. The carve-out #3 split from Phase 1 already exposes everything the orchestrator needs as `bio::*` from same-crate orchestrator code. `pub(crate)` functions in BIO (e.g., `app_bootstrap_init::initialize`) are reachable because the orchestrator lives inside the library crate at `bio::ui::orchestrator::*`.

The previous plan's `main.rs` dispatch-flip is removed; no CRITICAL DIRECTIVE carve-out is invoked in this phase (Phase 1 already handled carve-out #3).

## Implementation tasks

### P2.T1 — Define `NavDestination`

- **What:** Create `src/ui/orchestrator/nav_destination.rs` with the enum:
  ```rust
  pub enum NavDestination {
      Home,
      Install,
      Create,
      Settings,
      Workspace { modlist_id: Option<String> },
  }
  ```
  Variant order matches SPEC §2.1: `Home (1), Install (2), Create (3), Settings (4)`. `Workspace` is appended as a non-rail destination. `impl NavDestination { pub fn rail_items() -> [NavDestination; 4] }` returns an owned array of the four rail items; `Workspace` is excluded.

  **Why an owned array, not a `&'static` slice (per L6).** Const-construction of a `&'static [NavDestination]` would fail because `NavDestination::Workspace { modlist_id: Option<String> }` contains a heap-allocated `String` — the enum is not `'static`-constructible at const-eval time. The owned array works because the 4 rail items are all unit variants (`Home`, `Install`, `Create`, `Settings`); they're constructed at call time. The array is cheap to construct (4 unit variants, total size ≈ enum discriminant × 4) and is returned by value.

  Explore is intentionally omitted from `NavDestination` for the v1 alpha (SPEC Appendix C lists it as a future v2 track). The enum can be extended later by adding a variant; no other code paths assume a fixed variant set.
- **Where:** Create new file.
- **Acceptance:** `NavDestination::rail_items().len() == 4`. Each rail variant has a `label()` returning the wireframe-verbatim string (`"Home"`, `"Install"`, `"Create"`, `"Settings"`) and an `icon()` returning the glyph (`"⌂"`, `"↓"`, `"✎"`, `"⚙"`).
- **SPEC:** §2.1.

### P2.T2 — Implement `nav_rail::render`

- **What:** Build the left-rail egui pane: 200px width, `redesign_rail_bg` background, 1.5px solid right border in `redesign_border_strong`. Top section: 36×36px brand mark square in `redesign_accent` with a 1.5px `redesign_border_strong` border and 2px×2px drop shadow, containing the `∞` glyph centered; right of it the wordmark stack (`Poppins 10px / 500 / uppercase letter-spacing 1.2px` "INFINITY" over `Poppins 9px / text-faint` "ORCHESTRATOR"). Below: a 1.5px dashed bottom border. Then the 4 nav items as buttons. Bottom: dashed top border + 8×8px status dot + the validation summary.
- **Where:** Create `src/ui/orchestrator/nav_rail.rs`.
- **Acceptance:** Side-by-side with the wireframe `app.jsx::sk-nav` rendering, the rail matches: same proportions, same fonts, same hover behavior (4-5% hover overlay on inactive items, `redesign_accent` fill + 2px shadow on active item). Clicking a nav item updates `*current` to the matching destination. The `rail_locked: Option<&RailLockReason>` arg is unused in Phase 2 (passed as `None`); Phase 7 wires it.
- **SPEC:** §2.1, §13.15 (rail-nav lock — wired in Phase 7).

### P2.T3 — Implement `OrchestratorApp`

- **What:** Create `OrchestratorApp` struct with fields:
  ```rust
  pub struct OrchestratorApp {
      pub nav: NavDestination,
      pub wizard_state: bio::app::state::WizardState,
      pub settings_store: bio::settings::SettingsStore,
      pub dev_mode: bool,
      pub exe_fingerprint: String,
      pub path_validation: PathValidationSummary,
      pub theme_palette: bio::ui::shared::redesign_tokens::ThemePalette, // per H3: theme state lives on the app, not in a global atomic. Read once per frame, passed into render code explicitly.
      // Phase 3 adds: pub registry, pub registry_error, pub persistence_cycle.
      // Phase 4 adds: pub settings_screen_state, pub tool_version_cache.
      // Phase 5 adds: pub home_state, pub install_state.
      // Phase 6 adds: pub create_state, pub workspace_state, pub workspace_state_loader.
      // Phase 7 adds: pub install_runtime_state.
  }
  ```
  **`OrchestratorApp::new(dev_mode)` per H5: call `bio::app::app_bootstrap_init::initialize(dev_mode)` directly.** That function is `pub(crate) fn` per `src/core/app/app_bootstrap_init.rs:17` and is reachable from same-crate orchestrator code (the Phase 1 carve-out #3 puts the orchestrator inside the library crate). It returns an `AppBootstrap { settings_store, exe_fingerprint, step1, github_auth_login, startup_status }` struct. The orchestrator:
  1. Calls `let bootstrap = bio::app::app_bootstrap_init::initialize(dev_mode);`.
  2. Builds a fresh `WizardState::default()`.
  3. Sets `wizard_state.step1 = bootstrap.step1`.
  4. Stores `bootstrap.settings_store` and `bootstrap.exe_fingerprint` on `OrchestratorApp`.
  5. Initializes `path_validation` by running BIO's existing public path-validation entry — `bio::app::state::state_validation::validate_all_paths` (or whichever public entry BIO exposes; verify visibility).

  No "copy the relevant logic" fallback is needed (H5 mitigation). `initialize` is reachable directly.

  Implement `eframe::App::update` by calling `bio::ui::shell::shell_chrome::render_shell(ctx, modlist_count, jobs_running, |ui| { ... })` (Phase 1 module). Inside the body closure: render the left rail via `egui::SidePanel::left("orchestrator_nav").exact_width(REDESIGN_NAV_WIDTH_PX)`, then the main content via `egui::CentralPanel::default()` dispatching to `page_router::render`. `modlist_count` is `0` in Phase 2 (Phase 3 wires the real value).
- **Where:** Create `src/ui/orchestrator/orchestrator_app.rs`.
- **Acceptance:** App opens. Titlebar + left rail + main panel + statusbar all render. Clicking a nav item switches the visible main-panel content. `WizardState` is owned in-process; the legacy `BIO` binary's state is unaffected. `OrchestratorApp::new` calls `bio::app::app_bootstrap_init::initialize` directly (no inline copy).
- **SPEC:** §2.1, overview "Architecture" section.

### P2.T4 — Implement `page_router::render`

- **What:** A `match orchestrator.nav` dispatch:
  - `NavDestination::Home` → `stubs::render_home_stub(ui, orchestrator)`
  - `NavDestination::Install` → `stubs::render_install_stub(ui)`
  - `NavDestination::Create` → `stubs::render_create_stub(ui)`
  - `NavDestination::Settings` → `stubs::render_settings_stub(ui)`
  - `NavDestination::Workspace { .. }` → `stubs::render_workspace_stub(ui)`

  **Per H3:** the previous plan's references to "`crate::ui::app_update_loop::run`" are incorrect. The real path is `bio::ui::app::update_loop::run`, which is a **private module** inside `src/ui/app.rs` (declared at `src/ui/app.rs:31-32` via `#[path = "app_update_loop.rs"] mod update_loop;`). The orchestrator does **not** call this — it's `WizardApp`'s frame entry, not a public API. Plan citations of "`app_update_loop::run`" as a dispatch surface are corrected to `bio::ui::app::update_loop::run` and are read-only references for understanding which `bio::app::*` calls BIO makes per `Step{N}Action`. The orchestrator dispatches by calling those `bio::app::*` functions directly.

  In Phase 6 the `Workspace` arm calls `bio::ui::workspace::workspace_view::render(ui, orchestrator, ...)`, which internally calls BIO's `bio::ui::step{2,3}::page_step{N}::render(ui, &mut orchestrator.wizard_state, dev_mode, &exe_fingerprint)` (per C4, Step 4 is rendered by an orchestrator-side wrapper, not BIO's `page_step4::render`).
- **Where:** Create `src/ui/orchestrator/page_router.rs`.
- **Acceptance:** Each destination renders something. The `Workspace` destination renders the placeholder stub (not the legacy wizard).
- **SPEC:** §2.1.

### P2.T5 — Implement the five stub pages

- **What:** In `stubs.rs`, each stub renders a `ScreenTitle` + a faint sub-line referencing the planned phase:
  - `render_home_stub(ui, orchestrator)`: title "Welcome back, adventurer", sub "Coming in Phase 5 — SPEC §3", plus a dev-mode-only `Open workspace stub (dev)` button below that flips `orchestrator.nav` to `NavDestination::Workspace { modlist_id: None }`. Phase 3 adds a second dev button (`Seed test modlist (dev)`). This button is a Phase 2 scaffolding affordance only. In Phase 5 (Home gains real content), the button is removed — not migrated to a Diagnostics panel.
  - `render_install_stub(ui)`: title "Install shared modlist", sub "Coming in Phase 5 — SPEC §4".
  - `render_create_stub(ui)`: title "Create / edit modlist", sub "Coming in Phase 6 — SPEC §5".
  - `render_settings_stub(ui)`: title "Settings", sub "Coming in Phase 4 — SPEC §11".
  - `render_workspace_stub(ui)`: title "Workspace", sub "Coming in Phase 6 — SPEC §2.2".
- **Where:** Create `src/ui/orchestrator/stubs.rs`.
- **Acceptance:** Each stub renders without crashing. Home stub's dev button is gated by `dev_mode`.
- **SPEC:** §2.1.

### P2.T6 — Implement the four primitive widgets (`screen_title`, `btn`, `r_box`, `label`)

- **What:** Each widget mirrors its wireframe counterpart. The `r_box` corner-label uses a small Poppins 10px label rendered at the top-left of the box's border (consult `screens.jsx::Box` and the `.sk-corner-label` CSS rule in the wireframe). `redesign_btn` supports `primary: bool` (fills with `redesign_accent`, adds 2px×2px drop shadow), `small: bool` (smaller padding + font), `disabled: bool` (50% opacity, no click response). `redesign_label` is plain Poppins 13px; `redesign_label_hand` is Poppins 14px in `accent-deep`.
- **Where:** Create files under `src/ui/orchestrator/widgets/`.
- **Acceptance:** Calling each function in a debug context renders a widget visually consistent with the wireframe at the same DPI.
- **SPEC:** §1.2 (sketchy aesthetic), §12.3 (border + shadow rules).

### P2.T7 — Replace the Phase 1 placeholder with `OrchestratorApp` in the binary entry

- **What:** In `src/bin/infinity_orchestrator.rs` (the file created in Phase 1), replace the local `PlaceholderApp` with a construction + run of `OrchestratorApp`:
  ```rust
  eframe::run_native(
      "Infinity Orchestrator",
      native_options,
      Box::new(move |cc| {
          bio::ui::shared::redesign_fonts::install_redesign_fonts(&cc.egui_ctx);
          bio::ui::shared::typography_global::configure_typography(&cc.egui_ctx);
          Box::new(bio::ui::orchestrator::OrchestratorApp::new(dev_mode))
      }),
  )?
  ```
  The `OrchestratorApp::new(dev_mode)` constructor lives in Phase 2's `orchestrator_app.rs`. Fonts are installed before typography (H7 ordering).
- **Where:** Edit `src/bin/infinity_orchestrator.rs` (a Phase 1 new file — editable in subsequent phases).
- **Acceptance:** `cargo run --bin infinity_orchestrator` launches the orchestrator shell. Titlebar + rail + body + statusbar all visible.
- **SPEC:** §2.1, overview "Architecture" section.

### P2.T8 — Status line on left rail

- **What:** `nav_status::compute_path_validation_summary(&wizard_state, &settings)` returns the rail's status. Wire `OrchestratorApp::update` to compute this each frame (cheap — just reads cached validation results) and pass into `nav_rail::render`. **The validation logic itself stays in BIO's existing `state_validation*.rs` files unmodified;** this task adds a read-only formatting helper.
- **Where:** Create `src/ui/orchestrator/nav_status.rs`; wire from `orchestrator_app.rs`.
- **Acceptance:** Launching with all paths configured shows the green dot + "weidu v… · all paths ok". Launching with no paths shows red dot + "× N path issues".
- **SPEC:** §2.1 (status dot at the bottom of the rail).

## Open questions / risks

- **Background-thread receivers.** `WizardApp` owns receivers for scan / OAuth / update / install events. `OrchestratorApp` will own its own equivalents. Phase 2 doesn't wire any of these (no flows run yet); Phases 4-7 add them one feature at a time. Each addition uses BIO's existing `bio::app::*` channel-creation functions (all `pub` or same-crate-reachable `pub(crate)`); no BIO source is modified.
- **The `Workspace` destination's behavior in Phase 2.** It's a stub that renders "Workspace — Phase 6". Phase 6 replaces the stub with the real workspace view that calls BIO's per-step page renderers (and the orchestrator-side Step 4 wrapper per C4). Phase 2 never hosts `WizardApp`.
- **Native options parity.** `bio::ui::frame::frame_window::native_options()` is BIO's public window-options builder. If the orchestrator wants the same window size / icon, it can call this function directly (no modification). If the function pulls state from `WizardApp`, replicate the relevant constants inline in the new binary's `main`.
- **`bootstrap.startup_status` warnings.** `app_bootstrap_init::initialize` returns a `startup_status: Option<String>` for warnings (compat-rules init failures, settings load failures, etc.). The orchestrator surfaces these in Phase 4's Settings → General sub-tab as an info banner (or via the registry-error path in Phase 3 if the failure is critical). Phase 2 ignores the field (acceptable: warnings are non-fatal, and the orchestrator's empty Settings stub doesn't yet have a place to display them).

## Verification

1. `cargo build --bin infinity_orchestrator --release` succeeds.
2. `cargo build --bin BIO --release` continues to succeed.
3. Launch the orchestrator: titlebar + left rail + main panel + statusbar render.
4. Click each of the four nav items: each stub page renders with its `ScreenTitle`.
5. From the Home stub's `Open workspace stub (dev)` button (dev mode only), navigate to Workspace: the workspace stub renders. Click Home again: Home stub re-renders.
6. Launch `BIO`: the existing wizard renders unchanged.
7. Confirm visual diff against wireframe (left rail width, brand mark position, nav-item active state).
8. Verify `OrchestratorApp::new` calls `bio::app::app_bootstrap_init::initialize(dev_mode)` directly (no inline duplicated logic — H5 mitigation).
