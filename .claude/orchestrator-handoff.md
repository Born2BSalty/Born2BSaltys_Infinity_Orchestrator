# Orchestrator Handoff

You are the **orchestrator** for the Infinity Orchestrator redesign: you slice phases into runs, dispatch the `plan-implementer` agent per run, **independently verify** its work, and commit/push. You do not implement features yourself (small follow-up fixes excepted). This doc hands you the live thread.

> Two different "handoff" docs exist — don't confuse them. `infinity_orchestrator/HANDOFF.md` = project/impl state (read it). **This file** = how to run the orchestration.

## Read first

1. `.claude/spec-authority.md` — doctrine + Current-project block. The CRITICAL DIRECTIVE (`infinity_orchestrator/SPEC.md` §1, six carve-outs) governs everything: only those six edits to existing BIO source are allowed; everything else is net-new.
2. `.claude/agents/plan-implementer.md` — the implementation agent's contract.
3. Auto-memories (loaded via `MEMORY.md`, but internalize): **no `Co-Authored-By: Claude`** ever (attribute solely `Xgatt <xgatt86@gmail.com>`); **auto-rebuild** after edits; **keep SPEC/plan/HANDOFF in sync** when behavior changes; **affordance-forward empty states** (instructional copy over terse "(not set)"); **scoped `cargo fmt` before every commit** — rustfmt only the staged redesign `.rs` files, NEVER blanket `cargo fmt`/`cargo clippy --fix` (those rewrite protected BIO source = directive violation).
4. `infinity_orchestrator/plan/phase-05-home-install-paste.md` — the active phase work order.

## Where the thread is

Branch `overhaul/infinity_orchestrator`, HEAD = the Phase 5 **Run 4** implementation commit (atop the provenance spec-amendment `acff12f`), pushed, clean tree. Phases 1–4 done. **Phase 5: Runs 1–4 done; Run 5 (Downloading, P5.T12) is the next action.**

| Run | Scope | Status |
|---|---|---|
| 1 | Home — visual + nav (P5.T1-6, T8 widget, T15 + shared widgets) | ✅ done + 4 follow-ups |
| 2 | Home — actions live (P5.T7 delete, T16 toast, T17 open-folder, T18 reinstall stub) | ✅ done + 1 follow-up |
| 3 | Install — shell + paste stage + stage-4 stub (P5.T9, T13, T14) | ✅ done + 3 follow-ups |
| 4 | Install — Preview parse + 6 tabs + carve-out #5 provenance trio + `ForkInfoPopup` (P5.T10, T11) | ✅ done — independently verified (BIO-guard clean, carve-out = exact 5 items, 178 lib tests, both binaries build) |
| **5** | **Install — Downloading stage (P5.T12)** | **NEXT** |

Run-slicing rationale + per-run breakpoints are in this session's history; the canonical task list is the phase doc. After Run 5, Phase 5 is done → Phase 6 (Create + Workspace shell) is the next big phase (its own multi-run slicing; see `plan/phase-06-*.md`, HANDOFF "Finishing the plan" pacing).

### Run 4 — ✅ DONE (verified, committed, pushed). Run 5 (P5.T12 Downloading) is next.

**Run 4 outcome (independently verified by the orchestrator, not the agent's word):** BIO-source guard clean (only the authorized `modlist_share.rs`); the carve-out #5 diff is *exactly* the 5 enumerated items + the gate-required `#[cfg(test)]` module, purely additive, `export_modlist_share_code`/private envelope untouched; `cargo test --lib` 178/0 (163 baseline +15 Run-4 incl. serde-default round-trips); both binaries build; `stage_preview.rs`/`fork_info_popup.rs` spot-read SPEC-faithful (fallback copy, gate logic, `⑂` painted vector / `↳` glyph, L1/L2 fixes honored). Surfaced judgment calls: a `cargo fmt` scope-creep the agent fully reverted (orchestrator-confirmed clean) → propose standardizing scoped-fmt on `rustfmt --edition 2024 <leaf files>`; pre-existing edition-2024 rustfmt debt in HEAD redesign code (out of scope); honest parse-error render + `pub(crate)` API scoping (within spec intent); open question **D** (Overview "Mods" value) still rendered `—` and flagged for a user decision. The detailed authorized-scope brief below is retained as the audit record of what Run 4 was permitted to touch.

**Next action = Run 5 (P5.T12, Install Downloading stage)** — per-mod download/extract grid wired to BIO's existing `app_step2_update_*` channels (orchestrator-owned receivers, no BIO modification). No BIO-source touch expected (net-new only). Slice/brief it the same way; phase doc P5.T12 is the canonical task.

---

#### Run 4 authorized-scope brief (historical / audit record)

Run 4 was **the only BIO-source touch in all of Phase 5**: carve-out #5 on `src/core/app/modlist_share.rs`, now the **provenance trio + `allow_auto_install`** (user-directed spec change 2026-05-15 — see SPEC §1 "Modlist-share provenance application", §13.3 Provenance/Generation, §10.9; overview.md revision log). The **exact and only** authorized BIO edits (P5.T10 enumerates them):
1. `ModlistSharePayload` (`#[derive(Deserialize)]`): `#[serde(default = "default_true")] allow_auto_install: bool` + `#[serde(default)]` `name: Option<String>`, `author: Option<String>`, `forked_from: Vec<ForkAncestor>`.
2. New `fn default_true() -> bool { true }`.
3. New `struct ForkAncestor { name: String, author: String }` (the `forked_from` element type) with derive **`#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]`** — the full set (not just `Deserialize`) so Phase 6's `ModlistEntry.forked_from` reuse needs no follow-up BIO edit; within carve-out #5 (precedent: `ModlistShareConfigFile` already derives `Serialize` in the same file).
4. The symmetric fields on `ModlistSharePreview` (`pub(crate)`).
5. Four `share_preview()` propagation lines (payload→preview, one per field).
Nothing else — all `#[serde(default)]`, zero behavior change for today's BIO. **Generation (`pack_meta`) is NOT Run 4** and is never a BIO edit: it's a net-new orchestrator sibling (`registry::share_export`, Phase 6/7) that composes `export_modlist_share_code` + a standard envelope round-trip. Run 4 is **consume-only** — it reads the four fields off `parsed_preview`, drives the title/subline from packed `name`/`author` (fallback `Shared modlist` / author-less), and adds `⑂ fork info` → the new `src/ui/orchestrator/widgets/dialogs/fork_info_popup.rs` (`ForkInfoPopup`, reused by Phase 6). `state_install.rs` currently has only `preview_cached: bool` — Run 4 grows it with the real preview state + `fork_info_open: bool`. Brief the agent: `modlist_share.rs` is the **sole** allowed BIO edit, the diff must be **exactly those five mechanical items**, and surface `SPEC CONFLICT` if anything else in BIO source seems needed. Review that diff against the SPEC §1 provenance paragraph with maximum scrutiny — it is the highest-risk diff in the phase.

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
