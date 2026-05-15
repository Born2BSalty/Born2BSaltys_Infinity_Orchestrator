# Orchestrator Handoff

You are the **orchestrator** for the Infinity Orchestrator redesign: you slice phases into runs, dispatch the `plan-implementer` agent per run, **independently verify** its work, and commit/push. You do not implement features yourself (small follow-up fixes excepted). This doc hands you the live thread.

> Two different "handoff" docs exist — don't confuse them. `infinity_orchestrator/HANDOFF.md` = project/impl state (read it). **This file** = how to run the orchestration.

## Read first

1. `.claude/spec-authority.md` — doctrine + Current-project block. The CRITICAL DIRECTIVE (`infinity_orchestrator/SPEC.md` §1, six carve-outs) governs everything: only those six edits to existing BIO source are allowed; everything else is net-new.
2. `.claude/agents/plan-implementer.md` — the implementation agent's contract.
3. Auto-memories (loaded via `MEMORY.md`, but internalize): **no `Co-Authored-By: Claude`** ever (attribute solely `Xgatt <xgatt86@gmail.com>`); **auto-rebuild** after edits; **keep SPEC/plan/HANDOFF in sync** when behavior changes; **affordance-forward empty states** (instructional copy over terse "(not set)"); **scoped `cargo fmt` before every commit** — rustfmt only the staged redesign `.rs` files, NEVER blanket `cargo fmt`/`cargo clippy --fix` (those rewrite protected BIO source = directive violation).
4. `infinity_orchestrator/plan/phase-05-home-install-paste.md` — the active phase work order.

## Where the thread is

Branch `overhaul/infinity_orchestrator`, HEAD **`bb6c74f`**, pushed, clean tree. Phases 1–4 done. **Phase 5 in progress, sliced into 5 runs:**

| Run | Scope | Status |
|---|---|---|
| 1 | Home — visual + nav (P5.T1-6, T8 widget, T15 + shared widgets) | ✅ done + 4 follow-ups |
| 2 | Home — actions live (P5.T7 delete, T16 toast, T17 open-folder, T18 reinstall stub) | ✅ done + 1 follow-up |
| 3 | Install — shell + paste stage + stage-4 stub (P5.T9, T13, T14) | ✅ done + 3 follow-ups |
| **4** | **Install — Preview parse + 6 tabs + `allow_auto_install` carve-out #5 (P5.T10, T11)** | **NEXT — highest scrutiny** |
| 5 | Install — Downloading stage (P5.T12) | pending |

Run-slicing rationale + per-run breakpoints are in this session's history; the canonical task list is the phase doc. After Run 5, Phase 5 is done → Phase 6 (Create + Workspace shell) is the next big phase (its own multi-run slicing; see `plan/phase-06-*.md`, HANDOFF "Finishing the plan" pacing).

### Run 4 — the immediate next action (read carefully)

Run 4 is **the only BIO-source touch in all of Phase 5**: carve-out #5, a **two-surface** schema-additive change on `src/core/app/modlist_share.rs`:
1. `ModlistSharePayload::allow_auto_install: bool` with `#[serde(default = "default_true")]` (deserialize-only struct; drafts → `false`, default `true` so pre-redesign codes stay auto-install-eligible).
2. `ModlistSharePreview::allow_auto_install: bool`, propagated through `share_preview()` (one line) — the surface `stage_preview` reads for the draft-gate.
Both are mechanical, zero-behavior-change for today's BIO. Plan P5.T10 documents it. Everything else in Run 4 (`preview_modlist_share_code` reuse, the 6 tabs, the gate UI, `InstallScreenState` growth) is net-new orchestrator code. `state_install.rs` currently has only `preview_cached: bool` — Run 4 grows it with the real preview state. Brief the agent that `modlist_share.rs` is the **sole** allowed BIO edit and to surface `SPEC CONFLICT` if anything else needs it. Review that diff against carve-out #5 wording with extra care.

## How to run a run (the loop that works)

1. **Dispatch** `plan-implementer` (it's a discovered project agent — `subagent_type: plan-implementer`), `run_in_background: true`. Brief structure that's been working: (a) required reading in order [agent def, spec-authority, phase doc, HANDOFF, the SPEC §/wireframe lines the tasks cite, the Run-N-1 files it builds on]; (b) **exact task IDs in scope + explicit NOT-in-scope list**; (c) prep already settled (so it doesn't redo/re-decide); (d) constraints (net-new only except named carve-out files; `theme_palette` passed explicitly, `redesign_*` tokens only; symbol-glyph rule — see gotchas); (e) verification gate = the run's manual-test breakpoint + 3 build cmds; (f) **"Do NOT commit — implement+verify+report"** with a numbered report spec.
2. **Verify independently — never trust the agent summary.** Always: `git status` + grep the staged set for BIO/protected paths (the guard below) — must be empty; re-run `cargo build --bin infinity_orchestrator --release`, `cargo test --lib` (note the count; behavior-neutral runs must keep it), `cargo build --bin BIO --release`; spot-read the 1–2 highest-risk files (e.g. anything destructive like `remove_dir_all`, any `derive`/`Default` change, any carve-out diff). For lint/format runs, independently re-bucket clippy (redesign vs BIO) — see below.
3. **Scoped rustfmt** the staged redesign files, re-add.
4. **Commit per run** (one commit; Xgatt-only attribution; message describes scope + verification + "no BIO source"), **push**. Then sync SPEC/plan/HANDOFF if behavior the docs describe changed.
5. Surface the agent's judgment calls to the user (propose-don't-incorporate items) and the manual-test script; wait for "good, next run" before dispatching the next.

### BIO-source guard (run on every staged set before commit)
```
git diff --cached --name-only | grep -E 'src/core/|src/ui/step|src/ui/app|src/ui/run|src/ui/frame|src/ui/layout|src/ui/mod\.rs|src/ui/shared/(theme|typography|layout_tokens|tooltip)_global|src/settings/(model|store)\.rs|src/main\.rs|src/lib\.rs|src/ui/shared/mod\.rs|infinity_orchestrator/'
```
Non-empty ⇒ ABORT (CRITICAL DIRECTIVE). `infinity_orchestrator/` is excluded only because doc-sync commits legitimately touch it — judge per commit.

### Redesign-owned path set (for clippy/fmt bucketing & scope)
`src/ui/{orchestrator,home,install,settings,shell}/`, `src/registry/`, `src/ui/shared/{redesign_tokens,redesign_fonts,format_relative}.rs`, `src/settings/{redesign_fields,redesign_store}.rs`, `src/lib.rs`, `src/bin/`. Everything else = protected BIO. Bucket clippy with: `grep -oE '\-\-> [^:]+' out.txt | sed 's/--> //' | tr '\\' '/'` then grep that set. Independent baselines: redesign clippy = **0** (just cleaned), protected BIO ≈ **1172** (never changes — we don't lint/gate BIO).

## Gotchas (cost real time if forgotten)

- **Windows exe file-lock:** if `cargo build` link fails `Access is denied. (os error 5)`, the user has `infinity_orchestrator.exe` open. The *compile* succeeded; just tell them to close it and retry the link. Not a code error.
- **Don't trust `grep "^Finished"` chained** — `cargo test --lib | tail -1` catches a trailing blank line; use `grep "test result:"`. Background-build tail can mislead; read the output file.
- **Symbol-glyph coverage (cmap-verified, see HANDOFF caveat):** Poppins TTFs are a 217-glyph Latin subset. `FiraCodeNerdFont-Light.ttf` (full, 10,801 glyphs) HAS `∞ ✓ ← →` (base-FiraCode) but NOT `⚠`/`⚙`/`☰` (Misc-Symbols block, U+2600–26FF). Render base-FiraCode glyphs in `firacode_nerd`; **paint vectors** for Misc-Symbols/emoji (precedent: `left_rail.rs` nav icons, `destination_not_empty.rs::paint_warning_triangle`). Don't assume coverage — check the cmap (`python -m pip install --user fonttools`; `TTFont(p).getBestCmap()`).
- **Stable rustfmt skips let-chain files** (`if let … && …`, e.g. `orchestrator_app.rs`) atomically — they're left unchanged, not corrupted. Expected.
- **`cargo clippy --fix` / `cargo fmt --all` are forbidden** — they rewrite the whole crate incl. protected BIO. Always scope to changed/redesign files.
- **Clippy policy is settled:** default `cargo clippy -- -D warnings` is the real gate (redesign passes). The user's strict pedantic/nursery profile is NOT a blocking gate (BIO half is un-passable by construction). Redesign code was triaged clean (fix Cat-1 / justified module-scoped `#![allow]` Cat-2 colour-casts & Cat-3 churn). Don't re-open this.
- **`derive(Default)` traps:** `ThemeChoice` defaults to `Dark` (NOT first variant `Light`); `RedesignSettings` is a manual `impl Default` (`validate_paths_on_startup: true`) — must NOT be derived. The serde-default tests assert these; if they pass, value-identity held.

## Test fixtures / runtime

- Seed registry for non-empty Home/Install testing: `C:\Users\spany\AppData\Roaming\bio\modlists.json` (2 entries, both `destination_folder:""` — exercises the empty-folder delete guard). Still in place. Revert = delete it (close app first; exit-flush rewrites it).
- Runtime config dir: `%APPDATA%\bio\`. App must be closed before linking and before swapping `modlists.json` (it loads on launch, flushes on exit).

## User working style (observed)

Tight iterate-test-fix loop; screenshots in `C:\Users\spany\OneDrive\Pictures\Screenshots\`. Reviews UX/copy closely and makes deliberate wireframe deviations (chip padding ≈7px not 4px; "click to set your name" not "(not set)"; toggle-buttons not radios) — these are recorded in SPEC §3.1/§11.1/§4.1 as intentional so reviews don't re-flag them; preserve that pattern. Challenges unproven assertions (rightly) — verify empirically, don't reason-and-assert. Wants per-run manual-test scripts and commit+push each verified unit. Don't auto-fix the agent's flagged judgment calls — surface them.
