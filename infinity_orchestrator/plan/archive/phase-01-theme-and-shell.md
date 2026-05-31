# Phase 1 — Library/binary split + theme tokens + fonts + shell modules + new binary entry

## Summary

Lay both the **structural foundation** (the carve-out #3 library/binary split) and the **visual foundation** (theme tokens, fonts, shell chrome modules) that every later phase compiles against. Four categories of work:

1. **Carve-out #3 structural split.** Add `src/lib.rs` declaring the entire BIO module tree as `pub mod ...`; slim `src/main.rs` down to a ~12-line shim that calls `bio::ui::run` / `bio::app::dispatch::run`; create `src/bin/infinity_orchestrator.rs` for the new binary; add `[lib]` + `[[bin]] name = "infinity_orchestrator"` blocks to `Cargo.toml`. No logic changes; behavior preserved bit-for-bit.
2. **Fonts and theme tokens.** Embed Poppins and FiraCode Nerd into the build, add the redesign's color tokens (light + dark palettes) and layout/spacing tokens via additive `pub fn` accessors / `pub const`s in `src/ui/shared/` (analog of the existing module style — no edits to existing accessors).
3. **Shell chrome modules.** Create the (non-invoked) titlebar / statusbar / shell-frame modules so Phase 2 can wire them around `OrchestratorApp`. Nothing renders them in Phase 1.
4. **New binary entry.** `src/bin/infinity_orchestrator.rs` runs a placeholder eframe window in Phase 1 (Phase 2 replaces the placeholder with `OrchestratorApp`).

The legacy `BIO` binary continues to dispatch through `bio::ui::run` → `WizardApp`. Behavior is bit-for-bit identical to today.

## What ships after this phase

- `cargo build --bin BIO_legacy --release` succeeds — produces the existing `BIO_legacy` binary, behaviorally unchanged. (`src/main.rs` is now a thin shim but its runtime behavior is identical.)
- `cargo build --bin BIO --release` succeeds — produces a new binary that launches an empty placeholder eframe window with the title "Infinity Orchestrator (Phase 1 stub)" centered in 13px Poppins on the redesign's dark-palette `page_bg` color.
- `cargo build` (no target flag) builds both binaries.
- Both binaries coexist in `target/release/`.
- Dark theme is the default; Light theme tokens are present in the palette enum but not yet user-switchable.
- Fonts Poppins (300 / 500 / 700) and FiraCode Nerd are bundled into the new binary at compile time.

## What's still missing

- `OrchestratorApp` struct (Phase 2).
- The shell chrome (titlebar + statusbar) is **not yet invoked** — its modules exist but no one calls them. Phase 2 introduces `OrchestratorApp` and wires the chrome around it.
- Left-rail navigation (Phase 2).
- Theme switcher in Settings (Phase 4 surfaces the toggle; the tokens already exist).
- Any new screens (Home, Install, Create, Settings) — Phase 4-5.
- Workspace view (Phase 6).
- Statusbar `<N> modlists` and `<jobs running>` reading from registry / install state — Phase 3 / Phase 7.

## Dependencies

None. This is the foundation.

## File inventory

### New files

| Path | Purpose | Depends on |
|------|---------|-----------|
| `src/lib.rs` | **Carve-out #3 split.** Declares the entire BIO module tree as `pub mod ...` (mirrors today's `src/main.rs` `mod` block but with `pub` visibility). | — |
| `src/bin/infinity_orchestrator.rs` | **The new binary's `main`.** Depends on the library crate via `use bio::*`. Phase 1 contains a minimal placeholder: configure tracing (reusing `bio::logging::setup::init`), install redesign fonts on the egui context, then run `eframe::run_native` with an empty `eframe::App` impl that paints `redesign_page_bg()` and a centered placeholder label. Phase 2 replaces the placeholder app with `OrchestratorApp`. | redesign fonts, tokens, library crate |
| `assets/fonts/Poppins-Light.ttf` (or `.woff2`) | Embed Poppins 300 | — |
| `assets/fonts/Poppins-Medium.ttf` | Embed Poppins 500 | — |
| `assets/fonts/Poppins-Bold.ttf` | Embed Poppins 700 | — |
| `assets/fonts/FiraCodeNerdFont-Light.ttf` | Embed FiraCode Nerd 300 | — |
| `src/ui/shell/mod.rs` | `pub mod shell_chrome; pub mod shell_statusbar; pub mod shell_titlebar;` | — |
| `src/ui/shell/shell_chrome.rs` | `render_shell(ctx, body_fn)` — owns the outer egui frame layout: titlebar (top), body (center), statusbar (bottom). Calls into `shell_titlebar::render` and `shell_statusbar::render`. **Not invoked in Phase 1**; ready for Phase 2's `OrchestratorApp::update` to call. | `shell_titlebar`, `shell_statusbar` |
| `src/ui/shell/shell_titlebar.rs` | `render(ui)` — 34px titlebar with traffic-light dots + center title + window controls. Pure visual; no window-drag wiring in this phase. | theme tokens |
| `src/ui/shell/shell_statusbar.rs` | `render(ui, modlist_count, jobs_running)` — 26px footer with status segments. | theme tokens |
| `src/ui/shared/redesign_tokens.rs` | New module re-exported via `theme_global.rs`. Defines redesign-palette `egui::Color32` constants per SPEC §12.1 (light + dark), pill tones per §12.2, selection / hover overlays per §12.3. Exposes `ThemePalette { Light, Dark }` enum (plain `Copy` — no global storage) + pure-function accessors that take a `palette: ThemePalette` argument. | — |
| `src/ui/shared/redesign_fonts.rs` | `install_redesign_fonts(ctx)` — builds a complete `FontDefinitions` from scratch (`FontDefinitions::default()` as the base + the redesign families) and registers it via `ctx.set_fonts(...)`. Per H7: no additive composition; the function fully owns the `FontDefinitions` it installs. | — |

### BIO files read from / consumed (no modifications beyond the split)

- `bio::logging::setup::init` (was `crate::logging::setup::init`) — called from the new binary's `main` to configure `tracing`. Used identically to how `src/main.rs` uses it today.
- `bio::cli::args::Cli` (was `crate::cli::args::Cli`) — the new binary may parse a stripped-down subset of CLI flags (just `--dev-mode` and `--log-level`) for development. Reuse the existing `Cli` struct or define a minimal local `OrchestratorCli`. Either path leaves `Cli` itself unmodified.
- `bio::ui::shared::typography_global::configure_typography` (was `crate::ui::shared::typography_global::configure_typography`) — called from the new binary's `main` **after** `install_redesign_fonts`. Per H7, the redesign installs `FontDefinitions` first; `configure_typography` only mutates `Style`, so it composes cleanly when called second. No modification.

### BIO files needing allowed mild refactor

Three files are touched by the carve-out #3 split. **No logic changes** — only the structural split's deletion/movement of `mod` declarations and the `crate::` → `bio::` path rewrite required by `main.rs` no longer being the crate root.

| File | Carve-out | Change (strict) |
|------|-----------|-----------------|
| `src/main.rs` | #3 | Delete the `#[path = "core/<x>/mod.rs"] mod <x>;` block (now in `lib.rs`). Rewrite the four `use` lines and two dispatch call sites from `crate::` to `bio::`. Body of `main()` itself preserved verbatim. End-state is the 12-line shim shown in the overview's Architecture section. |
| `Cargo.toml` | #3 | Add `[lib] name = "bio" path = "src/lib.rs"` block. Add `[[bin]] name = "infinity_orchestrator" path = "src/bin/infinity_orchestrator.rs"` block. The existing `[[bin]] name = "BIO" path = "src/main.rs"` block stays in place. Cargo.toml is build configuration; the additions are part of carve-out #3. |
| (existing BIO module tree — read-only) | #3 | No file content changes. Visibility of every top-level module is implicitly elevated from "crate-private" (the `mod ...;` in `main.rs`) to "crate-public" (the `pub mod ...;` in `lib.rs`) because `lib.rs` is now the crate root. Per SPEC §1 carve-out #3, this implicit elevation is mechanical and authorized. |

The previous plan's "Phase 1 is strictly additive" wording is replaced. Carve-out #3 is not "additive" in the strict sense — `main.rs` does change — but the change is **mechanical and behavior-preserving**, with `git diff src/main.rs` showing only the `mod` block deletion and the path-prefix rewrite. No new behavior, no new branches.

### `Cargo.toml` additions (build configuration, part of carve-out #3)

Add a `[lib]` section and a new `[[bin]]` block:

```toml
[lib]
name = "bio"
path = "src/lib.rs"

[[bin]]
name = "BIO"
path = "src/main.rs"           # existing — stays

[[bin]]
name = "infinity_orchestrator"
path = "src/bin/infinity_orchestrator.rs"  # new
```

Both binaries link the `bio` library. The `bio` crate name matches today's `[package] name = "bio"`.

## Implementation tasks

### P1.T0 — Carve-out #3 structural split

- **What:** Apply the three-step mechanical restructure:
  1. **Create `src/lib.rs`** with the exact module declarations from today's `src/main.rs` (lines 9-26), but with `pub` visibility:
     ```rust
     // src/lib.rs
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
  2. **Slim `src/main.rs`** to the 12-line shim (per overview Architecture section). Delete the `mod` block; rewrite the three `use` lines and two call sites from `crate::` to `bio::`:
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
  3. **Edit `Cargo.toml`** to add the `[lib]` block and the new `[[bin]]` block per the section above.
- **Where:** `src/lib.rs` (new), `src/main.rs` (slim down), `Cargo.toml` (add `[lib]` + `[[bin]]`).
- **Acceptance:**
  - `cargo build --bin BIO_legacy --release` succeeds with no behavior change. Launching `target/release/BIO_legacy` produces the existing wizard UI.
  - `cargo build --lib --release` succeeds (the library compiles standalone).
  - `cargo build --bin BIO --release` produces an empty binary (P1.T8 fills in the placeholder app).
  - `git diff src/main.rs` shows only: the `mod` block deleted, three `use` lines rewritten `crate::` → `bio::`, two dispatch call sites rewritten `crate::` → `bio::`. No other change.
- **SPEC:** §1 CRITICAL DIRECTIVE carve-out #3, overview "Architecture" section.

### P1.T1 — Embed redesign fonts (per H7: complete `FontDefinitions`, no additive composition)

- **What:** Place the Poppins (300/500/700) `.ttf` files (from Google Fonts) and the FiraCode Nerd (300) `.ttf` file (from the Nerd Fonts project) into `assets/fonts/`. egui requires `.ttf`; the wireframe's `.woff2` files are not directly usable. `include_bytes!` them in a new file `src/ui/shared/redesign_fonts.rs`. The fonts the wireframe uses are already on disk in `infinity_orchestrator/wireframe-preview/poppins-*.woff2` and `firacode-nerd-*.woff2` for reference; obtain matching `.ttf` files from the upstream sources and copy them into `assets/fonts/` so the binary has them at build time.
- **Where:** Create `src/ui/shared/redesign_fonts.rs` with `pub fn install_redesign_fonts(ctx: &egui::Context)`. Per H7, the function builds a **complete `FontDefinitions` from scratch**:
  ```rust
  pub fn install_redesign_fonts(ctx: &egui::Context) {
      let mut fonts = egui::FontDefinitions::default();
      fonts.font_data.insert(
          "poppins_light".to_owned(),
          egui::FontData::from_static(include_bytes!("../../../assets/fonts/Poppins-Light.ttf")),
      );
      // ... insert poppins_medium, poppins_bold, firacode_nerd similarly ...
      fonts.families
          .entry(egui::FontFamily::Proportional)
          .or_default()
          .insert(0, "poppins_medium".to_owned());
      fonts.families
          .entry(egui::FontFamily::Monospace)
          .or_default()
          .insert(0, "firacode_nerd".to_owned());
      ctx.set_fonts(fonts);
  }
  ```
  **No `ctx.fonts(|f| ...)` read; no additive composition.** The function fully replaces the `FontDefinitions` egui holds. `configure_typography` (called separately by the binary's `main`) only mutates `Style`, so the two compose cleanly when `install_redesign_fonts` is called first.
- **Acceptance:** A unit test (`#[test] fn redesign_fonts_register()`) builds a headless egui context, calls `install_redesign_fonts`, and asserts that the new font families are present (at least `poppins_light`, `poppins_medium`, `poppins_bold`, `firacode_nerd`).
- **SPEC:** §1.2 (typography), §12 (theme tokens).

### P1.T2 — Define `ThemePalette` + color tokens in `src/ui/shared/redesign_tokens.rs`

- **What:** Create `src/ui/shared/redesign_tokens.rs` containing:
  - `pub enum ThemePalette { Light, Dark }` (a plain `Copy + Eq` enum — **no global storage**, no `AtomicU8`, no singleton). Per-frame the active palette is owned by `OrchestratorApp` and passed explicitly to render code.
  - Two `pub(crate) const` palette tables (`LIGHT: PaletteValues`, `DARK: PaletteValues`) holding all the `egui::Color32` values per SPEC §12.1.
  - `pub fn` accessors for every token, each taking `palette: ThemePalette` as its first argument: `pub fn redesign_page_bg(palette: ThemePalette) -> egui::Color32`, `redesign_shell_bg(palette)`, `redesign_chrome_bg(palette)`, `redesign_rail_bg(palette)`, `redesign_border_strong(palette)`, `redesign_border_soft(palette)`, `redesign_text_primary(palette)`, `redesign_text_muted(palette)`, `redesign_text_faint(palette)`, `redesign_text_fainter(palette)`, `redesign_input_bg(palette)`, `redesign_shadow(palette)`, `redesign_success(palette)`, `redesign_status_dot(palette)`, `redesign_accent(palette)`, `redesign_accent_hover(palette)`, `redesign_accent_deep(palette)`. Plus pill tones `redesign_pill_danger(palette)`, `redesign_pill_warn(palette)`, `redesign_pill_info(palette)`, `redesign_pill_neutral(palette)` (§12.2) and misc rules `redesign_selection_highlight(palette)`, `redesign_selection_highlight_hover(palette)`, `redesign_hover_overlay(palette)` (§12.3).
  - Each accessor reads from the matching palette table; pure function, no I/O, no mutable global.
- **Where:** Create new file `src/ui/shared/redesign_tokens.rs`. Register in `src/ui/shared/mod.rs` via `pub mod redesign_tokens;` (the existing `mod.rs` already follows this pattern for `theme_global`, `typography_global`, `tooltip_global`, `layout_tokens_global`).
- **Acceptance:** `redesign_shell_bg(ThemePalette::Light)` returns `#f5f8fc`; `redesign_shell_bg(ThemePalette::Dark)` returns `#111A21`. No global state, no atomic. Calls are referentially transparent and trivially testable in unit tests.
- **Light palette completeness.** SPEC §12.1 specifies the Dark palette in full but leaves some token values silent for Light. Fill the gaps from wireframe `index.html:14-36`:

  | Token | Light value | Source |
  |-------|-------------|--------|
  | `--text-muted` | `#5c6a7a` | wireframe `index.html:14-36` |
  | `--text-faint` | `#8896a8` | wireframe `index.html:14-36` |
  | `--text-fainter` | `#aebbcb` | wireframe `index.html:14-36` |
  | `--accent-hover` (dark-only in SPEC) | `#14B8A6` (= accent) | extends SPEC; Light uses same as accent |

  Extending the spec is allowed where the spec is silent; wireframe `index.html` is authoritative for any value not in SPEC §12.1.
- **SPEC:** §12.1, §12.2, §12.3.

> **Palette ownership and per-frame propagation.** The active `ThemePalette` is stored on `OrchestratorApp::theme_palette` (Phase 2 adds the field) and is read by `OrchestratorApp::update` once per frame. At the start of every frame, before any render code runs, the orchestrator applies the palette to `egui::Context::style_mut()` (so global egui visuals like default text color stay in sync) and passes the palette value into every shell render function (`shell_titlebar::render(ui, palette, ...)`, `shell_statusbar::render(ui, palette, ...)`, etc.) plus every destination renderer. The Settings → General theme toggle (Phase 4) writes the new palette into `OrchestratorApp::theme_palette` and into `bio_settings.json` via `RedesignSettings`; the change is visible on the very next frame. **No render function reads a global theme value.**

### P1.T3 — Add redesign layout token constants

- **What:** Add the redesign spacing/border constants in `src/ui/shared/redesign_tokens.rs` (same file as the colors — no need to fragment) as `pub const`s: `REDESIGN_BORDER_WIDTH_PX: f32 = 1.5`, `REDESIGN_SHELL_BORDER_WIDTH_PX: f32 = 2.0`, `REDESIGN_BORDER_RADIUS_PX: f32 = 3.0`, `REDESIGN_SHADOW_OFFSET_PX: f32 = 6.0`, `REDESIGN_SHADOW_OFFSET_BTN_PX: f32 = 2.0`, `REDESIGN_TITLEBAR_HEIGHT_PX: f32 = 34.0`, `REDESIGN_STATUSBAR_HEIGHT_PX: f32 = 26.0`, `REDESIGN_NAV_WIDTH_PX: f32 = 200.0`, `REDESIGN_DOT_BG_SPACING_PX: f32 = 20.0`, `REDESIGN_PAGE_PADDING_X_PX: f32 = 28.0`, `REDESIGN_PAGE_PADDING_Y_PX: f32 = 24.0`.
- **Where:** Add to `src/ui/shared/redesign_tokens.rs`. No edits to the existing `layout_tokens_global.rs`.
- **Acceptance:** All constants are present and visible from `bio::ui::shared::redesign_tokens::*`.
- **Note:** outer shell border (titlebar / statusbar separators / shell frame) uses 2px per wireframe `index.html:81`; inner section borders use 1.5px per SPEC §1.2.
- **SPEC:** §1.2 (sketchy borders / shadow), §12.3 (misc visual rules), §2.1 (left-rail width 200px labels mode).

### P1.T4 — Implement `src/ui/shell/shell_titlebar.rs`

- **What:** A `pub fn render(ui: &mut egui::Ui)` that paints the 34px titlebar: 1.5px bottom border in `redesign_border_strong`, `redesign_chrome_bg` background, three 12×12px circles (traffic-light dots, drawn as outlined circles via `egui::Painter`), centered title "INFINITY ORCHESTRATOR" in Poppins 10px weight 500 letter-spacing 1.5px text-transform uppercase, right-aligned `— ▢ ×` glyphs in 12px `redesign_text_muted`. No drag-to-move and no actual close/minimize wiring this phase.
- **Where:** Create `src/ui/shell/shell_titlebar.rs`.
- **Acceptance:** When called from a debug `eframe::App::update` body, the function paints a 34px strip matching the wireframe `index.html`'s `.sk-titlebar` rules (line 89-108) and `app.jsx`'s titlebar JSX (line 80-92).
- **SPEC:** §1.2 (custom titlebar).

### P1.T5 — Implement `src/ui/shell/shell_statusbar.rs`

- **What:** A `pub fn render(ui: &mut egui::Ui, modlist_count: usize, jobs_running: usize)` that paints the 26px footer: 1.5px top border in `redesign_border_strong`, `redesign_chrome_bg` background, left-aligned segments `● connected · <N> modlists · <J> jobs running` separated by ` · `, right-aligned `v<crate version>`. The status dot is `8×8px` filled in `redesign_status_dot` with a 1px `redesign_border_strong` ring.
- **Where:** Create `src/ui/shell/shell_statusbar.rs`.
- **Acceptance:** When called from a debug `eframe::App::update` body, renders a strip identical to the wireframe `app.jsx::sk-statusbar` (line 148-155). `modlist_count` and `jobs_running` are caller-provided (Phase 3 / Phase 7 wire real sources).
- **SPEC:** §1.2 (26px footer status bar always visible).

### P1.T6 — Implement `src/ui/shell/shell_chrome.rs::render_shell`

- **What:** A `pub fn render_shell<F: FnOnce(&mut egui::Ui)>(ctx: &egui::Context, modlist_count: usize, jobs_running: usize, body: F)` that uses `egui::TopBottomPanel::top("redesign_titlebar").exact_height(REDESIGN_TITLEBAR_HEIGHT_PX)` for the titlebar, `egui::TopBottomPanel::bottom("redesign_statusbar").exact_height(REDESIGN_STATUSBAR_HEIGHT_PX)` for the statusbar, and `egui::CentralPanel::default()` for the body. Inside CentralPanel, calls `body(ui)`. The function paints the `redesign_page_bg()` fill as the central-panel background. The dotted radial background per §12.3 is deferred to Phase 8 (acceptable to ship Phase 1 with a solid `page_bg`).
- **Where:** Create `src/ui/shell/shell_chrome.rs`.
- **Acceptance:** Function exists and compiles. **Not invoked in Phase 1** (Phase 2's `OrchestratorApp::update` is the first caller).
- **SPEC:** §1.2 (overall shell shape).

### P1.T7 — (merged into P1.T0 / Cargo.toml additions; previously a standalone task)

The previous Phase 1 plan had a separate task to add `[[bin]] name = "infinity_orchestrator"` to `Cargo.toml`. That work now belongs to P1.T0 (carve-out #3 structural split) alongside the `[lib]` block addition. Both Cargo.toml edits land together.

### P1.T8 — Create `src/bin/infinity_orchestrator.rs` (placeholder)

- **What:** Create the new file with a minimal `main` that:
  1. Parses a tiny CLI (just `--dev-mode` boolean and `--log-level` string) using `clap`, or reuses `bio::cli::args::Cli` if that's simpler (the orchestrator only needs to recognize `--dev-mode` for now).
  2. Calls `bio::logging::setup::init(&log_level)` (BIO's existing public function — no modification).
  3. Builds `eframe::NativeOptions` (window size, min size, icon — match the BIO native options where possible by reading the existing `bio::ui::frame::frame_window::native_options` function as a reference; alternatively define them inline).
  4. Calls `eframe::run_native("Infinity Orchestrator", options, Box::new(|cc| { bio::ui::shared::redesign_fonts::install_redesign_fonts(&cc.egui_ctx); bio::ui::shared::typography_global::configure_typography(&cc.egui_ctx); Box::new(PlaceholderApp::new(dev_mode)) }))`. **Order matters per H7: fonts first, then typography.**
  5. `PlaceholderApp` is a struct defined locally in this file (not under `src/ui/orchestrator/` yet — that arrives in Phase 2). Its `eframe::App::update` impl uses `egui::CentralPanel::default().show(ctx, |ui| { ui.painter().rect_filled(ui.max_rect(), 0.0, redesign_page_bg(ThemePalette::Dark)); ui.centered_and_justified(|ui| { ui.label(egui::RichText::new("Infinity Orchestrator (Phase 1 stub)").size(13.0).color(redesign_text_primary(ThemePalette::Dark))); }); });`. Both accessors take a `palette: ThemePalette` argument per P1.T2; the placeholder hard-codes `ThemePalette::Dark` because the theme switcher arrives in Phase 4.

  **Dev-mode resolution (per M12):** Phase 1's placeholder uses just the CLI flag for `dev_mode`. From Phase 4 onward, `OrchestratorApp::dev_mode = cli_flag || redesign_settings.diagnostic_mode` — the CLI flag `-d` and the Settings → General Diagnostic mode toggle both enable dev mode; either one is sufficient. On app launch, the toggle's persisted value is OR'd with the CLI flag. Phase 4's P4.T2 wires the runtime toggle.
- **Where:** Create `src/bin/infinity_orchestrator.rs`. The directory `src/bin/` is new; verify it before creation with `ls src/` (the registered shell tool).
- **Acceptance:** `cargo run --bin BIO` opens an eframe window with the dark `page_bg` and the centered placeholder label. The label renders in Poppins. The window closes cleanly. `cargo build --bin BIO_legacy` still works.
- **SPEC:** §1 (CRITICAL DIRECTIVE), overview "Architecture" section.

### P1.T9 — Acceptance smoke test

- **What:** Run `cargo build --bin BIO_legacy --release` and `cargo build --bin BIO --release` and `cargo build --lib --release`. Launch each binary. Confirm `BIO_legacy` still renders the wizard Step 1 normally (the carve-out #3 shim preserves behavior). Confirm `BIO` renders the placeholder. Run `cargo test --lib` for the new font-registration test from P1.T1.
- **Where:** Manual verification.
- **Acceptance:** All three builds succeed, neither binary panics on launch, the font-registration test passes. `git diff src/main.rs` shows only the mechanical mod-block deletion and the `crate::` → `bio::` path rewrite.

## Open questions / risks

- The wireframe's dotted radial background is purely decorative. Phase 1 ships a solid `page_bg` fill; Phase 8 may add the dot pattern via `egui::Painter`. Documented in Phase 8.
- Font licensing: Poppins (Open Font License) and FiraCode Nerd (OFL) are GPL-3.0-compatible (both SIL-OFL).
- **`Cargo.toml` change classification.** The CRITICAL DIRECTIVE forbids modifications to BIO **source**. `Cargo.toml` is build configuration. Adding a `[lib]` block + an `[[bin]]` entry is authorized by carve-out #3 (structural split). The `[lib]` block does not change any existing component's behavior; it exposes the existing module tree as a library.
- **The `src/main.rs` slim-down.** Authorized by carve-out #3. The `git diff` for `main.rs` shows only the `mod` block deletion and the `crate::` → `bio::` path rewrite — no logic changes, no body modifications, no signature changes. Behavior is bit-for-bit identical to today's `BIO` binary because the `bio::cli::*`, `bio::config::*`, `bio::logging::*`, `bio::ui::run`, `bio::app::dispatch::run` paths resolve to the exact same code as today's `crate::cli::*`, etc.
- **Wireframe `sk-annotation` / `sk-anno-nav` overlays do not ship (per L13).** The wireframe's `sk-annotation` / `sk-anno-nav` overlay annotations (visible in `infinity_orchestrator/wireframe-preview/index.html:186-196`) are wireframe-iteration debug aids; they do **not** ship in production. Same status as the Tweaks panel (SPEC §14.2). The orchestrator shell never renders these overlays — implementers should ignore them when porting the wireframe layout.

## Verification

1. `cargo build --bin BIO_legacy --release` succeeds with no new warnings. Run `target/release/BIO_legacy`: existing wizard renders unchanged.
2. `cargo build --bin BIO --release` succeeds. Run `target/release/BIO`: placeholder window renders with the dark palette and Poppins label.
3. `cargo build --lib --release` succeeds (library compiles standalone).
4. `cargo test --lib` — the new font registration test passes.
5. `git diff src/main.rs`: shows only the carve-out #3 mechanical changes (mod block deletion + path rewrites). No logic edits.
6. `git diff Cargo.toml`: shows the new `[lib]` block + the new `[[bin]] name = "BIO"` block. The existing `[[bin]] name = "BIO_legacy"` block stays.
7. `grep -r "Poppins\|FiraCode" target/release/BIO` confirms the fonts are bundled into the new binary.
