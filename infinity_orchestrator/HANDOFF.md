# Infinity Orchestrator — Handoff (remaining work)

Redesign of the `bio` Rust crate into a multi-modlist workspace app (`eframe`/`egui`), alongside the preserved legacy `BIO_legacy` binary. **This file is the work-left-to-do snapshot — nothing else.** Durable reference lives elsewhere: the CRITICAL DIRECTIVE + carve-outs in `SPEC.md §1`; architecture, build/verify commands, and design rationale in `SPEC.md`; the dated decision history in `plan/decision-log.md`; per-task status in `plan/phase-08-popup-reskins-polish.md`; per-run/session state in `.claude/reference/orchestrator-handoff.md`.

**Phases 1–7: shipped and merged. Phase 8 is the only open phase.**

## Phase 8 — remaining (verified 2026-06-01)

1. **`P8.T3`** — residual light-theme token swaps in the WeiDU-line / prompt-popup renderers (`format_step2`, `format_step3`, `prompt_popup_step2`). Cosmetic.
2. **`P8.T12`** — unified `clipboard.rs` copy helper. Copy works today via inline `copy_text` paths; this centralizes it and adds the SPEC-specified inline confirmations.
3. **`P8.T13`** — final smoke pass: every screen, both palettes, no regressions; legacy `BIO_legacy` still builds and runs.
4. **`P8.T14`** — per-modlist data-ownership refactor (SPEC §13.12b), subtasks T14.1–T14.5. **Not started — the biggest remaining chunk.** Two-PR landing strategy is in the phase-08 plan doc.

All other Phase-8 tasks (T1/T2/T4–T11/T15/T16, plus the `T10.f` toast event migration — Group A shipped via PR #34; Groups B/C intentionally not migrated per §10.8) are delivered or no-work-needed — see the phase-08 plan doc for per-task status.

## Queued tasks (not started)

- **Re-import of a copied code stalls at 0/0 downloads.** Deterministic repro: legacy code → Create → "import and modify" (fork import) → install → Home Kebab → Copy import code → Create → "import and modify" again with that copied code → the download stage sticks at **0/0**. The copied post-install code carries `allow_auto_install=true` + baked `archive_meta {size,hash}`, and the prior install already content-addressed the archives — so candidate causes are the checksum-then-skip pass classifying every asset as already-present (0 to fetch) while the pipeline never advances, or the fork re-import arm path failing to arm against an already-staged set. **Not root-caused** — needs a flow trace (`/root-cause-from-flow`) from the re-import entry through `arm_pipeline_once` / `archive_skip` / `fork_pipeline_arm` before any fix.

## Open risks / bugs

- **🔴 R7** — `delete_modlist` follows symlinks on Linux (`fs::remove_dir_all`); a symlinked install path destroys the real target. Linux-only; Windows unaffected.
- **R6** — Continue-partial-install: the third radio renders but `stage_paste` jumps Paste → InstallingStub, skipping arming/registration. Dead UI; non-destructive.
- **Gap #4** — silent skip in `build_extract_jobs`.
- Full R1–R13 risk map: `INSTALL_WORKFLOWS_TRACE.md`.

## Awaiting a directive decision

- **Prompt-popup vertical growth** — root cause is `set_min_size(available_size())` inside BIO's `prompt_popup_step2` (`Text` branch); protected source, and no carve-out covers it (carve-out #9's size-clamp pattern names the other popups, not this one). Held for you: authorize a 1-line BIO carve-out, or a sanctioned orchestrator-side clamp.

## Deferred backlog (user-deferred; not task-scoped)

- **Full light-mode color pass** — the redesign is tuned dark-first; light reads off app-wide (incl. `redesign_overlay_shadow`). A coherent retune, not a per-token fix.
- **4 custom-frame dialog shadows** — Confirm / Fork-info / Load-Draft / Share-code bypass `window_shadow`.
- **Fork/import preview WeiDU lines → 3-hue** — colors-only; reconcile SPEC §6.7's "Step 3 / Step 4 only" scope during the light pass.
- **Auto-update asset-substitution** — `apply_successful_update_check_outcome` latent behavior left as-is per prior user decision.

## Doc-sync owed

- Carve-outs **#11–#14** (Step-2 / Step-3 chrome reskins, shipped via PRs #24 / #27) are not yet formalized in `SPEC.md §1`, which enumerates through #9. Formalize in a docs PR.
