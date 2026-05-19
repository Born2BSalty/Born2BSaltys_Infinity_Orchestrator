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
| 6 | Create screen + Workspace shell (Steps 2–4) | ✓ **COMPLETE** — 4-run slice: **R1 done** (workspace spine + Step-2 C4 chrome, hardened 1b–1e), **R2 done** (Step-4 C4 renderer + workspace header/rename + save-draft + game tabs), **Step-3 C4 chrome done** (P6.T2d — `src/ui/workspace/step3/`, net-new chrome wrapping BIO's reused drag-reorder list; SPEC §7.1 / phase-06 P6.T2d / phase-08 P8.T5 cascade landed with it; overview 2026-05-17 §4); **R3 done** (Create screen `stage_choose` + Load Draft dialog + Create/Resume routing — P6.T7/T9/T13/T14; the `create_modlist` 5-arg PLAN-GAP + the SPEC §10.2 file-picker doc-drift were premise-checked and fixed in the same commit; overview 2026-05-17 §5); **R4 done** (P6.T8 fork sub-flow + lineage append + P6.T11 dirty-bit workspace persistence + P6.T15 nav-away flush; overview 2026-05-17 §6) — **Phase 6 COMPLETE; next is Phase 7 (install runtime).** Post-completion hardening: DATA-LOSS order-wipe regression closed (Fix-Run-3/4), Create #4 wireframe-alignment fixed + orchestrator-render-verified (Fix-Run-5/6), `egui_kittest` UI render gate added. 5 minor Create issues + box-equal-height + save-model redesign + deselect-last → Phase 8. Detail: overview.md 2026-05-18 revision log |
| 7 | Step 5 install runtime + Reinstall + import-code auto-write + install concurrency + rail-nav lock | IN PROGRESS — 4-run slice in .claude/orchestrator-handoff.md. **Run 1 done** (Step-5 runtime spine + workspace chrome): P7.T1 (`OrchestratorApp` formalizes `step5_terminal`/`step5_terminal_error`/`step5_console_view`/`step5_prep_rx`/`step5_pending_start`; the orchestrator `update` loop drives the Step-5 channels every frame via the SAME `bio::app::app_step5_flow` sequence `bio::ui::app::update_loop::run` uses — `poll_step5_terminal`+`poll_step5_prep` pre-render, `start_if_requested` post-render; `mod install_runtime` registered in `src/lib.rs` [the plan's "in `src/bin/infinity_orchestrator.rs`" is a PLAN GAP — that binary is a shim with no `mod` block; `mod registry;` actually lives in `lib.rs`, the established carve-out-#3 companion-provision pattern]); P7.T2 (`src/ui/workspace/step5/` net-new — `page_workspace_step5::render` wraps `bio::ui::step5::page_step5::render` read-only with the empty-pre-install success-banner + post-install rows ABOVE the panel per H9; replaced + deleted the Phase-6 `workspace_step5_stub`; `success_banner`/`post_install_actions` C3-gated render nothing pre-install; `share_paste_code_dialog`/`state_workspace_step5` minimal — the latter holds the Run-1 install-clicked marker); P7.T8 (`workspace_nav_bar` `← Previous` carries the VERBATIM SPEC §9.2 tooltip when disabled; `workspace_view` computes `disable_prev = install_complete \|\| step5.install_running \|\| workspace_step5.install_clicked`). Zero BIO-source edits; `page_step5::render` + `bio::app::*` read-only; `terminal: None` pre-install path. 298/0 lib tests (+9 vs 289). Render gate `tests/ui_snapshot_workspace_step5.rs` (1280/1045/960). **Runs 2–4 + fix-runs done — Phase 7 COMPLETE** (the per-run + per-fix-run record lives in `infinity_orchestrator/PENDING_VERIFICATION.md`, the authoritative live state). **Bundled Fix-Run (2026-05-18, plan P7.T17 "Bundled Fix-Run" entry) — T-C / T-D stand; T-A + T-B UNWOUND by THE AMENDMENT:** T-C Install-screen console right-edge clip + T-D Downloading screen (D1 off-render-thread one-shot staging — the freeze fix; D2 scrollable grid) stand. **Downloading-window follow-up (2026-05-18, user-directed):** overall bar driven by BIO's real "N/M" aggregate (no longer stuck at 45%), the **per-mod progress bar restored** (advances individually per mod — a core requirement), the scroll area height-bounded so the Cancel footer stays on-screen; `stage_downloading.rs` only, zero BIO. **THE AMENDMENT (2026-05-18, late — Phase-7 fix-arc Run 1, "Clean base"):** upstream BIO fix `a38e360` (merged `8df994a`) removed WeiDU's "`-u` log folder cannot contain spaces" preflight from `state_validation_paths.rs` — the sole reason for the prior Fix-A relocation — so **T-A is reverted**: the `-u` `weidu_component_logs` dir is per-install **inside the destination** again (`<dest>/weidu_component_logs`; original SPEC §13.12 #2 / §13.12a; the WeiDU-log SOURCE dirs were always there and are unchanged), `per_install_dirs::{resolve,derive_per_install_dirs}` are back to the pre-Fix-A 3-arg form (the id-threading is unwound; `register_install_modlist_paste` mints its own id again — registered entry byte-identical), and **`registry::store_workspace::modlist_data_dir` is kept** as the canonical `workspace.json` parent (only its `-u` routing was removed). **T-B is dropped** (user decision): the appdata dir now holds only `workspace.json`, so the Home Kebab "Open data folder" item + its wiring + the now-dead `open_modlist_data_folder` helper are removed; **"Open install folder" is unchanged** on both card states. Zero BIO source; `BIO` build a true no-op; `cargo test --lib` 386/0. SPEC §13.12 #2/§13.12a + §3 Kebab tables, plan P7.T17, PENDING_VERIFICATION reversed. |
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
- **Home** is the real screen (Phase 5 Runs 1–2): title + subtitle, filter chips (Installed / In progress / All) with counts + default-selection logic, modlist cards (in-progress `resume` / installed `open` + Kebab), `add a modlist` CTAs, `game installs detected` block, first-launch setup CTA, bottom-center toasts. Kebab actions are live: Copy import code (clipboard + toast), Delete (danger confirm → registry entry + guarded on-disk folder removal), Open install folder, Reinstall (Phase-7 placeholder toast). **Rename ships visible but inert by design in Phase 5** — the registry-write rename mechanism + the Workspace ✎ inline rename land in Phase 6 (`operations_rename.rs`, SPEC §2.2). Recorded as an intentional staged deviation (SPEC §3.2 in-progress/installed Kebab tables + plan P5.T2 acceptance), not drift — do not re-flag.
- **Install Modlist** is wired (Phase 5 Runs 3–5): the paste stage (destination FolderInput + `DestinationNotEmptyWarning` with Clear/Backup/Continue + import-code textarea, capped-to-footer + internal scroll; **a valid destination — a real existing folder — is required before proceeding** (SPEC §4.1); the warning Box is legible in Light + Dark), the **Preview** stage (parsed `ModlistSharePreview` → packed name/author title+subline with honest fallback, Overview Box, 6 file-folder tabs, `allow_auto_install` draft-gate with disabled Import + `Open in Create →`, `⑂ fork info` → `ForkInfoPopup`), the **Downloading** stage as the §4.3 **chassis** (overall-progress Box + 4-col mod grid + Cancel/auto-advance, grid empty until Phase 7 binds live data), and the stage-4 stub render.
- **Create** is the real screen (Phase 6 Runs 3–4 + Fix-Run 2): the `choose` mode = Setup Box (modlist name + game + destination `FolderInput` + conditional `DestinationNotEmptyWarning`, Clear/Backup only) and — **Fix-Run 2 (user-directed deviation, SPEC §5.1/§5/§5.3)** — a `Choose one` header + **two selectable boxes** (whole-box click selects; no in-box CTAs) + a single bottom-right **`Start →`** (styled like the workspace `Next →`); the **game ComboBox shows only for the from-scratch box** (redesign chrome, EET default), the **import box derives the game from the pasted share code** (read-only note instead). `Start →` routes to the Workspace (scratch) or fork-paste. The fork sub-flow (paste → preview → download chassis; lineage append) + Load Draft dialog ship; **Fix-Run 2 wired the Load Draft Kebab `Delete`** to the shared Home delete machinery (danger confirm → `operations::delete_modlist`; SPEC §5.2 deviation). App-wide: every text input's sketchy border now hugs the outer box (a shared input primitive — the indented-input fix); affected glyphs (`→`, `✓`) render in `firacode_nerd` (the symbol-glyph rule). The workspace nav step-indicator is removed on all 4 steps (SPEC §2.2 deviation); Step 3 renders both hint lines + no count line (SPEC §7.1 amended); the left rail highlights `Create` inside a Workspace (SPEC §2.1).
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
./target/release/infinity_orchestrator -d     # dev mode (diagnostics export + extra logging; the Phase-3 "Seed test modlist" button was stub-only and is gone since Phase 5 replaced the Home stub — re-prep the seed registry per orchestrator-handoff "Test fixtures / runtime")
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
  - Step body — **Step 2 is an orchestrator-side C4 chrome wrapper** (`workspace_step2::render`, P6.T2c — net-new redesign chrome (`step2_tab_row` + `step2_search`): title, full-width `flex` search + Rescan (disabled pre-Phase-7 per §13.12a) / Cancel-while-scanning, redesign GameTabs + Select-via-Log + Updates + clickable compat/prompt pills + count + Kebab[Show/Hide Details · Clear All · Select Visible · Collapse All · Expand All · Jump to Selected], **no** "Restart App With Diagnostics", Details pane hidden-by-default per SPEC §6) that reuses **only** BIO's tree / details / compat / prompt / updates sub-renderers + the same `pub(crate)` toolbar-action helpers `render_controls` itself calls (read-only); BIO's `render_controls` / `render_tabs` / `render_header` / `page_step2` / `frame_step2` are **not** called (the 2026-05-16 SPEC-CONFLICT resolution — the wireframe's Step 2 is structurally different from BIO's and colour-only carve-out #6 cannot restructure `frame_step2`). **Step 3 is an orchestrator-side C4 chrome wrapper** (`workspace_step3::render`, P6.T2d — **shipped**): net-new redesign chrome (`step3_action_row` + `step3_tab_row`): the action-row count "_N_ components ready to install on _<tab>_ · across _M_ mods" (reuses the shared `pub` `workspace_step4::active_tab_items` resolver so the Step-3/Step-4 count never drifts; no Save button — Save is Step-4 only per §7.6), the **shared** redesign GameTabs (the one `widgets::game_tab` widget Steps 2/3/4 render identically; single-game skips the strip via `workspace_step4::is_dual_game`), the aggregate conflict/prompt clickable pills (open the compat/prompt popups via the same `pub(crate)` `toolbar_support_step3::open_toolbar_issue_popup` / `prompt_popup_step2::open_toolbar_prompt_popup` BIO uses), and redesign `Undo`/`Redo`/`Collapse All`/`Expand All` (the exact `pub(crate)` `toolbar_support_step3::{undo_active,redo_active,collapse_all_active,expand_all_active}` helpers `content_step3::render_toolbar` calls), **no** "Export diagnostics" / "Restart App With Diagnostics", **no** BIO heading/hint (the shell renders the per-step hint). It reuses **only** BIO's drag-reorder list body (`bio::ui::step3::list_step3::render`, `pub(crate)`, read-only) inside an orchestrator-owned hard-clipped fixed-size rect (the verbatim Step-2 `clipped_pane`) so the list never bleeds into the nav bar; the reused-seam prologue (`state_step3::normalize_active_tab` → `toolbar_support_step3::build_toolbar_summary` → set `step3.{bgee,bg2ee}_has_conflict` → list → `content_step2::render_compat_popup` + `prompt_popup_step2::render_prompt_popup`) matches `content_step3::render` (`content_step3.rs:224-261`) exactly. BIO's `page_step3`/`content_step3`/`render_toolbar`/`frame_step3` are **not** called (the legacy `BIO` binary still renders its own — unaffected). No `Step3Action` (H2) — returns `()`; the dirty-bit fingerprint over `wizard_state.step3.<tab>_items` detects reorder/collapse/undo for persistence (unchanged). Step 4 is an orchestrator-side renderer (per C4 — replaces BIO's `page_step4::render` to avoid the double Save button).
  - The 6 Step-2 background-thread receivers are drained every frame by `OrchestratorApp::poll_step2_channels` (the narrower-call mirror of `bio::app::app_update_cycle::poll_before_render`'s Step-2 portion — `poll_before_render` is monolithic and also requires Step-5 runtime args the orchestrator does not own pre-Phase-7). Without it the scan worker starts but never reports and Cancel never completes. A **dev-only** scan-folder affordance (behind `-d`; absent in normal mode; enabled on `dev_mode` only) lets a developer point BIO's scan at an arbitrary folder, because pre-Phase-7 there is no per-install extracted-mods folder to scan (SPEC §13.12a) — it is the **functional scan path until P7.T17**. The **production "Rescan Mods Folder" button is inert (disabled + explanatory tooltip) pre-Phase-7** for the same reason (no valid per-install extracted-mods target until P7.T17) — the same accepted §13.12a Phase-7 deferral pattern as the §4.3 Downloading chassis; it lights up automatically when P7.T17 derives the per-install Mods folder. While a scan runs, the Rescan button is replaced in place by **"Cancel Scan"** (`Step2Action::CancelScan`) — a deliberate wireframe-omission addition (a necessary capability; recorded). **Rescan is non-destructive (SPEC §6.3): it reconciles, never wipes** — net-new orchestrator logic (`step2_rescan_reconcile`, BIO has no reusable rescan-preserves-selection mechanism) snapshots the selection at scan-trigger time and, on scan-**completion** (after `poll_step2_channels` lands the fresh mods; only on a *successful* `Finished`), re-applies it onto the freshly-scanned mod list (matched by `tp2`+component id, `selected_order` preserved), drops only mods/components no longer present + surfaces _"N component(s) dropped — M mod(s) no longer present"_ in the scan-status footer (no dialog). Step 3 is re-synced from Step 2 on the Step2→Step3 forward nav edge by mirroring BIO's exact `decide_next_action`→`sync_step3_from_step2` trigger (no BIO edit; Step-3 reorder preserved via BIO's own `reconcile_step3_items`). **Select-via-WeiDU-Log is destructive (it replaces every selection on the tab) and is gated by the SPEC §6.10 danger `ConfirmDialog`** (the shared widget Home Delete/Reinstall reuse — wireframe `askWeiduImport` copy verbatim): clicking the tab-row button arms `WorkspaceStep2State::pending_weidu_log_confirm` (target tab); `workspace_step2::render` shows the danger confirm; only on **Confirm** does the `step2_log_glue` picker+apply path run; **either cancel point — the dialog *or* the file picker — is a no-op that preserves the current selection** (the picker no longer falls back to the resolved default log; that BIO-legacy parity silently wiped selections). On **Confirm** the picker+apply path additionally **discards the imported tab's stale Step-3 order** (`9b5b9d5`) so the next Step2→Step3 sync rebuilds it fresh in weidu.log order — cold-resume pre-populates `state.step3.<tab>_items`, and without this reset BIO's `reconcile_step3_items` would preserve the stale resumed order on a destructive whole-tab re-import; only the imported tab is cleared, and BIO's incremental-edit preserve behavior is deliberately untouched.
  - Workspace nav bar: `← Previous` / `Next →`. On the **first** workspace step (Step 2) `← Previous` routes back to **Home** (SPEC §2.2 — affordance-forward: the user reached the workspace via a Home `resume`/`open`; recorded, overview 2026-05-16, deviation from the wireframe's former first-step *disabled* state). `← Previous` is enabled on the first step and force-disabled **only** by `disable_prev` (the Phase-7 install-running / post-install gate; `false` until Phase 7).
- Workspace state loader: populates `WizardState` from per-modlist `workspace.json` on open; extracts back on save / nav-away / debounced write. **Loader is never invoked while an install is running** (per C5 — rail-nav lock).
- `sync_paths_from_settings` re-asserts `Step1Settings` paths into `wizard_state.step1` **once on workspace open** (not per frame): the orchestrator's Settings → Paths tab edits the same in-memory `wizard_state.step1` the workspace renders from, so edits propagate by construction without a close/reopen (M2 — open-only; overview 2026-05-16).
- Per-frame dirty bit gates persistence writes (per H1) — no per-frame extract+compare.
- Step action dispatch tables in the phase doc enumerate every `Step2Action` / `Step4Action` variant + which `bio::app::*` public function handles it (per M4).

**Dependencies:** Phases 2 + 3 + 4 + 5.

### Phase 7 — Step 5 install runtime + Reinstall + import-code auto-write + install concurrency + rail-nav lock

**Ships:**
- Step 5 inside the workspace renders BIO's existing `page_step5::render` (the full embedded panel: Command card, Summary card, Actions/Diagnostics menus, Prompt Answers, console box wrapping `EmbeddedTerminal`, prompt input row). New chrome wraps **around** it — success banner row above, post-install action row above-and-adjacent to the (now disabled) Install button (per SPEC §9.2 + H9).
- Install start hook: writes `modlist-import-code.txt` to the install destination before WeiDU runs (per SPEC §13.13). Write semantics per button variant: `Install` / `Restart Install` / `Reinstall` overwrite; `Resume Install` does not (per H10).
- Post-install state transition: clean exit (the C3 triple: `install_running == false && last_exit_code == Some(0) && last_install_failed == false`) flips the registry entry from `in-progress` to `installed`, regenerates `latest_share_code` with `allow_auto_install = true`, **and rewrites the on-disk `<destination>/modlist-import-code.txt` with that same `allow_auto_install = true` code** (SPEC §13.13 — the on-disk artifact becomes the verified, directly-installable code post-success, matching `latest_share_code`; reuses `import_code_writer`'s path/filename; non-fatal/defensive; **H8: import-code file only — never a registry snapshot to disk**). A non-clean exit leaves the install-start `allow_auto_install = false` draft on disk untouched (the rewrite is exclusively inside the C3-gated `flip_to_installed`). Async size computation on a worker thread (per M5).
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
- **Directory architecture + content-addressed archives (SPEC §13.12a).** Global, Settings-defined (§11.2): **Mods archive** (ALL downloads for ALL modlists, always), **Mods backup**, **Game sources**. **Every per-install directory lives inside the modlist's destination** (no exception): the **Mods extract/stage/scan folder** (removed on clean success), the `-u` **`weidu_component_logs/`** dir (SPEC §13.12 #2), the **WeiDU-log SOURCE folders** (`<dest>/weidu_log_source/{bgee,bg2ee}`), + the **game-install clone dirs** (already specced by §13.12 #3/#4 — always cloned, fixed names; the redesign never surfaces BIO's no-clone path, BIO untouched). `per_install_dirs::resolve(destination, game)` / `derive_per_install_dirs(step1, destination, game)` are the 3-arg pre-id-threading form; they take no `modlist_id` and have no `_with_data_dir` split (tests stay DATA-LOSS-safe by being temp-path / in-memory only). Upstream BIO fix **`a38e360`** ("Allow spaces in per-component WeiDU log folder", merged into `overhaul/infinity_orchestrator` as **`8df994a`**) removed WeiDU's `-u` component-log-folder no-space preflight from `src/core/app/state/state_validation_paths.rs`, so there is no longer any constraint pushing the `-u` dir out of the user's free-form destination — it sits beside the other per-install dirs. (`registry::store_workspace::modlist_data_dir` remains the single canonical **`workspace.json`**-parent resolver; it no longer anchors any `-u`/Kebab path.) The global Mods-archive is **content-addressed**: hash-on-write, same name+hash ⇒ cross-modlist dedupe, same name/different hash ⇒ both coexist, the modlist lock records its hash, extract selects the matching archive per install. This is a **net-new orchestrator staging layer wrapping** `app_step2_update_download`/`app_step2_update_extract` (zero BIO modification). The orchestrator drives BIO's `import_modlist_share_code` → saved-log/auto-build pipeline; global paths reach the owned `WizardState` via `sync_paths_from_settings` (the Install screen does NOT collect game paths). Lands in **Phase 7 P7.T17** (the pipeline terminates in the install runtime); Phase 5 shipped the §4.3 chassis only. Rationale: overview.md 2026-05-16 + 2026-05-18 (THE AMENDMENT) revision log.
- **Per-install WeiDU-log source folders are part of the §13.12a derived set (settled — the real "Install-Modlist-paste download never starts / inert 0/0" root cause).** `per_install_dirs::derive_per_install_dirs` derives, inside the destination, **two** per-install WeiDU-log source phase folders (`<dest>/weidu_log_source/{bgee,bg2ee}` — distinct so an EET import's BGEE-phase + BG2EE-phase logs never collide) and sets the six `Step1State` log fields (`<game>_log_folder` / `<game>_log_file` / the `eet_*` pair) **before** `import_modlist_share_code`, so BIO's importer write target (`modlist_share::import_log_target_path`) **and** the saved-log/auto-build applier read path (`app_step2_log::resolve_*_weidu_log_path`) resolve to the same per-install file in every install mode the imported payload can carry. Previously these stayed `Step1State::default()` empties ⇒ the importer `Err`'d ⇒ the whole Install-Modlist-paste / Reinstall pipeline never armed (permanent inert "0 / 0 mods"). The fields survive the import's `clone` + `reset_workflow_keep_step1`; zero BIO edit (every field is a pre-existing `pub` field BIO's own importer/applier read). A non-masking arm-failure banner (`InstallScreenState::pipeline_arm_error`, surfaced in `stage_downloading::render_chrome`) now makes any residual prep/import failure diagnosable instead of a silent inert screen (the one-shot latch is kept — no per-frame re-import churn). SPEC §13.12a (per-install set + Pipeline-reuse contract) + plan P7.T17 carry the full record. This **supersedes** the prior FIX-1 (`arm_download_archive_policy`) as the actual root cause for that symptom — FIX-1 is a necessary downstream precondition (kept as-is), not the fix.
- **`ForkInfoPopup` collapse-chevron is a Phase-8 deferral, not missing scope.** SPEC §10.9 says the popup carries the global §10 collapse chevron. Like every redesign popup that gets one, the chevron is the Phase 8 carve-out #2 `.collapsible(true)` flip / `popup_collapse_anchor.rs` work — `fork_info_popup.rs` ships before Phase 8 with `.collapsible(false)` **by design**, bit-for-bit consistent with its sibling popups (`confirm_dialog.rs`). Recorded in SPEC §10.9 + here so a §10.9-vs-code reviewer does not re-flag it.
- **Lessons — phase-local reasoning is a real failure mode.** Run 5's implementer concluded "the Install screen has no game paths" because its brief was phase-local (phase-05 + the BIO download engine) and never stated that Phase-4 Settings → Paths supplies them globally + `sync_paths_from_settings` feeds the owned `WizardState`. Fix operationalized in `.claude/orchestrator-handoff.md`: every implementer brief now carries a standing **"Already built / cross-phase context"** block, and on verify the orchestrator sanity-checks an escalation's *premises* against the HANDOFF status table — not just its novel technical claim.
- **Cold-resume Step-2 restore (Run 2b — the #1 fix, settled).** `workspace_state_loader::populate` only *marks* the persisted `order_<tab>` onto the **currently-scanned** mod set; on a cold resume that set is empty so nothing matched (the Run-1 MISS vs P6.T1). Restore is `src/ui/workspace/step2/step2_resume_scan.rs`: on workspace open it re-points `wizard_state.step1.mods_folder` to the workspace's recorded `ModlistWorkspaceState.dev_scanned_mods_folder` and re-runs **BIO's own scan** via the existing `Step2Action::StartScan` dispatch. BIO's scan worker persists its scan cache (`save_scan_cache`); a per-tp2 `cache_get` hit skips WeiDU (`bio::app::step2::scan`, read-only), so the resume scan is WeiDU-free when the cache is fresh + tp2 files unchanged — BIO's `to_mod_states` rebuilds the full set, zero reimplementation. On the async scan-completion edge `step2_rescan_reconcile::reconcile_on_scan_complete` re-applies a snapshot built from the persisted order (preserving `selected_order`) and, for the resume case (`workspace_view.step2.resume_pending`), rebuilds Step 3 via BIO's `step3_sync::build_step3_items`. The recorded folder is written synchronously by the dev-scan trigger **before** `StartScan` (the §13.13 survive-a-crash rationale). Production records nothing (no per-install mods folder until Phase-7 P7.T17 — §13.12a), so production resume legitimately finds nothing — that is the §13.12a deferral, **not** a bug. Do not re-flag.
- **Step-4 WeiDU-line is 3 hues (RESOLVED — overview 2026-05-16; settled, do not re-litigate).** `weidu_line.rs::build_weidu_job` renders the canonical line in **three high-contrast hues per SPEC §6.7**: TP2 path amber `#d4a35c` (literal), `#0 #id` dark-blue `#2f6fb7` (literal), comment success-green `redesign_success` (theme-aware). The earlier plan-pinned neutral-grey mapping (which read 2-tone) was the drift; the user confirmed SPEC §6.7 / the wireframe `WeiduLine` as authoritative all along. `weidu_line.rs` is the correct 3-hue renderer (its `three_colour_split_matches_wireframe_hues` test asserts the literal hexes). **Scope:** SPEC §6.7 is explicitly "WeiDU log syntax coloring (**Step 3 / Step 4 only**)" + the inline Step-2 leaf encoding — it does **not** extend to the Install/fork **preview** weidu-log tabs, which the canonical wireframe `PreviewText` (`screens.jsx:512-529`) renders as a single flat `var(--text)` monospace block and SPEC §4.2 specifies only as "monospace". (This bears on Fix-Run-2 item #7b — see the run report's `SPEC CONFLICT`.)
- **OPEN PLAN GAP — per-component prompt-popup vertical growth (Fix-Run-2 #4a, NOT fixed — awaiting a directive decision).** BIO's `src/ui/step2/prompt/prompt_popup_step2.rs` `render_prompt_popup` (`PromptPopupMode::Text` branch) does `ui.set_min_size(ui.available_size())` inside its `egui::Window` — a classic egui sizing feedback loop: content min = window available ⇒ window grows ⇒ available grows ⇒ … (visibly grows as the pointer moves, egui repainting each frame). The orchestrator reuses this BIO popup verbatim (the `WizardApp::render_shared_popups` path); the loop is **entirely inside BIO's `egui::Window` closure** and independent of the C4 wrapper's rect/clip (the `ui` the wrapper passes does not affect the window's internal `available_size()`). The root cause is in **protected BIO source**; no carve-out covers a prompt-popup sizing fix; the CRITICAL DIRECTIVE forbids editing it. Per the brief's hard constraint ("ZERO BIO source edits … if a BIO edit seems needed → STOP, emit `PLAN GAP`") this was **not** fixed and is **not** worked around with a fragile orchestrator-side egui-memory clamp (non-root, fights BIO's `resizable(true)`, contrary to the brief's "root-cause + fix" intent). Awaiting a user/directive decision (e.g. authorize a 1-line BIO carve-out: `set_min_size` → a fixed/bounded size; or a sanctioned orchestrator clamp). The aggregate (toolbar) prompt popup has the same line but was explicitly out of #4a scope. See the Fix-Run-2 report `PLAN GAP`.

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
- **C4 — orchestrator-side step renderers (Steps 2 / 3 / 4).** The wireframe's Step-2/3/4 chrome is structurally different from BIO's, and colour-only carve-out #6 cannot restructure BIO's `frame_step2` / `content_step3` / `content_step4` toolbars (the CRITICAL DIRECTIVE forbids editing them). Resolution: Phase 6 ships net-new orchestrator-side renderers that rebuild the wireframe chrome and reuse **only** BIO's heavy interaction sub-renderers read-only — **Step 4** (`workspace_step4`, also resolves the original "double Save button": BIO's `content_step4` paints its own `Save weidu.log's`, so the orchestrator must not call it), **Step 2** (`workspace_step2` — net-new chrome, reuses BIO's tree/details/popups), and **Step 3 — SHIPPED** (`src/ui/workspace/step3/`, P6.T2d: net-new action-row count + shared redesign GameTabs + aggregate conflict/prompt pills + redesign Undo/Redo/Collapse/Expand, **no** Export-diagnostics, **no** BIO heading; reuses BIO's drag-reorder list body `list_step3::render` + the `pub(crate)` `toolbar_support_step3` / `prompt_popup_step2` helpers `content_step3::render_toolbar` itself calls, all read-only — no behavior change). For each, BIO's `page_step{2,3,4}` / `frame_step{2,3}` / `content_step3` / `render_toolbar` are **not** called by the workspace step router; the legacy `BIO` binary still renders its own (unaffected). Step-3 returns `()` (no action enum, H2 — the dirty-bit fingerprint over `wizard_state.step3.<tab>_items` detects reorder/collapse/undo). The earlier premature Step-3 doc-cascade (retracted, overview 2026-05-17 §3) is now superseded by this **shipped** implementation + the real cascade landed with it (SPEC §7.1 / phase-06 P6.T2d / phase-08 P8.T5).
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
