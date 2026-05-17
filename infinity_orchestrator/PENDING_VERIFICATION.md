# Pending User Verification

Changes committed but **never seen rendered by you** — every item below was committed while the app held the `infinity_orchestrator.exe` lock, so none has been visually verified. Work through this when you're back.

_Last updated 2026-05-17 (Phase 6 Run 3 dispatched — see "In flight" section). Branch `overhaul/infinity_orchestrator`, all committed items pushed to origin._

---

## ⚠️ Precondition — do this ONCE before testing anything

Every on-disk binary is **stale** (commits landed under the exe lock). Before any item:

1. Fully close Infinity Orchestrator.
2. `cargo build --bin infinity_orchestrator --release` — run it **twice**; the second must end in a no-op `Finished` with **no `Compiling bio`** line (that is the only proof the binary is current).
3. The seed registry (`%APPDATA%\bio\modlists.json`) was mutated during this session's testing. Re-prep the canonical 2-entry seed if the in-progress modlist is gone — mechanism in the orchestrator skill (`.claude/skills/orchestrator/SKILL.md` → "Test fixtures / runtime"). The **in-progress** seed modlist is what the resume → Step-2/3/4 flow needs.
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

### ☐ 4. Step-3 C4 chrome — *the original item that started all this*
- **Phase / Run:** Phase 6, **P6.T2d** (Step-3 C4 — new this session).
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

---

## ⏳ In flight — Phase 6 Run 3 (NOT yet testable)

Dispatched 2026-05-17 to a plan-implementer; **not yet implemented, built, or orchestrator-verified.** Do **not** test these until the orchestrator marks them ready (it will move them up into "Test items" with a commit + confirmed-fresh-binary note once the run returns and is independently verified). Listed now so the scope is visible; derived from the phase-06 acceptance criteria (the contract the run must meet).

### ☐ 5. Create screen — choose-mode setup + starting-point cards
- **Phase / Run:** Phase 6, **Run 3** — **P6.T7** + **P6.T13** (wire Create into `page_router`).
- **Will check:** the Create rail item opens the real screen (not the stub): a setup Box with modlist **name**, **game** ComboBox (EET default / BGEE / BG2EE / IWDEE), destination FolderInput + the conditional DestinationNotEmptyWarning on a non-empty dir, partial-install option disabled. Two cards: `start →` (with name+game+valid existing destination → a new entry appears in `modlists.json` and the **Workspace opens at Step 2**, header `Editing <name>`) and `paste share code →` (Run-3 shows a deferred placeholder — the real fork flow is Run 4 / P6.T8).

### ☐ 6. Load Draft dialog (Resume in-progress build)
- **Phase / Run:** Phase 6, **Run 3** — **P6.T9** + **P6.T14** (Resume routing).
- **Will check:** Home `load draft` (and the in-progress card `resume`) opens a **non-blocking dialog** (NOT a file picker — SPEC §5.2) listing in-progress builds as the shared Home card chassis (resume + Kebab Copy-import-code / Delete), empty-state copy when none, `Cancel`-only footer. `resume` closes it and opens the Workspace at Step 2 with that build's state.

## Review of older items (2026-05-17, Run-3 dispatch)

Reviewed items 1–4 against Run 3's surface (Create / Load Draft / routing — a distinct screen from the Step-2/3/4 workspace): **none are partial-from-an-old-run and none are superseded or made irrelevant by Run 3.** Items 1–4 are complete-and-committed, awaiting only the human visual sign-off; they stand as written. (This section is where future runs record any test item a later change makes moot — none yet.)

## Not in this list (and why)

- `9b5b9d5` — destructive Select-via-WeiDU-Log rebuilding the imported tab's Step-3 order (Phase 6, Run 2 follow-up). Committed in a **prior** session; its message states user-verified end-to-end. Re-verify only if you want certainty given this project's stale-binary history.
- The 2026-05-17 `9b5b9d5` doc-sync, the orchestrator-skill split, and the premature Step-3-C4-cascade **retraction** were documentation / governance only — nothing to visually test.

## Orchestrator-side verification already done (so you don't re-do it)

For every item above the orchestrator independently verified, pre-commit: BIO-source guard empty (no protected BIO touched), `cargo test --lib` green (233/0 after Step-3 C4; behavior-neutral), `%APPDATA%\bio\modlists.json` byte-identical across the test run (no data-loss), `cargo build --bin BIO --release` links clean, the C4 boundary grep-proven (zero orchestrator calls of `page_step3`/`content_step3`/`render_toolbar`), and the high-risk files spot-read. **What remains is purely your visual/UX sign-off** — the part only a human at the screen can do.
