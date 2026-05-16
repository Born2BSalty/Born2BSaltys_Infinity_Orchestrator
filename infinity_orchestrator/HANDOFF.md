# Infinity Orchestrator — Handoff

A working snapshot of where the redesign sits today, what's left, and the context needed to finish.

The project is the redesign of the existing `bio` Rust crate (Born2BSalty's Infinity Orchestrator) into a multi-modlist workspace app. Built on `eframe` / `egui`. The redesign preserves BIO's deterministic install pipeline and adds a new visual language + a modlist registry on top.

---

## Status at handoff

| Phase | Subject | Status |
|---|---|---|
| 1 | Theme tokens, fonts, shell modules, new binary entry | ✓ done, builds clean |
| 2 | Navigation + routing (`OrchestratorApp`, shell chrome, left rail, page router, stubs) | ✓ done, builds clean |
| 3 | Modlist registry + per-modlist workspace state files | ✓ done, builds clean |
| 4 | Settings screen (5 sub-tabs) + per-edit debounced path validation | ✓ done, builds clean |
| 5 | Home + Install Modlist (paste / preview / download stages) | ✓ done — Runs 1–5 (Home + actions + Install shell/paste/Preview+provenance/stage-4 stub + Run 5 = §4.3 Downloading **chassis**). Live download data + per-install dirs + content-addressed staging deferred to Phase 7 P7.T17 (pipeline terminates in the install runtime) — SPEC §13.12a |
| 6 | Create screen + Workspace shell (Steps 2–4) | not started |
| 7 | Step 5 install runtime + Reinstall + import-code auto-write + install concurrency + rail-nav lock | not started |
| 8 | Popup reskins + state-aware theme reads across BIO surfaces + polish | not started |

After phases 5–8 land, the binary is feature-complete per the SPEC (modulo the deferred items in Appendix B and the known caveats below).

---

## What ships today

- Two binaries coexist:
  - `BIO` — the legacy linear-wizard app, untouched in behavior, still launches from `cargo run --bin BIO`.
  - `infinity_orchestrator` — the new redesigned app, launches from `cargo run --bin infinity_orchestrator`.
- Both build cleanly on macOS. Windows cross-compilation from macOS is not currently working — see the *Windows builds* section at the bottom of this doc.
- 163/163 lib tests pass (Phase 5 Run 3 added DestChoice→flag + warning-option-label tests).
- The orchestrator binary opens an `eframe` window (1280×820, min 1024×700) with:
  - **Titlebar** (34px, sketchy border, `Infinity Orchestrator` title centered, traffic-light dots top-left).
  - **Left rail** (200px) with the brand mark + 4 nav items (Home / Install / Create / Settings) + a bottom status indicator (`weidu vN · all paths ok` or per-path error count).
  - **Body** with the active destination's content.
  - **Statusbar** (26px) at the bottom showing modlist count + jobs-running placeholder.
- **Home** is the real screen (Phase 5 Runs 1–2): title + subtitle, filter chips (Installed / In progress / All) with counts + default-selection logic, modlist cards (in-progress `resume` / installed `open` + Kebab), `add a modlist` CTAs, `game installs detected` block, first-launch setup CTA, bottom-center toasts. Kebab actions are live: Copy import code (clipboard + toast), Delete (danger confirm → registry entry + guarded on-disk folder removal), Open install folder, Reinstall (Phase-7 placeholder toast). Rename is still inert (later run).
- **Install Modlist** is wired (Phase 5 Runs 3–5): the paste stage (destination FolderInput + `DestinationNotEmptyWarning` with Clear/Backup/Continue + import-code textarea, capped-to-footer + internal scroll; **a valid destination — a real existing folder — is required before proceeding** (SPEC §4.1); the warning Box is legible in Light + Dark), the **Preview** stage (parsed `ModlistSharePreview` → packed name/author title+subline with honest fallback, Overview Box, 6 file-folder tabs, `allow_auto_install` draft-gate with disabled Import + `Open in Create →`, `⑂ fork info` → `ForkInfoPopup`), the **Downloading** stage as the §4.3 **chassis** (overall-progress Box + 4-col mod grid + Cancel/auto-advance, grid empty until Phase 7 binds live data), and the stage-4 stub render.
- **Create** still shows a stub placeholder (Phase 6).
- **Settings**: real five-tab screen (General / Paths / Tools / Accounts / Advanced) with:
  - Live theme-palette toggle (Light / Dark) that updates next frame.
  - Per-keystroke debounced path validation that updates the rail status row.
  - GitHub OAuth `connect` button opens BIO's existing device-flow popup verbatim.
  - All settings persist immediately to `bio_settings.json` (existing BIO fields) and a new `bio_redesign_settings.json` (orchestrator-only fields).
- Modlist registry (`modlists.json`) + per-modlist workspace state (`modlists/<id>/workspace.json`) read/write via the new orchestrator-owned persistence cycle. Atomic writes via temp-file-then-rename. Corrupt registry → terminal error pane on next launch (no silent recovery).

---

## Build setup

Required toolchains on macOS (Apple Silicon — adapt paths for Intel):

```bash
# Rust
export PATH="$HOME/.rustup/toolchains/stable-aarch64-apple-darwin/bin:$PATH"

# Java (needed by `lapdu-parser-rust`'s build script for ANTLR codegen)
export PATH="/opt/homebrew/opt/openjdk/bin:$PATH"
```

If either is missing:
- `rustup` from `https://rustup.rs/` (Homebrew has `brew install rustup` as a wrapper).
- `brew install openjdk` (or any JDK 11+).

Build / test commands:

```bash
cargo build --bin BIO --release
cargo build --bin infinity_orchestrator --release
cargo test --lib
```

Both binaries land in `target/release/`. The orchestrator binary is ~11 MB after Phase 4.

Run the orchestrator (the eframe window will appear; on macOS it may open behind your Terminal — Cmd+Tab to switch):

```bash
./target/release/infinity_orchestrator        # production mode
./target/release/infinity_orchestrator -d     # dev mode (enables Seed test modlist button etc.)
```

---

## File layout

```
infinity_orchestrator/                  # this folder (artifacts: spec, plan, wireframe, handoff)
├── SPEC.md                             # canonical product spec (read first)
├── HANDOFF.md                          # this file
├── plan/
│   ├── overview.md                     # phasing philosophy + revision log
│   ├── phase-01-theme-and-shell.md
│   ├── phase-02-nav-routing.md
│   ├── phase-03-modlist-registry.md
│   ├── phase-04-settings.md
│   ├── phase-05-home-install-paste.md
│   ├── phase-06-create-workspace-shell.md
│   ├── phase-07-install-runtime.md
│   └── phase-08-popup-reskins-polish.md
└── wireframe-preview/                  # canonical visual reference (HTML+React preview)
    ├── build.html                      # built single-file preview (open in browser)
    ├── index.html                      # CSS tokens + font-face declarations + shell layout
    ├── screens.jsx                     # every screen + popup component
    ├── app.jsx                         # top-level shell + nav + route dispatch
    └── tweaks-panel.jsx                # design-iteration tool only (NOT shipped)

src/                                    # the actual `bio` crate
├── lib.rs                              # library root (Phase 1 carve-out #3)
├── main.rs                             # thin shim for the BIO binary
├── bin/
│   └── infinity_orchestrator.rs        # the new binary's main (Phase 1+2)
├── core/                               # BIO's existing core logic — TREAT AS PROTECTED
│   ├── app/                            # state machines, install runner, scan worker, ...
│   ├── cli/                            # CLI args
│   ├── config/                         # compat rules, mod-source manifests
│   ├── install/                        # install pipeline
│   ├── parser/                         # TP2 / weidu.log parsing (ANTLR-generated)
│   └── ...
├── settings/                           # bio_settings.json model + store
│   ├── model.rs                        # AppSettings + Step1Settings (BIO source)
│   ├── store.rs                        # SettingsStore (BIO source)
│   ├── redesign_fields.rs              # RedesignSettings (Phase 4 net-new)
│   └── redesign_store.rs               # RedesignSettingsStore (Phase 4 net-new)
├── ui/                                 # all UI rendering
│   ├── shared/                         # theme tokens, fonts, layout constants
│   │   ├── theme_global.rs             # BIO existing
│   │   ├── layout_tokens_global.rs     # BIO existing
│   │   ├── typography_global.rs        # BIO existing
│   │   ├── redesign_tokens.rs          # Phase 1 — REDESIGN CANONICAL TOKEN STORE
│   │   └── redesign_fonts.rs           # Phase 1 — font loader
│   ├── shell/                          # Phase 1 — shell chrome
│   │   ├── shell_chrome.rs
│   │   ├── shell_titlebar.rs
│   │   └── shell_statusbar.rs
│   ├── orchestrator/                   # Phase 2 — new orchestrator code
│   │   ├── orchestrator_app.rs         # OrchestratorApp (eframe::App impl)
│   │   ├── nav_destination.rs          # NavDestination enum + rail items
│   │   ├── left_rail.rs                # left rail widget
│   │   ├── page_router.rs              # destination dispatch
│   │   ├── nav_status.rs               # path-validation summary for rail status
│   │   ├── registry_error_panel.rs     # Phase 3 — terminal error UI
│   │   ├── widgets/                    # btn, r_box, label, screen_title
│   │   └── stubs/                      # placeholder destinations
│   ├── settings/                       # Phase 4 — Settings screen
│   │   ├── page_settings.rs            # 5-tab top-level
│   │   ├── state_settings.rs
│   │   ├── tab_general.rs              # name + theme + language + validate-on-startup + diag
│   │   ├── tab_paths.rs                # game/working folders + Validate now
│   │   ├── tab_tools.rs                # weidu / mod_installer / 7z / git binaries
│   │   ├── tab_accounts.rs             # GitHub / Nexus / Mega cards
│   │   ├── tab_advanced.rs             # timing + install behavior + WeiDU flags
│   │   ├── validate_now.rs             # synchronous validation
│   │   ├── validate_debounce.rs        # H11 — per-edit debounced validation
│   │   ├── oauth_glue.rs               # GitHub OAuth wrapper
│   │   └── widgets/                    # tab_strip, path_row, value_row, etc.
│   ├── step1/  step2/  step3/  step4/  step5/   # BIO existing — protected by CRITICAL DIRECTIVE
│   ├── app.rs  app_methods.rs ...      # BIO existing — WizardApp + handlers
│   └── frame/                          # BIO existing — window setup
├── registry/                           # Phase 3 — modlist registry
│   ├── model.rs                        # ModlistRegistry, ModlistEntry, ModlistState
│   ├── store.rs                        # RegistryStore (atomic load/save)
│   ├── workspace_model.rs              # ModlistWorkspaceState
│   ├── store_workspace.rs              # WorkspaceStore
│   ├── persistence_cycle.rs            # debounced writes + flush
│   ├── dev_seed.rs                     # dev-only seed for Phase 3 testing
│   ├── ids.rs                          # ULID-style ID generator
│   ├── errors.rs                       # RegistryError { Io, Parse, Corrupt }
│   └── operations.rs                   # stub for Phase 5 (create/rename/delete)
└── ...

assets/
├── fonts/                              # Poppins (300/500/700) + FiraCode Nerd 300 TTFs
├── icon.ico                            # Windows
└── icon.png                            # cross-platform

target/release/
├── BIO                                 # existing binary, ~26 MB
└── infinity_orchestrator               # new binary, ~11 MB after Phase 4

vendor/
└── lapdu-parser-rust-master/           # vendored TP2 parser (needs Java for ANTLR codegen)

third_party/
└── egui_term/                          # patched egui_term crate
```

Persistence files at runtime live in:
- macOS: `~/Library/Application Support/bio/`
- Linux: `~/.config/bio/`
- Windows: `%APPDATA%\bio\`

Specifically:
- `bio_settings.json` (existing BIO)
- `bio_redesign_settings.json` (Phase 4 net-new)
- `modlists.json` (Phase 3 net-new)
- `modlists/<id>/workspace.json` (Phase 3 net-new, one per modlist)
- `prompt_answers.json` (existing BIO)
- `step2_compat_rules_user.toml` (existing BIO)
- `mod_downloads_user.toml` (existing BIO)

---

## The CRITICAL DIRECTIVE (do not modify existing BIO components)

`SPEC.md` §1 is the single most important rule for this project. Read it before touching code.

**Two legal options for every redesign surface:**
1. Reuse the existing BIO component as-is (with theme-token styling applied).
2. Create a net-new component alongside (not on top of) the existing BIO code.

**Six approved carve-outs** for mild refactors to existing BIO source — anything outside these is disallowed:

1. **Theme-token extraction** — swap inline `Color32::from_rgb(...)` / `f32` literals for token reads. Pure value substitution.
2. **Window-chrome config flips** — single-line `.collapsible(false)` → `.collapsible(true)` and similar on `egui::Window` builders. Body content, signatures, behavior unchanged.
3. **Library/binary structural split** — done in Phase 1. Adds `src/lib.rs`, slims `src/main.rs` to a shim, adds a new `[[bin]]`. *Companion provision:* additive `pub mod foo;` lines in existing BIO `mod.rs` files are allowed to register new sibling modules (no reordering, no edits to existing lines).
4. **WizardApp → WizardState signature refactor** — BIO functions whose body only mutates `app.state` may be refactored to take `&mut WizardState`. Body unchanged. Audit found no actual cases needing this in v1; carve-out stays as a safety net.
5. **Schema-additive serde field additions** — new optional `#[serde(default = ...)]` fields on existing BIO serde structs. Default must preserve today's BIO behavior. Used by `allow_auto_install` on `ModlistSharePayload` (Phase 7).
6. **State-aware theme-token reads** — Phase 8 expanded. Inline color literals inside state-dependent conditionals may be swapped for `redesign_*(palette)` accessor calls, **provided** the conditional structure is unchanged (no new branches, no removed branches, no logic mutations). Function gains a `palette: ThemePalette` argument.

**Decision order when a BIO function is not a clean fit:**

1. **Direct reuse** if any `bio::app::*` / `bio::core::*` / `bio::ui::shared::*` public API does what's needed.
2. **Net-new sibling** for *simple* workflows (state mutations, dialog wrappers, format helpers, single-screen panels). This is the default fallback.
3. **Carve-out escalation** for *complex* workflows that can't be cleanly siblinged (install pipeline, share-code interop, multi-step state coordination). Requires explicit user approval.

Net-new is for simple things; carve-outs are for complex things that can't be cleanly cloned. Most "BIO function isn't reachable" flags are simple — build a sibling and move on.

---

## Source-of-truth ordering for new work

When phase implementation needs a value or behavior:

1. **`infinity_orchestrator/SPEC.md`** — the canonical product spec.
2. **`infinity_orchestrator/wireframe-preview/build.html`** + its source files — the canonical visual reference. **For UI / UX / layout / copy / spacing / pixel values, the wireframe wins over the spec.**
3. **The relevant `infinity_orchestrator/plan/phase-XX-*.md`** — your work order for this phase.
4. Existing BIO behavior — fallback only when spec, wireframe, and plan are silent.

Wireframe source files to read directly (don't paraphrase via the spec):
- `wireframe-preview/index.html` — CSS `:root` variables + font-face declarations.
- `wireframe-preview/screens.jsx` — every screen + popup component.
- `wireframe-preview/app.jsx` — top-level shell + nav.

The Tweaks panel (`tweaks-panel.jsx`) is wireframe-iteration only and does NOT ship.

---

## Remaining phases — quick reference

Each phase doc in `infinity_orchestrator/plan/` is the canonical work order. Summaries below.

### Phase 5 — Home + Install Modlist (paste / preview / download stages)

**Ships:**
- Real Home screen replaces the stub: title row, filter chips (`Installed (N)` / `In progress (P)` / `All (N+P)`), scrollable card list (mod name + meta line + `play` (renamed from wireframe's `play`, opens install folder for v1) / `resume` + Kebab), "Add a modlist" Box with `paste import code` / `create your own` CTAs, `game installs detected` block driven by Phase 4's path validation events, first-launch empty-registry CTA card.
- Install Modlist destination's first three stages: paste textarea + destination folder + DestinationNotEmptyWarning (3 radio options: `clear` / `backup` / `continue partial install`); preview screen with overview Box + 6-tab content Box (Summary / BGEE WeiDU / BG2EE WeiDU / User Downloads / Installed Refs / Mod Configs); downloading stage with per-mod progress grid.
- Stage 4 (the actual install) is stubbed and rolled in during Phase 7.
- Delete confirm dialog removes the modlist registry entry **and** the install folder.
- `allow_auto_install` flag check at preview stage: codes generated by drafts have the bit `false`; the preview disables the Install button and routes the user to `Create → Import and modify` instead. Per SPEC §4.2 + §13.3.

**Dependencies:** Phases 2 + 3 + 4.

### Phase 6 — Create + Workspace shell

**Ships:**
- Create destination: choose-mode setup Box (name + game ComboBox + destination FolderInput) + two starting-point cards (`New modlist from downloaded mods` / `Import and modify another modlist`) + `load draft` button opening the Load Draft dialog.
- Fork-paste / fork-preview / fork-download sub-flow for "Import and modify".
- WorkspaceView shell hosting Steps 2–4 (Step 5 stubbed; Phase 7 wires it):
  - Header row: `Editing <modlist name>` + ✎ rename inline edit + Fork badge + `save draft` / `Share import code` buttons.
  - 4-step progress bar.
  - Per-step hint line.
  - Step body — Steps 2 and 3 render via direct calls to BIO's existing `pub fn page_step2::render` / `page_step3::render` with the orchestrator's owned `WizardState`. Step 4 is an orchestrator-side renderer (per C4 — replaces BIO's `page_step4::render` to avoid the double Save button).
  - Workspace nav bar: `← Previous` / `Next →`.
- Workspace state loader: populates `WizardState` from per-modlist `workspace.json` on open; extracts back on save / nav-away / debounced write. **Loader is never invoked while an install is running** (per C5 — rail-nav lock).
- Per-frame `sync_paths_from_settings` mirrors `Step1Settings` paths into `wizard_state.step1` every workspace frame, so Settings → Paths edits propagate without requiring a workspace close/reopen (per M2).
- Per-frame dirty bit gates persistence writes (per H1) — no per-frame extract+compare.
- Step action dispatch tables in the phase doc enumerate every `Step2Action` / `Step4Action` variant + which `bio::app::*` public function handles it (per M4).

**Dependencies:** Phases 2 + 3 + 4 + 5.

### Phase 7 — Step 5 install runtime + Reinstall + import-code auto-write + install concurrency + rail-nav lock

**Ships:**
- Step 5 inside the workspace renders BIO's existing `page_step5::render` (the full embedded panel: Command card, Summary card, Actions/Diagnostics menus, Prompt Answers, console box wrapping `EmbeddedTerminal`, prompt input row). New chrome wraps **around** it — success banner row above, post-install action row above-and-adjacent to the (now disabled) Install button (per SPEC §9.2 + H9).
- Install start hook: writes `modlist-import-code.txt` to the install destination before WeiDU runs (per SPEC §13.13). Write semantics per button variant: `Install` / `Restart Install` / `Reinstall` overwrite; `Resume Install` does not (per H10).
- Post-install state transition: clean exit (the C3 triple: `install_running == false && last_exit_code == Some(0) && last_install_failed == false`) flips the registry entry from `in-progress` to `installed`, regenerates `latest_share_code` with `allow_auto_install = true`. Async size computation on a worker thread (per M5).
- `SharePasteCodeDialog` opens from the workspace header's `Share import code` button (post-install only).
- Reinstall flow from Home Kebab: danger confirm modal → routes to Install Modlist preview stage with overwrite-install forced → user clicks Install → registry flips back to in-progress → install runs (per SPEC §3.1 + H2). Cancel-preview leaves modlist in `installed` (per M5).
- **Install concurrency policy** (SPEC §13.15): only one install runs at a time. **Rail navigation is hard-locked** while an install runs (per C5) — every left-rail item disabled with the SPEC tooltip. User can only stay in the running install's workspace until cancel or completion.
- Install Modlist stage 4 wired (the real install runtime; not in the workspace chrome).
- **P7.T17 — per-install dirs + content-addressed archive staging + import→auto-build pipeline drive (SPEC §13.12a).** Derives the per-install Mods folder + the install-critical game-clone dirs (#2 `-u`, #3 `-p`/`-n`, #4 `-g`) inside the destination with forced clone flags; the net-new content-addressed staging layer wraps `app_step2_update_download`/`app_step2_update_extract` (zero BIO change — dedupe/coexist/extract-by-hash); drives `import_modlist_share_code` → saved-log/auto-build → download/extract → install; binds the Phase-5 §4.3 Downloading chassis (and the Phase-6 fork-download chassis) to live data. Global paths come from Settings → Paths via `sync_paths_from_settings`.
- `pending_reinstall_id: Option<String>` on `OrchestratorApp` (per L12) tracks the in-flight reinstall route.
- Automatic flag policies: #1 (`-s` / `-c`) + #5 (`--download`) wired in Phase 7 P7.T16; **#2 (`-u`) + #3 (`-p`/`-n`) + #4 (`-g`) wired in Phase 7 P7.T17** (their per-install dirs are install-critical — an install can't run without them, so they cannot be deferred to Phase 8 — SPEC §13.12a). Only #6 (`prepare_target_dirs`/`backup` from the `DestChoice` mapping, already the pure `DestChoice::to_flags` from Run 3) and #7 (`-autolog`/`-logapp`/`-log-extern`, hardcoded) remain for Phase 8.

**Dependencies:** Phases 2 + 3 + 5 + 6.

### Phase 8 — Popup reskins + state-aware theme reads + polish

**Ships:**
- Theme-token extraction (carve-out #1) on the popup files: `compat_popup_step2.rs`, `compat_window_step2.rs`, `prompt_popup_step2.rs`, `update_check_popup_step2.rs` + its companions, `github_auth_popup_step1.rs`.
- `.collapsible(false)` → `.collapsible(true)` flips (carve-out #2) on those popups so the global collapse chevron pattern works.
- **State-aware theme-token reads (carve-out #6)** on the Step 2 tree (`tree_compat_display_step2.rs`, `tree_component_row_step2.rs`, `tree_parent_step2.rs`, `tree_header_marker_step2.rs`, `format_step2.rs`), Step 2 Details panel (`details_pane_step2.rs`, `details_paths_step2.rs`, `details_selection_step2.rs`), Step 3 reorder list (`list_rows_step3.rs`, `content_step3.rs`, `format_step3.rs`, `toolbar_compat_step2.rs`), Step 5 sub-renderers (`content_install_row_step5.rs`, `content_cancel_step5.rs`, `content_dev_header_step5.rs`, `status_phase_step5.rs`, `status_console_step5.rs`).
- Anchor-on-collapse wrapper for popups (if egui's native title-bar collapse doesn't auto-anchor) in `src/ui/orchestrator/widgets/popup_collapse_anchor.rs`.
- Residual automatic flag policies from SPEC §13.12: **#6 + #7 only** (#1/#5 → Phase 7 P7.T16; #2/#3/#4 install-critical per-install dirs → Phase 7 P7.T17, SPEC §13.12a) + Settings-surface removal.
- Dotted radial background pattern matching the wireframe's `body` background.
- Toast notifications, hover affordances, copy-to-clipboard polish.
- Final smoke pass.

After Phase 8, every workspace surface visually matches the wireframe — Step 2 tree, Step 3 list, Step 5 console all render in the redesign's dark teal-on-slate palette.

**Dependencies:** Phases 1–7 (phases 5+6+7 surface the BIO renderers Phase 8 touches).

---

## Known caveats (carry these forward)

- **Latin-subset fonts.** Phase 1 derived Poppins TTFs from the wireframe's `.woff2` Latin-only subsets. FiraCode Nerd is full coverage. Before any non-stub UI ships in production, replace Poppins TTFs in `assets/fonts/` with full Latin-Extended `.ttf` builds from upstream Google Fonts. Code uses `include_bytes!` so swapping the files + rebuilding is sufficient.
- **`PromptInfo` private-interface warnings** (2 warnings) in `src/core/app/terminal/output.rs` are pre-existing BIO source; not introduced by the orchestrator work. Mention if you ever clean them; otherwise leave.
- **BIO's `configure_typography` must not be called from orchestrator code.** It calls `ctx.set_fonts(FontDefinitions::default())` which wipes the redesign font registrations. The orchestrator only calls `install_redesign_fonts`.
- **`FontFamily::Name("X")` requires `X` to be registered** in `install_redesign_fonts`. Registered names: `poppins_light`, `poppins_medium`, `poppins_bold`, `firacode_nerd`. Using an unregistered name causes a runtime panic (`FontFamily::Name(\"X\") is not bound to any fonts`).
- **Symbol-glyph coverage is split — know which font has what (cmap-verified).** The shipped Poppins TTFs are a Latin-only **217-glyph subset**: every non-Latin glyph tofus in any `poppins_*` family (and a tofu'd `✓` silently becomes `?`, which has masqueraded as real state — a *detected* game looked *missing*). `assets/fonts/FiraCodeNerdFont-Light.ttf` is the **full** 10,801-glyph Nerd build and **does** cover base-FiraCode ranges — math/arrows/dingbat-checks: `∞` U+221E, `✓` U+2713, `→` U+2192, `←` U+2190 all present (cmap-verified) — so render *those* glyphs in `firacode_nerd`, prose in Poppins. **But FiraCode Nerd does NOT cover the "Miscellaneous Symbols" block (U+2600–26FF).** Verified absent even in the full build: `⚠` U+26A0 (and by the same token `⚙` U+2699, `☰` U+2630, etc. — note `nav_destination::icon()` still returns these as strings, harmless only because `left_rail` paints the rail icons as vectors and never renders that string). For any Misc-Symbols / emoji glyph, **paint a vector** — `left_rail.rs`'s nav icons and `destination_not_empty.rs`'s `paint_warning_triangle` are the precedent; it decouples from font coverage entirely. The Latin-Extended Poppins swap above fixes none of this (these aren't Latin-Extended). Don't assume a glyph exists in a font — check the cmap (`python -m fontTools` / the bundled fonttools approach used in this session).
- **Per-frame theme propagation.** The active `ThemePalette` lives on `OrchestratorApp::theme_palette`. Pass it explicitly into every render function that needs colors. There is no global theme state.
- **Share-code provenance (`name`/`author`/`forked_from`).** Packed *inside* the BIO-MODLIST-V1 payload (not a string prefix) via the net-new orchestrator `registry::share_export::pack_meta` envelope — BIO's generator/consumer are unmodified beyond the carve-out #5 `#[serde(default)]` fields. `author` ← `RedesignSettings.user_name` (SPEC §11.1); `name` ← `ModlistEntry.name`; `forked_from` is append-only (`ForkAncestor { name, author }`, oldest→newest) so original creators stay credited. Displayed in the Install/fork preview title+subline and the `ForkInfoPopup` (SPEC §10.9; the same `fork_info_popup.rs` widget serves the Install preview, fork-preview, and workspace `⑂ view fork details`). Codes lacking the fields fall back to `Shared modlist` / author-less — intentional, not a defect. Full spec: SPEC §13.3 (Provenance + Generation mechanism), §1 carve-out #5; rationale in overview.md 2026-05-15 revision-log entry.
- **Directory architecture + content-addressed archives (SPEC §13.12a).** Global, Settings-defined (§11.2): **Mods archive** (ALL downloads for ALL modlists, always), **Mods backup**, **Game sources**. Per-install, inside the destination: the **Mods extract/stage/scan folder** (removed on clean success) + the **game-install clone dirs** (already specced by §13.12 #3/#4 — always cloned, fixed names; the redesign never surfaces BIO's no-clone path, BIO untouched). The global Mods-archive is **content-addressed**: hash-on-write, same name+hash ⇒ cross-modlist dedupe, same name/different hash ⇒ both coexist, the modlist lock records its hash, extract selects the matching archive per install. This is a **net-new orchestrator staging layer wrapping** `app_step2_update_download`/`app_step2_update_extract` (zero BIO modification). The orchestrator drives BIO's `import_modlist_share_code` → saved-log/auto-build pipeline; global paths reach the owned `WizardState` via `sync_paths_from_settings` (the Install screen does NOT collect game paths). Lands in **Phase 7 P7.T17** (the pipeline terminates in the install runtime); Phase 5 shipped the §4.3 chassis only. Rationale: overview.md 2026-05-16 revision log.
- **Lessons — phase-local reasoning is a real failure mode.** Run 5's implementer concluded "the Install screen has no game paths" because its brief was phase-local (phase-05 + the BIO download engine) and never stated that Phase-4 Settings → Paths supplies them globally + `sync_paths_from_settings` feeds the owned `WizardState`. Fix operationalized in `.claude/orchestrator-handoff.md`: every implementer brief now carries a standing **"Already built / cross-phase context"** block, and on verify the orchestrator sanity-checks an escalation's *premises* against the HANDOFF status table — not just its novel technical claim.

---

## How to implement a phase

For each remaining phase (5–8), the recommended flow is:

1. Read the phase doc at `infinity_orchestrator/plan/phase-XX-*.md` end to end.
2. Read the relevant SPEC sections (cross-referenced in the phase doc).
3. Read the matching wireframe components in `wireframe-preview/screens.jsx` directly — don't paraphrase through the spec for visual values.
4. Read `src/CLAUDE.md` and `src/ui/CLAUDE.md` for codebase orientation, plus the specific BIO files the phase doc cites.
5. Implement the phase's tasks in order. Strictly additive new files except where the phase explicitly authorizes a carve-out.
6. After each task, `cargo build --bin infinity_orchestrator --release` and visually verify against the wireframe.
7. Run `cargo test --lib` regularly — 116/116 should pass plus whatever new tests the phase adds.
8. End each phase with `cargo build --bin BIO --release` to confirm the legacy binary is still unaffected.

### Sample implementation-agent prompt template

If dispatching an AI agent to implement a phase:

```
Execute Phase N of the Infinity Orchestrator implementation plan. Follow the
plan and spec EXACTLY. Surface plan-vs-spec or plan-vs-source conflicts in
your final report — don't invent fixes; flag them.

## Hard rules

1. Scope = Phase N only. Nothing from later phases.
2. CRITICAL DIRECTIVE compliance. Only the 6 SPEC carve-outs. New files
   only, except where the phase doc explicitly authorizes a carve-out.
3. Phases 1–(N-1) artifacts already exist on disk. Build on them.
4. Plan ↔ source-of-truth conflict → surface, don't decide.

## Required reading

1. `infinity_orchestrator/SPEC.md` §1 CRITICAL DIRECTIVE + relevant sections
   for Phase N.
2. `infinity_orchestrator/plan/phase-NN-*.md` — full doc, your work order.
3. `infinity_orchestrator/plan/overview.md` — architecture context.
4. `infinity_orchestrator/HANDOFF.md` — current project state + caveats.
5. Phase 1–(N-1) deliverables on disk.
6. BIO source files cited by the phase doc.

## Build verification

After each task / at end of phase:

  export PATH="/opt/homebrew/opt/openjdk/bin:$HOME/.rustup/toolchains/stable-aarch64-apple-darwin/bin:$PATH"
  cargo build --bin BIO --release
  cargo build --bin infinity_orchestrator --release
  cargo test --lib

Both binaries must build clean. Tests must pass.

## Output

1. Tasks completed (P_N.T# with file paths).
2. Discrepancies surfaced.
3. Build result.
4. Run result (`./target/release/infinity_orchestrator` stays alive past
   window-open with no panic).
5. Files created / modified.

Begin.
```

---

## Lessons learned (carry these forward)

These tripped us up in earlier phases; flagging so future phases can avoid them.

- **`pub mod foo;` in existing BIO `mod.rs` was initially ambiguous.** Phase 1 surfaced this as a discrepancy because the CRITICAL DIRECTIVE originally only authorized `main.rs` + `lib.rs` edits. Resolution: carve-out #3 got a "companion provision" allowing additive `pub mod` lines in existing `mod.rs` files. Phases 5–8 can use this freely for registering new sibling modules in `src/ui/orchestrator/`, `src/ui/settings/`, etc.
- **`configure_typography` wipes the redesign font config.** Phase 1's first run panicked because the orchestrator called BIO's `configure_typography` after `install_redesign_fonts`. BIO's function calls `ctx.set_fonts(FontDefinitions::default())` which replaces everything. The orchestrator now skips `configure_typography` entirely. Don't reintroduce it.
- **Plan task-numbering can drift from prose references.** When the plan says "see P7.T6" but the actual task numbered T6 covers something different, trust the prose and renumber. Agent runs caught a few of these (M4 referenced P7.T5 but the size computation lives in P7.T6).
- **Step renderer signatures aren't all symmetric.** Step 2 returns `Option<Step2Action>`, Step 3 returns `()` (no action enum — mutates `WizardState` directly), Step 4 has an action enum but the orchestrator doesn't call BIO's renderer (per C4 it uses its own orchestrator-side body), Step 5 has extra channel-receiver arguments. The plan's overview enumerates each (per M1).
- **`OrchestratorApp` needs 6 Step 2 channel receivers, not 5** (per the late-surfaced M-new-1 from review pass 3). `bio::app::app_update_cycle::poll_before_render` takes a `step2_update_extract_rx` in addition to the more obvious 5. Phase 7 P7.T1 enumerates the field set.
- **`ModlistSharePreview` needs `allow_auto_install` added too** (per M-new-2), not just `ModlistSharePayload`. Phase 5 P5.T10 documents the addition under carve-out #5.
- **Carve-out #5 now carries the provenance trio, not just `allow_auto_install`** (user-directed spec change, 2026-05-15 — overview.md revision log). The Phase-5 Run-4 BIO touch on `modlist_share.rs` is exactly: `allow_auto_install` + `name`/`author`/`forked_from` (`#[serde(default)]`) on `ModlistSharePayload` **and** `ModlistSharePreview`, a `default_true()` fn, a `ForkAncestor` struct (`#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]` — the full set, so Phase 6's `ModlistEntry.forked_from` reuse needs no follow-up BIO edit), and 4 `share_preview()` propagation lines — nothing else. SPEC §1 "Modlist-share provenance application" is the exact authorized surface; the BIO-source guard must still find this the *only* BIO edit in Phase 5.
- **Share-code generation is a net-new orchestrator sibling (`registry::share_export::pack_meta`), never a BIO edit.** It composes `export_modlist_share_code` and does a standard zlib+base64url+`serde_json::Value` envelope round-trip injecting the four keys. **This fixed a latent plan defect:** the earlier P7.T3/T6 wording ("re-decode the payload, flip the bit, re-encode") was unimplementable because BIO's envelope primitives (`base64url_*`/`zlib_*`/`decode_share_payload`) are *private*. Generation lands Phase 7 (P7.T3 install-start, P7.T6 `flip_to_installed`); fork-lineage append + `ModlistEntry.author`/`forked_from` land Phase 6 (P6.T8). Run 4 is consume-only.
- **`unrar-sys` is hostile to cross-compilation from non-Windows.** Three different toolchains (local MinGW, cross via Docker, cargo-xwin with clang-cl) hit three different errors. If a Windows build is needed before all phases are done, the realistic path is GitHub Actions running on `windows-latest` (a `.github/workflows/build-windows.yml` setup — see Windows section below).

---

## Windows builds

Not currently working from macOS. We tried:

1. **MinGW local (`x86_64-pc-windows-gnu`)** — failed on `unrar-sys`'s missing Windows-API symbols (`WinNT()`, `IsWindows11OrGreater()`) + a pthread static-vs-dynamic library conflict.
2. **`cross` (Docker MinGW)** — failed on a case-sensitive header (`#include <PowrProf.h>` vs filesystem `powrprof.h`).
3. **`cargo-xwin` (MSVC ABI via Windows SDK)** — failed on SSSE3 intrinsics in `unrar-sys`'s `rs16.cpp` without a `-mssse3` flag; flag wasn't propagatable via env vars to the build script.

The root cause is `unrar-sys`'s heavy Windows-native C++ build assumptions. Each toolchain hits a different paper cut.

The pragmatic Windows build path is **GitHub Actions** running on `windows-latest` (real native Windows, no cross-compile). Sample workflow:

```yaml
# .github/workflows/build-windows.yml
name: Build Windows
on: [push, workflow_dispatch]
jobs:
  build:
    runs-on: windows-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: actions/setup-java@v4
        with:
          distribution: temurin
          java-version: '21'
      - run: cargo build --bin infinity_orchestrator --release
      - uses: actions/upload-artifact@v4
        with:
          name: infinity_orchestrator-windows
          path: target/release/infinity_orchestrator.exe
```

Push the workflow file; download the `.exe` from the Actions tab when done. Alternatively, build on any real Windows machine — `cargo build --bin infinity_orchestrator --release` runs natively without issue (we verified the codebase has no Windows-specific bugs; the issue is purely cross-compile tooling).

The macOS / Linux native builds work fine and are the default in development. Cross-platform release builds can be set up at any time without affecting the rest of the plan.

---

## Adversarial review history

The plan went through three adversarial review passes before implementation. All findings (5 critical + 11 high + 12 medium + ~14 low across three reviews) have been resolved or applied. Highlights worth remembering:

- **C1 — lib+bin split.** Original plan assumed `OrchestratorApp` could host `WizardApp`; reviewer found this required `pub(super)` flips not authorized by the directive. Resolution: standalone `OrchestratorApp` + lib+bin split (carve-out #3). The orchestrator and `WizardApp` are parallel `eframe::App` impls, both compiled from the same `bio` library.
- **C4 — Step 4 double Save button.** BIO's `page_step4::content_step4.rs` already renders a `Save weidu.log's` button; the wireframe places one in a new top action row. Two save buttons would render. Resolution: Phase 6 replaces BIO's Step 4 body with an orchestrator-side renderer (still calls BIO's save action via `bio::app::app_step4_*` public dispatch — only the rendering is net-new).
- **C5 — workspace state corruption mid-install.** If the user navigated to a different modlist while an install was running, the workspace state loader would reset `WizardState.step5`, panicking the install runtime. Resolution: rail navigation is hard-locked while an install runs.
- **`allow_auto_install` bit** (introduced by user mid-plan as a new feature). Draft / mid-install share codes have the bit `false`; auto-install is gated in the Install Modlist preview. Only `flip_to_installed` produces codes with the bit `true`. Carve-out #5 authorizes the schema addition on `ModlistSharePayload` and `ModlistSharePreview`.
- **Share-code provenance trio** (user-directed spec change, 2026-05-15). `name` / `author` / `forked_from` are packed into the payload alongside `allow_auto_install` under the same carve-out #5 (now 4 fields). Generation is the net-new `registry::share_export::pack_meta` envelope (composes BIO, zero BIO-generator edit); `forked_from` lineage is append-only so original modlist authors stay credited through forks; surfaced via the `ForkInfoPopup` (SPEC §10.9). This also resolved a latent defect where the prior P7 plan assumed BIO's *private* envelope primitives were reachable. SPEC §13.3 + §1 carve-out #5 + overview.md revision log carry the full record.
- **Phase 8 visual reskin** (M7) — originally Phase 8 aggressively pruned to only popups + console line tones, leaving Step 2/3/4 visually mismatched with the wireframe. The user authorized carve-out #6 (state-aware theme-token reads) to expand Phase 8 to cover all 24 in-scope BIO files. After Phase 8 every workspace surface visually matches the wireframe.

For the full review reports, the second-pass review is preserved at `/tmp/review2.md` (transient; may be cleaned up by the OS). The plan's `overview.md` revision log captures the high-level decisions.

---

## Finishing the plan — recommended pacing

- **Phase 5** is the next-biggest visible milestone. After it lands, Home looks real, Install Modlist's first three stages work end-to-end, modlist cards persist across launches. ~1 long agent run.
- **Phase 6** is the most complex remaining phase (workspace shell + 4 steps + Create + Load Draft). Plan on dispatching it as its own dedicated agent run with the full Phase 6 doc as the work order.
- **Phase 7** is the install-runtime phase. Substantial but the install pipeline itself is BIO's existing code — Phase 7's work is wrapping it. Expect agent run ~similar to Phase 6.
- **Phase 8** is mostly mechanical (theme-token extraction + carve-out #6 conditional swaps across ~24 files) but slow due to file count. Can be split into 8a (popups + console — carve-outs #1 + #2) and 8b (Step 2/3/5 state-aware — carve-out #6) if a single run is too long.

After Phase 8: cargo build, smoke test, ship.
