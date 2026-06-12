# Infinity Orchestrator — Handoff (remaining work)

Redesign of the `bio` Rust crate into a multi-modlist workspace app (`eframe`/`egui`), alongside the preserved legacy `BIO_legacy` binary. **This file is the work-left-to-do snapshot — nothing else.** Durable reference lives elsewhere: the CRITICAL DIRECTIVE + carve-outs in `SPEC.md §1`; architecture, build/verify commands, and design rationale in `SPEC.md`; the dated decision history in `plan/decision-log.md`; per-task status in `plan/phase-08-popup-reskins-polish.md`; per-run/session state in `.claude/reference/orchestrator-handoff.md`.

**Phases 1–7: shipped and merged. Phase 8 is the only open phase.**

## Phase 8 — remaining (verified 2026-06-01)

1. **`P8.T3`** — residual light-theme token swaps in the WeiDU-line / prompt-popup renderers (`format_step2`, `format_step3`, `prompt_popup_step2`). Cosmetic.
2. **`P8.T12`** — unified `clipboard.rs` copy helper. Copy works today via inline `copy_text` paths; this centralizes it and adds the SPEC-specified inline confirmations.
3. **`P8.T13`** — final smoke pass: every screen, both palettes, no regressions; legacy `BIO_legacy` still builds and runs.
4. **`P8.T14`** — per-modlist data-ownership refactor (SPEC §13.12b), subtasks T14.1–T14.5. **Not started — the biggest remaining chunk.** Two-PR landing strategy is in the phase-08 plan doc.

All other Phase-8 tasks (T1/T2/T4–T11/T15/T16, plus the `T10.f` toast event migration — Group A shipped via PR #34; Groups B/C intentionally not migrated per §10.8) are delivered or no-work-needed — see the phase-08 plan doc for per-task status.

## Open risks / bugs

- **🔴 R7** — `delete_modlist` follows symlinks on Linux (`fs::remove_dir_all`); a symlinked install path destroys the real target. Linux-only; Windows unaffected.
- **R6** — Continue-partial-install: the third radio renders but `stage_paste` jumps Paste → InstallingStub, skipping arming/registration. Dead UI; non-destructive.
- **Gap #4** — silent skip in `build_extract_jobs`.
- **EET source-override doubling — FIXED (carve-out #19).** EET modlists wrote every mod twice into their `mod_downloads_user.toml` + baked share code (both game tabs hold the full set, no dedup in `build_resolved_source_overrides`); an editor re-pin updated only the first copy so the stale later copy shadowed it (last-one-wins). Fixed: export dedup + `save_user_mod_download_source_block` now removes all matching blocks (self-heals). **Pending follow-up (own run): one-time cleanup of already-doubled per-modlist files + stored `latest_share_code`s** (re-export) — the save-dedup only self-heals a mod on its next edit; existing files stay doubled (harmless: redundant, resolver dedups) until then.
- **Per-modlist gap — WeiDU interactive prompt answers (`prompt_answers.json`) are still global (DEFERRED — revisit later).** Same 3-way cross-modlist bleed that carve-out #16 fixed for download-sources / installed-refs, but lower severity: the store is a process-global `OnceLock` map (`prompt_memory_storage.rs`), **not** in the share-code payload (so an interactively-answered prompt doesn't travel with the modlist), **clobbered on every install** (each install writes answers back to the one global file), and **resolved globally** for all modlists (`scripted_inputs.rs::merge_from_prompt_memory` `or_insert` + the `scripted.rs` JSON fallback). Lower severity because per-modlist scripted answers (`@wlb-inputs`, PR #42) already take priority and DO travel in the share code — the residual bleed is only prompts with no scripted token that fall through to the global JSON store. **Intended per-modlist home already exists but is unwired:** `ModlistWorkspaceState.prompt_overrides` (`workspace_model.rs:46`) is dead — only cloned on load (`workspace_state_loader.rs:231`), never written from input nor read into install; wiring it is the natural fix shape. Would be its own scoped run + carve-out. (Surfaced by a data-ownership audit 2026-06-09; user deferred.)
- Full R1–R13 risk map: `INSTALL_WORKFLOWS_TRACE.md`.

## Awaiting a directive decision

- **Prompt-popup vertical growth** — root cause is `set_min_size(available_size())` inside BIO's `prompt_popup_step2` (`Text` branch); protected source, and no carve-out covers it (carve-out #9's size-clamp pattern names the other popups, not this one). Held for you: authorize a 1-line BIO carve-out, or a sanctioned orchestrator-side clamp.

## Deferred backlog (user-deferred; not task-scoped)

- **Full light-mode color pass** — the redesign is tuned dark-first; light reads off app-wide (incl. `redesign_overlay_shadow`). A coherent retune, not a per-token fix.
- **4 custom-frame dialog shadows** — Confirm / Fork-info / Load-Draft / Share-code bypass `window_shadow`.
- **Fork/import preview WeiDU lines → 3-hue** — colors-only; reconcile SPEC §6.7's "Step 3 / Step 4 only" scope during the light pass.
- **Auto-update asset-substitution** — `apply_successful_update_check_outcome` latent behavior left as-is per prior user decision.

## Doc-sync owed

- `SPEC.md §1` now enumerates carve-outs through **#16** — the per-modlist download-source & installed-refs carve-out (formalized 2026-06-08; implemented on `feat/per-modlist-versions`; the detailed spec + plan are archived for posterity outside the tracked repo).
- Still owed: the reskin carve-outs **#11–#14** (Step-2 / Step-3 chrome, shipped via PRs #24 / #27) remain unformalized in `SPEC.md §1`; and `SPEC.md §13.12b`'s "no new carve-out required" line for the download-source/installed-refs overlay is superseded by #16 (the per-modlist slice now lands via carve-out #16 + an ambient resolver, not the old orchestrator-interception design). Reconcile both in a docs pass.
