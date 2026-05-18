# Orchestrator Handoff — Live Thread

> **Role, the four principles, the run loop, the BIO-source guard, the gotchas, the fixture mechanism, and how to work with this user are NOT here** — they live in the **orchestrator skill** (`.claude/skills/orchestrator/SKILL.md`; invoke `/orchestrator` or read it first). **This file is ONLY the perishable live thread** — where the work is right now. Also distinct: `infinity_orchestrator/HANDOFF.md` = project/impl state.

## ⏸ CURRENT STATE — pick up EXACTLY here (2026-05-17)

**Branch `overhaul/infinity_orchestrator` @ `28d9975`** (Fix-Run-4 code; Fix-Run-3 `0b9d53d` + doc-sync `c9a8bab` beneath it; doc-sync for Fix-Run-4 on top — see `git log`). Phase-7 prep branch `-p7` @ `da88f6d`. Option-B comms = skill doctrine.

### ✅ DATA-LOSS regression FIXED by Fix-Run 3 **+ Fix-Run 4** (orchestrator-verified; awaiting the user's clean retest)

**Fix-Run 3 (`0b9d53d`) was INCOMPLETE — user testing exposed it.** It guarded the production/never-refilled path but keyed on `scanned_empty`; with a one-frame scan the cold-resume completion edge is missed, so the scanned set lands NON-empty while the order is empty → `scanned_empty=false` → Fix-Run-3 defeated → nav-away/dirty-sync persist empty. **Fix-Run 4 (`28d9975`) completes it:** Part 1 arms `was_scanning=true` at scan dispatch (shared helper, both the dev-rescan and cold-resume sites) so a one-frame completion is detected and the restore actually reconciles; Part 2 a `restore_pending` early-return in `flush_workspace_on_nav_away` + `sync_active_workspace_if_dirty` so NO save path can write while a cold-resume restore is pending. Fix-Run-3's `order_for_tab` retained for the production path. Verified independently (every diff spot-read, gate re-run): exactly 4 redesign files, BIO-guard empty, `cargo test --lib` **285/0** (278 +7 incl. the fail-before/pass-after missed-edge regression), both binaries build, **`modlists.json` + every seed `workspace.json` byte-identical across the test run**. The deselect-last/single-component edit-loss is a SEPARATE pre-existing limitation, DEFERRED (the "minimal" boundary) — test with ≥1 component kept selected, never by emptying to zero. The save-model redesign (Revert/dirty-indicator/autosave-toggle/prompts) was critiqued + deferred to a later phase.

### ⚠️ Seed state — fragile; backed up

The `KRS5ZBMT0028` seed is good **only because the user ctrl-z-restored it in a text editor after Fix-Run-3 wiped it** — the app's save-draft did NOT durably persist it (that *is* the bug). The orchestrator backed that rescued state up outside the app's reach: `%APPDATA%\bio\workspace-KRS5ZBMT0028-GOOD-backup-20260517-233939.json` (sha `15A3092A…`, 12,602 B, both tabs). No on-disk artifact reconstructs a populated seed (`dev_seed` writes default-empty + a new id); regenerate via step-0 on the FIXED binary. "Oli's Test List" (`KRW4TGJ60080`) preserved.

### Owed / open
- **Process rule (user-flagged, now hard):** "the rebuild is your job — always rebuild before asking me to test." While a subagent is mid-flight / the tree is dirty the on-disk exe is NOT testable; the orchestrator re-runs the no-op gate **itself** as the final act and names the exact verified exe. Captured in the `feedback-auto-rebuild` memory.
- **SKILL.md DATA-LOSS-sentinel hardening — STILL PENDING USER AUTHORIZATION** (recovery step 3; auto-blocked self-config edit; discipline lives in this file + overview until applied).
- **Next:** orchestrator pushes Fix-Run-4 + doc-sync, runs the final no-op rebuild gate, then hands the user a retest naming the exact verified exe (step-0 reseed on the fixed binary → step-1 resume→nav-away race). Then Phase-6 visual sign-off, then Phase 7 (`-p7`).

### Phase-6 user-feedback round status (mostly closed, gated by the data-loss item)

- **Fix-Run 1** (persistence round-trip, `8dfb905`) — user-verified working earlier; the destructive regression it introduced is **FIXED by Fix-Run-3 `0b9d53d` + Fix-Run-4 `28d9975`** (Fix-Run-3 alone was incomplete — see the RESOLVED block; the round-trip logic was always correct, the unsafe reset-before-refill + save path is now guarded in `extract` *and* gated by `restore_pending`).
- **Fix-Run 2** (UI/UX cleanup + SPEC cascade + 7c fork test-code, `41d70f7`) — done, orchestrator-independently verified (275/0, guard empty, exactly 21 files), committed. Items: indented-input root fix, Step-3 both hints + count removed, nav step-indicator removed (all 4 steps), Create choose-UX redesign, glyphs, Load-Draft real delete, rail-highlight=Create, the minted fork code (test `mint_emits_a_bio_decodable_forked_provenance_code`, verbatim in the Fix-Run-2 report / handed to user).
- **Escalations RESOLVED** (`4ec1c09`, user decision): #7b (preview weidu 3-hue) + #4a (prompt-popup growth) **both → Phase 8** (phase-08 deferred backlog; PENDING_VERIFICATION FR2-9 flipped to Phase-8-deferred; overview 2026-05-17). Zero open Phase-6 escalations.
- **`PENDING_VERIFICATION.md`** holds the Fix-Run-2 visual checklist for the user — but the seed-resume-dependent items are blocked until the data-loss hotfix.

### Next action for the resuming session

The DATA-LOSS regression is **FIXED + verified** (`0b9d53d`; see the RESOLVED block above). Next = await the user's manual-test result (step-0 reseed on the fixed binary → step-1 resume-race test), apply the **SKILL.md sentinel hardening once the user authorizes it** (auto-blocked as self-config — recovery step 3, still owed), then Phase-6 visual sign-off, then Phase 7 (prep on the `-p7` branch). The skill (`/orchestrator`) + the auto-memories are the operative directive set.

> _Older blocks below are pre-this-session history — superseded by the block above; kept for audit, do not action them as "next"._

## Pending user verification

`infinity_orchestrator/PENDING_VERIFICATION.md` — changes committed but **never seen rendered by the user** (all under the exe lock): Step-3 C4 chrome (`fad78c3`, P6.T2d) + the cosmetic set (alignment / uniform GameTab / rename pencil, `ab4453b`). Each item is tagged with its phase + run and exactly what to check. The user is away; do not mark these done until they visually sign off on a gate-fresh binary.

## Active phase work order

`infinity_orchestrator/plan/phase-06-create-workspace-shell.md` — the active phase work order (Phase 6 in progress: R1/R2 + Step-3 C4 P6.T2d shipped; **Run 3 = P6.T7/T9/T13/T14 dispatched 2026-05-17**; Run 4 = P6.T8 fork + P6.T11 dirty-bit + P6.T15 nav-flush). (Update this pointer as phases advance.)

## Where the thread is

Branch `overhaul/infinity_orchestrator`. Last **pushed** = **`179123d`** (origin: `github.com/Xgatt/Born2BSaltys_Infinity_Orchestrator`; `bak` was a mis-tracking accident removed 2026-05-16, `remote.pushDefault=origin` pins it). **HEAD = LOCAL, unpushed** — `577961f` (step-5 rebuild-gate + Windows file-lock gotcha hardening) + the 2026-05-17 Run-2b-false-alarm doc correction on top; **push pending user authorization** (auto-mode classifier blocked it as a presumed main-branch push — it is a routine feature-branch push). Tree clean. Phases 1–4 done. Phase 5 COMPLETE (adversarial-reviewed + QA-resolved; live download → Phase 7 P7.T17 per SPEC §13.12a).

**Phase 6 IN PROGRESS — user-approved 4-run slice:** R1 spine · R2 Step-4 C4 + header/rename/save-draft/game-tabs · R3 Create + Load Draft · R4 fork + dirty-bit persistence + nav-away flush.

- **R1 DONE + user-accepted** — workspace spine + the Step-2 **C4 chrome wrapper**, hardened across Runs 1b→1e (`150593f`…`17b1dcb`): net-new redesign Step-2 chrome per wireframe (NO BIO `render_controls`/`render_tabs`/`render_header`/`page_step2`/`frame_step2` — grep-verified every run); Step-2 channel poll; bounded/clip layout; native GameTab; Step2→Step3 nav-edge sync; rescan-reconcile (SPEC §6.3); first-step `← Previous`→Home (SPEC §2.2); Cancel-while-scanning; Select-via-WeiDU-Log §6.10 danger-confirm + non-destructive cancel; dev-scan (production Rescan inert pre-Phase-7 §13.12a).
- **R2 DONE + orchestrator-verified** — `6df87f9` (P6.T2b Step-4 C4 renderer — `page_step4::render` NOT called, C4 boundary holds; P6.T5 `workspace_header` + `operations_rename` REGISTRY-ONLY per §2.2; P6.T6 save-draft; P6.T10 game-tabs) → `301bf72` data-loss follow-up (a Run-2 test clobbered the real `modlists.json` on every `cargo test`; fixed test-only, **recurrence-guarded in Gotchas — directive-grade**, seed restored) → `4950b19` Run-2b (#1 cold-resume Step-2 restore via `step2_resume_scan` re-triggering BIO's cache-fed scan; #2 pencil glyph fixed; #4 verified wireframe-faithful) → `ee993a7` Run-2b #3 (Step-4 WeiDU line = the wireframe/§6.7 3 hues: amber `#d4a35c` / dark-blue `#2f6fb7` / `redesign_success`). 231 lib tests; guard empty; Step-2/1b-1e/Run-2 not regressed; both binaries link clean.

**Full per-run detail (root causes, premise-checks, judgment calls) = `infinity_orchestrator/plan/overview.md` 2026-05-16 + 2026-05-17 revision log.**

### ⏸ CURRENT STATE — pick up exactly here

**Phase 6 Run 2b re-test PASSED — user-confirmed end-to-end on a correctly-built binary. This IS the "good, next run" acceptance → next dispatch = Phase 6 Run 3.**

**Post-mortem (2026-05-17) — the reported "major regression" was a STALE BINARY, not a code defect.** The prior session handed off the Run-2b re-test against an `infinity_orchestrator.exe` that predated the Run-2b fixes (proven stale at this session's start: `cargo build --release` did a 47s `Compiling bio`, not a no-op). The user's "cold resume broken / rename doesn't persist / save-draft writes nothing / modlists.json minified" was entirely that stale artifact. Rebuilt from HEAD + **gate-confirmed true no-op**, the user re-ran the exact flow — **Select-BGEE-via-WeiDU.log + Select-BG2EE-via-WeiDU.log → save draft → ✎ rename → quit → relaunch → resume → mods + components + Step 3 order ALL restored** — flawlessly, full round-trip (a *broader* validation than the dev-scan path the old re-test script described). Run-2b code was correct in source the whole time. The deep "four silent-failure points / `registry_error` kill-switch / hollow-test" diagnosis chased a ghost the stale binary created — **retracted**; the order-source / failure-point forks are MOOT and closed. (Latent, non-blocking, NOT Run-2b bugs: save-draft/rename can no-op without user feedback = future UX polish; the 32 MiB debug-scan stack overflow is BIO-protected dev-ergonomics, off-scope.)

**Hardening shipped + proven by this exact incident:** `577961f` + the doc-correction commit on top (both **LOCAL — push was blocked by the auto-mode classifier misreading a feature-branch push as a main-branch push; pending user authorization**): step-5 mandatory final pre-handoff rebuild gate (confirmed no-op `Finished`, no `Compiling bio`, AFTER doc commits) + the Windows file-lock gotcha rewrite. Auto-memory `feedback-auto-rebuild` reframed to bind the non-editing orchestrator. **The gate already caught a real staleness this session — it works.**

**Before the next test cycle:** the user's `%APPDATA%\bio\modlists.json` is mutated off the documented 2-entry seed (`demo-modlist-2-test`/EET/3/7). Corrupt copy preserved as `%APPDATA%\bio\modlists.json.corrupt-evidence-20260516-235903` (+ the matching `workspace-…-evidence-…json`). Re-prep to the canonical 2-entry seed (the skill's "Test fixtures / runtime" mechanism) for clean Run-3 testing.

**Next dispatch — Phase 6 Run 3 (Create screen + Load Draft dialog + routing)** per the 4-run slice, following the skill's `## How to run a run` verbatim — esp. the **MANDATORY "Already built / cross-phase context" block** (Settings→Paths globals / no global mods folder / §13.12a / the Step-2+Step-4 C4 precedent / reuse Phase-5 ForkInfoPopup), the **directive-grade test-hygiene rule** (NO unit test may keep a `new_default()` store; verify via the sentinel-`modlists.json` byte-diff), **and the step-5 final pre-handoff rebuild gate**. Read `plan/phase-06-create-workspace-shell.md` P6.T7 (stage_choose) / P6.T8 (fork sub-flow — Run 4, NOT Run 3) / P6.T9 (Load Draft) / P6.T13 (wire Create into page_router) / P6.T14 (resume routing) — Run 3 = the Create + Load-Draft entry path, NOT the fork sub-flow (that's R4). If issues are reported: premise-check each against the actual code/plan/wireframe **and first confirm the binary under test is gate-fresh** before any deep dive.

> **Thread-staleness note (2026-05-17):** the blocks above predate this session's work (the `9b5b9d5` doc-sync, the Issue-1 alignment + uniform GameTab up to commit `ab4453b` — the Step-3 C4 doc-cascade in `ab4453b` was RETRACTED 2026-05-17 (premature/false-done; see overview 2026-05-17 §3) and Step-3 C4 is now a pending dispatched run — and the orchestrator-skill split). HEAD/“next dispatch” here are NOT current. Refresh this whole live-thread section before the next run; the authoritative recent record is `infinity_orchestrator/plan/overview.md` 2026-05-17 revision log. (Preserved verbatim rather than silently rewritten — updating thread state was out of scope for the skill/handoff split.)

| Run | Scope | Status |
|---|---|---|
| 1 | Home — visual + nav (P5.T1-6, T8 widget, T15 + shared widgets) | ✅ done + 4 follow-ups |
| 2 | Home — actions live (P5.T7 delete, T16 toast, T17 open-folder, T18 reinstall stub) | ✅ done + 1 follow-up |
| 3 | Install — shell + paste stage + stage-4 stub (P5.T9, T13, T14) | ✅ done + 3 follow-ups |
| 4 | Install — Preview parse + 6 tabs + carve-out #5 provenance trio + `ForkInfoPopup` (P5.T10, T11) | ✅ done — independently verified (BIO-guard clean, carve-out = exact 5 items, 178 lib tests, both binaries build); +1 follow-up: paste-code box capped to footer + scrolls internally |
| 5 | Install — Downloading §4.3 chassis (P5.T12) | ✅ done — verified (BIO-guard empty, 187 lib tests, both binaries build); live download data + per-install dirs + content-addressed staging deferred to Phase 7 P7.T17 (SPEC §13.12a). +QA follow-ups: paste-stage requires a valid destination folder (SPEC §4.1 amended) + `DestinationNotEmptyWarning` legibility fix |

Run-slicing rationale + per-run breakpoints are in session history; the canonical task list is the phase doc.

### Runs 4–5 — ✅ DONE. Phase 5 COMPLETE; Phase 6 (Create + Workspace shell) is the next dispatch.

**Run 4 outcome (independently verified by the orchestrator, not the agent's word):** BIO-source guard clean (only the authorized `modlist_share.rs`); the carve-out #5 diff is *exactly* the 5 enumerated items + the gate-required `#[cfg(test)]` module, purely additive, `export_modlist_share_code`/private envelope untouched; `cargo test --lib` 178/0 (163 baseline +15 Run-4 incl. serde-default round-trips); both binaries build; `stage_preview.rs`/`fork_info_popup.rs` spot-read SPEC-faithful. Surfaced judgment calls: a `cargo fmt` scope-creep the agent fully reverted (orchestrator-confirmed clean) → standardized scoped-fmt on `rustfmt --edition 2024 <leaf files>`; pre-existing edition-2024 rustfmt debt in HEAD redesign code (out of scope); honest parse-error render + `pub(crate)` API scoping (within spec intent); open question **D** (Overview "Mods" value) resolved later in `32781d2` (derive). The detailed authorized-scope brief below is retained as the audit record of what Run 4 was permitted to touch.

**Run 5 outcome (independently verified):** BIO-source guard empty (zero BIO files; only `src/ui/install/*`), `cargo test --lib` 187/0 (178 +9), both binaries build, `stage_downloading.rs` spot-read = the §4.3 chassis. The agent correctly **escalated a `SPEC CONFLICT`** instead of reimplementing/forking BIO's download engine — but two of its premises were wrong and the orchestrator caught them on the premise-check: it is **not** "BIO can't" (the import→auto-build pipeline exists and is reachable) and **not** "Install lacks game paths" (paths are global in Settings → Paths §11.2 and reach the owned `WizardState` via `sync_paths_from_settings`). Resolution (user, final authority): the spec hadn't defined the per-install/global directory model + content-addressed archives + the pipeline-reuse contract → added as SPEC **§13.12a**; Phase 5 ships the chassis, live wiring is **Phase 7 P7.T17**. See overview.md 2026-05-16 revision log.

**Phase 5 post-completion adversarial-review QA (2026-05-16, commit `32781d2`).** Read-only `Plan` agent scoped to Phase 5 impl + the spec/plan cascade (`df6b78a..HEAD`), governed by `spec-authority.md`. Verdict: directive-clean, zero PR#11-style hollowness, carve-out #5 exactly the authorized surface, doc cascade complete, wireframe authority not inverted, 187 tests substantive. Findings, all resolved in `32781d2`: **SC-1** Install Preview Overview `Mods` `—` vs SPEC §4.2 — user chose *derive*; net-new `install/preview_counts.rs` reusing public `Component::parse_weidu_line` (ZERO BIO edit). **M-1** dead Home `Rename` kebab — *keep + record* (SPEC §3.2 + phase-05 P5.T2 + HANDOFF). **H-1** stale `SPEC CONFLICT` banners reworded. **L-1** ForkInfoPopup collapse-chevron Phase-8 deferral cross-referenced. Verification: 189/0 tests, both binaries build, BIO-source guard empty.

---

#### Run 4 authorized-scope brief (historical / audit record)

Run 4 was **the only BIO-source touch in all of Phase 5**: carve-out #5 on `src/core/app/modlist_share.rs`, now the **provenance trio + `allow_auto_install`** (user-directed spec change 2026-05-15 — see SPEC §1 "Modlist-share provenance application", §13.3 Provenance/Generation, §10.9; overview.md revision log). The **exact and only** authorized BIO edits (P5.T10 enumerates them):
1. `ModlistSharePayload` (`#[derive(Deserialize)]`): `#[serde(default = "default_true")] allow_auto_install: bool` + `#[serde(default)]` `name: Option<String>`, `author: Option<String>`, `forked_from: Vec<ForkAncestor>`.
2. New `fn default_true() -> bool { true }`.
3. New `struct ForkAncestor { name: String, author: String }` with derive **`#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]`** — the full set so Phase 6's `ModlistEntry.forked_from` reuse needs no follow-up BIO edit; within carve-out #5.
4. The symmetric fields on `ModlistSharePreview` (`pub(crate)`).
5. Four `share_preview()` propagation lines (payload→preview, one per field).
Nothing else — all `#[serde(default)]`, zero behavior change for today's BIO. **Generation (`pack_meta`) is NOT Run 4** and is never a BIO edit: net-new orchestrator sibling (`registry::share_export`, Phase 6/7) composing `export_modlist_share_code` + a standard envelope round-trip. Run 4 is **consume-only**.

## Current test seed (live state)

Seed registry for non-empty Home/Install/Workspace testing: `C:\Users\spany\AppData\Roaming\bio\modlists.json`. As re-prepped 2026-05-16: **2 entries** — `RUN1BBBBBBBB` "Polished BG2EE" (`installed`) + `KRS5ZBMT0028` "demo-modlist-2" (`in_progress`, BGEE, 9 mods · 136 components · paused Step 3, + its `modlists/<id>/workspace.json`). The in-progress one is what Home-`resume` breakpoints need; both exercise the empty-`destination_folder` delete guard. (Restored 2026-05-16 after the Run-2 test-clobber data-loss bug — commit `301bf72`.) **Note:** mutated off-seed during the 2026-05-17 session's testing; re-prep before the next clean cycle (mechanism = the skill's "Test fixtures / runtime").
