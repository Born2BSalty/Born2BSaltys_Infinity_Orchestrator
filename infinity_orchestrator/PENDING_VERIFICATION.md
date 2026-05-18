# Pending User Verification

Changes committed but **never seen rendered by you** — items below were committed while the app held the `infinity_orchestrator.exe` lock, so none has been visually verified. Work through this when you're back.

_Last updated 2026-05-18 (Phase-6 closeout: the 6 user-confirmed Fix-Run-2 items, #8 forked share code, and the DATA-LOSS fix removed as user-confirmed closed; the Create #4 cluster marked orchestrator-render-verified pending one-pass live confirm; the 5 minor Create issues + box-equal-height relocated to Phase 8; Phase-7 verify-at-phase-end section added). Branch `overhaul/infinity_orchestrator`._

> **Fix-Run-1 (persistence round-trip + in-memory isolation, commit `8dfb905`) — ✅ USER-VERIFIED DONE.** The per-modlist cold-resume round-trip + save-draft + rename were user-verified end-to-end on a gate-confirmed binary. Do not re-test; do not touch that code (`workspace_state_loader.rs`, the dirty-bit/nav-flush, `step2_resume_scan.rs`).

> **DATA-LOSS workspace.json order-wipe regression (Fix-Run-3 `0b9d53d` + Fix-Run-4 `28d9975`) — ✅ USER-CONFIRMED CLOSED.** User retested across resume→nav-away/quit/save-draft cycles; holds up. Do not re-test.

---

## ⚠️ Precondition — do this ONCE before testing anything

Every on-disk binary is **stale** (commits landed under the exe lock). Before any item:

1. Fully close Infinity Orchestrator.
2. `cargo build --bin infinity_orchestrator --release` — run it **twice**; the second must end in a no-op `Finished` with **no `Compiling bio`** line (that is the only proof the binary is current).
3. The seed registry (`%APPDATA%\bio\modlists.json`) was mutated during prior testing. Re-prep the canonical 2-entry seed if the in-progress modlist is gone — mechanism in the orchestrator skill (`.claude/skills/orchestrator/SKILL.md` → "Test fixtures / runtime"). The **in-progress** seed modlist is what the resume → Step-2/3/4 flow needs.
4. Launch `infinity_orchestrator -d`, Home → in-progress card → `resume`.

If you skip step 2 you will be looking at the OLD build and will think nothing changed.

---

## Test items

### ☐ 1. Workspace content alignment
- **Phase / Run:** Phase 6, **Run 1** (Step-2 C4 chrome + workspace shell hint line) — follow-up polish.
- **Commit:** `ab4453b`.
- **You reported:** "Mods / Components", the "Choose components to install." hint, and the search box never shared a left edge ("not quite lined up", several iterations).
- **What changed:** one shared constant `WORKSPACE_CONTENT_TEXT_INSET = 0` (the structural content edge = progress-bar / pane / search-box left edge) drives the title **and** the per-step hint; the search input's *inner* text padding was decoupled (`SEARCH_INPUT_TEXT_PAD = 8`) so the text isn't glued to its border.
- **Check (Step 2):** the progress bar, the "Choose components to install." hint, the "Mods / Components" title, and the search box's left border all sit on **one vertical line**. The search *placeholder text* is ~8px inside its box — intentional (a bordered input aligns by its box edge, not its inner text). If the whole column needs nudging it is now a **one-number** tweak (`WORKSPACE_CONTENT_TEXT_INSET` in `redesign_tokens.rs`).

### ☐ 2. Uniform GameTab — no bottom bar
- **Phase / Run:** Phase 6, **Runs 1 + 2 + P6.T2d** (the BGEE/BG2EE tab strip appears on Step 2 [R1], Step 4 [R2], and Step 3 [P6.T2d] — now ONE shared widget).
- **Commits:** `ab4453b` (shared widget + Step-2/4 rewired), `fad78c3` (Step 3 uses it).
- **You reported:** tabs had a bottom bar (the idle tab's bottom border).
- **What changed:** one shared `widgets::game_tab` — fill + rounded-top + border on **top/left/right only, never a bottom edge**, in any state. The two duplicate per-step painters were deleted.
- **Check (Step 2, Step 3, Step 4):** the BGEE/BG2EE tabs have **no bottom bar** in any state (active or idle) and look **identical** across all three steps. Single-game modlists skip the strip entirely.

### ☐ 3. Rename pencil glyph (✎)
- **Phase / Run:** Phase 6, **Run 2** (P6.T5 — `workspace_header` + `operations_rename` inline rename).
- **Commit:** `ab4453b`.
- **What changed:** the ✎ rename glyph reworked to a crisp **filled** vector (was a thin/blocky outline) + optically centered against the modlist-name label.
- **Check (workspace header, the "Editing <name> ✎" row):** the pencil reads as a clean filled pencil, vertically centered with the name (not blocky, not sitting low). Clicking it still opens the inline rename (Enter commits, Esc cancels; registry-entry-only — no on-disk folder rename).

### ☐ 4. Step-3 C4 chrome
- **Phase / Run:** Phase 6, **P6.T2d** (Step-3 C4).
- **Commit:** `fad78c3`.
- **Goal:** Step 3's top area looks like the wireframe (it was raw BIO).
- **Check (resume the in-progress seed modlist → Step 3):**
  - Redesign action-row count "_N_ components ready to install on _\<tab\>_ · across _M_ mods" (right-aligned, faint); **no** Save button.
  - The shared redesign GameTabs (same look as Step 2/4, no bottom bar — see item 2).
  - Aggregate conflict / prompt pills (only when count > 0); clicking the conflict pill opens the compat popup, the prompt pill the prompt popup.
  - Redesign `Undo` / `Redo` / `Collapse All` / `Expand All` (Undo/Redo disabled when the stack is empty).
  - **NO** "Export diagnostics" / "Restart App With Diagnostics"; **NO** BIO "Step 3: Reorder and Resolve" heading or duplicate hint.
  - BIO's drag list still fully works inside the chrome: drag-reorder (order renumbers, `(copy)` groups spawn/merge), collapse/expand chevron, Undo/Redo a reorder, shift-click a contiguous range, conflict/prompt row pills open their popups.
  - Resize the window short / narrow — the list never overlaps or pushes the workspace nav bar (`← Previous` / `Next →` always visible).
  - EET: switch BGEE↔BG2EE — count + pills + list track the active tab. Single-game: tab strip absent (like Step 4).

### ☐ 5. Create screen — choose-mode setup + routing
- **Phase / Run:** Phase 6, **Run 3** — **P6.T7** + **P6.T13** (wire Create into `page_router`).
- **Check:** the Create rail item opens the real screen (not the stub): a setup Box with modlist **name**, **game** ComboBox (defaults to **EET**; options EET / BGEE / BG2EE / IWDEE), destination FolderInput + the conditional DestinationNotEmptyWarning on a non-empty dir (Clear / Backup only — **no** Continue-partial, SPEC §5.1). From-scratch path: a name + valid existing destination → a new entry appears in `%APPDATA%\bio\modlists.json`, `modlists\<id>\workspace.json` is created, and the **Workspace opens at Step 2**, header `Editing <name>`; blank destination falls back to `<config>\modlists\installs\<slug>`. (The selectable-box choose UX itself is item FR2-4 below.)

### ☐ 6. Load Draft dialog (Resume in-progress build)
- **Phase / Run:** Phase 6, **Run 3** — **P6.T9** + **P6.T14** (Resume routing).
- **Check:** Create's `load draft` button opens a **non-blocking dialog** titled "Resume in-progress build" (it is **NOT** a file picker — SPEC §5.2/§10.2) listing in-progress builds as the shared Home card chassis (`resume` + Kebab `Copy import code` / `Delete`), empty-state copy when none, `Cancel`-only footer. `resume` closes it and opens the Workspace at Step 2 (`Editing <name>`). `Copy import code` → transient `✓ Copied…` (or an honest "no import code yet" for a pre-Phase-7 build). The Home in-progress card `resume` (already shipped) still routes to the Workspace too. (The `Delete` Kebab item is now wired — verify via FR2-6 below.)

## Phase 6 Run 4 — SHIPPED + orchestrator-verified, pending your visual sign-off (**Phase 6 COMPLETE**)

Shipped in the Run-4 commit (P6.T8 fork sub-flow + P6.T11 dirty-bit persistence + P6.T15 nav-away flush). Independently verified: BIO-guard empty, the expected file set, `cargo test --lib` green, `modlists.json` byte-identical (no clobber), lineage-append premise-checked append-only. **The Precondition at the top of this doc applies** before testing — exe was locked.

### ☐ 7. Create → fork sub-flow (Import and modify)
- **Phase / Run:** Phase 6, **Run 4** — **P6.T8**.
- **Check:** Create → import box → `Start →` → fork-paste (import-code Box; `Preview →` disabled until pasted) → paste a known BIO-MODLIST-V1 code → `Preview →` shows the **parent's** packed name/author + Overview + 6 tabs, **no draft banner / no disabled primary even for an `allow_auto_install=false` code** (forking is always allowed), `⑂ fork info` → the reused popup shows the **parent's** lineage → `Begin Import →` → the fork-download chassis (empty grid, live fetch is Phase 7 — forward-compatible, not a bug) → the forked Workspace opens at Step 2 with a `⑂ Fork` badge.

### ☐ 8. Fork lineage / credit (`modlists.json`)
- **Phase / Run:** Phase 6, **Run 4** — **P6.T8** (SPEC §13.3 credit guarantee).
- **Check:** after a fork, the new `modlists.json` entry has `author` = your Settings → General user name (absent if blank) and `forked_from` = the **parent's chain + the immediate parent appended last** (append-only — every prior ancestor + author preserved verbatim; nothing rewritten).

### ☐ 9. Workspace persistence — dirty-bit + nav-away flush
- **Phase / Run:** Phase 6, **Run 4** — **P6.T11** + **P6.T15**.
- **Check:** in a workspace, toggle a Step-2 checkbox → `modlists/<id>/workspace.json` written ~1 s later (debounce); drag/collapse/undo in Step 3 → same; leave it idle → no rewrite (zero idle cost). Then: edit a Step-2 checkbox, **immediately** click Home (before the 1 s debounce), quit, relaunch, reopen → the change **persisted** (the synchronous nav-away flush).

## Create #4 cluster — orchestrator-render-verified; pending your one-pass live confirm

### ☐ FR2-4. Create — selectable-box choose UX (Create #4, P1–P5)
- **Status:** **orchestrator-render-verified; pending user one-pass live confirm (Fix-Run-6 `e305f92`)** — the orchestrator viewed full-shell rendered PNGs at 1280/1045/1021/960 × scratch/import via the `egui_kittest` gate; every user-reported #4 sub-issue is visually fixed in those renders. Final human one-pass live confirm on the gate-fresh binary is all that remains.
- **Check (Create rail item):** a **`Choose one`** header; **two equal-height selectable boxes** (click anywhere on a box to select it — the selected box gets an accent border + faint tint + legible text; **no** in-box `start →` / `paste share code →` buttons); a single primary **`Start →`** at the **bottom-right** inside the shared dashed-border footer, styled like the workspace `Next →` button, footer bottom-pinned. The shared input chassis (name / game / destination / Browse) shares one height + right-edge alignment; the right margin is present. `Start →` with the from-scratch box selected creates the modlist + opens the Workspace; with the import box selected it enters fork-paste. The **game ComboBox shows only when the from-scratch box is selected** (redesign chrome, EET default); selecting the **import box replaces it** with a read-only "imported" note.

### FR2-9. (DEFERRED TO PHASE 8 — do not expect fixed here) Prompt popup vertical growth (#4a)
- **Resolved as deferred (user decision 2026-05-17): moved to Phase 8.** The per-component prompt popup still grows as the mouse moves; root cause is one line in **protected BIO** (`src/ui/step2/prompt/prompt_popup_step2.rs:31`). Phase 8 likely introduces a new carve-out for important-but-small BIO bug fixes — this is the first candidate. Recorded in `plan/phase-08-popup-reskins-polish.md` deferred backlog + overview 2026-05-17. **Not a Phase-6 gap any more — do not re-flag; do not test for it here.** (Decision-1 sibling: #7b preview-weidu-3-hue is also Phase-8-deferred, same backlog — it was never a Phase-6 testable item.)

## Deferred to Phase 8 — minor Create-screen polish (do NOT test here)

→ Phase 8 (see `plan/phase-08-popup-reskins-polish.md` "Create-screen UI cleanup — deferred from Phase-6 verification (user, 2026-05-18)", with screenshots). The 5 minor Create issues (box-title↔input gap; game-box height vs modlist-name; game-box + "imported" text vertical centering; destination-box height; Browse-button height — all 4 controls equal height) **and** the two side-by-side "Choose one" boxes equal-height problem (misaligned on click-into-create until a resize, then off by a pixel or two — implement the standard equal-height technique, not another ad-hoc measured-max pass) are residual refinements on top of Fix-Run-6; they are minor and verified via the `egui_kittest` render gate in Phase 8. Fix-Run-6 (footer pin, right-margin, selected-box contrast, P1–P5) is NOT reverted.

## Not in this list (and why)

- `9b5b9d5` — destructive Select-via-WeiDU-Log rebuilding the imported tab's Step-3 order (Phase 6, Run 2 follow-up). Committed in a **prior** session; its message states user-verified end-to-end. Re-verify only if you want certainty given this project's stale-binary history.
- The 6 user-confirmed Fix-Run-2 items (input border / nav step-indicator / Step-3 two hints / glyphs / load-draft delete / rail-unstuck), #8 forked share code (FR2-8), and the DATA-LOSS Fix-Run-3/4 fix — **user-confirmed closed**, removed from this list.

## Phase 7 — verify at phase end (one pass)

Per `infinity_orchestrator/plan/phase-07-install-runtime.md` "Verification" steps 1–11 + the C3 clean-exit and C5 rail-lock checks. Populated per-run as Phase 7 runs land; do not test until the orchestrator signals Phase 7 complete.

## Orchestrator-side verification already done (so you don't re-do it)

For every item above the orchestrator independently verified, pre-commit: BIO-source guard empty (no protected BIO touched), `cargo test --lib` green (behavior-neutral), `%APPDATA%\bio\modlists.json` byte-identical across the test run (no data-loss), `cargo build --bin BIO --release` links clean, the C4 boundary grep-proven (zero orchestrator calls of `page_step3`/`content_step3`/`render_toolbar`), the high-risk files spot-read, and (for any redesign-UI change) the `egui_kittest` full-shell multi-width rendered PNG personally reviewed. **What remains is purely your visual/UX sign-off** — the part only a human at the screen can do.
