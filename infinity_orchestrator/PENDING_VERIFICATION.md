# Pending User Verification

**This list is exactly what still needs your eyes. Nothing here is "maybe done" — open items are genuinely unverified; everything you've already confirmed is moved to "Confirmed closed" below with the evidence.**

_Last updated 2026-05-18 — Phase-6 verification plate CLEARED by your decision: #1 (Workspace Step-2 alignment), #3 (workspace rename pencil), #4 (Step-3 C4 chrome) user-verified → Confirmed closed; FR2-4 (the Create one-pass) + #2 (the residual grey bar under the GameTab strip — reused-BIO content-pane grey top) punted to the Phase-8 verification pass (the Phase-6 fixes stand and are NOT reverted — only the live visual sign-off moves). Branch `overhaul/infinity_orchestrator`._

> **Fix-Run-1 (persistence round-trip, `8dfb905`) — ✅ USER-VERIFIED.** Do not re-test; do not touch `workspace_state_loader.rs` / dirty-bit / nav-flush / `step2_resume_scan.rs`.

> **DATA-LOSS workspace.json order-wipe (Fix-Run-3 `0b9d53d` + Fix-Run-4 `28d9975`) — ✅ USER-CONFIRMED CLOSED.** Retested across resume→nav-away/quit/save-draft cycles; holds. Do not re-test.

---

## ✅ Open items — NONE (Phase-6 verification plate is clear)

Every Phase-6 verification item is resolved: **#1, #3, #4 you verified this session** (moved to "Confirmed closed" with evidence); **FR2-4 and #2 are deferred to the Phase-8 one-pass** (see "Deferred to Phase 8" — the fixes are shipped and unreverted, only the eyeballing moves). **The next user verification event is the Phase-7 phase-end pass — do not test until the orchestrator signals Phase 7 complete.**

---

## ⚠️ Precondition — do this ONCE before any test pass

Every on-disk binary may be **stale** (commits land under the exe lock):

1. Fully close Infinity Orchestrator.
2. `cargo build --bin infinity_orchestrator --release` — run it **twice**; the second must end in a no-op `Finished` with **no `Compiling bio`** (the only proof the binary is current).
3. If the in-progress seed modlist is gone, re-prep the canonical seed (orchestrator skill → "Test fixtures / runtime"). The **in-progress** seed is what resume → Step-2/3/4 needs.
4. Launch `infinity_orchestrator -d`, Home → in-progress card → `resume`.

Skip step 2 and you'll be looking at the OLD build.

---

## ✅ Confirmed closed — do NOT re-test (with evidence)

| Was | Evidence it's done |
|---|---|
| **#1 Workspace content alignment (Step 2)** — Phase 6 Run 1 follow-up, `ab4453b` | You confirmed verified, 2026-05-18 session ("#1 is correct. verified. close out in phase 6"). Progress bar / "Choose components to install." hint / "Mods / Components" title / search-box left border share one vertical line (placeholder text ~8px inside its box is intentional). |
| **#3 Rename pencil glyph (✎) — workspace header** — Phase 6 Run 2 / P6.T5, `ab4453b` | You confirmed verified, 2026-05-18 session ("#3 is verified"). |
| **#4 Step-3 C4 chrome** — Phase 6 P6.T2d, `fad78c3` | You confirmed verified, 2026-05-18 session ("#4 is verified"). |
| 6 Fix-Run-2 items: input border, nav step-indicator removed, Step-3 two hint lines, glyphs (→/✓), load-draft **delete**, rail-unstuck | You confirmed each in the 2026-05-18 verification round ("1. Input border fix is good… 7. Rail no longer stuck, confirmed"). |
| #8 forked share code (FR2-8) | You: "confirmed that the import code with the fork stuff worked". |
| Old #5 — Create choose-mode setup + routing (P6.T7/T13) | Folded into FR2-4 (now Phase-8 deferred) — same screen; one Create pass covers it. |
| Old #6 — Load Draft dialog (P6.T9/T14) | You confirmed "load draft delete functional" — opening the dialog + using its Kebab Delete exercises the dialog (opens, lists in-progress builds, non-blocking). Resume routing is the same path as the Home `resume` you've used throughout. |
| Old #7 — Create → fork sub-flow (P6.T8) | You: "the import code with the fork stuff worked, and I can see fork lineage in fork info" — the import→preview→fork→fork-info path. |
| Old #8 — Fork lineage / credit, `modlists.json` (P6.T8) | You: "I can see fork lineage in fork info" (UI reads `forked_from` off the entry); orchestrator independently premise-checked append-only. |
| Old #9 — Workspace persistence: dirty-bit + nav-away flush (P6.T11/T15) | These ARE the paths the DATA-LOSS retest exhausted; you confirmed it holds across resume→nav-away/quit/save-draft. Persistence verified by that retest. |
| DATA-LOSS Fix-Run-3/4; Fix-Run-1 | User-confirmed (see the two blockquotes at top). |
| `9b5b9d5` (Select-via-WeiDU-Log Step-3 order) | Prior-session user-verified end-to-end per its commit message. Re-verify only if you want certainty given stale-binary history. |

---

## Deferred to Phase 8 — do NOT test here

**Phase-6 verification items punted to the Phase-8 one-pass (user, 2026-05-18) — the fixes are shipped and NOT reverted; only the live visual sign-off moves:**

- **FR2-4 — Create screen full one-pass** (setup + routing + the #4 selectable-box UX). Orchestrator-render-verified at 1280/1045/1021/960 × scratch/import via the `egui_kittest` gate (Fix-Run-5 `600dbb3` + Fix-Run-6 `e305f92`). Verified in the Phase-8 pass alongside the Create-screen cleanup. Plan-backed: `plan/phase-08-popup-reskins-polish.md` → "Create-screen UI cleanup — deferred from Phase-6 verification (user, 2026-05-18)".
- **#2 — Residual grey bar under the GameTab strip (Steps 2/3/4)** (`ab4453b`, `fad78c3`). The GameTab widget's own bottom bar/border **was fixed in Phase 6** (no bottom bar in any state, identical across Steps 2/3/4, single-game skips the strip). What remains is a **grey-bar appearance under the tabs caused by the reused BIO content pane below painting a grey top edge** against the redesign chrome — a reused-BIO colour issue, not the redesign widget. Root cause + fix is the Phase-8 carve-out #6 "BIO grey pane-border recolor" (state-aware `theme_global::*` → `redesign_*(palette)` swap on the content-pane top/border accessor on the Step-2/3/4 reused render path). Plan-backed: `plan/phase-08-popup-reskins-polish.md` → "Deferred backlog … 1. BIO grey pane-border recolor (carve-out #6)" (now sharpened with this concrete instance).

**Phase-8 plan also carries** (→ `plan/phase-08-popup-reskins-polish.md`): the 5 minor Create control-sizing issues + the two side-by-side box equal-height jitter (implement the **standard** equal-height technique, not another ad-hoc measured-max pass); the Fix-Run-6 changes (footer pin, right-margin, selected-box contrast, P1–P5) are NOT reverted. Also Phase-8: FR2-9 prompt-popup vertical growth (#4a, protected-BIO root cause), #7b preview-weidu-3-hue, the save-model UX redesign, the deselect-last/single-component edit-loss, the orphaned `forward_primary_button` dead-code, and promoting the render gate to `try_snapshot` golden baselines.

## Phase 7 — verify at phase end (one pass)

Per `infinity_orchestrator/plan/phase-07-install-runtime.md` "Verification" steps 1–11 + the C3 clean-exit and C5 rail-lock checks. Populated per-run as Phase 7 runs land; **do not test until the orchestrator signals Phase 7 complete.** This is the immediate next verification event (FR2-4 / #2 are a *separate* later Phase-8 pass).

### Run 1 — Step-5 runtime spine + workspace chrome + `← Previous` lock (P7.T1/T2/T8) — committed as the Run-1 commit

**What you verify at the Phase-7 end pass (Run-1 surface):**
- In a workspace, go to **Step 5**: BIO's full embedded panel renders inside the redesign chrome — Command card (+ `Copy Command`), Summary card, `Actions` / `Restart App With Diagnostics` / `Prompt Answers`, console filter labels, `Auto-scroll`, Console box, prompt input row, `Phase: Idle`. **Pre-install the success-banner row and post-install action row are absent/empty** (they fill in Run 3 on a clean install).
- Click **Install**: the workspace **`← Previous` becomes disabled** with the verbatim SPEC §9.2 tooltip *"Disabled while install is running or after a successful install"*. (Run 1 wires only click→lock; the install actually starting is Run 2, banner/registry-flip Run 3 — at the end pass this is exercised as part of the full install flow.)
- Relates to Phase-7 Verification **#2** (Step 5 renders BIO's pre-install cards) + the `← Previous`-disable portion of **#4/#6**. C3/C5 are Run 2/3 surfaces.

**Orchestrator already independently verified (do not re-do):** BIO-source guard CLEAN — zero protected BIO touched (triple-confirmed: guard regex, `git diff` over core/step/main, BIO-binary builds clean); `src/lib.rs` change is the purely-additive carve-out-#3 companion provision (verified via diff); `cargo test --lib` **298/0** (+9 behavior-neutral predicate tests); `infinity_orchestrator` build a proven **true no-op**; **DATA-LOSS sentinel byte-identical** to the pre-arc baseline (re-hashed ×2 — post-agent-run and post-own-test-run); the **3 render-gate PNGs** (1280/1045/960, `tests/ui_snapshot_workspace_step5.rs`) **personally opened and judged** — chrome correct at every width, empty pre-install slots invisible as designed; the only sub-1024 anomalies are inside BIO's read-only reused panel below the app's 1024 px minimum (not redesign defects — the Fix-Run-6 precedent); the two PLAN GAPs (`step5_pending_start` type; `mod install_runtime` location) premise-checked against `WizardApp` / the `pub mod registry;` precedent and resolved as plan-prose corrections (no spec/behavior change), doc-synced into `phase-07-install-runtime.md` P7.T1.

## Orchestrator-side verification already done (so you don't re-do it)

For every item the orchestrator independently verified pre-commit: BIO-source guard empty, `cargo test --lib` green, `%APPDATA%\bio\modlists.json` + every seed `workspace.json` byte-identical across the test run, `cargo build --bin BIO --release` clean, the C4 boundary grep-proven, high-risk files spot-read, and (for any redesign-UI change) the `egui_kittest` full-shell multi-width rendered PNG personally reviewed. **What remains is purely your visual/UX sign-off** — the part only a human at the screen can do.
