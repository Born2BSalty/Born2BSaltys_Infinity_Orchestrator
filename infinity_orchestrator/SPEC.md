# Infinity Orchestrator — Product Specification

**Status:** Draft (derived from the wireframe at `wireframe-preview/` and the current `bio` crate)
**Last updated:** 2026-05-12

This spec describes a **redesign of BIO** (Born2BSalty's Infinity Orchestrator) — the existing Rust desktop app for installing WeiDU mods. It is *not* a brief for a new app. The codebase, install pipeline, WeiDU integration, mod-source manifests, compat rules engine, prompt eval, share-code format, GitHub OAuth, CLI subcommands, and diagnostics bundle all stay. This spec describes **what changes**: a new wireframe-driven information architecture, new visual language, a multi-modlist registry on top of today's single-workspace model, and a handful of behavioral additions called out per section.

The wireframe at `wireframe-preview/build.html` is the canonical reference for **how the redesign looks and feels**. This document is the canonical reference for **how it differs from today's BIO**.

> **Required local artifact.** Both implementation planning and implementation itself assume the wireframe is **available locally in this repo** at `wireframe-preview/`. Planners must open `wireframe-preview/build.html` (the built single-file preview) in a browser while reading this spec — visual details (exact colors, spacing, copy strings, hover states, layout proportions) are sourced from the wireframe, not duplicated in prose. The source files used to build the preview live alongside it: `wireframe-preview/screens.jsx` (every screen + popup component), `wireframe-preview/app.jsx` (top-level shell + routing), `wireframe-preview/index.html` (CSS tokens). When the spec and the wireframe disagree, the wireframe wins (see fidelity rule below).

**Implementation target.** The app is the existing `bio` Rust crate (binary name `BIO`), built on **`eframe` / `egui`**. This spec is a UI brief for that target, not a web-app spec. The wireframe is implemented as an HTML/React preview purely because it's faster to iterate on visuals there — every layout, every widget, every interaction described here translates to an egui widget tree backed by the existing crate's state machines (`src/core/app/state/state_step*.rs`, etc.).

Where the spec references concrete values pulled from the wireframe (color hex codes, pixel sizes, padding, font names), treat them as **design tokens** to wire into `src/ui/shared/theme_global.rs` / `typography_global.rs` / `layout_tokens_global.rs`. Where the spec uses descriptors like "modal", "dropdown", "tab row", "drag handle", "hover affordance", "list", "row" — those are the egui equivalents (`egui::Window`, `egui::ComboBox`, custom tab strip, drag-and-drop already implemented in `state_drag_step3.rs`, hover-on-response, `egui::ScrollArea`, etc.). The wireframe never adds a widget that has no egui counterpart.

HTML/CSS/JSX terminology in this doc (e.g., `gridTemplateColumns`, `display: flex`, `var(--shell-bg)`, `<select>`, `onClick`) is **wireframe shorthand** for the visual or behavioral intent. The implementer reads them and produces the equivalent egui layout (a `Grid`, a horizontal/vertical layout, a theme color from `theme_global.rs`, a `ComboBox`, a `clicked()` response branch). Component names in PascalCase from the wireframe (`Btn`, `Box`, `Pill`, `Kebab`, `ScreenTitle`, `DetailsRow`, `SubFlowFooter`, etc.) are **wireframe component names**; the egui implementation may use small helpers under different names, but each maps to one widget pattern.

**Fidelity rule.** The wireframe defines the visual design and interaction patterns *exactly*. Implementations must match:

- **Layout** — section order, paddings, gaps, header structure, two-column splits, tab placement, button placement, etc.
- **Theme tokens** — every color, every spacing constant, every typography token in the wireframe maps to the egui theme. The token *names* (e.g., `shell-bg`, `border-strong`) are descriptive; ship them as Rust constants in the `shared/` modules.
- **Fonts** — Poppins for UI text, FiraCode Nerd for monospace + the PUA icon-glyph range. Embed via egui's font definition system; the existing crate already bundles Nerd Font this way.
- **Copy** — UI strings (button labels, hints, placeholders, pill text, dialog titles, tooltip text, toast text) match the wireframe verbatim, including casing.
- **Controls** — the toggles, dropdowns, segmented buttons, tabs, Kebabs, and modals available on each screen are exactly the set the wireframe shows. Nothing added, nothing removed (except as called out in Appendix A).
- **Interaction model** — click targets, hover affordances, selection highlights, drag-and-drop rules, popup triggers, confirmation behavior all match the wireframe.

When the spec and the wireframe disagree on UX, the wireframe wins. When the spec and today's `bio` crate disagree on feature scope, the spec wins. Features in today's `bio` that are not represented in the wireframe must be either folded into the new design or surfaced in [Appendix A](#appendix-a--bio-features-not-yet-in-the-wireframe) so the team can decide whether to keep, drop, or redesign them. No feature is silently dropped; no new behavior is invented in this spec beyond what the wireframe or today's BIO already does.

> **CRITICAL DIRECTIVE — Do not modify existing BIO components.**
>
> The Infinity Orchestrator plan and implementation **must not delete, rewrite, or functionally modify any existing component in the `bio` crate**. For every redesign surface, the implementation has exactly two legal options:
> 1. **Reuse the existing BIO component as-is** (with theme-token-driven styling applied — see the BIO-reuse rule below). Behavior, data plumbing, signatures, and call sites stay identical to today's BIO.
> 2. **Create a net-new component** that lives alongside (not on top of) the existing BIO code. New components may compose existing BIO components, but they may not patch or replace them.
>
> **The only modification allowed to existing BIO code is a mild refactor.** Six flavors qualify:
>
> 1. **Theme-token extraction** — when style values are baked inline (e.g., hard-coded `egui::Color32` literals, inline padding values), extract them into theme tokens (`src/ui/shared/theme_global.rs`, `layout_tokens_global.rs`) and replace the inline literals with the token reads.
> 2. **Window-chrome config flips** — single-line widget chrome changes (e.g., `.collapsible(false)` → `.collapsible(true)`, `.resizable(false)` → `.resizable(true)`) on existing `egui::Window` calls, where the popup's body content, data plumbing, signatures, and behavior all stay identical and only egui's built-in chrome decoration changes.
> 3. **Library/binary structural split** — the project's current single-binary crate (`src/main.rs` declares the entire module tree) is restructured into a `lib + bins` layout: a new `src/lib.rs` declares `pub mod app; pub mod ui; pub mod settings; ...`, `src/main.rs` becomes a thin shim that calls `bio::ui::run(...)`, and a new `src/bin/infinity_orchestrator.rs` calls into the same library. This is mechanical — no logic changes in any file; the only edits to `main.rs` are deletion of module declarations (now in `lib.rs`) and a one-line call to the library entry point. Existing behavior is preserved bit-for-bit. The orchestrator lives as a sibling module inside the library crate so `pub(crate)` BIO items remain reachable from orchestrator code. **Companion provision (per Phase 1 D#1 resolution):** when the redesign adds new sibling modules under existing BIO directories (e.g., a new `src/ui/shared/redesign_tokens.rs` alongside today's `theme_global.rs`), a single additive `pub mod foo;` line in the corresponding existing `mod.rs` file (e.g., `src/ui/shared/mod.rs`) is allowed to register the new module. No existing line in the `mod.rs` may be reordered, edited, or removed; only an additive line. This is the minimal idiomatic Rust mechanism for exposing a new file in an existing module tree and is purely mechanical.
> 4. **WizardApp → WizardState signature refactor** — BIO functions whose body only mutates `app.state` (i.e., they take `&mut WizardApp` solely for its `state: WizardState` field) may be refactored to take `&mut WizardState` directly. The function body is unchanged; existing call sites inside `WizardApp` update to pass `&mut self.state` instead of `self`. This lets the orchestrator reuse these functions without constructing a fake `WizardApp`. Scope is tight: the refactor applies **only** to functions whose mutation surface is exclusively `app.state` — functions that also touch channels, the terminal, settings store, or other `WizardApp` fields stay as-is and the orchestrator builds a net-new equivalent. Per-function audit is required before refactoring.
> 5. **Schema-additive serde field additions** — new optional `#[serde(default = ...)]` fields may be added to existing BIO serde structs when needed for a redesign feature. The new field's default value must preserve today's BIO behavior for any data that omits the field (e.g., existing share codes parse without error, and the code path they trigger is identical to today's). No existing field may be renamed, removed, or have its type changed. The field is added in BIO source; BIO's own code does not need to read or write the new field — the orchestrator does. **Paired in-memory counterparts.** When the schema-additive field lands on a serde struct that has an in-memory companion (notably `Step1Settings` ↔ `Step1State`, paired via `From` impls in `state_convert.rs`), the carve-out also authorizes the symmetric field addition on the in-memory struct plus the two `From` mappings — without that pairing, the value would not round-trip and the carve-out would be useless. The pairing is mechanical: same field name, same type, default = `Default::default()` for the in-memory side. Per-field rationale required (one paragraph documenting why a wrapping struct in the orchestrator is insufficient).
> 6. **State-aware theme-token reads** — for BIO source files that render redesign-relevant UI surfaces (Step 2 tree, Step 2 Details panel, Step 3 reorder list, Step 5 inner sub-renderers, top-nav chrome, etc.), inline `egui::Color32` literals may be replaced with theme-token reads **even when the literal sits inside conditional logic that decides between colors based on state** (e.g., hover state, selected state, conflict tone, dev-mode banner). Scope is tight: the conditional structure of the function must be preserved exactly — no new branches, no removed branches, no reordered branches, no logic mutations, no behavior changes. Only the color expressions inside each branch may change, and only to swap inline `Color32::from_rgb(...)` calls (and equivalent inline color constructions) for `redesign_*(palette)` accessor calls. The function gains a `palette: ThemePalette` parameter (carrying the orchestrator's active palette) and call sites are updated to pass it. This carve-out is intended to enable visual fidelity for the wireframe-redesigned workspace surfaces without requiring orchestrator-side re-implementation of BIO's complex interaction logic (drag-and-drop, scan integration, expand-collapse, multi-select). Per-file scope documentation required (Phase 8 file inventory tables): each file is annotated with the conditional sites that get the token-read swap, with line-number citations to BIO source.
>
> Anything else — logic changes, body restructures, new branches in behavior, visibility flips other than the structural-split implications, signature changes beyond the WizardApp→WizardState exception above, or struct mutations beyond optional schema-additive serde fields — is **disallowed** as a modification to existing BIO source.
>
> All altered functionality belongs in net-new components. If a redesign requires different behavior from a BIO surface, the planner must add a new component (e.g., `src/ui/home/page_home.rs`, `src/ui/workspace/workspace_view.rs`) rather than diverge the existing BIO code.
>
> This directive overrides any phrasing elsewhere in the spec that could be read as "update BIO X to do Y" — re-read it as "add a new wrapper / sibling that does Y; the BIO X stays as-is."
>
> **Decision order when BIO's existing API is not a clean fit.** When the planner finds that a BIO function is private, takes the wrong type signature, or otherwise can't be called directly from the orchestrator, do **not** treat it as a blocker. Walk this decision order:
>
> 1. **Direct reuse first.** If BIO exposes any public API at any layer (`bio::app::*`, `bio::core::*`, `bio::ui::shared::*`) that does what the orchestrator needs — even if not the exact function name the original plan called out — call it. The lib+bin split (carve-out #3) makes most of BIO's surface reachable from orchestrator code; many cases that look private at the `WizardApp`-method level have a public counterpart at the lower `bio::app::*` layer.
>
> 2. **Net-new sibling for SIMPLE workflows.** When direct reuse isn't a clean fit AND the work is **simple enough that a faithful re-implementation has low drift risk** — e.g., a single state mutation, a folder-picker wrapper, a small format helper, a one-screen UI panel that reads BIO state but doesn't drive multi-step pipelines — build a sibling component in the orchestrator (e.g., `src/ui/orchestrator/...`). This is the directive's option 2, and it is **the default path** when direct reuse is blocked on a simple surface. No carve-out is needed; the sibling is just a new component in the orchestrator's namespace.
>
> 3. **Carve-out for COMPLEX workflows that can't be siblings.** When (a) direct reuse is blocked AND (b) the workflow is complex enough that a sibling re-implementation would carry serious drift risk over time — significant business logic, multiple interdependent state mutations, install-pipeline behavior, share-code interop format, anything where two parallel implementations would inevitably diverge — escalate for an explicit carve-out per the four flavors above. This is when carve-outs #3 / #4 / #5 exist: for cases where the cost of duplication exceeds the cost of a tight, mechanical BIO modification.
>
> **Net-new is for simple things; carve-outs are for complex things that can't be cleanly cloned.** Most "BIO function isn't reachable" flags are simple — build a sibling and move on. A carve-out is the right answer only when re-implementing the workflow in orchestrator code would create a real maintenance hazard.

**BIO-reuse rule (when a "BIO-fidelity" callout appears).** Many sections in this spec are labeled BIO-fidelity. The rule:

- **UI** (widgets, popups, dialogs, the embedded terminal, asset pickers, etc.) — **take the existing BIO implementation and re-skin it with the new theme tokens.** Don't rebuild it from scratch to match the wireframe; the wireframe is a visual reskin reference, not a rebuild brief. Behavior, data plumbing, and UX stay identical to today's BIO.
- **Backend** (engines, runners, file formats, fetchers, exporters) — reuse as-is. No styling involved.
- **Inheritance.** When a parent surface is BIO-fidelity, every sub-modal, child popup, and nested widget it opens is also BIO-fidelity by default. The redesign only deviates where the spec explicitly says so.
- **Default for un-wired wireframe surfaces.** Anything the wireframe draws but doesn't wire up (e.g., a visible button with no action, a glyph with no enforcement, a hint that references a not-drawn behavior) defaults to BIO-fidelity: take whatever BIO does today for that surface and ship it with the new theme tokens. No spec entry required.

When a BIO-fidelity callout in §13.x or elsewhere doesn't repeat "re-skin with theme tokens", this rule still applies to whatever UI surfaces the callout names.

---

## 1. Overview & Vision

BIO is a desktop app + CLI for installing WeiDU mods into Baldur's Gate Enhanced Edition (BGEE), Baldur's Gate II Enhanced Edition (BG2EE), Icewind Dale Enhanced Edition (IWDEE), and the Enhanced Edition Trilogy (EET). It scans local mod folders, resolves install order and compatibility issues, runs the WeiDU installer with live console + auto-answered prompts, and produces a reproducible **import code** that can be shared with other users to recreate the same modlist on their machines.

The redesign reframes BIO from a single-modlist linear wizard into a **multi-modlist workspace app** with four top-level destinations (Home, Install, Create, Settings). Global setup that used to live in "Step 1" (paths, tools, language) is now hoisted into Settings. Per-modlist setup (name, target game, destination folder) lives in the Create flow. The 5-step pipeline (Scan → Reorder → Review → Install) becomes a tabbed **workspace** the user lives inside while editing one modlist.

### 1.1 Design principles

- **Conversational, not configurable.** Common workflows are first-class buttons; obscure WeiDU flags live in Settings → Advanced.
- **Reproducibility.** Every modlist can be saved as a draft (.txt) or exported as a BIO-MODLIST-V1 share code that recreates the entire workspace on another machine.
- **Direct manipulation.** Drag to reorder. Click to select. Click pills to drill in. The tree is the source of truth, not a form.
- **Stateful flow.** Selections in Step 2 flow into the order in Step 3 flow into the review in Step 4 flow into the install in Step 5 — all without re-entering data.
- **Hidden until needed.** The Details panel is hidden by default (openable via per-row `[?]` or pinned open via the toolbar Kebab toggle). Compatibility popups open only when the user clicks a pill.

### 1.2 Brand & visual language

- App name: **Infinity Orchestrator** (BIO is the binary, project, and shorthand).
- Brand mark: `∞` glyph on a teal-accent square.
- Typography: **Poppins** for UI text, **FiraCode Nerd** for monospace/code/icon-glyphs. Both fonts are inlined into the build so the desktop app ships self-contained.
- Themes: **Light** (pale blueprint paper, navy ink) and **Dark** (teal-on-deep-slate). Dark is the default. See [§12 Theming](#12-theming).
- Sketchy/notebook aesthetic: 1.5px solid borders, 6×6px hard drop shadow, dotted radial background, rounded 3–6px corners.
- Custom titlebar with traffic-light dots (visual only) + center title + window controls.
- 26px footer status bar always visible: connection status · modlist count · jobs running · version.

---

## 2. Information architecture

### 2.1 Top-level navigation (persistent left rail)

| Order | Item | Icon | Purpose |
|-------|------|------|---------|
| 1 | Home | `⌂` | Welcome + installed modlists + "Add a modlist" CTAs |
| 2 | Install | `↓` | Paste an import code → install someone else's modlist as-is |
| 3 | Create | `✎` | Author a new modlist (scratch / import-and-modify / load a draft) |
| 4 | Settings | `⚙` | Global app settings (5 sub-tabs) |

The left rail also shows:
- Brand mark (`∞` square) + "INFINITY / ORCHESTRATOR" wordmark above the nav items.
- A status dot at the bottom: "weidu v249 · all paths ok" (green) when configured correctly, or red+error count when paths are missing/broken.
- The rail renders in **labels** mode by default (icons + text). Icons-only is a wireframe-iteration variant only — not user-configurable in production (see [§14.2](#142-tweaks-panel)).

### 2.2 The workspace (Steps 2–5)

When the user opens a modlist (from Home, Create, or via Load Draft), the entire main pane becomes a **workspace view** for that modlist:

- Header reads **"Editing _\<modlist name\>_"** in small caps, followed by a small **✎ rename** icon. Clicking the icon swaps the title text for an inline text input + `save` / `cancel` buttons; pressing Enter saves, pressing Escape cancels. **Rename only touches the registry entry** — the install folder on disk is **not** renamed (paths embedded in `weidu.log` or shared codes stay valid). Registry write debounced like all other state changes ([§13.14](#1314-persistence-timing)).
- A progress bar shows the 4 workspace steps: **Step 2 — Scan and Select**, **Step 3 — Reorder and Resolve**, **Step 4 — Review**, **Step 5 — Install**.
- Below the progress bar is a one-line hint describing the current step.
- Below the hint is the active step's content area.
- Below that is a bottom nav row with `← Back` / `Next →` buttons and an indicator like "Step 3 · Reorder and Resolve · next: Review".
- A header-right button area: `⑂ view fork details` (when forked), and **save draft** (Steps 2–4) or **Share import code** (Step 5 only, enabled only after a successful install).
- The left rail remains available so the user can jump out to Settings or Home mid-modlist. The workspace state persists.

There is no longer a "Step 1" inside the workspace. Setup migrated to Settings and the Create screen.

---

## 3. Home

> **BIO reuse:** Home is a **new** screen — today's BIO opens directly into the workspace. The redesign reuses BIO's settings/state machinery (`src/settings/`, the `bio_settings.json` loader) to know which games are installed and where, and reads from the new `modlists.json` registry ([§13.1](#131-modlist-persistence)) to populate the card lists. No existing BIO screen is being repainted here; this section defines the new entry point.

### 3.1 Layout

`sk-page` with `ScreenTitle`:

- **Title:** "Welcome back, adventurer"
- **Subtitle:** segments separated by ` · `, with empty segments omitted, in this order: `<N> modlists installed` · `<P> in progress` (if P > 0) · `last played <game> <relative time>`.

Below the title, a 2-column grid (`2fr 1fr`) with a 20px gap and 20px bottom margin:

- **Left column** — a single Box containing a **filter-chip row** above a unified scrollable list of modlist cards. The chip pattern scales cleanly to long lists in either category and keeps the homepage layout stable as the library grows.
- **Right column** — `Box label="add a modlist"` containing the CTAs (see [§3.4](#34-add-a-modlist-section)), then a `game installs detected` block.

A modlist is **in-progress** until its first successful install completes; on success it transitions to **installed** (see [§9.2](#92-post-install-layout-installcompletetrue)). This is the same registry record — just a state flip — so deleting an in-progress build is functionally identical to deleting any other registry entry.

#### Filter chips

A horizontal row at the top of the left Box, three rounded-pill buttons (lighter visual treatment than the regular primary `Btn`: no drop shadow, 14px border radius). The chips render with slightly taller vertical padding than the wireframe's literal `4px` (≈7px) — a deliberate user-directed visual call (the 4px read too cramped against the rest of the Home chrome); horizontal padding stays at the wireframe's 12px. This is an intentional deviation, not drift.

- **`Installed (N)`** — finished modlists. The default selection when the user lands on Home, since the steady-state experience is "play your existing libraries".
- **`In progress (P)`** — only rendered when `P > 0`. Shows the count of in-progress builds.
- **`All (N + P)`** — always rendered; combines both lists in the order installed-first, in-progress-after.

When the inferred default category is empty (e.g., a brand-new install with `N = 0`), the chip selection falls back to whichever category has content (`In progress`, else `All`). The active chip is filled in `var(--accent)` (teal) with dark text; inactive chips use `var(--shell-bg)` with normal text.

#### Cards in the filtered list

Each visible row is a card laid out identically regardless of which chip is active. The card chassis is the same as before ([§3.2](#32-cards-shared-shape)):

- **Modlist name** (bold) + meta line (hand style, faint) on the left.
- A state-dependent action cluster on the right:
  - For an **in-progress** card: primary **`resume`** + Kebab with `Copy import code` / `Delete`.
  - For an **installed** card: **`open`** + Kebab with `Copy import code` / `Open install folder` / `Rename` / `Reinstall` / `Delete`. (The wireframe's `play` button is renamed `open` for v1 alpha — see [§3.2](#installed-modlist-card).)

**Reinstall semantics.** `Reinstall` is a full from-scratch reinstall of the same modlist with the same components and order — no editing of the selection is supported in this flow (re-edit is a later functionality). Clicking it opens a danger-styled `ConfirmDialog`:

- **Title:** `Reinstall "<modlist name>"?`
- **Body:** "This will erase the current install folder and re-run the entire install from scratch. Your component selection and order are preserved; the modlist will move back to **in-progress** while the install runs, then return to installed when complete. The existing files at the destination will be deleted." + the install folder path in monospace. Closing line: "This action cannot be undone."
- **Buttons:** `Cancel` + danger-tinted primary `Reinstall`.

On confirm: the app **routes through the Install Modlist preview stage** ([§4.2](#42-stage-2--preview)) using the modlist's stored share code as the source — same screen as a paste-code install, just pre-populated (overview Box + the preview tabs for Summary / WeiDU logs / User Downloads / Installed Refs / Mod Configs). The user reviews one last time, then clicks **Reinstall →** to actually run it. The modlist state flips to `in-progress` **only when the install starts** (not when the preview opens; cancelling the preview leaves the modlist `installed`).

**Overwrite-install mode forced.** Reinstall always starts the install in **overwrite-install mode**. The `DestinationNotEmptyWarning` is skipped (the user already confirmed via the Reinstall confirm modal), and `prepare_target_dirs_before_install` is forced ON for this install — equivalent to the warning's "replace contents" choice. The existing install runner then wipes and rebuilds the target dirs before WeiDU runs. Same concurrency rule as any other install ([§13.15](#1315-install-concurrency-policy)).

**Delete semantics.** `Delete` removes **both** the registry entry and the on-disk install folder. Applies equally to **in-progress** and **installed** modlists. Clicking Delete opens a danger-styled `ConfirmDialog`:

- **Title:** `Delete "<modlist name>"?`
- **Body:** "This will permanently remove:" followed by a bulleted list naming the two things that go (registry entry → disappears from Home; install folder → with the absolute path in monospace). Closing line: "This action cannot be undone."
- **Buttons:** `Cancel` + danger-tinted primary `Delete`.

On confirm: the registry entry is removed (debounced write to `modlists.json`) and the install folder is recursively deleted. A success toast confirms (`✓ Deleted "<name>"`).

Mixed views (the **`All`** chip) interleave both card types; the action button (Resume vs Play) and the meta line ("paused at Step _N_" vs "installed _\<when\>_") together disambiguate state without needing an extra status pill.

#### Empty states

When the current chip's filtered list is empty, the body shows a single faint line:

- `Installed`: "No installed modlists yet. Create one or paste an import code to add the first."
- `In progress`: "No in-progress builds. Start a new modlist from \"create your own\"."
- `All`: "No modlists yet."

A toast notification can appear bottom-center: `✓ Copied import code for "<modlist>"` — auto-dismisses after ~1.8s.

### 3.2 Cards (shared shape)

Both in-progress builds and installed modlists use the same card chassis: a Box with the modlist name on the left, a meta line beneath it (hand style, faint), and a right-aligned action cluster.

#### In-progress build card

- **Modlist name** (bold).
- **Meta line:** `<N> mods · <C> components · last touched <relative time> · paused at Step <K>` (e.g., "9 mods · 136 components · last touched 2 hours ago · paused at Step 3").
- **Right cluster:** primary teal **`resume`** button + **Kebab** menu.

##### In-progress Kebab items

| Item | Action |
|------|--------|
| Copy import code | Writes the build's current BIO-MODLIST-V1 code to the clipboard, shows the toast. (Same code that gets auto-written to the destination at install start — see [§13.13](#1313-import-code-auto-generated-on-install-start).) |
| Rename | Renames the modlist registry entry. Same effect as the workspace's inline ✎ rename ([§2.2](#22-the-workspace-steps-25)) — implementation can either open a small rename dialog or push the user back into the workspace with the title in edit mode. |
| Delete | Removes the in-progress build (after confirmation; the destination folder on disk is untouched). |

The **`resume`** button opens the workspace at **Step 2 (Scan and Select)**, pre-populated with the build's order, selections, and any pending settings. The workspace header reads **"Editing _\<build name\>_"** ([§2.2](#22-the-workspace-steps-25)), the per-game-install tabs reflect the build's stored game choice, and the user can immediately navigate to whatever step they want via the workspace progress bar. (The wireframe demo always lands on Step 2; a future refinement could remember the last-active step per build.)

#### Installed modlist card

- **Modlist name** (bold).
- **Meta line:** `<N> mods · <size> · installed <relative time>` (e.g., "47 mods · 2.3 GB · installed 2 days ago").
- **Right cluster:** **`open`** button + **Kebab** menu. (Long-term, this button launches the installed game — the wireframe's label is "play" — but until a game-launcher implementation lands the v1 alpha labels it **`open`** and opens the install folder in the OS file manager. Same fallback as the Kebab's `Open install folder` action; the button label is just shorter. When a launcher arrives in a future release the label flips back to `play` and the action launches the game directly.)

##### Installed Kebab items

| Item | Action |
|------|--------|
| Copy import code | Writes the modlist's BIO-MODLIST-V1 code to the clipboard, shows the toast. |
| Open install folder | Opens the modlist's destination folder in the OS file manager. If the folder no longer exists on disk (deleted externally), surface an error message in the standard status / error message area near the bottom of the screen — do not attempt to open or recreate the folder. |
| Rename | Renames the modlist registry entry. |
| Delete | Removes the modlist record (after confirmation; does not touch the install on disk). |

### 3.3 "Add a modlist" section

Two CTAs, top-to-bottom:

1. **paste import code** (primary teal button) — navigates to Install.
2. **create your own** (neutral button) — navigates to Create.

Labels are intentionally lowercase to read as fluent verb phrases.

Below the CTAs, the **game installs detected** block lists what BIO auto-found on this machine:

- ✓ BGEE
- ✓ BG2EE
- ? IWDEE · not found

Detection comes from the same logic that today populates Step 1 path validation. The full path is configurable in Settings → Paths.

**Refresh semantics.** The block re-evaluates automatically whenever path validation runs: on app start (if `Validate all paths on startup` is on — see [§11.1](#111-general)), on any path edit in Settings → Paths (debounced — see [§11.2](#112-paths)), and on any new modlist install that creates a target folder. The block reflects the latest validation result — no manual refresh control.

### 3.4 First-launch / empty-registry state

When the modlist registry is empty (zero installed AND zero in-progress), the left column's main Box (which would normally hold the filter chips + list) replaces its contents with a **setup call-to-action card**:

- A short heading like `Welcome to Infinity Orchestrator` or similar.
- One faint sub-line: `Get set up first — point BIO at your games and tools.`
- A single primary button: **`Open Settings`** that navigates to Settings → Paths.

This carries through until the user creates or installs their first modlist. The "Add a modlist" Box on the right column still renders normally (so a user who already has a share code can paste it without going through Settings), but trying to install before paths are configured will surface an inline path-validation warning at install time — same path-validation today's BIO does on Step 1, just reused.

When the registry is non-empty but Settings → Paths / Tools is incomplete (game folders or `weidu` binary missing), Home renders normally with one extra muted line under the title: `Some game paths aren't configured. Visit Settings to complete setup.` — clicking the text deep-links to Settings → Paths.

---

## 4. Install Modlist (paste import code)

> **BIO reuse:** Install Modlist wraps today's BIO share-code parse + fetch + install pipeline (`src/core/app/app_step2_router.rs`, `src/core/app/mod_downloads.rs`, the install runner — §13.9). New chrome only: the staged screens (paste → preview → downloading → installing) are net-new components; the backend operations they call into are all existing BIO code.

The "consume someone else's modlist" flow. A linear sub-flow with four stages: **paste → preview → downloading → installing**. The same chassis is used for the Create → Import-and-modify flow, with different button labels and a different terminal target (review vs run).

The whole flow uses the `SubFlowFooter` pattern: Back on the left, optional hint, primary CTA on the right.

### 4.1 Stage 1 — Paste

`ScreenTitle title="Install shared modlist" sub="set destination + mods paths, paste a BIO share code, then preview before importing"`

The page is a flex-column with three sections:

1. **Destination folder** — `FolderInput` with `browse` button. If the destination is non-empty, a warning Box appears with radio choices (e.g., "this folder is not empty — replace contents / continue partial install / cancel"). Continue-partial mode skips the share-code requirement entirely.
2. **Import code** — only shown if not in continue-partial mode. `Box label="import code"` with a multi-line monospace textarea (placeholder: `BIO-MODLIST-V1:eJyrVkrLz1eyUkpKLFKqBQA...` + "Paste the full code here.").
3. **Footer** — `SubFlowFooter` with primary **Preview →** (or **Continue Install →** if partial), hint "no install starts until preview is accepted" (or "no share code needed").

### 4.2 Stage 2 — Preview

`ScreenTitle` shows the **modlist name** as title (e.g., "Born2BSalty's EET tactical playthrough"), with a subline `<author> · review what will be installed before BIO downloads anything`.

Two stacked sections:

1. **Overview Box** — 4-column grid: `Game: <code>`, `Mods: <count>`, `Components: <count>`, `BGEE/BG2EE entries: <n>/<n>`.
2. **Content Box** with **tabs above** in the file-folder style (active tab merges into the box). Tabs: `Summary`, `BGEE WeiDU`, `BG2EE WeiDU`, `User Downloads`, `Installed Refs`, `Mod Configs`. The content box stretches to fill the remaining vertical space and renders the active tab's content in monospace.

**Preview tab contents** (rendered from the share-code payload):

- **Summary** — BIO version, game install, install mode, WeiDU log entry counts per game, included data flags, "What import will do" bulleted recap.
- **BGEE WeiDU** — verbatim contents of the BGEE `weidu.log` from the share code.
- **BG2EE WeiDU** — same for BG2EE.
- **User Downloads** — TOML excerpt of `mod_downloads_user.toml` overrides packaged in the share.
- **Installed Refs** — TOML of pinned `[refs]` and `[sources]` (which mod source each tp2 came from, plus commit/tag pin).
- **Mod Configs** — list of mod-config files restored from the share (`<mod> | <source> | <file>`).

Footer: `← Back` → paste stage, **Import Modlist →** primary, hint "downloads, extracts, then runs install — no review step".

**`allow_auto_install` gate (per [§13.3](#133-share-code-bio-modlist-v1)).** The preview parses the decoded payload's `allow_auto_install` flag (defaulting to `true` for older codes that lack the field). If the bit is `false` — meaning the code was generated from a draft / mid-install / unverified source — the preview shows an additional info banner above the Overview Box:

> *"Draft modlist code — this is not from a verified install. Review and customize the components in Create → Import and modify before installing."*

When the bit is `false`, the **Import Modlist →** primary CTA is **disabled** (greyed out with tooltip "Auto-install disabled for draft codes — open in Create to review"), and a new secondary CTA **`Open in Create →`** is rendered in the footer that routes to `Create → Import and modify` with the same share code pre-pasted. From there the user reviews/customizes the modlist; the resulting workspace's install path generates its own draft code until it reaches `Installed`.

### 4.3 Stage 3 — Downloading

Uses the shared `ImportDownloadScreen` component:

- `ScreenTitle title="Downloading & extracting" sub="fetching mod archives — install starts automatically when ready"`
- `Box label="overall progress"` — "_N_ / _T_ mods · _P_%" big label + a progress bar.
- `Box label="mod progress"` — a 4-column grid (mod / source / status / progress bar) for every mod in the modlist. Per-row status: `queued`, `downloading <p>%`, `extracting...`, `✓ staged`. Colors: faint for queued, normal for active, success-green for done.
- Footer: `Cancel` (← back) + a placeholder `simulate complete →` in the wireframe; in production the next stage transitions automatically when downloads complete.

This screen is reused by Create → fork-download with a different title/sub/continueLabel.

### 4.4 Stage 4 — Installing

The active install runtime. (See [§8 Step 5 — Install](#8-step-5--install) for the full spec; this stage embeds the same panel without the workspace chrome since the user came in from Install, not Create.)

The wireframe uses `InstallProgressScreen` here. It shows:

- Header: `Installing modlist · <fork name> · live install console` + a back button.
- Status row: `Installing` pill, "Component _N_ / _T_", "ran _MM:SS_ · auto-answering prompts", "console auto-scrolls as new lines arrive".
- Action row: **Cancel Install** (primary), **Actions ▾**, **Diagnostics ▾**, **Prompt Answers**, plus radio filters (`General`, `Important Only`, `Installed Only`) and an `Auto-scroll` toggle.
- `Box label="Console"` — scrolling live log of `mod_installer` output. Coloring: info=normal, success="SUCCESSFULLY INSTALLED..." in green, muted streaming-marker.
- Prompt-input row: a labeled `Box` containing "Type a prompt response:" + a monospace text-input + `Send` button. The input is disabled until WeiDU actually requests input.

After a successful install, the user is **not** offered a Share import code button from this entry point — the code that produced this install is the one they already pasted. Instead, the action row gains **`Return to Home`** + **`Open install folder`** ([§9.2](#92-post-install-layout-installcompletetrue), Appendix B.2).

---

## 5. Create — author a new modlist

> **BIO reuse:** Create wraps today's BIO Step 1 (`src/core/app/state/state_step1.rs` + `src/ui/step1/`) — the destination-folder picker, the `DestinationNotEmptyWarning`, and the validation logic stay. The redesign adds: the modlist-name field, the game selector (today inferred elsewhere), the starting-point cards, and the Load Draft dialog. The Workspace it routes into reuses Steps 2–5 from today's BIO with the new chrome.

The Create screen is the front door to the workspace. It has one initial state (`choose`) and routes into one of three downstream flows.

### 5.1 Initial — `choose` mode

#### Title row

- `ScreenTitle title="Create / edit modlist" sub="name your modlist, set destination + mods paths, then pick a starting point"`
- Top-right action: **load draft** button — opens `LoadDraftDialog`.

#### Setup Box

A single Box containing three rows:

1. **Modlist name + Game** (split row).
   - Left (flex 1): label "modlist name" + real text input. Placeholder "e.g. Tactical EET 2026".
   - Right (auto width): label "game" + a dropdown (egui `ComboBox`) with options `EET`, `BGEE`, `BG2EE`, `IWDEE`. **EET is the default selection.** Styled to match the Language dropdown in Settings → General (sketchy border, input-bg fill, Poppins 14px, 8px×12px padding). Selecting `EET` makes the workspace show both BGEE and BG2EE tabs in Steps 2–4; the three single-game options show only one tab named for the selected game.
   - **The game choice is immutable once the workspace opens.** The dropdown is editable here on Create only. After the workspace is entered (and the order array's tab structure depends on the choice), the field is no longer surfaced to the user. To install the same set of mods against a different game, the user creates a new modlist (the share code from the original can be loaded into the new workspace if applicable).
2. **Destination folder** — `FolderInput` with `browse` button.
3. **DestinationNotEmptyWarning** (conditional) — shown if `destination folder` is set to a non-empty directory. Allows the user to pick how to proceed (e.g., "this folder isn't empty — replace / continue / cancel"). Continue-partial is disallowed in Create (only Install supports it).

#### Starting-point options (two cards, side-by-side)

1. **New modlist from downloaded mods** — primary button **`start →`**. Routes to `WorkspaceView` in `scratch` mode. Description: "Scan your local mods folder, pick components, reorder, then install. Starts from an empty selection."
2. **Import and modify another modlist** — primary button **`paste share code →`**. Routes to `fork-paste`. Description: "Paste a share code. BIO downloads the mods, preselects components, applies the order, then drops you on Step 2 to review and adjust."

### 5.2 Load Draft dialog (resume in-progress build)

Triggered from the **load draft** button at the top-right of the Create screen ([§5.1](#51-initial--choose-mode)). Modal dialog over the Create screen.

- **Title:** "Resume in-progress build"
- **Subtitle:** "Pick a build to resume. BIO restores its order, selection, and settings and drops you back where you left off."
- **Body:** a vertical list of in-progress build cards using the same chassis as the Home page section ([§3.2](#32-cards-shared-shape)). Each card shows the build's name, meta line, **`resume`** primary button, and a **Kebab** with `Copy import code` / `Delete`. Empty state when no in-progress builds: `No in-progress builds. Start a new modlist from Create.` in faint type.
- **Footer:** `Cancel` only — no primary "Load" button at the dialog level since each card has its own `resume`.
- A transient `✓ Copied import code for "<modlist>"` confirmation can appear inside the dialog after using the Kebab's Copy action.

The dialog is intentionally **not a file picker** — the redesign treats in-progress builds as registry-tracked entities, not loose files. The on-disk `modlist-import-code.txt` from a previous attempt (see [§13.13](#1313-import-code-auto-generated-on-install-start)) is reachable via the **Install Modlist** paste flow if the user has a code from outside the registry.

On `resume`, the dialog closes and the workspace opens at **Step 2 (Scan and Select)** with the build's name in the header (`Editing <build name>`), its stored game tabs, and its order/selection pre-loaded — identical to the Home page Resume button ([§3.2](#32-cards-shared-shape)).

### 5.3 Fork-paste / Fork-preview / Fork-download

The "Import and modify" sub-flow uses the same chassis as the Install Modlist flow:

- **Fork-paste** (`ForkPasteScreen`) — `Box label="import code"` with the same paste textarea as Install. `SubFlowFooter` with `Back` (→ choose) + **`Preview →`** primary, hint "no download starts until preview is accepted".
- **Fork-preview** (`ForkPreviewScreen`) — identical to Install's preview stage but the title comes from the parsed share code and the primary CTA is **`Begin Import →`** with hint "downloads mods, applies selection + order, then drops you on Step 2".
- **Fork-download** — uses `ImportDownloadScreen` with title "Downloading fork", hint "after download: components auto-selected · order applied · lands on Step 2 for review", continueLabel "continue to Step 2 →".

After download, the workspace opens with the fork's name in the header (e.g., **"Editing Born2BSalty's EET tactical playthrough"**), a small `⑂ Fork` badge next to the title, and a banner "Forked from _\<name\>_ by _\<author\>_ · _N_ mods · _C_ components preselected".

---

## 6. Step 2 — Scan and Select

The first workspace tab. Goal: let the user discover every component in their mods folder, see which conflict / match / prompt, and pick what to install.

### 6.1 Layout

`sk-page` flex-column, padding `20px 28px`:

1. Workspace header (modlist title row + fork pill if forked + save draft / share import code button).
2. `WorkspaceProgressBar` showing the four steps with completed-state.
3. Current-step hint line.
4. Step content area (a `SourcesPanel` flex-column).
5. `WorkspaceNavBar` at the bottom.

The `SourcesPanel` content:

```
[ Mods / Components ]               <- title
[ search box ] [ Rescan ]           <- search + rescan
[ tab1 tab2 ][ actions/pills ]     <- BGEE/BG2EE tab row with right-aligned tools
[ tree box (full width)            <- component tree (or 1fr+420px when Details open)
  ... lots of rows ...
                                  +-----------+
                                  | Details   |
                                  | panel     |
                                  +-----------+
```

### 6.2 Title

A 15px medium-weight Label reading "Mods / Components".

### 6.3 Search + Rescan row

- Wide `Input` (placeholder "Search mods or components...").
- Right-aligned **`Rescan Mods Folder`** TopButton. Rescan is non-destructive — no confirmation dialog. Triggers a fresh TP2 scan of the configured mods folder; progress shows in the status bar.

### 6.4 Toolbar

A composite row sitting flush above the tree box, using the **file-folder tab pattern**:

- **Game tabs** (left side): `BGEE` / `BG2EE` `GameTab`s when the modlist's game is `EET`; a single tab (e.g., `BGEE`) for single-game modlists. The active tab merges visually with the tree box below (its bottom border matches the box's interior color).
- **Action cluster** (right side, in the same row, growing to fill space):
  - **Select \<TAB\> via WeiDU Log** TopButton — **shown only when the workspace was entered via Create → New modlist from downloaded mods**; hidden in Import-and-modify and Resume workflows. **BIO-fidelity** — reuse today's parse + apply behavior and confirmation dialog wiring (`src/ui/step2/` action handlers); UI re-skinned per the BIO-reuse rule. Opens a confirmation dialog → file picker for a `weidu.log` → parse + apply.
  - **Updates...** TopButton — opens the [Update Check popup](#611-update-check-popup-updatespopup) ([§6.11](#611-update-check-popup-updatespopup)).
  - **Mismatch pill** — `<TAB> Mismatch <count>`. Aggregate count of compatibility issues for the active tab. Hover tooltip mirrors today's BIO text: "_N_ compatibility issues in the _\<tab\>_ Step 2 tab. Active badge category: Mismatch (_N_). Dominant category: Mismatch (_N_)." Click opens a `PillPopup` listing categories (`Mismatch (game)`, `Conflict`, `Order`, `Missing dependency`, `Path requirement`) with counts plus a sample list of conflicting components.
  - **PROMPT pill** — `PROMPT <count>`. Count of components on this tab that will prompt during install. Hover tooltip "See parsed prompts on the current tab". Click opens a `PillPopup` listing each prompting component with its detected prompts and auto-answer.
  - **Count text** — `<selected> / <total> on <TAB>`, faint, right-aligned.
  - **Kebab** (`···`) — menu items, in this order: `Show Details panel` (persistent toggle, prefixed with `✓ ` when on), `Clear All`, `Select Visible`, `Collapse All`, `Expand All`, `Jump to Selected`. The `Show Details panel` toggle pins the right-side Details pane open across row selections; the per-row hover `[?]` button still works alongside it ([§6.8](#68-details-panel)).

### 6.5 Tree

> **BIO-fidelity for the Step 2 tree** — see [BIO-reuse rule](#1-overview--vision). **Use the existing BIO Step 2 tree widget** (`src/ui/step2/tree/`); re-skin it with the new theme tokens; ship. Do not rebuild the tree from scratch.
>
> The component listing is one of BIO's core strengths and must be preserved as-is. Indentation, how collapsing groups inside a mod work (e.g., the section headers Stratagems and EEUITweaks use), how parent components nest subcomponents that behave as radio buttons, how the mod-level tri-state checkbox toggles its leaves, how row click selects without toggling, right-click context menu, search filtering, jump-to-selected — all already work correctly in BIO; keep them.
>
> The **only** additions in the redesign are:
>
> 1. The Details panel on the right ([§6.8](#68-details-panel)) is hidden by default and opens via the per-row hover **[?]** affordance.
> 2. Inside the Details panel, each detail row has hover-revealed copy/open icons (and a click-to-copy on the value column).
>
> The new pill popups, compat popup, prompt popup, and Updates popup are launched **from** the tree but don't change the tree itself.

The tree renders mods → optional intra-mod groups → components → optional WeiDU subcomponent parents → subcomponents. Read top-down it looks like:

```
▾ ▣ EEFixPack (3/3) v Beta 2
  ☑ #02 #0  // Core Fixes: Beta 2
  ☑ #03 #2  // Game Text Update: Beta 2
  ☐ #0 #5  // Drow Item Restorations: Beta 2

▾ ▣ EEUITweaks (8/9) v 4.0.7  [3 conflicts]
  ☑ #14 #1000 // Mods Options: 4.0.7
  ☑ #15 #1010 // Hidden Game Options: 4.0.7
  ▾ Portrait Selectors (pick one · 1/3)
    ◉ #16 #1030 // BillyYank's Multi-Portrait Mod: 4.0.7
    ○    #0 #1031 // Vanilla portraits only: 4.0.7
    ○    #0 #1032 // Custom portrait pack: 4.0.7
  ☑ #17 #1020 // Mr2150's Random PC Generator: 4.0.7  [Conflict]
  ...
```

Three row tiers, matching BIO's `Step2Selection` model (which only supports `Mod` and `Component` selection — see Details panel inventory):

#### Mod row (top-level parent — `Step2Selection::Mod`)

- Indent: 0.
- Expand glyph: `▾` (expanded) / `▸` (collapsed). 16px, generous hit padding.
- Mod-level tri-state checkbox glyph: `▣` (all components checked), `◩` (some), `☐` (none). Clickable; toggles **all** of the mod's leaves at once (BIO bulk-toggle). Hover tooltip explains.
- Mod name (bold).
- `(<checked>/<total>)` count, faint.
- `v<version>` faint.
- Aggregate pill (e.g., `3 conflicts`) immediately after the version (not right-aligned).
- Hover-only / selected-only **[?]** action button at the row's far right that opens the Details panel for this mod.
- **Click on the row body** (anywhere not the chevron or the checkbox) → **expands/collapses** the mod **and** selects it (teal highlight; Details panel will display the mod's info if open).

#### Parent component group (e.g., Portrait Selectors — NOT a selectable row)

This is the WeiDU subcomponent-parent: a `CollapsingHeader` whose only purpose is grouping a set of mutually-exclusive subcomponents.

- Indent: 30px.
- Expand chevron `▾`/`▸`.
- Group name in Poppins, faint color (text-muted).
- `(pick one · <sub-checked>/<sub-total>)` count, faint — the `pick one` prefix makes the radio behavior unambiguous to the user.
- **NOT selectable** — no hover-highlight, no teal-selected state, no `[?]` button. The Details panel cannot focus on a parent group (BIO has no `Step2Selection::ParentGroup`).
- **No checkbox glyph** — the group itself isn't installable. Only its subcomponents are.
- **Click on the row body** → toggles expand/collapse only.

#### Leaf component row (`Step2Selection::Component`)

- Indent: 30px when nested directly under a mod, **46px** when nested under a parent component group (so the subcomponent's radio glyph aligns with the start of the parent's label).
- Larger checkbox glyph (17px, generous hit area). **Shape changes by context:**
  - Regular leaf: square checkbox `☑` (checked) / `☐` (unchecked).
  - **Subcomponent under a parent group: radio glyph `◉` (selected) / `○` (unselected).** Behavior: clicking a radio glyph picks that variant and unchecks all sibling subcomponents in the same parent group (single-select). Clicking an already-selected radio glyph unchecks it (the group has no variant chosen).
- If checked: a 2-digit `#NN` order prefix in faint type (the order this component will install in; carried through to Step 3).
- Component number cluster: `#0 #<id>` in dark blue.
- Component label: `// <name>: <version>` in green if checked, faint if not, with ellipsis on overflow.
- Pills (clickable) immediately after the label:
  - **Conflict** (danger / coral)
  - **Conditional** (info / teal)
  - **Prompt** (warn / amber)
- Hover / selected **[?]** at the row's right for Details.
- **Click on the row body** (anywhere not the checkbox/radio, not a pill, not the `[?]`) → **selects** the row (teal highlight; the Details panel, if open, updates to this component). Row click does **NOT** toggle the checkbox — only the checkbox glyph itself toggles. This matches today's BIO.

#### Row visual states

- **Hover**: subtle `--hover-overlay` background tint on selectable rows (mod, leaf). Parent component groups don't show a hover background since they're not selectable.
- **Selected**: teal tint (`rgba(20,184,166,0.18)`); persists across hover. Parent groups never enter this state.
- **Drag**: not applicable in Step 2. (Drag-reorder lives in Step 3 — see [§7](#7-step-3--reorder-and-resolve).)

### 6.6 Pills are clickable everywhere

- Every pill on a tree row has `cursor: pointer`, a 1px hover-lift, slight brightness boost, and a 2px drop shadow on hover.
- Clicking a pill **stops propagation** (no row expand/collapse, no checkbox toggle, no parent-mod toggle), **selects the row** (so it gets the teal highlight), and opens the appropriate structured popup:

| Pill | Opens | Mode |
|------|-------|------|
| Conflict (leaf) | `CompatPopup` ([§10.4](#104-compatpopup)) | `single`, filter pre-selected to `Conflict`, item = clicked leaf |
| Conditional (leaf) | `CompatPopup` | `single`, filter pre-selected to `Conditional`, item = clicked leaf |
| Prompt (leaf) | `PromptPopup` ([§10.5](#105-promptpopup)) | `single`, focused on clicked component |
| `N conflicts` (mod row) | `CompatPopup` | `aggregate`, filter pre-selected to `Conflict`, item = first conflict in this mod |
| `<TAB> Mismatch <N>` (toolbar) | `CompatPopup` | `aggregate`, filter pre-selected to `Mismatch` (the dominant category for that badge) |
| `PROMPT <N>` (toolbar) | `PromptPopup` | `aggregate` for the active game tab |

The full set of compat categories supported by the rules engine is: `conflict`, `mismatch`, `order_block`, `missing_dep`, `included`, `not_needed`, `not_compatible`, `path_requirement`, `conditional`, `deprecated`, `warning`. Each maps to a pill tone:

| Category | Pill tone | Color |
|----------|-----------|-------|
| `conflict`, `not_compatible`, `mismatch`, `order_block`, `missing_dep` | **danger** | coral (`#e69a96`) |
| `path_requirement`, `warning`, `deprecated` | **warn** | amber (`#e8c441`) |
| `conditional`, `included`, `not_needed` | **info** | soft teal (`#a8d2cc`) |

These map cleanly to today's BIO compat issue kinds. The generic `PillPopup` chassis ([§10.6](#106-pillpopup)) is retained for any one-off informational popups but is not used for compat or prompt content.

### 6.7 WeiDU log syntax coloring (Step 3 / Step 4 only)

When a component is rendered in WeiDU-log style (Steps 3 and 4), the line is colored in three parts:

- **TP2 path** (`~EEUITWEAKS\EEUITWEAKS.TP2~`) — soft amber (`#d4a35c`).
- **Component numbers** (`#0 #1030`) — dark blue (`#2f6fb7`).
- **Comment** (`// BillyYank's Multi-Portrait Mod: 4.0.7`) — success green (`var(--success)`).

In Step 2 the same color encoding is used inline on leaf rows.

### 6.8 Details panel

Hidden by default. Width: 420px. Appears as a second grid column to the right of the tree.

Two ways to open: (a) click the hover **[?]** affordance on any tree row (mod, parent component, or leaf) — opens the panel for *that* row; (b) toggle `Show Details panel` in the toolbar Kebab ([§6.4](#64-toolbar)) — pins the panel open across row selections so each clicked row populates it. Closes via the **✕** in the panel's top-right corner or by toggling the Kebab item off.

The content **branches on selection kind**, matching today's BIO's two selection types: `Step2Selection::Mod` ([§6.8.2](#682-mod-header-view-kind-mod-or-parent)) and `Step2Selection::Component` ([§6.8.3](#683-leaf-component-view-kind-leaf)). Parent component groups (the WeiDU subcomponent-parent header like Portrait Selectors) are intentionally **not selectable** ([§6.5](#65-tree)), so the panel never opens for them.

#### 6.8.1 Empty state (no selection)

`Box label="Details"` with `✕` close. A faint Label: "Click a mod or component to see its details."

#### 6.8.2 Mod-header view (kind `mod`)

- **Title**: mod name (15px, weight 500).
- **Sub-line** (hand style, faint): `Mod · <TP2>`.
- **Version**: `Version: <version>`.
- **Selection** section header.
  - `DetailsRow` rows: `TP2 File`, `Shown`, `Hidden`, `Raw`. (`Shown`/`Hidden`/`Raw` are component counts — `Shown + Hidden = Raw`.)
- Divider.
- **Paths / Links** section header.
  - `DetailsRow` rows: `TP2 Folder`, `TP2 Path`, `INI Path`, `Readme` (each with copy + open icons). `Web` row appears only when the mod has a web URL configured. The `Readme` value reads `No data` in muted amber when absent.
- **Package** section header.
  - `DetailsRow` rows: `Installed Source`, `Update Source` (with `(default)` or `(selected)` suffix), `Latest Version`, `URL`, `GitHub`. The `Add Source` / `Reload Sources` buttons appear inline only when no source is configured for the mod (Appendix A.5).
- Action buttons at the bottom: **`Check This Mod`**, **`Lock Updates`** / **`Unlock Updates`** (label flips based on lock state). The `Check This Mod` button is disabled when the modlist's install mode is `install_exactly_from_weidu_logs`.

There is no `Component Block` or `WeiDU Line` block on the mod-header view.

#### 6.8.3 Leaf-component view (kind `leaf`)

- **Title**: parent mod name.
- **Sub-line** (hand style, faint): `Component #<id> · <mod name>`.
- **Version**: `Version: <version>`.
- **Selection** section header.
  - `DetailsRow` rows, in order: `Component` (the component's label + version), `ID` (the component's numeric ID), `Checked` (`Checked` in success-green or `Unchecked` in faint), `State` (`Selectable` in success-green, `Disabled` in amber with hover-tooltip showing the disabled reason), `Language` (e.g., `en_US`), `TP2 File`, `Shown`, `Hidden`, `Raw`, and `Order` (only when checked).
- **Compatibility** section header *(only when the component has a compat issue — Conflict, Conditional, etc.)*. Tone-colored to match the pill (coral for conflict, soft teal for conditional).
  - `DetailsRow` rows: `Source Type` (e.g., `Conflicts` / `Conditional` / `Requires` / `Game Mismatch`), `Issue` (the BIO issue code: `FORBID_HIT`, `REQ_MISSING`, `GAME_MISMATCH`, `CONDITIONAL`, `ORDER_WARN`, `DEPRECATED`), `Reason` (human-readable, amber), `Rule Origin` (TOML filename: `step2_compat_rules_default.toml` or `step2_compat_rules_user.toml`), `Related` (related mod + component), and (if present) `Conflict Graph` and `Matched Rule` (the matching rule expression).
- Divider.
- **Paths / Links** section — same as the mod-header view.
- **Package** section — same as the mod-header view, with the same action buttons.
- Divider.
- **Component Block** — a collapsible row (chevron `▾`/`▸` toggles) with a copy-icon on the header. When open, shows a multi-line monospace excerpt of the TP2 source for this component (FiraCode Nerd, fontSize 11). Default state: **open**.
- **WeiDU Line** — another collapsible row with a copy-icon. When open, shows the single WeiDU line for this component in monospace (e.g., `~EEUITWEAKS/EEUITWEAKS.TP2~ #0 #1000 // Mods Options: 4.0.7`). Default state: **collapsed**.

#### 6.8.4 `DetailsRow` behavior (shared)

- Three columns: label | value | actions.
- **Value column** takes the remaining width and ellipses on overflow. **Clicking the value copies it** and triggers a transient "Copied!" inline tag.
- **Actions column** (right-aligned, fixed): hover-revealed `copy` icon (always for copyable values) + `open` icon (for paths and URLs). Both use Heroicons-style SVG (no font dependency).
- The "Copied!" tag stays visible until the next mouseleave — clicking either the value or the copy icon shows it. Click the **open** icon to launch the path/URL in the OS handler.
- Rows with `actions={[]}` or `copyable={false}` (e.g., the `Checked`, `State`, `Reason` rows) skip the action icons and the value is informational only.

### 6.9 Behaviors and edge cases

- Search filters the tree as the user types. Subcomponents whose parent doesn't match still appear if they match. Expand/collapse state is preserved.
- Collapsed state is per `(gameTab, tp2)` and per `(gameTab, tp2, parent name)` so switching tabs doesn't blow away expand state.
- Checked state is **global** across the workspace (lives at WorkspaceView level), so:
  - Toggling in Step 2 immediately changes Step 3's order array.
  - Toggling off removes the item from the order. Toggling on appends to the end.
  - Step 4's review reflects the same data.

### 6.10 Confirmation dialogs

`ConfirmDialog` modal pattern (shared component): backdrop, sketchy box, title, message, Cancel + primary Confirm button (red if `danger`).

Step 2 uses ConfirmDialog only for **Select via WeiDU Log** (since it's destructive — it replaces all selections on the tab). Rescan is non-destructive and has no dialog.

### 6.11 Update Check popup (`UpdatesPopup`)

> **BIO-fidelity for the Updates popup** — see [BIO-reuse rule](#1-overview--vision). **Use the existing BIO Update Check window** (`src/ui/step2/update_check/update_check_popup_step2.rs`); re-skin with the new theme tokens; ship. Don't rebuild the popup from scratch.
>
> Specifically, don't re-introduce things the wireframe added during exploration: no tone-coded section headers, no per-row version transitions in the lists, no extra per-row action labels (`Open`, `Open Folder`), no tab indicator or status text in the title bar. BIO has none of these, and BIO is the source of truth here.

Opens from the toolbar's **`Updates...`** TopButton ([§6.4](#64-toolbar)).

#### 6.11.1 Modal chassis

- Centered modal over a `rgba(0,0,0,0.55)` backdrop. Click backdrop to close.
- Width: `min(780px, 94vw)` (wide enough for the 5-column Source Choices grid). Max height: `82vh`. Three-region layout: **header** (top), **scrollable body** (middle, flex 1), **footer** (bottom).
- Sketchy chassis: 1.5px border, `var(--shell-bg)`, `5px 5px 0 var(--shadow)` drop shadow.

#### 6.11.2 Header

- Title only (18px weight 500): **`Check Updates`** (default modes) or **`Check Mod List`** (when the modlist's install mode is `install_exactly_from_weidu_logs`).
- No tab indicator, no status text, no action button. (BIO's header is just the title; status lives in the body — see §6.11.4.)
- Bottom border: `1.5px dashed var(--border-soft)`.

#### 6.11.3 Body — Source Choices grid

The first major body element. **Lists every mod on the active tab that has at least one configured source** (not just mods with multiple alternatives). Each mod gets a row showing its current source plus per-row actions. Mods with a single configured source still appear here, with a single-option dropdown — so the user can confirm or edit before running the check. Mods with no entry at all are excluded; they show up later in the `No Source Entries` section.

The section header reads `Source Choices (<N>)` in the same flat `var(--rail-bg)` style as every category section.

5 columns wide: **mod name | source dropdown | Edit Source | Open Source | Discover Forks**.

- **Mod name** — left, plain text.
- **Source dropdown** — an egui `ComboBox` listing the mod's configured source IDs (e.g., `gibberlings3`, `weasel-fork`). Default = the source currently selected for this mod. Changing the value updates which source the check phase queries for this mod.
- **`Edit Source`** — opens the source-editor modal for the currently selected source ([Appendix A.5](#a5-mod-source-editor-popup--asset-picker)).
- **`Open Source`** — launches the source's URL in the OS browser.
- **`Discover Forks`** — opens the GitHub-forks modal to find alternative repos.

All three buttons disable while a check / download / extract is in progress.

#### 6.11.4 Body — Status text

A single status line at the top of the body (above the category sections), color-coded muted/success:

- Before any check (scanned-mods): `No update check run yet.`
- During check: `Checking updates / missing mod sources <done>/<total>` — or in exact-log mode: `Checking missing mod sources <done>/<total>`.
- During download: `Downloading update archives...` (or per-mode variant: `Downloading missing mod / update archives...`, `Downloading missing mod archives...`).
- During extract: `Extracting downloaded missing mods...` (or per-mode variant).
- After a successful check (status row hidden — the category sections speak for themselves).
- Selection drift (scanned-mods, after a check, selections changed): `Current selection differs from the last check. Run Check Updates again.`
- Exact-log mode, all good: `No missing mods found. Exact-log install is good to go.` (replaces all category sections).

#### 6.11.5 Body — Category sections

Below the status text and (optional) source-choices grid, category sections appear in fixed order. Each section is rendered as a labeled box: a flat section header (`var(--rail-bg)` background, sketchy border, 12px Poppins weight 500) showing the section name and count, followed by a 2-column grid (`1fr auto`) of rows. Each row has the mod's label on the left and a single action button on the right. **No tone-coding** of section backgrounds — all sections use the same neutral background; the row's category is conveyed by its position under the labeled header. **No version transitions** in row labels — just the mod name.

Sections (in fixed order; hidden when empty):

| Section title | Per-row action | Notes |
|---|---|---|
| **Updates** | `Edit Source` | Renamed to `Downloadable Missing Mods` in exact-log mode. |
| **Manual Sources** | `Edit Source` | Mods whose source requires manual download. |
| **No Source Entries** | `Add Mod` | Mods with no configured source. Action label changes to `Add Mod`. |
| **Source Check Failed** | `Edit Source` | The check phase failed for these mods. |
| **Downloaded** | `Edit Source` | Post-download confirmation list. |
| **Download Failed** | `Edit Source` | Post-download failures. |
| **Extracted** | `Edit Source` | Post-extract confirmation list. |
| **Extract Failed** | `Edit Source` | Post-extract failures. |

The per-row action is **always `Edit Source`** except for `No Source Entries` (and the `Missing` variant in exact-log mode), where it's **`Add Mod`**. No `Open` / `Open Folder` actions on rows (those are only on the global footer / source-choices grid in BIO).

#### 6.11.6 Footer

Horizontal row (`1.5px dashed var(--border-soft)` top border). Buttons left-to-right, matching BIO exactly:

1. **`Check Updates`** primary (`Check Mod List` in exact-log mode). Disabled while a check, download, or extract is running. Triggers the check phase.
2. **`Add Source`** — opens the source editor for adding a new mod source entry. Always enabled.
3. **`Copy Report`** — copies a text report of the current popup content to the clipboard. Disabled when no check has run.
4. **`Download Updates`** primary (`Download Missing Mods` in exact-log mode; `Download Missing Mods / Updates` if both categories have content). Disabled until at least one downloadable asset is available.
5. *Right-aligned (margin-left auto):* **`Close`** — dismisses the popup. Always enabled.

In exact-log mode, an additional **`Use Latest For Exact-Version Misses`** button appears when one or more mods have an exact-version mismatch; clicking it opens a small confirmation dialog (`Download Latest Instead?`) with Yes/No.

#### 6.11.7 Related popups

The Updates popup can launch three secondary popups (see [Appendix A.5](#a5-mod-source-editor-popup--asset-picker)):

- **Source Editor** (from `Edit Source`, `Add Mod`, `Add Source`): modal with a TOML editor for the mod's source block, plus Save/Cancel.
- **Discover Forks** (from `Discover Forks` in the source-choice grid): modal listing detected GitHub forks (repo / branch / updated / Open / Add Source).
- **Fallback confirmation** (exact-log mode, from `Use Latest For Exact-Version Misses`): yes/no confirm.

#### 6.11.8 Lifecycle

- The popup does not auto-run on open. Clicking `Check Updates` starts the parallel fetch from each mod's configured source.
- While the check runs, only the **Close** button stays interactive; other footer buttons disable.
- Closing the popup mid-check does not cancel the running fetch — re-opening shows live status.

---

## 7. Step 3 — Reorder and Resolve

The user has selected components in Step 2. Step 3 lets them put those components in install order, see exactly how they group by mod, and surface conflicts that depend on order. Drag-and-drop is the headline interaction.

### 7.1 Layout

`sk-page` flex-column. Above the box:

- Hint line: "Right-click a component for more actions, including uncheck and prompt tools." (Menu is BIO-fidelity per [A.6](#a6-step-3-right-click-context-menu); items: uncheck, set `@wlb-inputs`, edit prompt JSON, clear prompt data.)
- Action row above the tabs: right-aligned count "_N_ components ready to install on _\<tab\>_ · across _M_ mods" (no `Save weidu.log's` button — that lives in Step 4, see [§8.1](#81-layout)).
- Tab row (file-folder style, merged with the box below): game tabs + a right cluster with the aggregate pills (`N conflicts`, `N prompts`) + `Undo` / `Redo` / `Collapse All` / `Expand All` TopButtons.

### 7.2 Reorder list

> Today's BIO already implements drag-and-drop reordering in egui — see `src/core/app/state/state_step3.rs`, `src/ui/step3/state_drag_step3.rs`, and `src/ui/step3/service_drag_ops_step3.rs`. The redesign keeps that drag machinery and adds the rules below. Implement against egui's drag-and-drop response API (the existing crate's pattern), not via web HTML5 drag attributes.

Inside the box, the list is grouped by **contiguous runs of the same tp2** (not by mod identity). Each group renders as:

#### Mod header row (drag handle)

- `🔒` lock glyph (clickable; toggles whether the group can be dragged or split — locked mods stay grouped together; locked state is not wired in the wireframe yet but the glyph is there).
- `🔗 ▾` / `🔗 ▸` chevron — clickable, **collapses/expands the group's component rows**. Click is captured locally so it doesn't initiate a drag on the row.
- Mod display name. If this group is a split fragment of a mod that lives elsewhere, append a faint `(copy)` suffix; the third+ run uses `(copy 2)`, `(copy 3)`, etc. The first run of any tp2 in the list is the canonical "original".
- `(N)` component count and `v<version>`.
- The whole header row is a drag source. Dragging it moves all of the group's components together as a block.

#### Component row

- Drag handle `≡` (grab cursor on hover).
- Order number `<idx + 1>.` — derived from the row's position in the order array, auto-renumbered on every reorder. Column auto-sizes to the largest number.
- WeiDU-style line, colored per [§6.7](#67-weidu-log-syntax-coloring-step-3--step-4-only).
- Pills (conflict / prompt) inline after the line.
- Row is a drag source. Click selects (teal highlight). Shift-click extends selection to a contiguous range (across mods if needed). Single-row drag moves just that row; if the dragged row is part of a multi-row selection, all selected rows move together as a block.
- (Ctrl/Cmd-click is **intentionally not bound**. The wireframe explicitly rejected it after testing — it created too much confusion in mod-manager-style lists. Range selection via shift-click is the only multi-select gesture.)

#### Drop indicators

- A 2px teal drop-line renders **above** the hovered row when the cursor is in the top half of that row, **below** when in the bottom half.
- The first row of each group **never** shows an above-line — instead, the group's header gets a top drop-line (the "drop above this whole group" affordance).
- The 12px visible gap **between groups** is itself a drop zone: hovering it shows a teal drop-line; releasing it drops the selection there (which equals "above the next group" / "after the previous group").

### 7.3 Reorder rules

These are the user-visible promises of the drag system:

1. **All of a mod moved → parent moves too.** Dragging an entire mod (via the mod header, or by selecting all its components) preserves its grouping at the new location with no `(copy)` suffix.
2. **Partial of a mod moved → spawns a `(copy)` group.** Dragging some (not all) components of mod X out leaves the rest as the canonical `X` at the original spot and creates an `X (copy)` group at the drop location with the moved components.
3. **(copy) group dragged back next to original → merges.** Because grouping is computed from contiguous tp2 runs, any time two same-tp2 runs become adjacent they collapse into a single group and the `(copy)` suffix disappears.
4. **Reorder within a parent.** Dropping inside the same group just rearranges order; no split happens.
5. **Auto-renumber.** The `1. 2. 3. …` order column is recomputed on every change.

### 7.4 Collapse / expand state

- Per-group, keyed by `${tp2}:${first-item-id}` (stable across reorders as long as the first item doesn't change).
- The `Collapse All` / `Expand All` toolbar buttons apply to the active tab.
- A collapsed group still works as a drop target via its header; the user doesn't need to expand it first.

### 7.5 Selection model

- Plain click selects exactly that row.
- Shift-click extends selection from the **anchor** (last plain-click) to the clicked row, inclusive. This range can span across mods.
- Selection persists when dragging — the moved block is the selected rows.
- Switching tabs **resets** selection (different orderings, different anchors).

### 7.6 Action row buttons

- **Undo / Redo** — wired to a stack of order snapshots per tab (today's BIO already has Step 3 undo/redo state). **Step 3 only**; Step 2 checkbox toggles do not participate in undo/redo and have no undo affordance.
- **Collapse All / Expand All** — toggle every group for the active tab.

(`Save weidu.log's` lives only in Step 4 per [§8.1](#81-layout) — the wireframe's mislabeled `order` sub-tab hint is a wireframe bug; the content of that panel is Step 4.)

---

## 8. Step 4 — Review

A read-only snapshot of the final install order. The least interactive of the four workspace steps.

### 8.1 Layout

`sk-page` flex-column:

- Action row above the tabs: **`Save weidu.log's`** button + right-aligned count "_N_ components ready to install on _\<tab\>_ · across _M_ mods". The button's save behavior is **BIO-fidelity** — it triggers the same save action that today's BIO Step 4 invokes (`crate::ui::step4::service_step4` save flow + the existing save-error popup). The redesign only relocates the button into the new top-of-step action row.
- Tab row (game tabs only; no toolbar pills on this step).
- Box containing a scrollable monospace list. For each item:
  - Right-aligned line number (1, 2, 3, …; column auto-sizes to total count; no leading zeros).
  - WeiDU-style line with the three-color encoding from [§6.7](#67-weidu-log-syntax-coloring-step-3--step-4-only).

If no components are selected for the active tab, the box shows `No components selected on <TAB>.` in faint type.

### 8.2 Exact-log mode (parity with today's BIO)

When the modlist's install mode is `install_exactly_from_weidu_logs`, Step 4 becomes a read-only viewer of the source WeiDU log files. The user cannot edit; they can only **Check Mod List** (which triggers the same update-check flow as Step 2's `Updates...` button). BIO-fidelity per [A.7](#a7-step-4-save-flow--exact-log-mode).

---

## 9. Step 5 — Install

> **BIO reuse:** Step 5 is the most reuse-heavy section. The install runner (`mod_installer` dispatch), embedded terminal (`third_party/egui_term/`), WeiDU child-process plumbing, stdout streaming, prompt detection + auto-answer engine, `prompt_answers.json` persistence, and diagnostics-bundle generator (`diagnostics/run_<timestamp>/`) all stay exactly as they are in today's BIO. The redesign **adds** (in net-new components per the CRITICAL DIRECTIVE) the surrounding UI chrome: the success banner, the post-install **Share import code** / **Return to Home** / **Open install folder** CTAs, the `WorkspaceNavBar` lock behavior, and the in-progress → installed registry transition. BIO's Step 5 code is not modified — the new chrome wraps/composes it.

The runtime. WeiDU runs in a child process; BIO streams stdout into the console panel, detects prompts, and auto-answers from memory + scripted inputs.

### 9.1 Pre-install layout (`FinalPlanPanel` — `installComplete=false`)

`sk-page` flex-column:

- Hint Label (success-green): "Dev Mode: RUST_LOG=DEBUG selected." — only shown in dev mode.
- Two-card row: **Command** (left) + **Summary** (right).
  - **Command** card lists the full WeiDU command line that will run. Each flag on its own line. A `Copy` button copies the rendered command to the clipboard.
  - **Summary** card is a 2-column grid: Game Install / Mods Folder / WeiDU binary / Language / BGEE log (and BG2EE log for EET).
- Action row: **`Install`** (primary), `Actions`, `Diagnostics`, `Prompt Answers`, plus a row of console-filter labels (`☑ General`, `☐ Important Only`, `☐ Installed Only`) and `☑ Auto-scroll`.
- Console box — `Console output appears here while mod_installer runs...` placeholder.

### 9.2 Post-install layout (`installComplete=true`)

When the install finishes successfully:

- A success banner replaces the dev-mode hint: a `Pill` styled in success-green saying `Installed`, plus `<N> mods · <C> components · no errors` and a right-aligned `ran <MM:SS> · finished <relative time>`.
- The Install button becomes a disabled **`✓ Installed`** TopButton.
- Two new primary actions appear next to the disabled Install button: **`Return to Home`** (navigates back to Home — the freshly-installed modlist now appears in the "installed modlists" Box) and **`Open install folder`** (reveals the modlist's destination folder in the OS-native file manager via `rfd`).
- The console shows the actual install transcript (success lines in green, info in normal text, the final `[install] all <C> components installed in <MM:SS> · 0 errors · 0 warnings`).
- The **`Share import code`** button in the workspace header (which was disabled across Steps 2–4 and the unfinished Step 5) flips to a **primary teal CTA**. Clicking it opens `SharePasteCodeDialog` (see [§10.3](#103-sharepastecodedialog)).
- The workspace's **`← Previous`** nav button (`WorkspaceNavBar`) is **disabled** once Install has been clicked — even before the install completes, and after it completes — to prevent the user from rolling back order/selection while the install is running or after it's already produced an installed game. A tooltip explains the lock. The user can still freely click the workspace progress bar's earlier steps for read-only inspection if the design allows that (today's wireframe progress bar is not clickable, so this is a moot consideration).
- **The modlist transitions from in-progress to installed** in the registry. The next time the user visits Home, the entry has moved from the "in-progress builds" Box to the "installed modlists" Box, with an updated meta line (`<N> mods · <size> · installed <relative time>`). The transition fires only on a clean exit (exit code 0 and no errors detected); cancelled / failed / partial installs keep the modlist in the in-progress section. Graceful cancels (Force unchecked in the cancel modal — see [Appendix A.10](#a10-cancel-install-confirmation-modal)) keep `resume_available` true so the user can resume via Continue Partial Install; force cancels clear `resume_available` and the install must be restarted from scratch.

### 9.3 During-install layout (`InstallProgressScreen`, used when entered from Install Modlist)

When the install is entered from the Install Modlist top-level (not from inside the workspace), the screen uses `InstallProgressScreen` (covered in [§4.4](#44-stage-4--installing)). Inside the workspace, the same content sits underneath the Step 5 chrome:

- Status row: `Installing` pill, "Component _N_ / _T_", elapsed runtime, console-info hint.
- Action row: `Cancel Install` (primary), `Actions ▾`, `Diagnostics ▾`, `Prompt Answers`, plus filter radios and auto-scroll toggle.
- Live console.
- Prompt input row at the bottom (`Type a prompt response:` + input + `Send`).

### 9.4 Behavior

The install flow itself follows today's BIO. Only one install can run at a time across the whole app — see [§13.15 Install concurrency policy](#1315-install-concurrency-policy).

1. The user clicks `Install` — Step 5 spawns `mod_installer` with the configured command line. The console starts streaming.
2. Each WeiDU prompt is detected and matched against the auto-answer memory (`prompt_answers.json`) or scripted `@wlb-inputs` inline tokens.
3. If a match is found, BIO sends the answer after configurable delays (initial + post-send, both configurable in Settings → Advanced).
4. If no match is found, BIO surfaces the prompt in the input row and (optionally) plays a sound cue (Settings → Advanced).
5. The user can cancel — pops a confirmation modal with a "Force" checkbox (force = hard kill, no graceful shutdown).
6. On exit code 0 and no errors → success state per [§9.2](#92-post-install-layout-installcompletetrue) (the `Installed` Pill, success-coded console lines, `Share import code` unlocks as primary CTA). Behavior on nonzero exit / detected errors is BIO-fidelity — Resume Install / Restart Install / Open last log surfaces all per [A.9](#a9-resume-install--restart-install-states) and [A.11](#a11-step-5-actions-and-diagnostics-dropdowns).

### 9.5 Actions menu

BIO-fidelity per [A.11](#a11-step-5-actions-and-diagnostics-dropdowns). Items:

- `Copy Console` — copies the full console text to the clipboard.
- `Save Console Log` — exports the console to a file in the working `logs/` folder.
- `Open Logs Folder` — opens the configured logs folder in the OS file manager.
- `Clear Console` — empties the console (does not affect the running install).
- `Open last log file` — only when the previous install failed; opens the log that captured the error.

### 9.6 Diagnostics menu (dev mode only)

BIO-fidelity per [A.11](#a11-step-5-actions-and-diagnostics-dropdowns). Items:

- `RUST_LOG level` radio: `Off`, `DEBUG`, `TRACE`.
- `Full Debug` toggle.
- `Raw Output` toggle.
- `Export diagnostics` (or `Restart App With Diagnostics` outside dev mode) — writes the diagnostics bundle described in [§13.10](#1310-diagnostics-export).

### 9.7 Prompt Answers window

Opens from the **`Prompt Answers`** button. A modal-window listing every entry in `prompt_answers.json`:

- Each row: component (mod #id), parsed prompt preview, current answer, enable toggle, hit count, edit button.
- Bottom controls: `Import`, `Export`, `Clear All`.
- Per-row context: `Edit Prompt JSON` (advanced manual entry), `Clear` (remove this entry).

BIO-fidelity per [A.8](#a8-step-5-prompt-answers-window) — reuse the existing modal as-is.

---

## 10. Shared dialogs

All dialogs listed here are **non-blocking `egui::Window` popups** — BIO-fidelity. They are centered, draggable, and dismissible by clicking a `Close` / `Cancel` button, but they do **not** block interaction with the rest of the app behind them. The wireframe's full-screen backdrop overlay is a wireframe-rendering convention only — the egui implementation must follow today's BIO popups (`compat_window_step2.rs`, `prompt_popup_step2.rs`, GitHub OAuth popup) which use non-modal `egui::Window`. No backdrop, no focus trap.

**Collapse chevron (global popup pattern, redesign addition).** Every popup in the redesign shows a small `▾` chevron in the top-left of its header row. Clicking it collapses the popup body, leaving only the header bar visible; clicking again expands it. Visual: rotated `▾`/`▸` glyph in `var(--text-muted)`, sized ~18×18, sits left of the title with 8px spacing. State is per-popup-instance and resets on close.

**Collapse direction (anchor the header under the cursor).** On collapse, the popup must **anchor its top to its current position** — the header (and chevron) stays exactly where the user clicked it; only the body shrinks away upward from below. The popup must **not** re-center vertically when its height changes. In egui, this means capturing the current window's top-Y on the collapse transition and pinning the window there for the duration of the collapsed state (instead of letting the layout re-flow to center). On expand, restore the window's normal positioning logic.

This **diverges from today's BIO**, which sets `.collapsible(false)` on every `egui::Window`. Per the CRITICAL DIRECTIVE's window-chrome-flip carve-out, flipping those calls in place to `.collapsible(true)` is an allowed mild refactor — egui then renders the built-in title-bar collapse chevron natively, no wrapper required. The popup body, signatures, and behavior stay identical.

The **anchor-on-collapse** behavior (above) requires verifying egui's default. If egui's native collapse already anchors the window's top-Y (standard behavior for title-bar-driven collapse, since the title bar stays put), no additional code is needed. If egui re-centers on height change, a thin wrapper that captures the top-Y at collapse and pins via `Window::current_pos` / `fixed_pos` is the right surface for the anchor logic.

### 10.1 Save Draft (inline, no dialog)

The workspace header's **`save draft`** button (Steps 2–4) does not open a modal. Clicking it persists the current workspace into the **in-progress builds** registry ([§3](#3-home), [§13.1](#131-modlist-persistence)) and flips the button label to **`✓ saved!`** inline for ~1.6 seconds, then reverts to `save draft`. The button has a hover tooltip: "Save this in-progress build so you can resume it from Home".

The save is **silent and destination-free**: there is no folder picker and no filename input. The build is identified by the modlist name shown in the workspace header (`Editing <name>`); subsequent saves overwrite the registry entry for that build. After saving, the build appears in:

- Home → `in-progress builds` Box ([§3.1](#31-layout)), and
- the **Resume in-progress build** dialog ([§10.2](#102-resume-in-progress-build-loaddraftdialog)).

If the user wants a portable on-disk artifact instead of (or in addition to) registry persistence, the **`modlist-import-code.txt`** that gets written to the destination on install start ([§13.13](#1313-import-code-auto-generated-on-install-start)) covers that use case — the user can copy that file anywhere or share it.

### 10.2 Resume in-progress build (LoadDraftDialog)

Modal opened from Create's `load draft` button:

- Title: "Load draft"
- Sub: "Pick a saved .txt draft. BIO reads the embedded share code and drops you straight on Step 2."
- `FolderInput` labeled "draft file" with `browse`.
- Preview card (appears once a file is chosen): name / game / mod count / component count from the parsed draft.
- Footer: `Cancel` + **`Load → Step 2`** primary, disabled until a file is selected. On click: opens the workspace pre-populated.

### 10.3 SharePasteCodeDialog

Modal opened from Step 5's `Share import code` button (only after a successful install):

- Title: "Share import code"
- Sub: "Anyone can paste this into BIO → Install to get the same modlist."
- A monospace, scrollable box containing the BIO-MODLIST-V1 code. **The code's `allow_auto_install` flag is `true`** (set by `flip_to_installed` per [§13.3](#133-share-code-bio-modlist-v1)) because this dialog only opens after a verified successful install.
- Footer: `Close` + **`Copy`** primary. On click: writes to clipboard, shows `✓ copied to clipboard` inline next to the buttons for ~1.5s.

### 10.4 CompatPopup

> **BIO-fidelity for the Compat popup** — see [BIO-reuse rule](#1-overview--vision). **Use the existing BIO compat window** (`src/ui/step2/compat/compat_window_step2.rs` + `compat_popup_step2.rs`); re-skin with the new theme tokens; ship. Don't rebuild it from scratch.
>
> Don't re-introduce the wireframe-only chrome that crept in during exploration: no Pill in the header (issue kind already shows up in the body's Kind row), no `tp2` sub-line, no aggregate counter — none of those are in BIO. The same window object serves both per-row pill clicks and the toolbar/mod-row aggregate badge; the only difference is which issue is selected when it opens.

#### Modal chassis

- Centered modal over a `rgba(0,0,0,0.55)` backdrop. Width: `min(620px, 94vw)`. Max height: `82vh`.
- Three-region layout: **header** / **scrollable body** / **footer**, separated by `1.5px dashed var(--border-soft)` dividers.

#### Header

- Title only: **`#<id> <component name>`** (bold, 16px weight 600). Matches BIO's `compat_popup_step2.rs:87-97`.
- **No** kind Pill, **no** tp2 sub-line, **no** aggregate counter — those are not in BIO. The issue kind shows up in the Kind row in the body; aggregate navigation uses the `Next →` footer button.

#### Body — issue details

Section rows (each a faint hand label + value):

- **Status** — only shown when the issue kind has a non-null status (excludes `included`, `not_needed`). Color matches tone: red coral for danger, amber for warn, teal for info, muted for neutral. Labels: `Resolve before continuing` (danger), `Warning only` (warn / info).
- **Kind** — the human label (Conflict / Mismatch / Install Order / Missing Dep / Path Requirement / Conditional / Deprecated / Warning / Included / Not needed / Not compatible).
- **Summary** — short monospace summary line (e.g., `Only available on \`BG2EE, EET\``). Shown only when populated.
- **Reason** — multi-line human explanation. Shown only when populated.
- **Related** — `<related mod> #<related component>` in monospace. Shown only when the issue references another component/mod.
- **Rule source** — TOML filename (monospace, faint). Shown only when populated.
- **Component block** — a collapsible row (chevron `▾`/`▸`) showing a multi-line TP2 source excerpt for the conflicting component. Default state: closed.

#### Body — filter row

Below the details, a `Filter by category` label and a wrapping row of `Btn small` buttons:

`All` · `Conflict` · `Order` · `Mismatch` · `Missing` · `Included` · `Path` · `Conditional` · `Deprecated` · `Warning` · `Other`

- Each button shows the count for that category in parentheses when nonzero (e.g., `Conflict 3`).
- The active filter is rendered primary teal.
- Buttons whose category has zero issues on this tab are disabled.
- Changing the filter resets `Next` cycling to the first matching issue.

#### Footer — action row

Left-to-right: **`Jump To This`** (focus the selected component in the tree), **`Jump To Related`** (focus the related mod/component; disabled when no related target), **`Next →`** (cycle to the next issue matching the current filter; disabled when `list.length ≤ 1`), **`Open Rule Source`** (opens the rule's TOML file in the OS editor; disabled when no rule source). Right-aligned: **`Close`**.

#### Aggregate mode behavior

When opened via the toolbar `<TAB> Mismatch <N>` badge or a mod-row `N conflicts` pill, the popup pre-selects the dominant filter (e.g., `Mismatch` for the toolbar badge) and lands on the first matching issue. `Next` cycles wraparound through the filtered list. Switching the filter resets the index to the first matching issue.

### 10.5 PromptPopup

> **BIO-fidelity for the Prompt popup** — see [BIO-reuse rule](#1-overview--vision). **Use the existing BIO prompt window** (`src/ui/step2/prompt/prompt_popup_step2.rs`); re-skin with the new theme tokens; ship. Don't rebuild it from scratch.
>
> Don't re-introduce the wireframe-only chrome that crept in during exploration: no Pill in the header, no separate sub-line — the literal BIO title (`Parsed prompts - <tp2>.tp2 #<id>` for single mode, `Prompt Components (<TAB>)` for aggregate) is the entire header. The two `PromptPopupMode` values (`Text` and `ToolbarIndex` — see `state_step2.rs`) drive which content renders, identical to BIO today.

#### Mode `single` (per-component)

Triggered from the leaf-row Prompt pill.

- **Window size:** `min(700px, 94vw)` × max `82vh`. (BIO uses 700×320 resizable.)
- **Header**: single literal title — **`Parsed prompts - <tp2>.tp2 #<id>`** (matches `prompt_popup_step2.rs:24`). The tp2-and-id portion is monospace (FiraCode Nerd). **No** Pill, **no** separate sub-line.
- **Body**:
  - Hand label `Prompt summary from Lapdu parser:` (verbatim BIO).
  - A box containing the parsed prompt summary. The summary text is multi-paragraph; first line is `Component: <id> - <label>`, then the parsed prompt question/answer blocks. In production this content comes from `evaluate_component_prompt_summary` and may include up to 6 deduplicated event blocks; Y/N validation-retry blocks are filtered out when a valid Y/N question is present.
  - **Jump to component** section (only when the parsed prompts reference other components by ID). Section header `Jump to component` (hand label), followed by a horizontally-wrapping row of small buttons. Each button renders the component ID in FiraCode Nerd monospace at `#2f6fb7` (dark blue), min-width 42px. Click jumps to that component in the tree.
- **Footer**: `Close` only.

#### Mode `aggregate` (toolbar)

Triggered from the toolbar `PROMPT <N>` pill.

- **Window size:** `min(420px, 94vw)` × max `82vh`. (BIO uses 420×320 resizable.)
- **Header**: single literal title — **`Prompt Components (<TAB>)`** (matches `toolbar_actions_step2.rs:82-86`). No sub-line.
- **Body**: vertical list of mod entries on the active tab. Each entry is a collapsible group (chevron `▾`/`▸`, default open):
  - Header row: rail-bg block with the mod name + `(<count>)` in faint type.
  - Expanded: horizontally-wrapping row of small component-ID buttons (same styling as the `single`-mode jump buttons). Click jumps to that component in the tree.
- **Footer**: `Close` only.
- Empty state: when there are no checked components with prompts on the active tab, body shows `No components with prompts on this tab.` in faint type.

### 10.6 PillPopup

Generic single-page modal used for one-off informational popups (not used by compat or prompt — see §10.4 and §10.5):

- Title row.
- Body: multi-paragraph or list-formatted text, monospace where appropriate, scrollable.
- Footer: just `Close`.
- Click-outside-to-dismiss.

### 10.7 ConfirmDialog

Generic confirm/cancel modal:

- Title + message.
- Footer: `Cancel` + primary `Confirm` (red-tinted when `danger`).
- Used for destructive actions like `Select via WeiDU Log`.

### 10.8 Toast

Bottom-center transient notification (no dismiss button). Used for clipboard copies and any other "this happened" feedback. Auto-dismisses ~1.8s.

---

## 11. Settings

A dedicated top-level screen with **five sub-tabs** (file-folder style). The active tab merges with a single Box that fills the remaining vertical space.

Each tab's body is independent. Settings persist immediately (no Save/Cancel buttons — the wireframe deliberately removed those after testing).

### 11.1 General

A `NameRow` at the top, then a 2-column grid of four settings:

#### Your name row (NameRow)

- Default display: name (e.g. `@b2bs`) + **`edit`** button. When no name is set, the value reads `click to set your name` in faint text (a deliberate affordance-forward choice over the wireframe's neutral `(not set)` — it tells a first-run user what to do). Hint: "credited as the author on any modlists you create or share".
- Edit mode: real text input (200px wide), placeholder `@yourhandle`, **`save`** primary + `cancel`.
- The NameRow renders inside the same `SettingsRow` chassis as the other General rows (label + hint stack on the left, control flush-right, dashed bottom rule) — it is not a bespoke standalone block.
- The configured name is used as the `author` field whenever the user creates a share code.

#### Other rows (2-col grid)

- **Theme** — segmented `light` / `dark` (primary indicates active). Hint: "light parchment or warm dark".
- **Language** — dropdown (egui `ComboBox`) of UI languages: `English` (default), `German`, `French`, `Spanish`, `Italian`, `Polish`, `Portuguese`, `Czech`, `Turkish`, `Ukrainian`. Hint: "language used across the BIO app". A faint `(coming soon)` label sits to the left of the ComboBox indicating that the selection persists but doesn't yet drive rendering (v1 alpha ships Latin-only Poppins and no i18n surface; the picker stays writable so the user's eventual choice survives the i18n rollout). (This is distinct from the **install language** used by WeiDU; see [§11.5](#115-advanced).)
- **Validate all paths on startup** — Toggle, default on. Hint: "warns if game folders moved".
- **Diagnostic mode** — Toggle, default off. Hint: "extra logging for bug reports".

### 11.2 Paths

Two labeled sections. Each section header renders in uppercase, muted color, 13px Poppins-medium (`GAME SOURCES`, `WORKING FOLDERS`) — the source strings are uppercased directly rather than via a runtime transform.

**`GAME SOURCES`** — PathRow entries (row labels are terse, matching the wireframe):

- `BGEE source` (e.g. `C:\GOG\Baldurs Gate Enhanced Edition` on Windows, `/Users/<you>/Library/Application Support/Baldur's Gate Enhanced Edition` on macOS, etc.)
- `BG2EE source`
- `IWDEE source` — empty until the user sets it

**`WORKING FOLDERS`** — PathRow entries:

- `Mods archive`
- `Mods backup`
- `Temp` (defaults to `%TEMP%\infinity-orch` on Windows, `$TMPDIR/infinity-orch` on macOS/Linux — auto-created on first install)

The wireframe also shows a `Tools` working-folder row; it is intentionally **not** rendered here — it conflates with the dedicated Tools sub-tab (which owns the binary paths). This Paths tab is scoped to game sources + working folders only (per Phase 4 plan P4.T3).

Each row is two lines tall:

- Top line: label (fixed width) + mono value field + `browse` button. The input fills whatever horizontal space is left between the label and the browse button.
- Bottom line: a fixed-height status slot aligned under the input column. When the field has a `Warning` or `Error` status, the row's specific reason text renders here in the matching color. When the field is `Empty`, `Ok`, or mid-revalidation, the slot stays blank — the input's border tint already carries the at-a-glance signal.

The status slot is always reserved (even when blank) so rows do not jitter when validation flips state.

**Path separators are OS-native.** Paths are displayed, stored, and validated using the host OS's native separator: `\` on Windows, `/` on macOS/Linux. The wireframe mocks use Windows-style backslashes throughout because the demo data is from a Windows install; the egui implementation should use Rust's `std::path::Path` and let the OS dictate separators. The `browse` button opens the OS's native folder picker (today's BIO already uses the `rfd` crate for this — keep that).

**Validation is automatic.** There is no explicit `Validate now` button. Path validation runs:

1. On app start (when `Validate all paths on startup` is on — see [§11.1](#111-general)), and once unconditionally in `OrchestratorApp::new` so prefilled paths show their inline status the moment Settings is opened.
2. Continuously while the user is editing in Settings → Paths, debounced ~200ms after each edit settles. During the debounce window the row's status slot reads `checking…` so the user has visible confirmation that validation is queued (egui paints lazily, so the orchestrator explicitly requests a repaint when the soonest debounce timer will fire — without this, an in-flight validation could appear to stall for several seconds until the user's next mouse or keyboard event).
3. Once before each install, in the install runner — see "Pre-fetch automatic validation" below.

The startup-validation toggle gates (1) only; (2) and (3) are unconditional.

**Per-row validation model.** Each path has one of four states. Visual treatment splits between the input's border tint (instant at-a-glance signal) and the status slot below the input (specific reason text only for `Warning`/`Error`):

| State | When | Visual |
|---|---|---|
| `Empty` | path not set | input border default, status slot blank |
| `Ok` | path is valid for its role | input border subtle success-tint, status slot blank |
| `Warning` | path is set and exists but suspicious for its role | input border subtle warn-tint, status slot `! <specific reason>` |
| `Error` | path is set but blocking-invalid (doesn't exist, or is a file when a folder is expected) | input border subtle danger-tint, status slot `× <specific reason>` |

While a debounced revalidation is pending (the 200ms window after an edit settles), the status slot reads `checking…` regardless of the previous state, and the input border stays neutral so the user doesn't see a brief red/green flash on a stale result.

The error message in `Warning`/`Error` describes the specific problem (`× not a folder`, `! no chitin.key/lang — not a recognizable game install`, `! looks like a game install — pick an empty folder`, etc.). There is no aggregate `× N path issues` summary at the bottom — each row carries its own feedback.

**Role-specific rules.** Each field validates against rules that match what the path is actually used for:

- **Game folders** (BGEE / BG2EE / IWDEE): `Ok` when the path exists, is a directory, and contains both `chitin.key` and a `lang/` subfolder (Infinity Engine install marker — same check as BIO's `state_validation_fs::check_game_dir`). `Warning` when the path exists but those markers are missing — the install may still work, but BIO can't confirm this is a game install. `Error` when the path is set but doesn't exist or is a file.
- **Working folders** (Mods archive / Mods backup / Temp): `Ok` when the path exists and is a directory that does NOT look like a game install (no `chitin.key`). `Warning` when the path doesn't exist yet (it will be created on first install) OR when it exists with a `chitin.key` (the user likely picked their game folder by mistake). `Error` when the path is set but is a file.

`Warning` is non-blocking: users with non-standard installs (custom layouts, moved markers) can proceed. `Error` is shown but does not block save — BIO's install runner still gates the actual install with the pre-fetch check below.

**Pre-fetch automatic validation.** Independently of the per-row checks, the install runner re-validates the modlist's destination folder + every referenced source path **once** before performing any downloads or extractions for that install. This catches missing-game-folder / unwritable-destination errors before BIO touches the network or unzips anything. The check runs on Step 5 install start, between the Install click and the first fetch.

**EET-specific paths** — Pre-EET dir, EET final dir, BG1 source/log per phase, BG2 source/log per phase. Not exposed as user-configurable fields. The install runner creates them with standard names inside the modlist's destination folder per [§13.12 policies #3–#4](#1312-automatic-flag-policies); see also [Appendix A.19](#a19-eet-workflow-surfacing).

### 11.3 Tools

Two writable PathRows backed by `Step1Settings`, one per binary BIO's install runner actually invokes. Same two-line layout as the Paths tab (label + input + browse on top; status text below the input column). The validator treats absolute paths as filesystem checks (`is_file`) and bare names as `$PATH` lookups: a bare name that resolves on `$PATH` shows `ok · <resolved absolute path>` so the user sees exactly which binary will run; a bare name that does NOT resolve shows `× not on $PATH — install or specify the full path` in danger tone.

- **WeiDU binary** → `Step1Settings::weidu_binary`. Hint shows detected version when available (e.g., `v249`).
- **Mod installer** → `Step1Settings::mod_installer_binary`.

### 11.4 Accounts

Cards listing connected services. Each card is a single horizontal row inside a redesign Box (per wireframe `screens.jsx::AccountCard` line 3884–3917):

```
[36×36 avatar] [Service Name] [optional "as @user" when connected]   …push right…   [pill] [Btn]
```

- **Avatar** — shell-bg fill, sketchy 1.5px border, 2×2 drop shadow, initials in `poppins_bold`. (Neutral notebook-card treatment — not an accent-filled tile, which would visually conflict with the active rail item.)
- **"as @user" label** — faint hand-style text, only rendered when the service is connected. Sits between the service name and the right-anchored cluster. The widget always prepends `@` to the supplied user label and strips any duplicate leading `@`, so callers can pass either `Xgatt` or `@Xgatt` and the rendered string is always `as @Xgatt`.
- **Pill** — small chip, no border, just the tone-matched fill with rounded ends (matches wireframe `Pill` line 745: 10px font, ~7px radius, `pill_text` dark slate on tinted fill). Connected: `connected` in info tone. Not connected: `not connected` in neutral tone. The pill states the connection — the user identity lives in the separate `"as @user"` label.
- **Btn** — primary fill (accent + drop shadow) when the action is the call to action — i.e. when NOT connected (`connect`). Non-primary (shell-bg) when connected (`disconnect`). Small variant.

Services:

- **GitHub** (`GH`) — fully wired via `oauth_glue`. Connect runs the OAuth device flow today's BIO already implements (see [§13.2 GitHub OAuth](#132-github-oauth)). Disconnect clears the stored token.
- **Nexus Mods** (`NX`) — visual stub. The `connect` button is rendered **disabled** (50% alpha, click-suppressed) with a `coming soon` hover tooltip. No stub-hint banner — the disabled state speaks for itself.
- **Mega** (`M`) — same disabled-stub treatment as Nexus Mods.

Disabling the unimplemented services (rather than letting the user click and showing an inline "not yet implemented" hint) is a deviation from the wireframe — both buttons are enabled in the wireframe mock — but matches the user-facing behavior we ship: visibly disabled affordances are clearer than buttons that lie to you. See [A.17](#a17-nexus-mods--mega-account-connections) for the underlying integration deferral.

### 11.5 Advanced

A 2-column grid (`ui.columns(2, ...)` → exactly 50 / 50 split per the wireframe's `gridTemplateColumns: "1fr 1fr"`). Each row uses an end-capped layout: label left-aligned, optional unit hint to its right, input / toggle flush-right at the column edge. The control stops at the column boundary regardless of label or hint length so a long hint never pushes adjacent rows.

**Gate-absorbed value fields.** Every ValueRow in this section follows the absorb-the-gate pattern: today's BIO pairs each value with a boolean "enable" gate; the redesign drops the boolean. An **empty / cleared** value field means "use BIO's hard-coded default" (shown in the input's placeholder text, e.g. `default 5`); a **filled** value field means "override with this value". Same expressive power, half the controls. See [Appendix A.15](#a15-bio-step-1-toggles-not-in-the-wireframes-advanced-tab) for the dropped gates. **The placeholder numbers below are the live `Step1Settings::default()` values and are authoritative** — the wireframe mock's illustrative figures (`3`, `7200`, `1007`, …) are not; a placeholder that says `default N` must always equal BIO's actual runtime default for that field so the user sees what they'll really get on an empty field.

**Left column — Timing & limits** (ValueRow components):

- `Custom scan depth` (placeholder `default 5`)
- `Mod install timeout` (placeholder `default 3600`, hint `sec`)
- `Auto-answer initial delay` (placeholder `default 2000`, hint `ms`)
- `Auto-answer post-send delay` (placeholder `default 5000`, hint `ms`)
- `Tick (dev)` (placeholder `default 500`, hint `ms`)
- `Prompt context lookback` (placeholder `default 10`)

**Right column — Install behavior** (ToggleRow components). The labels below are the redesign's terse rewrites and are **authoritative over the wireframe mock's longer phrasings** (`Sound cue when prompt input is required`, etc.). The `Casefold filename matching` row is **intentionally present** even though the wireframe mock omits it — it surfaces an existing BIO capability the mock simply didn't draw.

- `Prompt sound cue` (hint `beep when a prompt needs you`, default on)
- `Download missing mods` (hint `fetch GitHub/Weasel/Morpheus during install`, default on)
- `Casefold filename matching` (hint `ASCII case-insensitive lookups`, default off) — useful when importing a `weidu.log` from a different OS

**Right column — WeiDU command-line flags** (ToggleRow components):

- `-a  abort on warnings` (default off)
- `-x  strict matching` (default off) — bound to `Step1Settings::strict_matching`. The `-s` (skip installed) flag is **not** a user-visible toggle here; it's controlled by the Install Modlist `continue partial install` workflow per [§13.12](#1312-automatic-flag-policies) #1.
- `-o  overwrite` (default off)

### 11.6 Removed from Settings (intentionally)

The wireframe explicitly removed these from earlier drafts:

- "Enable WeiDU logging options (-u)" master toggle — folded into the always-on log mode behind the scenes.
- "Advanced mode" toggle — replaced by Diagnostic mode (more honest about what it does).

---

## 12. Theming

The values listed below are **design tokens**, not CSS. Wire them into the existing `src/ui/shared/theme_global.rs` (colors) and `src/ui/shared/layout_tokens_global.rs` (spacing/borders). The wireframe expresses them as CSS custom properties because that's what the React preview understands; the egui implementation should expose them as Rust constants (`egui::Color32` for colors, `f32` pixels for spacing).

### 12.1 Tokens

Two palettes (light and dark). Dark is the default for the redesigned app, applied at app start via the theme system (today's BIO already has a theme module — see `theme_global.rs`):

**Light (default for new users):**

| Token | Value |
|-------|-------|
| `--page-bg` | `#e8eef5` |
| `--shell-bg` | `#f5f8fc` |
| `--chrome-bg` | `#cfdce8` |
| `--rail-bg` | `#dde6f0` |
| `--border-strong` | `#1a2638` |
| `--text` | `#1a2638` |
| `--success` | `#5fa86a` |

**Dark (default in the wireframe — teal-on-deep-slate):**

| Token | Value |
|-------|-------|
| `--page-bg` | `#0B1116` |
| `--shell-bg` | `#111A21` |
| `--chrome-bg` / `--rail-bg` | `#15222B` |
| `--border-strong` / `--border-soft` | `#24333D` |
| `--text` | `#E6EDF3` |
| `--text-muted` | `#A7B3BD` |
| `--text-faint` | `#6B7785` |
| `--success` | `#4ADE80` |
| `--accent` (default) | `#14B8A6` |
| `--accent-hover` | `#2DD4BF` |

The accent is fixed at the design-decided value — there is no user override surface in production. The Tweaks panel's accent picker exists only in the wireframe (see [§14.2](#142-tweaks-panel)).

### 12.2 Pill tones

| Tone | Background | Use |
|------|-----------|-----|
| `danger` | `#e69a96` (coral) | Conflict, Mismatch, blocking issues |
| `warn` | `#e8c441` (amber) | Prompt, deprecated |
| `info` | `#a8d2cc` (soft teal) | Conditional, Connected (accounts), Included |
| `neutral` | `#c4cad1` (warm grey) | Default |

All pills use dark text (`#1a2638`) regardless of tone.

### 12.3 Misc visual rules

- Selection highlight: `rgba(20,184,166,0.18)` (teal at 18% alpha); hover-on-selected: 22% alpha.
- Hover overlay (general): `--hover-overlay`, very subtle (~4–5% alpha overlay).
- Sketchy border: 1.5px solid `--border-strong`, 3px radius.
- Drop shadow: hard offset `6px 6px 0 var(--shadow)` for the main shell, `2px 2px 0` for primary buttons.

### 12.4 EmbeddedTerminal cell colors (out of v1 alpha scope)

The `EmbeddedTerminal` widget from `third_party/egui_term/` has its own cell-color handling (driven by ANSI escape sequences from WeiDU's output). Theme-token reskins do not reach inside the terminal — the cell colors stay as-is across theme switches. This is a known v1 alpha gap; matching the terminal cell colors to the redesign palette is out of scope for v1 alpha. The line-tone classifier in `status_console_step5.rs` (which colors lines *before* they're handed to the terminal) **is** in scope and is covered by Phase 8's carve-out #6.

---

## 13. Cross-cutting features (parity with today's BIO unless noted)

These are the back-end / cross-cutting features the redesign carries forward. Almost all of these are existing BIO features kept as-is (no UX changes); items flagged inline indicate where the redesign extends or rewires them. Items that are entirely new are called out explicitly.

### 13.1 Modlist persistence

What stays vs what's new at the file level:

| File | Status | Role in the redesign |
|------|--------|----------------------|
| `bio_settings.json` | **existing** | App-global settings (paths, tools, theme, language, advanced flags). No schema change required — the redesign reads from it via the existing `src/settings/` loader. |
| `prompt_answers.json` | **existing** | Per-component auto-answers. No schema change. |
| `step2_compat_rules.toml` (+ default) | **existing** | Compat rules. No schema change. |
| `mod_downloads_user.toml` (+ default) | **existing** | Mod sources. No schema change. |
| `modlists.json` | **new** | The registry the Home cards and Load Draft dialog both read from. **No migration** — existing BIO users start fresh; the old single-workspace state in `bio_settings.json` is not auto-converted into a registry entry. |
| `modlists/<id>/workspace.json` | **new** | Per-modlist workspace state (Step 2/3 order arrays, checked components, expand state, prompt overrides). One file per registry entry. |
| `app_bootstrap.rs` / `app_lifecycle.rs` / `app_update_cycle.rs` persistence machinery | **existing, extended** | Debounce/throttle + drop-time flush logic is reused; new code adds `modlists.json` and per-modlist workspace files to the same write paths. |

Behavior the new `modlists.json` registry must support:

- Per-entry record: name, game, destination folder, **state** (`in-progress` or `installed`), creation date, last-touched date, install date (when state == `installed`), last-played date (optional), mod count, component count, total size (when known), and the current BIO-MODLIST-V1 code.
- **State transitions:**
  - A modlist is created in **`in-progress`** state by any of: Create → New from downloaded mods, Create → Import and modify (after fork download), or the act of pasting an Install Modlist share code (when the user enters the workspace to review before install — currently the Install Modlist flow goes straight to install, so this is theoretical).
  - The state flips to **`installed`** on the first successful install ([§9.2](#92-post-install-layout-installcompletetrue)). The transition records the install date and refreshes the meta fields.
  - **Reinstall** (Home Kebab → Reinstall on an installed card — [§3.2](#cards-in-the-filtered-list)) flips the state from `installed` back to `in-progress`, wipes the install folder, and re-runs the install. On successful completion, the state flips back to `installed`. No edit to selection/order is allowed during this flow.
  - Cancelled / failed / partial installs leave the modlist in **`in-progress`** so the user can resume from the workspace (the order, selection, and any edits are preserved). Whether the *install attempt itself* is resumable via Continue Partial Install depends on cancel type: graceful → resumable; force → terminal (see [Appendix A.10](#a10-cancel-install-confirmation-modal)).
  - **No re-edit of an installed modlist** — once a modlist is in the `installed` state, the only path to changing its components is `Reinstall` (full destructive re-do) or `Delete` + create a new modlist. Re-edit is a later functionality, intentionally out of scope.
- Each modlist's `WorkspaceView` state (order arrays, checked components, expand state per group, prompt answer overrides, etc.) lives in its own per-modlist workspace file so that re-opening a modlist resumes where the user left off. This replaces today's BIO single-workspace in-memory state with a registry-indexed persisted state.
- The Home page section split — `in-progress builds` vs `installed modlists` — and the **Load Draft / Resume in-progress build** dialog ([§5.2](#52-load-draft-dialog-resume-in-progress-build)) are both views over the same registry filtered by state.

### 13.2 GitHub OAuth

**BIO-fidelity.** Reuse the existing popup (`src/ui/step1/github_auth_popup_step1.rs`) and runner (`src/core/app/app_step1_github_oauth.rs`) as-is, with the new theme tokens applied. The only change: trigger from the Settings → Accounts → GitHub card instead of the old Step 1 button.

### 13.3 Share code (BIO-MODLIST-V1)

The redesigned app **reuses today's BIO format** with one schema-additive field (per CRITICAL DIRECTIVE carve-out #5):

- `BIO-MODLIST-V1:<base64url(zlib(json))>`
- Payload includes: `format_version`, `bio_version`, `game_install` (BGEE/BG2EE/EET/IWDEE), `install_mode`, `weidu_logs` per game, `source_overrides` (a `mod_downloads_user.toml` excerpt), `installed_refs` (pinned tp2 → source + commit/tag), `mod_configs` (pre-supplied config files), and **`allow_auto_install: bool`** (see below).
- Generated post-install (Share import code dialog) and at draft / install-start time (with the bit set differently — see below).
- Consumed by Install Modlist, Create → Import-and-modify, and Load Draft.

#### `allow_auto_install` (schema-additive field, default `true`)

A boolean flag indicating whether the code is eligible for direct auto-install (a one-click install in the Install Modlist paste flow), or whether the recipient must fork it through Create → Import-and-modify to review/customize before installing.

**Default value:** `true`. Defaulting to `true` preserves today's BIO behavior — every existing pre-redesign share code (which has no field) is treated as auto-install-eligible, matching how today's BIO consumes them. New codes generated by Infinity Orchestrator opt into a more conservative posture by explicitly setting the bit (see below).

**When set to `false` by Infinity Orchestrator:**
- Save Draft (workspace header button, Steps 2-4) generates a code with `allow_auto_install = false`. The draft represents work-in-progress that hasn't been validated by an end-to-end install.
- `modlist-import-code.txt` auto-write at install start ([§13.13](#1313-import-code-auto-generated-on-install-start)) writes a code with `allow_auto_install = false`. At install start, the install is in-flight and hasn't completed successfully.
- The registry entry's `latest_share_code` field is stored with `allow_auto_install = false` for any in-progress modlist.
- Any code generated by the orchestrator before the modlist has reached the `Installed` state has the bit `false`.

**When set to `true` by Infinity Orchestrator:**
- `flip_to_installed` (the post-success registry transition — [§9.2](#92-post-install-layout-installcompletetrue)) **re-generates** the modlist's `latest_share_code` with `allow_auto_install = true`. This is the only code path inside Infinity Orchestrator that produces an auto-install-eligible code.
- The Share import code dialog ([§10.3](#103-sharepastecodedialog)) reads from `latest_share_code` (already flipped to `true` by `flip_to_installed`), so the user who clicks the dialog after a successful install gets a code that other recipients can directly auto-install.

**Consumption — Install Modlist paste flow:**
- The paste-stage parser reads `allow_auto_install` from the decoded payload (defaulting to `true` if absent — backward compatibility with pre-redesign codes).
- If `allow_auto_install == true`: existing flow continues — preview → Install → install runs. No additional UX.
- If `allow_auto_install == false`: the preview stage shows an info banner: *"This is a draft modlist code (not from a verified install). Review and customize the components before installing."* The primary **Install** CTA is **disabled** in this state. A new primary CTA `Open in Create →` routes the user to `Create → Import-and-modify` with the code pre-pasted. From Create, the user reviews and modifies the modlist; the resulting workspace's install path generates its own draft code (still `allow_auto_install = false`) until it reaches `Installed`.

**Consumption — other paths:**
- **Create → Import-and-modify (fork-paste)** ignores `allow_auto_install`. Forking is always allowed regardless of the bit, because the user is actively reviewing/customizing.
- **Load Draft** dialog ignores the bit — it always opens the workspace for review.
- **Reinstall** ([§3.1](#cards-in-the-filtered-list)) reads from the registry's `latest_share_code`. Reinstall only acts on `Installed` modlists, where the bit was already flipped to `true` by `flip_to_installed` — so Reinstall is always permitted.

**Rationale for carve-out #5 (vs. a wrapping struct in the orchestrator):** A wrapping struct would mean two parallel serde types — BIO's `ModlistSharePayload` and an orchestrator-side wrapper that mirrors all its fields plus `allow_auto_install`. The maintenance burden compounds every time a future field is added to BIO; the wrapper drifts and needs manual reconciliation. The carve-out lets the field live in one canonical struct with `#[serde(default = "default_true")]` for full backward compatibility on the consume side. The addition is mechanical (one field + one default function) and zero behavior change for existing flows: every code BIO has ever produced is treated identically because the field defaults to `true`.

### 13.4 Mod source manifests

**BIO-fidelity.** Reuse `default_mod_downloads.toml` + `user_mod_downloads.toml` and the existing fetch/selection logic. Source editor popup → Appendix A.5.

### 13.5 Compatibility rules

**BIO-fidelity.** Reuse `default_step2_compat_rules.toml` + `step2_compat_rules_user.toml` and the compat engine (`src/core/app/compat/`). Step 2 UI ([§6](#6-step-2--scan-and-select)) consumes the unified output.

### 13.6 Auto-answer prompt memory

**BIO-fidelity.** Reuse `prompt_answers.json` and the existing two-tier resolver (inline `@wlb-inputs` → saved memory). Step 5 Prompt Answers window → Appendix A.8.

### 13.7 Mod download / fetch

**BIO-fidelity.** Reuse the existing fetchers, archive extractors, and `Mods archive` storage (backend). Asset picker UI is re-skinned per the BIO-reuse rule.

### 13.8 Update check

**BIO-fidelity for the engine.** UI now drawn in the wireframe ([§6.11](#611-update-check-popup-updatespopup)) — re-skin the existing `src/ui/step2/update_check/` popup with new theme tokens; remaining sub-modals (Source Editor, Discover Forks, fallback confirm) → Appendix A.4.

### 13.9 Install runner

**BIO-fidelity.** Reuse the existing runner (`mod_installer` spawn, prompt detection, stdin routing, timeout/exit-code handling, partial-install resume). The embedded terminal and Step 5 console are UI — re-skinned per the BIO-reuse rule.

### 13.10 Diagnostics export

**BIO-fidelity.** Reuse the existing exporter (`diagnostics/run_<timestamp>/` bundle). Completion UX (toast / Open folder / error path) → Appendix A.12.

### 13.11 CLI

The CLI subcommands stay (parity):

- `bio gui` (default) — launches the GUI described in this spec.
- `bio normal --game-directory <D> --log-file <L> [--mod-directories ...]` — headless install for BGEE/BG2EE/IWDEE.
- `bio eet --bg1-game-directory <D1> --bg1-log-file <L1> --bg2-game-directory <D2> --bg2-log-file <L2> [...]` — headless EET install.
- `bio scan components --game-directory <D> [--filter-by-selected-language ...]` — text/JSON list of TP2 components.
- `bio scan languages --game-directory <D>` — list available languages per TP2.
- Global flags: `--log-level`, `--dev-mode` (`-d`), `--help`, `--version`.

No CLI changes are required for the redesign. The GUI is the priority surface.

### 13.12 Automatic flag policies

Several WeiDU / installer flags that today's BIO exposes as user-configurable toggles in Step 1 are **removed from the user surface** in the redesign. The flags still take effect — the existing install runner emits them; the redesign just sets them automatically from workflow context instead of asking the user. This keeps Settings → Advanced focused on flags the user actually wants to override, and prevents the most common foot-guns (e.g., forgetting to set `-s` before continuing a partial install).

1. **`-s` (skip installed) and `-c` (check last installed)** — automatically ON when the user enters the **Continue Partial Install** workflow (the "this folder isn't empty" Box's `continue` choice in Install Modlist's paste stage). OFF for fresh installs. Today these are visible toggles on Step 1 ([Appendix A.15](#a15-bio-step-1-toggles-not-in-the-wireframes-advanced-tab)).

2. **`-u` (per-component WeiDU logging)** — always ON. On install start, the install runner creates a `weidu_component_logs/` folder inside the modlist's destination folder and points `-u` at it. The user does not configure a path. Today's BIO requires this path to be set in Step 1; the redesign derives it from the destination ([Appendix A.14](#a14-granular-weidu-log-mode-flags)).

3. **`-p` (clone BGEE → Pre-EET) and `-n` (clone BG2EE → EET final)** — always ON for **EET installs**. On install start, the install runner creates two folders with **standard fixed names** inside the modlist destination — `Baldur's Gate Enhanced Edition/` (the Pre-EET / BG1 phase target) and `Baldur's Gate II Enhanced Edition/` (the EET final / BG2 phase target) — and clones the source-game folders (from Settings → Paths) into them. The BIO Step 1 fields `eet_pre_dir` and `eet_new_dir` are not user-configurable; users cannot override the names or locations ([Appendix A.1](#a1-step-1-setup-wizard), [Appendix A.19](#a19-eet-workflow-surfacing)).

4. **`-g` (clone source game → target)** — always ON for **single-game installs** (BGEE, BG2EE, IWDEE). On install start, the install runner creates a target folder with a **standard fixed name** (`Baldur's Gate Enhanced Edition/` / `Baldur's Gate II Enhanced Edition/` / `Icewind Dale Enhanced Edition/`) inside the modlist destination and clones the source-game folder into it. The `generate_directory` BIO setting is not user-configurable ([Appendix A.1](#a1-step-1-setup-wizard)).

5. **`--download` (download missing mods)** — automatically ON when the user enters a workflow that consumes a share code: **Install Modlist (paste import code)**, **Create → Import and modify**, and **Load Draft**. For **Create → New modlist from downloaded mods** (fresh-create), the flag falls back to the user's `Download missing mods and keep archives` toggle in Settings → Advanced (default ON). This guarantees imported modlists fetch their dependencies without the user having to remember to enable downloads.

6. **`prepare_target_dirs_before_install` and `backup_targets_before_eet_copy`** — replaced by the **`DestinationNotEmptyWarning`** choice in the Install Modlist and Create flows. When the user picks a destination folder that already has content, the warning Box's radio options (labels are self-explanatory: e.g., "replace contents", "continue partial install", "cancel") determine the prep/backup behavior. The BIO Step 1 toggles for these are removed; the underlying flags are set based on the chosen option ([Appendix A.15](#a15-bio-step-1-toggles-not-in-the-wireframes-advanced-tab)). **`Reinstall` forces this ON** without showing the warning — the reinstall confirm modal already covers the destructive consent ([§3.2 Reinstall semantics](#cards-in-the-filtered-list)).

7. **`-autolog`, `-logapp`, `-log-extern`** — always ON. Hard-coded; no UI. Matches today's BIO defaults ([Appendix A.14](#a14-granular-weidu-log-mode-flags)).

The flags themselves stay on the install command line the existing BIO install runner emits — they're just no longer user-configurable surfaces. Dev mode's diagnostics export records the exact flag values used for any given install.

### 13.13 Import code auto-generated on install start

Every install — regardless of entry point (**Install Modlist** paste flow, **Create → New modlist from downloaded mods**, **Create → Import and modify**, **Load Draft**) — automatically generates a fresh **BIO-MODLIST-V1 import code** at the **start of the install** and writes it to the modlist's destination folder as a clearly-named text file. The default filename is `modlist-import-code.txt`.

The file content is the same string the user would get from **Share import code** ([§10.3](#103-sharepastecodedialog)) after a successful install **except for the `allow_auto_install` flag**: at install-start time the install has not yet succeeded, so the code is written with `allow_auto_install = false` ([§13.3](#133-share-code-bio-modlist-v1)). The file is written upfront — before WeiDU runs — so the artifact survives even if the install crashes, gets cancelled, or finishes with errors. After a successful install, the registry's `latest_share_code` is regenerated with `allow_auto_install = true`; the on-disk `modlist-import-code.txt` is **not** automatically rewritten on success (it would still carry the install-start version). The Share import code dialog reads from the registry value, which is the canonical "verified" code; the on-disk file is the "I crashed, here's what I was trying" artifact and is conservative-by-default.

**Write semantics per install-button variant** (per [§9.4](#94-behavior) / [A.9](#a9-resume-install--restart-install-states)):
- **Install** (fresh attempt) — file is written at install start.
- **Restart Install** (after a force-cancel) — file is **overwritten** with the current workspace state at install start. The previous file (from the cancelled attempt) is discarded.
- **Resume Install** (after a graceful cancel) — file is **not overwritten**. The original file is the source of truth for what's currently mid-install; resume continues that install. (Per [§9.2](#92-post-install-layout-installcompletetrue) the workspace is locked once Install is clicked, so the state cannot have drifted — the file on disk is still accurate.)
- **Reinstall** ([§3.1](#cards-in-the-filtered-list)) — file is **overwritten** at install start (it is a fresh install with potentially-updated share code).

Effects:

- Every install is **reproducible**: the import code is sitting next to the game install on disk. Forwarding the file to a collaborator recreates the same modlist.
- The user can recover the import code even if BIO's modlist registry loses track of the modlist (e.g., after an OS re-install or moving the destination folder).
- The Home Kebab → **Copy import code** action reads from this file when present.

### 13.14 Persistence timing

Inherits today's BIO persistence model (`src/ui/app_bootstrap.rs`, `src/ui/app_lifecycle.rs`, `app_update_cycle::persist_step1_if_needed`). Applies to **app settings** (`bio_settings.json`) **and** the new **modlist registry** (`modlists.json`) **and** the new **per-modlist workspace state files** (`modlists/<id>/workspace.json` under the platform config dir).

Writes happen at three moments:

1. **On every relevant change (debounced).** Each time the user touches a settings field, toggles a check, drags a row in Step 3, renames a modlist, etc., the corresponding file is queued for write. Writes are debounced/throttled the same way today's BIO already does it for Step 1 (see `app_update_cycle.rs`) so rapid typing or frequent drag updates don't thrash the disk.
2. **On nav-away from the workspace.** Clicking a left-rail destination (Home / Explore / Install / Create / Settings) while inside a modlist workspace flushes the in-flight workspace-state changes to disk before the screen transitions. This guarantees the build is recoverable from Home or the Resume dialog even if the user closes the app immediately after.
3. **On app drop / clean shutdown.** `app_lifecycle.rs` flushes everything one final time when the eframe loop exits.

Crash / hard-kill behavior: because changes are written on each debounced interval, the worst case loss is the throttled window's worth of edits. Modlist registry writes (add / rename / delete / state transition) are individually atomic and don't queue.

**Corrupt / missing state files.** Recovery policy splits by what the file holds. **Irreplaceable user data** (the modlist registry + per-modlist workspace state) gets a strict terminal-error policy — the redesign does **not** attempt clever recovery. **Reconstructable UI preferences** (`bio_redesign_settings.json`) get a lighter backup-and-default policy. Concretely:

- **`modlists.json` corrupt or unreadable at app start** — the app surfaces a terminal error state on Home (or wherever it lands) explaining the file path and the parse failure. It does not silently rebuild, wipe, or partially load the file. The corrupt file is renamed aside (`modlists.json.corrupt-<unix-ts>`) so it is preserved for the user to inspect or restore.
- **`modlists/<id>/workspace.json` missing or corrupt** when the user opens that modlist — the workspace cannot load; surface the same terminal error. The registry entry stays (so the user can still Delete it from Home), but the workspace itself is unusable until the file is restored or the entry is deleted.
- **State-file corruption discovered mid-install** — abort the install with a terminal error. Do not proceed with stale data.
- **`bio_redesign_settings.json` corrupt or unreadable at app start** — **not** a terminal error. The orchestrator renames the bad file aside (`bio_redesign_settings.json.corrupt-<unix-ts>`), logs a warning, and continues with `RedesignSettings::default()`. These are reconstructable preferences (display name, theme, language, diagnostic + validate-on-startup toggles) — bricking the app over a corrupt prefs file would be disproportionate. The backup rename is what prevents the silent data loss of the next debounced write overwriting the unreadable file. `load()` itself stays pure; the rename is an explicit step in `OrchestratorApp::new`, mirroring `RegistryStore::backup_corrupt_file`.

For the irreplaceable-data files: no auto-recovery, no "ignore and continue" path. The user must fix the underlying issue (restore from backup, delete the file, etc.) or remove the affected modlist.

Workspace state stored per modlist:
- order array per game tab
- checked components (derived from order; redundant to store, but cached)
- mod expand/collapse state per tab
- parent-group expand/collapse state per tab
- Step 3 group collapse state
- prompt-answer overrides scoped to this build (if any beyond global `prompt_answers.json`)
- the most-recent BIO-MODLIST-V1 code (for the Home Kebab `Copy import code`)

The modlist registry (`modlists.json`) for each entry stores: id, name, game, destination folder, state (`in-progress` | `installed`), creation date, last-touched date, install date (when installed), last-played date, mod count, component count, total size, and a pointer to the workspace-state file.

### 13.15 Install concurrency policy

**Only one install can run at a time across the entire app.** The status bar's `jobs running` counter is therefore 0 or 1, never higher.

While an install is running on modlist A, every other entry point to "start an install" is gated:

- **Step 5 `Install` button on any other modlist B's workspace** — disabled. Tooltip: "An install is already running for _<modlist A>_. Wait for it to finish before starting another."
- **Install Modlist (paste import code) flow** — the final "Install" CTA in the destination-not-empty stage is disabled with the same tooltip.
- **Home page `resume` / `play` buttons** — still let the user navigate into the workspace (read-only) and watch the running install if they click into modlist A; for any other modlist, the workspace opens normally but its Step 5 Install button is gated as above.
- **Status bar** — `1 job running` and (optionally) a faint right-aligned "_<modlist A>_ · _<elapsed>_" so the user can find the running install from any screen.

The user can still cancel the running install (Cancel Install in modlist A's Step 5). On cancel or completion, the gates lift immediately and the other Install buttons re-enable.

---

## 14. Dev mode and wireframe-only panels

### 14.1 Dev mode

Enabled via `bio -d gui` or `Diagnostic mode` toggle in Settings → General. Effects:

- Step 5 shows the dev header (RUST_LOG selector, Full Debug, Raw Output toggles).
- Step 5's Diagnostics menu becomes available.
- Diagnostics bundle is auto-exported on every install completion.
- Step 1-style "Dev Mode: RUST_LOG=DEBUG selected." banner shows on the Final Plan / Step 5 panel.

### 14.2 Tweaks panel

**Dropped from production.** The `TweaksPanel` exists only in the wireframe for design iteration (version v1/v2, nav style, density, accent, annotations). It does not ship in the BIO build. Any tuning the panel exposed gets baked into a single design decision before ship; no per-user override surface is added.

---

## 15. Data model summary

This isn't an implementation contract, but it's the user-facing data the redesign needs to track. Each row is tagged for which parts come from today's BIO and which the redesign adds:

| Concept | Status | Where it lives | Notes |
|---------|--------|----------------|-------|
| App-global settings | **existing** | `bio_settings.json` | Paths, tools, theme, language, name, advanced flags. Read/written by existing `src/settings/` loader. |
| Prompt memory | **existing** | `prompt_answers.json` | Per-component auto-answers. |
| User compat rules | **existing** | `step2_compat_rules.toml` | User overrides; default rules embedded. |
| User mod sources | **existing** | `mod_downloads_user.toml` | User overrides; default sources embedded. |
| Modlist registry | **new** | `modlists.json` | Index of installed and in-progress modlists for Home + Load Draft. |
| Per-modlist workspace state | **new** | `modlists/<id>/workspace.json` | Order arrays, checked components, expand state, prompt overrides; loaded when the user opens the modlist. Today's BIO holds equivalent state only in memory under the single active workspace. |
| Persistence machinery (debounce, nav-flush, drop-flush) | **existing, extended** | `app_bootstrap.rs` / `app_lifecycle.rs` / `app_update_cycle.rs` | Same throttle + drop-time flush logic; the redesign adds `modlists.json` and per-modlist files to the same write paths. |
| Draft files (legacy on-disk share codes) | **existing** | User-chosen locations, `.txt` | Plain-text BIO-MODLIST-V1 codes. Still consumable via Install Modlist paste; no longer the primary "resume" surface (the registry is). |
| Diagnostics bundles | **existing** | `diagnostics/run_<timestamp>/` in working dir | Auto in dev mode; manual otherwise. |

---

## Appendix A — BIO features not yet in the wireframe

Surfaced for review. Each item is a feature in today's `bio` crate that the wireframe does not (yet) render. For each one the team needs to decide: **keep in v1** (and add to the wireframe), **defer** (post-v1), or **drop**. The spec does not pre-decide.

Items marked **Visible**: the wireframe shows a button or surface but never renders the underlying behavior. Items marked **Invisible**: today's BIO has the feature but the wireframe shows nothing related to it.

### A.1 Step 1 setup wizard
**Status:** Resolved. Step 1 as a screen does not exist in the wireframe — paths moved to Settings, install-mode-equivalent moved to Create's three cards. Decisions for the remaining surfaces:

- **`install_mode` enum (4 modes).** *Kept internally, inferred from workflow.* The enum stays in the BIO data model (share-code payload, diagnostics bundle, CLI subcommands all reference it), but it is never user-configurable. The redesign infers it from the user's path through Create / Install Modlist / Destination-not-empty choice. Map: fresh-create → `install_normally`; destination-not-empty + `continue` → `continue_partial_install`; game pick = EET → `install_eet`; Install Modlist paste with `weidu_logs` payload → `install_exactly_from_weidu_logs`.
- **EET-specific paths** (Pre-EET dir, EET final dir, BG1 source/log per phase, BG2 source/log per phase). *Never exposed.* The install runner creates them with standard fixed names inside the modlist destination per [§13.12 policies #3–#4](#1312-automatic-flag-policies). Users cannot override.
- **`Test Paths` button.** *Replaced.* Settings → Paths now validates each row continuously (debounced on every edit) with per-row inline status — no explicit "test" button is needed ([§11.2](#112-paths)). Additionally, the install runner validates the destination + every referenced path once automatically before performing any downloads or extractions for an install ([§11.2](#112-paths) "Pre-fetch automatic validation").
- **`Reset Wizard State` button.** *Dropped.* The redesign has no single "the workspace" to reset — deleting a modlist from Home (or the registry-level state files) is the equivalent. Cache-clear power is not surfaced.
- **`@wlb-inputs` syntax helper.** *Subsumed.* The Step 3 right-click menu ([Appendix A.6](#a6-step-3-right-click-context-menu)) is where users actually set `@wlb-inputs` values, so no separate clipboard helper is needed in Settings or anywhere else.

### A.2 GitHub OAuth device-flow popup
**Status:** Resolved. Use the existing BIO popup (`src/ui/step1/github_auth_popup_step1.rs`) verbatim, with the new theme tokens applied. The Settings → Accounts → GitHub card's `connect` / `disconnect` / `reconnect` actions all open the same popup. Full behavior spec in [§13.2](#132-github-oauth) — the wireframe does not need to redraw it.

### A.3 "Select via WeiDU Log" import flow
**Status:** Resolved. BIO-fidelity — reuse the existing file-picker → parse → apply flow. Toolbar button shown only in the Create → New starting-point workflow ([§6.4](#64-toolbar)); hidden in Import-and-modify and Resume. UI re-skinned per the BIO-reuse rule.

### A.4 Update Check popup ("Updates..." button)
**Status:** Resolved. BIO-fidelity for all three sub-modals (Source Editor, Discover Forks, Fallback confirmation) via the inheritance clause of the BIO-reuse rule — the parent Updates popup is BIO-fidelity, so its children inherit. Reuse the existing widgets in `src/ui/step2/update_check/`.

### A.5 Per-mod source editor + asset picker
**Status:** Resolved. BIO-fidelity for both the source editor and the asset picker. Reuse existing widgets.

### A.6 Step 3 right-click context menu
**Status:** Resolved. BIO-fidelity — reuse the existing right-click menu and its child popups (`@wlb-inputs` editor, Prompt JSON editor) via the inheritance clause.

### A.7 Step 4 save flow + exact-log mode
**Status:** Resolved. BIO-fidelity for both the save flow (including save-error popup) and the exact-log read-only viewer. Reuse existing behavior and widgets.

### A.8 Step 5 Prompt Answers window
**Status:** Resolved. BIO-fidelity — reuse existing modal as-is. No design changes.

### A.9 Resume Install / Restart Install states
**Status:** Resolved. BIO-fidelity — reuse existing label-switching logic on the Step 5 install button (`Install` / `Resume Install` / `Restart Install` per `state.step5.resume_available`).

### A.10 Cancel Install confirmation modal
**Status:** Resolved. BIO-fidelity — reuse existing confirm modal (with Force checkbox) as-is. **Behavior addition:** graceful cancel (Force unchecked) leaves the install resumable via Continue Partial Install — the modlist's `state.step5.resume_available` stays true so the Step 5 button reads `Resume Install` on next visit. Force cancel marks the install attempt terminal — `resume_available` is cleared, only `Restart Install` (fresh re-attempt) is offered.

### A.11 Step 5 Actions and Diagnostics dropdowns
**Status:** Resolved. BIO-fidelity — reuse existing dropdown contents and behavior for both Actions ▾ and Diagnostics ▾.

### A.12 Diagnostics export bundle output
**Status:** Resolved. BIO-fidelity — reuse existing export flow and any completion / error surfaces BIO already shows.

### A.13 Mod config files (preloaded .ini/.conf)
**Status:** Resolved. BIO-fidelity — the share-code → fetch → extract pipeline continues to pull `config_files` listed in mod source manifests and drop them into the mod folders before WeiDU runs. No in-app editor in the redesign (planned-but-not-built in BIO today; spec inherits that state). The wireframe's `Mod Configs` preview tab stays as-is.

### A.14 Granular WeiDU log-mode flags
**Status:** Resolved. `-autolog`, `-logapp`, `-log-extern` all absorbed — always ON, hard-coded, no UI ([§13.12 policy #7](#1312-automatic-flag-policies)). `-u` already resolved by §13.12 policy #2.

### A.15 BIO Step 1 toggles not in the wireframe's Advanced tab

**Status:** Resolved.

**Resolved by [§13.12 Automatic flag policies](#1312-automatic-flag-policies):**

- `skip_installed` (`-s`) / `check_last_installed` (`-c`) — auto-ON for Continue Partial Install. (#1)
- `weidu_log_log_component` (`-u`) — always ON, path auto-derived. (#2)
- `new_pre_eet_dir_enabled` / `new_eet_dir_enabled` — fields auto-derived. (#3)
- `generate_directory_enabled` — field auto-derived. (#4)
- `prepare_target_dirs_before_install` / `backup_targets_before_eet_copy` — replaced by destination-not-empty workflow. (#6)
- `-autolog` / `-logapp` / `-log-extern` — absorbed, always ON. (#7)

**Resolved by absorb-the-gate pattern in [§11.5 Advanced](#115-advanced):**

- `custom_scan_depth`, `auto_answer_initial_delay_enabled`, `auto_answer_post_send_delay_enabled`, `lookback_enabled`, `tick_dev_enabled` — gates dropped; value fields alone (empty = default, filled = override).

**Resolved by surfacing in [§11.5 Advanced](#115-advanced):**

- `casefold` — added as a ToggleRow.
- `timeout_per_mod_enabled` — added as a gate-absorbed ValueRow marked experimental.

**Already surfaced elsewhere:**

- `bio_full_debug` — Step 5 Diagnostics ▾ menu (`src/ui/step5/menus_step5.rs`).
- `rust_log_debug` / `rust_log_trace` / `log_raw_output_dev` — Step 5 Diagnostics ▾ menu ([§9.6](#96-diagnostics--menu-dev-mode-only)).

**Dropped:**

- `weidu_log_mode_enabled` — redundant under §13.12 #2.

### A.16 Step 3 locked-block enforcement
**Status:** Resolved. BIO-fidelity per the "default for un-wired wireframe surfaces" clause — reuse existing per-mod lock enforcement (prevents drag-out-of-order for locked mods).

### A.17 Nexus Mods / Mega account connections
**Status:** Resolved. BIO-fidelity per the "default for un-wired wireframe surfaces" clause — BIO has no integration today, so the cards stay as visual stubs. `connect` buttons are no-ops (or show a "not yet implemented" hint) until real integration ships.

### A.18 Sound cue audition
**Status:** Resolved. BIO-fidelity per the "default for un-wired wireframe surfaces" clause — BIO has no audition control today, so none is added.

### A.19 EET workflow surfacing
**Status:** Resolved. Covered by [§13.12 policies #3–#4](#1312-automatic-flag-policies) (phase clone flags + auto-derived paths), [A.1](#a1-step-1-setup-wizard) (EET paths never exposed), and [A.20](#a20-eet-wlb-path-rewriting-on-imported-modlists) (silent WLB rewrite). The dual BGEE/BG2EE tabs in Steps 2–4 carry the workflow implicitly.

### A.20 EET WLB path rewriting on imported modlists
**Status:** Resolved. BIO-fidelity — silent fix-up stays silent (no confirmation surfaced).

### A.21 Modlist registry persistence
**Status:** Resolved — spec'd in [§13.1](#131-modlist-persistence) (new `modlists.json` registry).

### A.22 Per-modlist workspace state persistence
**Status:** Resolved — spec'd in [§13.1](#131-modlist-persistence) (new `modlists/<id>/workspace.json` files).

---

## Appendix B — Open questions

Genuine ambiguities raised by the wireframe-vs-BIO comparison. The spec deliberately does not pre-decide these — the team should answer them and then update the spec.

1. ✅ **Modlist deletion semantics.** Resolved: Delete removes **both** the registry entry and the install folder.

2. ✅ **Install Modlist flow → completion state.** Resolved: on successful install, Step 5's action row gains **`Return to Home`** and **`Open install folder`** primary buttons next to the disabled `✓ Installed` button (see [§9.2](#92-post-install-layout-installcompletetrue)).

3. ✅ **Tweaks panel exposure.** Resolved: dropped from production entirely (see [§14.2](#142-tweaks-panel)).

4. ✅ **Persistent Details panel toggle.** Resolved: `Show Details panel` lives in the Step 2 toolbar Kebab as a persistent toggle ([§6.4](#64-toolbar), [§6.8](#68-details-panel)). The per-row hover `[?]` button stays for one-off row inspection.

5. ✅ **Step 3 right-click discoverability.** Resolved: no per-row Kebab is added. Right-click stays the sole entry point per BIO-fidelity ([A.6](#a6-step-3-right-click-context-menu)).

6. ✅ **Loaded-draft game selection.** Resolved: workspace always adopts the draft's `game_install` from the BIO-MODLIST-V1 payload. The wireframe's EET hard-code is a demo-only artifact.

7. ✅ **Compat-popup navigation.** Moot. CompatPopup is BIO-fidelity ([§6.11](#611-update-check-popup-updatespopup)) so next/prev is inherited; the wireframe already draws the `Next →` button. `PillPopup` is a separate generic widget used for non-iterable content and does not need next/prev.

8. ✅ **Confirmation policy.** Resolved: confirm destructive / irreversible actions; skip confirms for easily-reversed actions. Current confirms: `Select <TAB> via WeiDU Log` (overwrites tab selections — [§6.4](#64-toolbar)), Home Kebab → Delete (removes registry entry + install folder — [§3.2](#cards-in-the-filtered-list)), Home Kebab → Reinstall (wipes install folder and re-runs install — [§3.2](#cards-in-the-filtered-list)), Cancel Install (BIO-fidelity with Force checkbox — [A.10](#a10-cancel-install-confirmation-modal)). No confirms for: drag-reorder, checkbox toggles, `Save weidu.log` writes.

9. **Subcomponent "pick one" semantics.** *Resolved:* WeiDU parent-component groups (e.g., Portrait Selectors) render as non-selectable collapsing headers, and their subcomponents render with radio glyphs (`◉` / `○`) and enforce single-selection (see [§6.5](#65-tree)). The wireframe matches today's BIO behavior.

10. ✅ **Home `game installs detected` refresh.** Resolved: auto-refresh driven by path validation events — app start (if validation toggle on), Settings → Paths edits (debounced), and post-install target-folder creation. Details in [§3.3](#33-add-a-modlist-section).

11. ✅ **Per-platform asset packaging.** Resolved: no UI added. BIO's existing per-platform asset selection (`pkg_windows` / `pkg_linux` / `pkg_macos`) stays silent — BIO-fidelity per the un-wired-surfaces rule.

---

## Appendix C — Glossary

- **Modlist** — a named collection of selected components in install order, scoped to a single game install. Persists across app launches. Top-level entity on Home.
- **Workspace** — the editing experience for one modlist (Steps 2–5).
- **Step 2 / 3 / 4 / 5** — the four tabs inside the workspace (Scan and Select / Reorder and Resolve / Review / Install). Step 1 from today's BIO is dissolved into Settings + Create.
- **Tab** (game tab) — within Steps 2–4, the per-game-install sub-tab. EET shows BGEE + BG2EE. Single-game modlists show only one tab.
- **Component / leaf** — a single installable TP2 component. Has a tp2 file + integer id.
- **Parent component** — a WeiDU group label (e.g., Portrait Selectors) under which one or more selectable subcomponents live.
- **Subcomponent** — a leaf component nested under a parent component.
- **Mod** — a tp2 file plus its associated components. Has a name + version.
- **Order** — the linear sequence of (checked) components that will be sent to WeiDU.
- **Group** (Step 3) — a contiguous run of same-tp2 components in the order. May be the canonical run for that tp2 or a split-off `(copy)` group.
- **Import code / share code / paste code** — synonyms for the BIO-MODLIST-V1 string. The user-facing label is **import code** (in dialogs and CTAs); internal name is **share code**.
- **Draft** — a `.txt` file containing an import code, saved from inside the workspace for later resumption.
- **Auto-answer** — the system that detects WeiDU prompts during install and sends pre-configured answers from `prompt_answers.json` or scripted `@wlb-inputs` tokens.
- **Pill** — a small colored badge on a row indicating an issue or property. Clickable.
- **v1 / v2** — release tracks (offline / community). Affects whether the Explore tab is present.

---

*End of spec.*
