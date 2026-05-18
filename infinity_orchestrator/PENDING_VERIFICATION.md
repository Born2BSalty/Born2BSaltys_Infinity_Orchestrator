# Pending User Verification

Changes committed but **never seen rendered by you** — every item below was committed while the app held the `infinity_orchestrator.exe` lock, so none has been visually verified. Work through this when you're back.

_Last updated 2026-05-17 (Phase-6 user-feedback **Fix-Run 2** implemented — see the Fix-Run-2 section below; Fix-Run-1 persistence round-trip marked user-verified done). Branch `overhaul/infinity_orchestrator`._

> **Fix-Run-1 (persistence round-trip + in-memory isolation, commit `8dfb905`) — ✅ USER-VERIFIED DONE.** The per-modlist cold-resume round-trip + save-draft + rename were user-verified end-to-end on a gate-confirmed binary (overview/HANDOFF 2026-05-17). Do not re-test; do not touch that code (`workspace_state_loader.rs`, the dirty-bit/nav-flush, `step2_resume_scan.rs`).

> **Items 5 & 6 below are partially SUPERSEDED by Fix-Run 2** — see the Fix-Run-2 section. Specifically: item 5's "two starting-point **cards** with in-box CTAs" is replaced by the selectable-box + single `Start →` UX; item 6's "the dialog's `Delete` Kebab item is intentionally inert" is now **wired** (full Home delete machinery). Verify those via the Fix-Run-2 items, not the stale wording in 5/6.

---

## ⚠️ Orchestrator note — read first (2026-05-17)

**A process failure on the Run-3 commit (`8fa52f8`), disclosed not buried.** It is **53 files**, not the ~11-file Run-3 deliverable: I ran `git add -A` and did not reconcile `git status` against the expected set before committing, so ~42 redesign files of **pre-existing `rustfmt --edition 2024` + CRLF-normalization churn** got bundled in. **Verified safe:** zero protected BIO (BIO-guard = only the authorized carve-out #3 line), all 42 redesign-owned, every change formatting-only, **proven semantically inert by clean compile + `cargo test --lib` 254/0 + byte-identical `modlists.json`**. The Run-3 substance is correct + independently verified. The pushed commit is left as-is (formatting-only/green; un-bundling would need a disproportionate history rewrite) — full record in overview.md 2026-05-17.

**Resolved — you approved 2026-05-17.** The orchestrator skill's commit step (`.claude/skills/orchestrator/SKILL.md` → "How to run a run" step 4) is now hardened: never `git add -A` for a run commit; stage an explicit file list; reconcile `git diff --cached --name-only` against the expected set immediately before commit; an unexpected staged-file delta is an ABORT-and-investigate gate like a BIO-guard hit. Applied + dogfooded on the commit that shipped this fix.

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

## Phase 6 Run 3 — SHIPPED + orchestrator-verified, pending your visual sign-off

Shipped in the Phase-6 Run-3 commit (Create + Load Draft + routing — P6.T7/T9/T13/T14). Orchestrator-independently verified pre-commit: BIO-source guard clean (only the authorized carve-out #3 `pub mod` line), zero scope creep, `cargo test --lib` **254/0** (+21 substantive), `modlists.json` byte-identical (no clobber), `cargo build --bin BIO` clean, the `create_modlist` PLAN-GAP + the SPEC §10.2 doc-drift premise-checked TRUE and fixed in the same commit. **The exe relink was blocked by your running app** — same as items 1–4: the **Precondition at the top of this doc applies** (close the app → rebuild twice to a confirmed no-op → relaunch) before testing these.

### ☐ 5. Create screen — choose-mode setup + starting-point cards
- **Phase / Run:** Phase 6, **Run 3** — **P6.T7** + **P6.T13** (wire Create into `page_router`).
- **Check:** the Create rail item opens the real screen (not the stub): a setup Box with modlist **name**, **game** ComboBox (defaults to **EET**; options EET / BGEE / BG2EE / IWDEE), destination FolderInput + the conditional DestinationNotEmptyWarning on a non-empty dir (Clear / Backup only — **no** Continue-partial, SPEC §5.1). Two starting-point cards: `start →` (with a name + valid existing destination → a new entry appears in `%APPDATA%\bio\modlists.json`, `modlists\<id>\workspace.json` is created, and the **Workspace opens at Step 2**, header `Editing <name>`; blank destination falls back to `<config>\modlists\installs\<slug>`) and `paste share code →` (shows a clear "lands in Run 4 (P6.T8)" placeholder + `← Back to choose` — the real fork flow is Run 4, **not** a bug).

### ☐ 6. Load Draft dialog (Resume in-progress build)
- **Phase / Run:** Phase 6, **Run 3** — **P6.T9** + **P6.T14** (Resume routing).
- **Check:** Create's `load draft` button opens a **non-blocking dialog** titled "Resume in-progress build" (it is **NOT** a file picker — SPEC §5.2/§10.2) listing in-progress builds as the shared Home card chassis (`resume` + Kebab `Copy import code` / `Delete`), empty-state copy when none, `Cancel`-only footer. `resume` closes it and opens the Workspace at Step 2 (`Editing <name>`). `Copy import code` → transient `✓ Copied…` (or an honest "no import code yet" for a pre-Phase-7 build). The Home in-progress card `resume` (already shipped) still routes to the Workspace too. **Note:** the dialog's `Delete` Kebab item is intentionally inert here (wireframe-faithful — in-progress deletion is done from Home; not a bug).

## Review of older items (2026-05-17, Run-3 dispatch)

Reviewed items 1–4 against Run 3's surface (Create / Load Draft / routing — a distinct screen from the Step-2/3/4 workspace): **none are partial-from-an-old-run and none are superseded or made irrelevant by Run 3.** Items 1–4 are complete-and-committed, awaiting only the human visual sign-off; they stand as written. (This section is where future runs record any test item a later change makes moot — none yet.)

## Phase 6 Run 4 — SHIPPED + orchestrator-verified, pending your visual sign-off (**Phase 6 COMPLETE**)

Shipped in the Run-4 commit (P6.T8 fork sub-flow + P6.T11 dirty-bit persistence + P6.T15 nav-away flush). Independently verified: BIO-guard empty, exactly the 11 expected files (the new explicit-reconcile staging gate's first code-run exercise), `cargo test --lib` 270/0, `modlists.json` byte-identical (no clobber), lineage-append premise-checked append-only. **The Precondition at the top of this doc applies** (close app → rebuild twice to a confirmed no-op → relaunch) before testing — exe was locked.

### ☐ 7. Create → fork sub-flow (Import and modify)
- **Phase / Run:** Phase 6, **Run 4** — **P6.T8**.
- **Check:** Create → `paste share code →` → fork-paste (import-code Box; `Preview →` disabled until pasted) → paste a known BIO-MODLIST-V1 code → `Preview →` shows the **parent's** packed name/author + Overview + 6 tabs, **no draft banner / no disabled primary even for an `allow_auto_install=false` code** (forking is always allowed), `⑂ fork info` → the reused popup shows the **parent's** lineage → `Begin Import →` → the fork-download chassis (empty grid, live fetch is Phase 7 — forward-compatible, not a bug) → the forked Workspace opens at Step 2 with a `⑂ Fork` badge.

### ☐ 8. Fork lineage / credit (`modlists.json`)
- **Phase / Run:** Phase 6, **Run 4** — **P6.T8** (SPEC §13.3 credit guarantee).
- **Check:** after a fork, the new `modlists.json` entry has `author` = your Settings → General user name (absent if blank) and `forked_from` = the **parent's chain + the immediate parent appended last** (append-only — every prior ancestor + author preserved verbatim; nothing rewritten).

### ☐ 9. Workspace persistence — dirty-bit + nav-away flush
- **Phase / Run:** Phase 6, **Run 4** — **P6.T11** + **P6.T15**.
- **Check:** in a workspace, toggle a Step-2 checkbox → `modlists/<id>/workspace.json` written ~1 s later (debounce); drag/collapse/undo in Step 3 → same; leave it idle → no rewrite (zero idle cost). Then: edit a Step-2 checkbox, **immediately** click Home (before the 1 s debounce), quit, relaunch, reopen → the change **persisted** (the synchronous nav-away flush).

## Phase-6 user-feedback Fix-Run 2 — implemented, pending your visual sign-off

UI/UX cleanup + the SPEC cascade + a provenance test-code mint. **Implemented + self-verified; the orchestrator independently verifies + commits.** The Precondition at the top of this doc applies (close app → rebuild twice to a confirmed no-op → relaunch). `cargo test --lib` **275/0**; `%APPDATA%\bio\modlists.json` SHA256 byte-identical pre/post (`3d8212d7…`).

### ☐ FR2-1. App-wide input border no longer indented
- **What changed:** a single shared input primitive now strokes the **outer allocated box**, not egui's margin-inset inner galley rect (root cause: egui `TextEdit` returns the inner rect).
- **Check:** the **modlist name** + **destination** inputs (Create), every **Settings → Paths / Advanced / General name** row input, and the **Step-2 search** box all have a border that **hugs the field box** — no dead inset / "indented" gap between the visible field and its border. Text still sits its normal padding inside the box.

### ☐ FR2-2. No nav step-indicator on any workspace step
- **Check:** on **all 4 steps** (2/3/4/5), the bottom nav row has `← Previous`, the `next: <label>` / `final step` forward hint, and `Next →` — **but NOT** the `on Step N · <Label> · step i of 4` line that used to sit between `← Previous` and the hint. (Deliberate — recorded in SPEC §2.2.)

### ☐ FR2-3. Step 3 — two hint lines, no count line
- **Check (resume the in-progress seed → Step 3):** **two** hint lines render — the shell hint *"Review and adjust install order. Drag to reorder; right-click for more actions."* (under the progress bar) **and** *"Right-click a component for more actions, including uncheck and prompt tools."* (above the tab row). The *"N components ready to install on <tab> · across M mods"* count line is **GONE** from Step 3 (it still shows in **Step 4**).

### ☐ FR2-4. Create — new selectable-box choose UX
- **Check (Create rail item):** a **`Choose one`** header; **two selectable boxes** (click anywhere on a box to select it — the selected box gets an accent border + faint tint; **no** in-box `start →` / `paste share code →` buttons); a single primary **`Start →`** at the **bottom-right**, styled like the workspace `Next →` button. `Start →` with the from-scratch box selected creates the modlist + opens the Workspace; with the import box selected it enters fork-paste. The **game ComboBox shows only when the from-scratch box is selected** (redesign chrome, EET default); selecting the **import box replaces it** with a read-only "comes from the imported modlist" note.

### ☐ FR2-5. Glyphs render (no `?` tofu)
- **Check:** the `Start →` arrow (Create) renders as a real `→` (not `?`); the Load-Draft `✓ Copied import code …` confirmation shows a real `✓` (not `?`).

### ☐ FR2-6. Load Draft `Delete` fully works
- **Check (Create → `load draft` → a build's Kebab → `Delete`):** a danger confirm (`Delete "<name>"?` + the install-folder body) appears; **Confirm** removes the entry from the list **and** from Home **and** deletes its on-disk install folder (guarded) — the exact Home delete. **Cancel** changes nothing. (SPEC §5.2 — user-directed deviation; the wireframe left this inert.)

### ☐ FR2-7. Rail no longer stuck on Home in a Workspace
- **Check:** from Home, `resume` an in-progress build → inside the Workspace the **left rail highlights `Create`**, not `Home`. (Matches the canonical wireframe — recorded SPEC §2.1.)

### ☐ FR2-8. Minted forked share code (paste it to test forking)
- **What:** the orchestrator hands you a `BIO-MODLIST-V1:` code minted by the 7c test helper (baked name/author + a 2-deep `⑂` lineage; format-correct by construction — round-tripped through BIO's own decoder).
- **Check:** Create → select the **import** box → `Start →` → fork-paste → paste the minted code → `Preview →` → the preview shows the baked **name** (`Tactical EET 2026 (shared)`) + **author** (`@b2bs`), and `⑂ fork info` opens the lineage popup showing the 2-ancestor chain (`Born2BSalty's EET Basics` by @b2bs ↳ `EET Tactical Mid` by @olim ↳ this).

### FR2-9. (DEFERRED TO PHASE 8 — do not expect fixed here) Prompt popup vertical growth (#4a)
- **Resolved as deferred (user decision 2026-05-17): moved to Phase 8.** The per-component prompt popup still grows as the mouse moves; root cause is one line in **protected BIO** (`src/ui/step2/prompt/prompt_popup_step2.rs:31`). Phase 8 likely introduces a new carve-out for important-but-small BIO bug fixes — this is the first candidate. Recorded in `plan/phase-08-popup-reskins-polish.md` deferred backlog + overview 2026-05-17. **Not a Phase-6 gap any more — do not re-flag; do not test for it here.** (Decision-1 sibling: #7b preview-weidu-3-hue is also Phase-8-deferred, same backlog — it was never a Phase-6 testable item.)

## Not in this list (and why)

- `9b5b9d5` — destructive Select-via-WeiDU-Log rebuilding the imported tab's Step-3 order (Phase 6, Run 2 follow-up). Committed in a **prior** session; its message states user-verified end-to-end. Re-verify only if you want certainty given this project's stale-binary history.
- The 2026-05-17 `9b5b9d5` doc-sync, the orchestrator-skill split, and the premature Step-3-C4-cascade **retraction** were documentation / governance only — nothing to visually test.

## Orchestrator-side verification already done (so you don't re-do it)

For every item above the orchestrator independently verified, pre-commit: BIO-source guard empty (no protected BIO touched), `cargo test --lib` green (233/0 after Step-3 C4; behavior-neutral), `%APPDATA%\bio\modlists.json` byte-identical across the test run (no data-loss), `cargo build --bin BIO --release` links clean, the C4 boundary grep-proven (zero orchestrator calls of `page_step3`/`content_step3`/`render_toolbar`), and the high-risk files spot-read. **What remains is purely your visual/UX sign-off** — the part only a human at the screen can do.
