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

### Run 2 — Install-start + concurrency + C5 rail-lock + flag policies + statusbar (P7.T3/T9/T9b/T16/T14) — committed as the Run-2 commit

**What you verify at the Phase-7 end pass (Run-2 surface):**
- In a workspace Step 5, click **Install**: a `modlist-import-code.txt` appears in the modlist's destination folder (BIO-MODLIST-V1, `allow_auto_install=false`); the registry entry gets an `install_started_at`; a fresh Create→New install kicks BIO's pipeline (console starts streaming).
- **While an install runs:** all four left-rail items (Home/Install/Create/Settings) are disabled/dimmed; hovering shows verbatim *"An install is already running for `<name>`. Wait for it to finish before starting another."* (SPEC §13.15); clicking does nothing — you stay in the running install's workspace. The **statusbar shows `1 job running · <modlist> · <elapsed>`** (ticking). Cancel/finish → rail re-enables + statusbar resets to `0 jobs running` next frame.
- Concurrency: with an install running, a *different* modlist's Step-5 Install is refused (defensive per-button gate behind the rail lock).
- Flag policies (inspect BIO's Command card): fresh Create→New = no `-s`/`-c`, `--download` follows Settings→Advanced; a forked (Import-and-modify) modlist = `--download` ON.
- Relates to Phase-7 Verification **#3** (rail+statusbar during install), **#8** (C5 — all rail items disabled, `populate_wizard_state_from_workspace` not called mid-install), **#5** (`modlist-import-code.txt` present), **#9** (#1/#5 flags). C3 banner/registry-flip is Run 3.

**Orchestrator already independently verified (do not re-do):** zero BIO source modified — **carve-out #5 intact** (`modlist_share.rs` untouched; `share_export::pack_meta` composes `export_modlist_share_code` read-only with its own standard zlib+base64url+serde_json codec, existing deps only); BIO-source guard CLEAN; `registry/model.rs` `install_started_at` is purely additive `#[serde(default)]` (backward-compatible, manual-`Default` updated, no derive trap); **DATA-LOSS sentinel byte-identical** to baseline (re-hashed post-Run-2 — the registry-schema + mutation changes did NOT write the real config dir); `cargo test --lib` **319/0** (+21 substantive tests — pack_meta false/true-bit round-trip, §13.13 write matrix, #1/#5 flag matrix, `format_elapsed`); `infinity_orchestrator` + `BIO` build clean; the **2 render PNGs** (1280/1045, `tests/ui_snapshot_c5_rail_lock.rs`) **personally opened** — all four rail items dimmed/disabled, statusbar `1 job running · Polished BG2EE · 03:07` exactly per SPEC §13.15; `start_hooks` order + H8 (no registry_snapshot) verified; Reinstall-flip (P7.T10) + the download pipeline (P7.T17) confirmed left as exact commented placeholders (no Run-4 scope creep); the SPEC §13.15 tooltip premise-checked against SPEC (plan paraphrase was wrong → plan corrected to SPEC verbatim, spec-authority); 4 PLAN GAPs (nav_rail→left_rail; `install_started_at` net-new; `RailLockReason` shape; tooltip text) + the InstallWorkflow workspace mapping doc-synced into phase-07.

### Run 3 — Post-install: success banner + actions + registry→Installed + share dialog (P7.T4/T5/T6/T7/T12/T13) — committed as the Run-3 commit

**What you verify at the Phase-7 end pass (Run-3 surface):**
- Complete a **clean install** (exit 0, no failure): a full-width green-bordered **success banner** appears above BIO's panel — green `Installed` pill + `<N> mods · <C> components · no errors` + right-aligned faint `ran <MM:SS> · finished <relative>`; a **`Return to Home` + `Open install folder`** row immediately below it (above BIO's now-disabled Install button per H9); the workspace header's **`Share import code`** flips to enabled **primary teal** → clicking opens the non-blocking dialog showing the code (Copy → `✓ copied`).
- The modlist **flips to `Installed`** in the registry (Home shows it under the *Installed* chip, with `install_date` + counts); the Home card's **size shows `—` briefly then fills in** (async worker) on a later frame.
- A **non-clean** exit (cancel / nonzero / failure) → **no** banner, **no** post-install row, entry **stays In-progress**.
- Graceful cancel → embedded Install button reads `Resume Install`; Force cancel → `Restart Install` (unchanged BIO behavior, survives the chrome).
- Relates to Phase-7 Verification **#4** (clean-exit banner/actions/share/registry-flip/Home-chip), **#11** (C3 — non-clean stays in-progress), **#6** (resume/restart label).

**Orchestrator already independently verified (do not re-do):** zero BIO source (`state_step5.rs`/`modlist_share.rs` read-only); BIO-guard CLEAN; changed set exactly the 11 Run-3 files (agent's self-rustfmt leaf-clean, no recursion casualty); `cargo test --lib` **330/0** (+11 substantive — `flip_to_installed` happy/missing-entry/counts/dir-size with **temp-path `RegistryStore` — DATA-LOSS-safe**, `format_install_duration`, the C3 `clean_exit` matrix); `infinity_orchestrator` proven **true no-op** + `BIO` no-op; **DATA-LOSS sentinel byte-identical** to baseline **even though `flip_to_installed` is a real `RegistryStore::save`** (the directive-grade check — temp-path test stores confirmed it); the **2 post-install PNGs** (1280/1045, `tests/ui_snapshot_ws_step5_success.rs`) **personally opened** — green Installed banner + counts + ran/finished, Return-to-Home/Open-folder above the panel per H9, Share-import-code primary-teal, all per the wireframe `FinalPlanPanel installComplete`; `flip_to_installed` is directive-compliant (`pack_meta` `allow_auto_install=true` ONLY here; on-disk import-code NOT rewritten; H8); the **C3 flip fires exactly once** (reuses the existing single-frame `install_was_running` edge, C3-gated — no per-frame registry rewrite — independently traced in `orchestrator_app.rs`); the async size worker handles every plan failure mode; `count_mods_and_components` premise-checked against `step4_save_row::active_tab_counts` (mirrors it, NOT invented); no PLAN GAP/SPEC CONFLICT (judgment calls within stub-fill scope); the `flip_to_installed` signature refinement (no caller `stats`; returns the size receiver) doc-synced into phase-07 P7.T6.

### Run 4a — Live download/extract/import pipeline + per-install dirs + content-addressed staging (P7.T17) — committed as the Run-4a commit

**What you verify at the Phase-7 end pass (Run-4a surface):**
- **Install Modlist** (paste a share code) → preview → **Downloading**: the §4.3 screen now shows **live per-mod progress** sourced from BIO's own download/extract workers (queued → downloading N% → extracting → ✓ staged; overall `M/N mods · P%` bar); on completion it advances to the install seam automatically (the real install screen is Run 4b — for now it enters the §4.4 stub which BIO's live runtime backs).
- Two modlists needing **different versions of the same-named archive coexist**; a second modlist reusing an **identical** archive does **not** re-download (content-addressed dedupe). The per-install **Mods** + **`weidu_component_logs`** + forced **game-clone** dirs are created **inside the destination**; the per-install Mods folder is removed on a clean success, left on failure/cancel.
- A fresh **Create → New** skips the import/download but still gets the per-install dirs (clone forced).
- Relates to Phase-7 Verification **#1** (live downloading), **#9** (per-install dirs/flags) + SPEC §13.12a. (The real install screen + Reinstall + the import-code matrix are Run 4b.)
- **Known minor (do NOT fail the pass — Phase-8 polish):** at a narrow window (~1045 px) the Downloading grid's source/status columns crowd together; the live data + the pipeline are correct, it's a Phase-5-chassis column-spacing nit.

**Orchestrator already independently verified (do not re-do):** **THE CONTRACT PROOF — zero BIO source modified; `cargo build --bin BIO` a true no-op** (the download/extract engine is genuinely *composed, not forked* — `archive_store` interposes only at BIO's deterministic archive path + the orchestrator-owned asset list + 2 net-new JSON sidecars; `auto_build_driver` drives BIO's *existing* `import_modlist_share_code` + `arm_auto_build`-minus-install-flip pipeline via the already-running per-frame poll — both spot-read in full); BIO-source guard CLEAN; changed set exactly the 9 Run-4a files; `cargo test --lib` **350/0** (+20 substantive, all temp-path — FNV determinism/known-answer, dedupe/coexist/no-redownload, per-install dir matrix, no-clone-never-set, the pipeline predicates); `infinity_orchestrator` proven **no-op**; **DATA-LOSS sentinel byte-identical to baseline DESPITE the arc's only `remove_dir_all`** (verified structurally safe — always `<dest>/mods`, `.exists()`-guarded, post-clean-success only, a test proves the clone folder is NOT removed) **+ a pipeline that writes `weidu.log`/`mod_downloads_user.toml`**; the **3 live-Downloading PNGs** (`tests/ui_snapshot_downloading.rs`, 1280/1045/960) **personally opened** — §4.3 chassis renders the live tones correctly; the agent's non-escalation premise-confirmed sound (reuse genuinely not blocked); the `on_install_start`-derives-dirs-only judgment premise-confirmed (avoids a real empty-Mods-folder bug); judgment calls (FNV-not-DefaultHasher; sidecars-not-schema-edit; popup-suppression; `page_install.rs` PLAN-GAP) all sound + doc-synced into phase-07 P7.T17.

### Run 4b — Install-progress screen + Reinstall flow + import-code matrix (P7.T15/T10/T11) — committed as the Run-4b commit

**What you verify at the Phase-7 end pass (Run-4b surface):**
- Install Modlist paste → preview → downloading → **the real installing screen** (SPEC §4.4): "Installing modlist" title + "<name> · live install console" + `← back to import`, BIO's embedded Step-5 panel, and (on clean exit) a `Return to Home` + `Open install folder` row — **no** workspace 4-step progress bar / Save-Draft / Share header (Install-Modlist has its own minimal chrome).
- From an **installed** card → Kebab → **Reinstall** → confirm → the Install-Modlist **preview opens with the overwrite banner**, and the registry **still shows `Installed`**. Click **Install** → registry flips `Installed → InProgress`, install runs (rail locks per C5); on clean exit → back to `Installed`. **Cancel at the preview** (Back / rail / close) → modlist **stays `Installed`** (the flip never fired).
- Import-code matrix: `modlist-import-code.txt` write/overwrite/skip per variant (Resume skips; Install/Restart/Reinstall write/overwrite). **⚠ See the gap note below — for the Install-Modlist-paste / Reinstall entry points this is delivered by the final Fix-Run, not Run 4b.**
- Relates to Phase-7 Verification **#7** (Reinstall: registry stays Installed at preview, flips at Install-click, returns to Installed on clean exit; cancel-stays-Installed) + the §4.4 installing screen.

**Orchestrator already independently verified (do not re-do):** zero BIO source (`page_step5`/`state_step5`/`modlist_share` read-only); BIO-guard CLEAN; changed set exactly the 14 Run-4b files; `cargo test --lib` **363/0** (+13 substantive — `flip_to_in_progress` happy/missing/non-Installed with **temp-path stores**, the §13.13 variant matrix, `from_step5_and_reinstall`, `reinstall_flip_at_install_click` ×3, stage_installing helpers); `infinity_orchestrator` proven **no-op** + `BIO` no-op; **DATA-LOSS sentinel byte-identical despite the real `flip_to_in_progress` `RegistryStore::save`**; the **1280 §4.4 render** personally opened (wireframe-faithful; the gate caught + the agent fixed a `←` Poppins-tofu — net-new glyph-aware back button per the HANDOFF convention, shared `redesign_btn` untouched); `flip_to_in_progress` spot-read (symmetric to `flip_to_installed`, atomic, in-memory revert on write-fail); the **PLAN GAP premise-confirmed against source** (the reinstall flip relocated from `on_install_start` — which the Reinstall route provably bypasses — to the `page_install.rs` Install-click; correctly classified PLAN-GAP not SPEC-CONFLICT; doc-synced into phase-07 P7.T10) + nav-away cleanup verified (cancel-stays-Installed); judgment calls (wireframe ScreenTitle header; `post_install_actions` reuse; no `success_banner` on §4.4; stub deleted per the Run-1 precedent) all sound.

> **✅ PHASE 7 COMPLETE — the §13.13 / registry-lifecycle gap is CLOSED (your "Full correct fix" decision, 2026-05-18, implemented + orchestrator-verified + committed; end-of-arc no-op rebuild gate green).** 5 runs + the behavior-neutral helper extraction + the final `install_modlist_registration` fix, all independently verified, committed locally (`5b8a60e`→ the final-fix commit); **nothing pushed** (awaiting your authorization). The per-run checklists above are your one-pass guide.
>
> ### Final-fix verification points (Install-Modlist-paste & Reinstall — §13.13 + registry lifecycle)
> **What you verify:** paste an Install-Modlist share code → preview → Install → the modlist now **appears on Home as In-progress** (it persists — previously it didn't), `modlist-import-code.txt` is in the destination (written *after* the import, before WeiDU) + `latest_share_code`/`install_started_at` recorded → on clean exit it **flips to Installed on Home**. Reinstall: from an installed card → Kebab → Reinstall → confirm → preview (registry still Installed) → Install → `modlist-import-code.txt` written (overwrite) + flips InProgress → on clean exit back to Installed. Resume-variant skips the import-code overwrite (checksum-stable). The Create→New / Create-import / Load-Draft (Workspace) paths are unchanged.
> **Orchestrator already verified (don't re-do):** `start_hooks.rs` provably untouched (`on_install_start` byte-identical — Workspace path unchanged); zero BIO; BIO-guard CLEAN; `cargo test --lib` **373/0** (+7 in-memory `install_modlist_registration` tests; the 366 baseline incl. all `start_hooks`/§13.13/flip tests unchanged); both binaries no-op; **DATA-LOSS sentinel byte-identical despite real `entries.push`+`registry_store.save`+`workspace.json` writes** (tests in-memory-only); `install_modlist_registration.rs` spot-read in full (reuses the `create_modlist` convention verbatim; Reinstall not double-registered; `active_install_modlist_id` clean-exit flip preserves Workspace precedence; `forked_from` verbatim — provenance-safe; never pre-flips `start_install_requested`).

## Orchestrator-side verification already done (so you don't re-do it)

For every item the orchestrator independently verified pre-commit: BIO-source guard empty, `cargo test --lib` green, `%APPDATA%\bio\modlists.json` + every seed `workspace.json` byte-identical across the test run, `cargo build --bin BIO --release` clean, the C4 boundary grep-proven, high-risk files spot-read, and (for any redesign-UI change) the `egui_kittest` full-shell multi-width rendered PNG personally reviewed. **What remains is purely your visual/UX sign-off** — the part only a human at the screen can do.
