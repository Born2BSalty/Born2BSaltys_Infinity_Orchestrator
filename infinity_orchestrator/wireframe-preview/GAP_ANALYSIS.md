# Wireframe ↔ App Gap Analysis — Create Flow

**Scope:** Bring the wireframe's "Create" section (New modlist + Fork existing modlist) to feature parity with the real BIO app (steps 2–5). Functionality only — styling is intentionally out of scope until parity is achieved.

**Status:** Tracking document. Update the checkboxes as features land. Do not redesign anything in this document; that's a follow-up phase.

---

## How to use this doc

- **Source of truth for behavior:** the Rust app in `src/core/app/` and `src/ui/step{1,2,3,4,5}/`. Explore those directories directly for module orientation.
- **Wireframe to edit:** `wireframe-preview/screens.jsx` (loose JSX). The bundled file at the repo root (`Infinity Orchestrator Wireframe _standalone.html`) was extracted into this directory as loose `app.jsx` / `screens.jsx` / `tweaks-panel.jsx` + an `index.html` that loads them via Babel-in-the-browser.
- **Preview server:** `cd wireframe-preview && python3 -m http.server 4310` → http://localhost:4310/. Refresh after edits.
- **Do not change styling.** Keep Patrick Hand / Caveat / JetBrains Mono fonts, dotted parchment bg, dusty amber accent, sketchy borders with `box-shadow: 6px 6px 0`, diagonal-stripe placeholders. Only add functionality.
- **Each gap below has a checkbox.** Tick it (`[x]`) when implemented in `screens.jsx`. If a gap is implemented but stubbed (mock data only), tick it but add `(stub)` in the line.
- **When you finish a chunk**, leave a short note under the section heading: what shipped, what was deferred.

---

## Current wireframe inventory (what's already there)

**Create entry (`CreateScreen` in `screens.jsx`):** Two-card chooser — "from local install" and "from existing modlist" — both routes drop into the same `WorkspaceView` with only a subtitle diff. No behavioral fork.

**`WorkspaceView`:** Sub-wizard with 4 tabs (Step 2 / 3 / 4 / 5), `save draft` button, title "Create / edit modlist". Tabs are simple primary-toggle `Btn` elements; the `WorkspaceTabs` component defined nearby is currently unused dead code.

| Tab | Component | What it renders today |
|-----|-----------|----------------------|
| Step 2 | `SourcesPanel` | Title, search, 6 toolbar buttons (Scan / Clear All / Select Visible / Collapse All / Expand All / Jump to Selected), BGEE/BG2EE tabs, 2 hardcoded issue pills, two-pane grid: `ComponentTree` (left) + `DetailsPanel` (right). |
| Step 3 | `ComponentsPanel` | Title, right-click hint, BGEE/BG2EE tabs, issue pill, scrollable mod list with mock TP2 lines. |
| Step 4 | `OrderPanel` | Title, "Save weidu.log's" button, Install Order box with BGEE/BG2EE tabs + ordered TP2 lines. |
| Step 5 | `FinalPlanPanel` | Title, "Dev Mode: RUST_LOG=DEBUG" label, Command/Summary split, single-row install/actions/diagnostics/prompt-answers buttons + 3 filter checkboxes + Auto-scroll, Console box. |

---

## A. Fork vs Scratch — behavioral fork (priority 1)

The two entry cards currently behave identically. Fork must feel like an import.

> **Status (2026-05-10):** A1/A2/A3/A4 shipped as a state machine inside `CreateScreen`: `choose → fork-paste → fork-preview → fork-download → fork-workspace`. `ForkBanner` renders inside `SourcesPanel` when `fork` prop is set. `WorkspaceView` now accepts a `fork` prop and threads it through. Both download progress and banner data are stubbed via the `FORK_MOD_LIST` / `FORK_META` constants near the top of the Fork sub-flow section. A5 is partially landed (subtitle propagation + banner; tree pre-selection visualization deferred to chunk B since the tree itself doesn't yet support a "preselected from fork" indicator).

- [x] **A1. Import preview screen** between the entry card and the workspace. Fields: share-code paste box OR draft picker · preview (modlist name, author, mod count, target game, components list, source list) · "Begin import" button.
- [x] **A2. Download phase screen** between preview and workspace. Per-mod progress (fetching archive · extracting · staged), success/fail tally, overall progress bar. Should resemble the existing Install screen's import path.
- [x] **A3. Pre-populated landing on Step 2.** All components from the share code already checked, install order already set, banner reads: *"Forked from {modlist name} · 47 components preselected · review and adjust before installing"*. (banner shipped; the "already checked / order applied" visual depends on chunk B tree affordances)
- [x] **A4. From Step 2 onward Fork and Scratch behave identically.** No further branch — same toolbar, same details pane, same tabs.
- [x] **A5. `WorkspaceView` state propagation.** The `source` prop today only tweaks the subtitle. It should branch initial state: scratch starts empty, fork pre-fills tree selections + order. (stub — fork prop threaded; deeper pre-selection visualization waits for B6)

---

## B. Step 2 — Scan & Select / Sources

App source-of-truth: `src/ui/step2/`, `src/core/app/state/state_step2.rs`, `src/core/app/step2/scan/`.

- [ ] **B1. In-flight scan UI.** While scanning: `Cancel Scan` button visible next to `Scan Mods Folder`, plus a progress line *"Scanning... 247 / 1,043 — Tactics21"*.
- [ ] **B2. Draggable splitter** between the tree pane and the details pane. (App ships one; default ~55/45.)
- [ ] **B3. Footer status line** under the panes: *"Scan complete · 47 mods · 1,302 components · 23 selected"* or scan progress text. Lives in `Step2State.scan_status`.
- [ ] **B4. Diagnostics button on the title row.** Dev mode: `Export diagnostics`. Normal mode: `Restart App With Diagnostics`.
- [ ] **B5. Details pane completeness.**
  - [ ] B5a. **Selection grid** rows: Component, ID, Checked, State, Compat (clickable pill that opens compat popup), Language, TP2 File, Shown / Hidden / Raw counts.
  - [ ] B5b. **Paths/Links grid** rows: TP2 Folder, TP2 Path, INI Path, Readme, Web URL — each row has small `C` (copy) and `O` (open) action buttons.
  - [ ] B5c. **Package grid** rows (when mod has download metadata): Installed Source, Update Source with "(selected)" or "(default)" suffix, Latest Version, URL, GitHub.
  - [ ] B5d. **Package action buttons** (conditional): `Add Source` + `Reload Sources` when no source set; `Check This Mod` + `Lock Updates` / `Unlock Updates` when source set.
  - [ ] B5e. **Collapsible "Component Block"** — raw TP2 block in monospace, default open, Copy button.
  - [ ] B5f. **Collapsible "WeiDU Line"** — raw line in monospace, default closed, Copy button.
- [ ] **B6. Tree row affordances.**
  - [ ] B6a. Selection-order badge (e.g. `#03`) shown only on checked components.
  - [ ] B6b. Inline compat icon per row.
  - [ ] B6c. Inline prompt icon per row.
  - [ ] B6d. Language tag.
  - [ ] B6e. Expand/collapse caret per mod row.
  - [ ] B6f. Search-match highlighting.
- [ ] **B7. Issue-pill semantics.** Encode kind: REQ_MISSING / FORBID_HIT / GAME_MISMATCH / CONDITIONAL / ORDER_WARN / DEPRECATED. Red for blocking, amber for warnings. Counts shown.
- [ ] **B8. Compat issue window** — opens when an issue pill is clicked. Kind heading + multi-line narrative + filter chips at top ("All · Conflicts · Missing deps · Game mismatch · Conditional · Deprecated") + list of affected components with "Jump to" links.
- [ ] **B9. Prompts window** — title, monospace prompt-summary body, `Copy` + `Close` buttons.
- [ ] **B10. Updates window** — table columns (Mod · Repo · Branch · Updated · Status), per-row `Edit Source` / `Open Source` / `Discover Forks`, footer `Copy Report` / `Close`.
- [ ] **B11. EET-mode tab gating.** BGEE/BG2EE tabs only appear in EET mode. In BGEE-only or BG2EE-only modes, render a single static label instead.

---

## C. Step 3 — Reorder & Resolve / Components

App source-of-truth: `src/ui/step3/`, `src/core/app/state/state_step3.rs`.

- [ ] **C1. Toolbar, right-aligned:** `Undo`, `Redo`, `Collapse All`, `Expand All`, `Export diagnostics` (dev) / `Restart App With Diagnostics` (normal).
- [ ] **C2. Tab-level prompt badge** alongside the issue badge (count of components with prompts in the active tab).
- [ ] **C3. Drag affordances per row.**
  - [ ] C3a. Drag handle (`≡`) on the left.
  - [ ] C3b. Order number (`01.`, `02.`, …).
  - [ ] C3c. Insert marker (horizontal accent line) preview during drag.
  - [ ] C3d. Multi-select drag indicator (shift-click anchor visualization).
- [ ] **C4. Block grouping.** Parent component visually groups its children with a collapse caret. Children indented under parent. Parent has the `🔗` glyph already shown in the mock — make it real.
- [ ] **C5. Per-block lock icon** (toggleable; lock prevents reorder of that block).
- [ ] **C6. Per-row inline pills.**
  - [ ] C6a. Compat-issue pill (clickable → opens the same compat popup as Step 2).
  - [ ] C6b. Prompt indicator.
- [ ] **C7. Right-click context menu** with 5 items: `Clone Parent (empty split target)`, `Uncheck in Step 2`, `Set @wlb-inputs...`, `Edit Prompt JSON...`, `Clear Prompt Data`.
- [ ] **C8. Prompt edit popup** (from "Edit Prompt JSON..." or "Set @wlb-inputs..."): mode-aware editor — JSON area for the JSON variant, answer textarea for the @wlb-inputs variant — plus status text + Save / Cancel buttons.
- [ ] **C9. Compat-rules-load-failed banner** at the top of the step when rule loading fails (amber).

---

## D. Step 4 — Review / Order

App source-of-truth: `src/ui/step4/`.

- [ ] **D1. Diagnostics button on title row.** Dev: `Export diagnostics`. Normal: `Restart App With Diagnostics`.
- [ ] **D2. Exact-Log mode variant.** Bootstrapping from an existing weidu.log changes the body:
  - [ ] D2a. `Check Mod List` button at top.
  - [ ] D2b. "Source WeiDU Logs" section instead of "Install Order".
  - [ ] D2c. Ready/not-ready headline (green if ready, red otherwise).
  - [ ] D2d. Tally rows: downloadable missing, manual sources, no-source entries, source-check failed, exact-version-fallback pending.
  - [ ] D2e. Source file path + size + modified date + colored log content (path orange, IDs another color, comments green).
- [ ] **D3. Single-game tab variant.** Outside EET, tabs collapse to a static label.

---

## E. Step 5 — Install

App source-of-truth: `src/ui/step5/`, `src/core/app/state/state_step5.rs`, `src/core/app/step5/`.

- [ ] **E1. Command card — `Copy Command` button.**
- [ ] **E2. Top context cards collapse after install starts** (annotation now, behavior later).
- [ ] **E3. Install button states.** Mutually exclusive:
  - [ ] E3a. `Install` (idle).
  - [ ] E3b. `Restart Install` (after one run).
  - [ ] E3c. `Resume Install` (after cancel/failure with resume available).
  - [ ] E3d. `Cancel Install` (during run, destructive style).
  - [ ] E3e. Pre-run *"Preparing target dirs..."* accent-colored status text.
- [ ] **E4. Confirm Cancel modal.** "Cancel active install?" + `Force cancel (emergency)` checkbox + two helper texts (safe vs warning) + Yes/No.
- [ ] **E5. Actions dropdown** (currently a flat button): `Copy Console` · `Save Console Log` · `Open Logs Folder` · `Clear Console` · `Open last log file` (last item only after failed run).
- [ ] **E6. Diagnostics dropdown.**
  - [ ] E6a. Dev mode: `RUST_LOG Off` / `RUST_LOG=DEBUG` / `RUST_LOG=TRACE` selectable chips + `Export diagnostics`.
  - [ ] E6b. Non-dev mode: flat `Restart App With Diagnostics` button.
- [ ] **E7. Export Modlist button** (only after successful install) → popup with monospace share-code textarea + `Copy` + `Close`.
- [ ] **E8. Prompt Answers window.** "Saved Prompt Answers" title + `Import JSON` / `Export JSON` buttons + separator + table (mod · component · prompt key · saved answer · timestamp · edit/delete).
- [ ] **E9. Console filter behavior.** General / Important Only / Installed Only are mutually exclusive (radio). Auto-scroll independent.
- [ ] **E10. Status row.** Phase pill ("Installing" / "Waiting for input" / "Done" / "Cancelled") + runtime counter + error copy link.
- [ ] **E11. Input row.** Text field with "Type to send to install..." placeholder + Send button, below the console (for manual prompt response).
- [ ] **E12. Prompt-required sound indicator.** Annotation showing the cue fires; underlying toggle lives in Settings → Advanced.

---

## F. Cross-cutting

- [ ] **F1. Step gating in the tab nav.** Step 3 dimmed until Step 2 has selections, Step 4 until Step 3 conflict-free, Step 5 until Step 4 saved. Locked tabs show a small lock icon.
- [ ] **F2. Compat severity colors used consistently.** Red = blocking (conflict / missing dep / deprecated). Amber = warning (game mismatch / conditional / order warn). Same palette across tree pills, details compat row, Step 3 row indicators, popup chips.
- [ ] **F3. Save Draft semantics.** The `save draft` button in `WorkspaceView` is decorative today. Decide: keep as stub, wire to local state, or remove entirely.

---

## Recommended implementation order

1. **A** — Fork vs Scratch differentiation (highest-level intent gap).
2. **B** — Step 2 details + 3 popups (largest single chunk).
3. **C** — Step 3 drag affordances + context menu + prompt edit popup.
4. **E** — Step 5 button states + cancel modal + dropdowns + status/input rows + Export Modlist + Prompt Answers window.
5. **D** — Step 4 exact-log variant.
6. **F** — Step gating and color tightening last.

Within each chunk, tackle items in declared order. If a feature requires invented data that doesn't exist in the wireframe yet (e.g. mock fork-import payload), put it inline in `screens.jsx` near the consumer — no shared data file until the design stabilizes.

---

## Anti-goals (for now)

- Don't add features the app doesn't have (no new business logic).
- Don't change colors, fonts, spacing, border styles, or any other visual token without explicit approval.
- Don't rewrite `WorkspaceView` into a different navigation pattern (e.g. left-rail-inside-Create). Keep the existing 4-tab top nav.
- Don't ship a real bundled standalone HTML yet — work in the loose-file `wireframe-preview/` setup until feature parity is reached.
- Don't pull in new dependencies. React 18 + Babel-in-browser + plain JSX, same as today.
