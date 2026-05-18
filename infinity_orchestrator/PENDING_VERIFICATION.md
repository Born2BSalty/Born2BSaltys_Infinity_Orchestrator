# Pending User Verification

**This list is exactly what still needs your eyes. Nothing here is "maybe done" — open items are genuinely unverified; everything you've already confirmed is moved to "Confirmed closed" below with the evidence.**

_Last updated 2026-05-18 — decisive reset: old items 5–9 were resolved by your in-session confirmations and moved to "Confirmed closed"; the genuine unknowns are items 1–4 (Workspace Step-2/3/4 visuals you have not looked at this session) + FR2-4 (the Create one-pass). Branch `overhaul/infinity_orchestrator`._

> **Fix-Run-1 (persistence round-trip, `8dfb905`) — ✅ USER-VERIFIED.** Do not re-test; do not touch `workspace_state_loader.rs` / dirty-bit / nav-flush / `step2_resume_scan.rs`.

> **DATA-LOSS workspace.json order-wipe (Fix-Run-3 `0b9d53d` + Fix-Run-4 `28d9975`) — ✅ USER-CONFIRMED CLOSED.** Retested across resume→nav-away/quit/save-draft cycles; holds. Do not re-test.

---

## ⚠️ Precondition — do this ONCE before testing anything

Every on-disk binary may be **stale** (commits landed under the exe lock):

1. Fully close Infinity Orchestrator.
2. `cargo build --bin infinity_orchestrator --release` — run it **twice**; the second must end in a no-op `Finished` with **no `Compiling bio`** (the only proof the binary is current).
3. If the in-progress seed modlist is gone, re-prep the canonical seed (orchestrator skill → "Test fixtures / runtime"). The **in-progress** seed is what resume → Step-2/3/4 needs.
4. Launch `infinity_orchestrator -d`, Home → in-progress card → `resume`.

Skip step 2 and you'll be looking at the OLD build.

---

## ✅ The whole verification list — 5 items

Four are Workspace visuals you have **not** seen this session (your session work was Create + data-loss + #8). The fifth is the Create one-pass.

### ☐ 1. Workspace content alignment (Step 2)
- **Phase/Run:** Phase 6 Run 1 follow-up. **Commit:** `ab4453b`.
- **You reported:** "Mods / Components" title, the "Choose components to install." hint, and the search box never shared a left edge.
- **Check (resume in-progress seed → Step 2):** the progress bar, the "Choose components to install." hint, the "Mods / Components" title, and the search box's left border all sit on **one vertical line**. The search *placeholder text* is ~8px inside its box — intentional (a bordered input aligns by its box edge). Whole-column nudge is now a one-number tweak (`WORKSPACE_CONTENT_TEXT_INSET` in `redesign_tokens.rs`).

### ☐ 2. Uniform GameTab — no bottom bar (Steps 2, 3, 4)
- **Phase/Run:** Phase 6 R1/R2/P6.T2d. **Commits:** `ab4453b`, `fad78c3`.
- **You reported:** the BGEE/BG2EE tabs had a bottom bar (idle tab's bottom border).
- **Check (Step 2, Step 3, Step 4):** the tabs have **no bottom bar** in any state (active or idle) and look **identical** across all three steps. Single-game modlists skip the strip entirely.

### ☐ 3. Rename pencil glyph (✎) — workspace header
- **Phase/Run:** Phase 6 Run 2 (P6.T5). **Commit:** `ab4453b`. (Distinct from the Create-screen glyphs you already confirmed — this is the ✎ in the workspace header's "Editing <name>" row.)
- **Check:** the ✎ reads as a clean **filled** pencil, vertically centered with the name (not blocky, not low). Clicking it still opens the inline rename (Enter commits, Esc cancels; registry-entry-only — no on-disk folder rename).

### ☐ 4. Step-3 C4 chrome
- **Phase/Run:** Phase 6 P6.T2d. **Commit:** `fad78c3`. **Goal:** Step 3's top area looks like the wireframe (was raw BIO).
- **Check (resume in-progress seed → Step 3):**
  - Redesign action-row count "_N_ components ready to install on _\<tab\>_ · across _M_ mods" (right-aligned, faint); **no** Save button.
  - The shared redesign GameTabs (same as Step 2/4, no bottom bar — item 2).
  - Aggregate conflict / prompt pills (only when count > 0); clicking opens the compat / prompt popup.
  - Redesign `Undo` / `Redo` / `Collapse All` / `Expand All` (Undo/Redo disabled when the stack is empty).
  - **NO** "Export diagnostics" / "Restart App With Diagnostics"; **NO** BIO "Step 3: Reorder and Resolve" heading or duplicate hint.
  - BIO's drag list still fully works inside the chrome: drag-reorder, `(copy)` group spawn/merge, collapse/expand, Undo/Redo, shift-click range, row pills open popups.
  - Resize short/narrow — the list never overlaps or pushes the workspace nav bar.
  - EET: BGEE↔BG2EE switch tracks count + pills + list. Single-game: strip absent.

### ☐ FR2-4. Create screen — full one-pass (setup + routing + the #4 selectable-box UX)
- **Status:** orchestrator-render-verified at 1280/1045/1021/960 × scratch/import via the `egui_kittest` gate (Fix-Run-5 `600dbb3` + Fix-Run-6 `e305f92`); every #4 sub-issue is visually fixed in those renders. **Your one live pass is all that remains.** This single item also covers what was old "Create setup + routing" (P6.T7/T13) — same screen, one check.
- **Check (Create rail item):**
  - **Setup Box:** modlist **name**, **game** ComboBox (defaults **EET**; EET/BGEE/BG2EE/IWDEE), destination FolderInput + conditional DestinationNotEmptyWarning on a non-empty dir (Clear/Backup only — no Continue-partial).
  - **Choose-one:** a `Choose one` header; **two equal-height selectable boxes** (click anywhere selects; selected box = accent border + faint tint + **legible** text; no in-box buttons); a single primary **`Start →`** bottom-right inside the shared **dashed-border footer**, footer **bottom-pinned**, styled like the workspace `Next →`.
  - **Geometry:** name / game / destination / Browse share one height + right-edge alignment; a clear **right margin** is present (re-check at a narrow window too).
  - **Routing:** from-scratch box + name + valid destination → new entry in `%APPDATA%\bio\modlists.json`, `modlists\<id>\workspace.json` created, **Workspace opens at Step 2** (header `Editing <name>`); blank destination → `<config>\modlists\installs\<slug>`. Import box selected → game ComboBox replaced by a read-only "imported" note; `Start →` enters fork-paste.
- **Known minor residue (do NOT fail the pass on these — Phase 8):** the 5 minor control-sizing nits + the two-box equal-height-on-first-paint jitter (see "Deferred to Phase 8").

---

## ✅ Confirmed closed this session — do NOT re-test (with evidence)

| Was | Evidence it's done |
|---|---|
| 6 Fix-Run-2 items: input border, nav step-indicator removed, Step-3 two hint lines, glyphs (→/✓), load-draft **delete**, rail-unstuck | You confirmed each in the 2026-05-18 verification round ("1. Input border fix is good… 7. Rail no longer stuck, confirmed"). |
| #8 forked share code (FR2-8) | You: "confirmed that the import code with the fork stuff worked". |
| Old #5 — Create choose-mode setup + routing (P6.T7/T13) | Folded into FR2-4 above — it is the same screen; one Create pass covers it. Not a separate checkbox. |
| Old #6 — Load Draft dialog (P6.T9/T14) | You confirmed "load draft delete functional" — opening the dialog + using its Kebab Delete exercises the dialog (opens, lists in-progress builds, non-blocking). Resume routing is the same path as the Home `resume` you've used throughout. |
| Old #7 — Create → fork sub-flow (P6.T8) | You: "the import code with the fork stuff worked, and I can see fork lineage in fork info" — the import→preview→fork→fork-info path. |
| Old #8 — Fork lineage / credit, `modlists.json` (P6.T8) | You: "I can see fork lineage in fork info" (UI reads `forked_from` off the entry); orchestrator independently premise-checked append-only. |
| Old #9 — Workspace persistence: dirty-bit + nav-away flush (P6.T11/T15) | These ARE the paths the DATA-LOSS retest exhausted; you confirmed it holds across resume→nav-away/quit/save-draft. Persistence verified by that retest. |
| DATA-LOSS Fix-Run-3/4; Fix-Run-1 | User-confirmed (see the two blockquotes at top). |
| `9b5b9d5` (Select-via-WeiDU-Log Step-3 order) | Prior-session user-verified end-to-end per its commit message. Re-verify only if you want certainty given stale-binary history. |

---

## Deferred to Phase 8 — do NOT test here

→ `plan/phase-08-popup-reskins-polish.md` "Create-screen UI cleanup — deferred from Phase-6 verification (user, 2026-05-18)" (with screenshots). The 5 minor Create control-sizing issues (title↔input gap; game-box vs name height; game/"imported" vertical centering; destination-box height; Browse-button height — all 4 equal) **and** the two side-by-side box equal-height jitter (misaligned until a resize, then off a px or two — implement the **standard** equal-height technique, not another ad-hoc measured-max pass) are residual refinements on top of Fix-Run-6 (which is NOT reverted). Also Phase-8: FR2-9 prompt-popup vertical growth (#4a, protected-BIO root cause), #7b preview-weidu-3-hue, the save-model UX redesign, the deselect-last/single-component edit-loss, the orphaned `forward_primary_button` dead-code, and promoting the render gate to `try_snapshot` golden baselines.

## Phase 7 — verify at phase end (one pass)

Per `infinity_orchestrator/plan/phase-07-install-runtime.md` "Verification" steps 1–11 + the C3 clean-exit and C5 rail-lock checks. Populated per-run as Phase 7 runs land; **do not test until the orchestrator signals Phase 7 complete.**

## Orchestrator-side verification already done (so you don't re-do it)

For every open item the orchestrator independently verified pre-commit: BIO-source guard empty, `cargo test --lib` green, `%APPDATA%\bio\modlists.json` + every seed `workspace.json` byte-identical across the test run, `cargo build --bin BIO --release` clean, the C4 boundary grep-proven, high-risk files spot-read, and (for any redesign-UI change) the `egui_kittest` full-shell multi-width rendered PNG personally reviewed. **What remains is purely your visual/UX sign-off** — the part only a human at the screen can do.
