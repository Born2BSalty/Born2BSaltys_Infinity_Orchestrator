# PR #11 "Overhaul" — Evaluation and Recommendation

**Reviewed:** 2026-05-14
**PR:** https://github.com/Born2BSalty/Born2BSaltys_Infinity_Orchestrator/pull/11
**PR head:** `overhaul` @ `562161bc`
**Base:** `main` @ `18d2f432`
**Reviewer:** code-only review of the GitHub diff; no local checkout (local Phase 1–4 work is uncommitted on `main` and must not be overwritten).
**Local state at review time:** Phases 1–4 done cleanly per `HANDOFF.md`; Phases 5–8 not started.

## Recommendation

**Continue building locally. Do not pull the PR.**

Salvage two narrow references:

1. The install-runtime start hooks (`src/install_runtime/start_hooks.rs` + the three `flag_policies_*.rs`)
2. The post-install state-flip logic in the PR's `orchestrator_app.rs:5754` (`record_install_success_if_needed`)

Everything else in the PR either violates the CRITICAL DIRECTIVE, is functionally hollow, or is ahead-of-plan polish that compounds the underlying problems. The PR superficially looks farther along (1046-line `orchestrator_app.rs`, ~9.9k additions, all four remaining phases touched) but the workspace tier — the heart of the redesign — is non-functional. Phase 5 and Phase 7 surfaces are mostly real; everything between them is stub-grade.

## Quantitative state

|  | Local | PR #11 |
|---|---|---|
| Phases done cleanly | 1–4 | 1–4, partial 5, hollow 6, partial 7, partial 8 |
| Tests | 116/116 pass | unknown — codex stopped at Phase 8 T8 with exe crashing |
| Core BIO files modified | 0 outside carve-outs | 12 files, mostly outside carve-outs |
| Files in redesign tree | ~50 | ~140 |
| `orchestrator_app.rs` size | ~500 lines | 1046 lines (monolith) |
| BIO binary still builds clean | yes | unknown |

## CRITICAL DIRECTIVE compliance — PR fails

`SPEC.md` §1 allows six carve-outs for edits to existing BIO source. The PR modifies these BIO files; flagging each:

| File | Change | Verdict |
|---|---|---|
| `src/core/app/compat/compat_deprecated_scan.rs` | +52 lines new logic (`deprecated_empty_subcomponent_placeholder_hits` function + lookup-chain mod) | **Violation** — new business logic, not a carve-out |
| `src/core/app/state/state_step2.rs` | +2 fields (`subcomponent_key`, `tp2_empty_placeholder_block`) | **Violation** — fields drive new logic in BIO source (the compat scan above), not "data the orchestrator reads" per carve-out #5 |
| `src/core/app/step2/scan/worker_build_states.rs` | +31 lines parsing TP2 component blocks for the new fields | **Violation** — feature work in BIO source |
| `src/core/app/step2/update/app_step2_update_check.rs` | +18 lines adding a Sentrizeal download source | **Violation** — feature work unrelated to redesign |
| `src/core/app/step2/update/app_step2_update_download.rs` | +35 lines Sentrizeal-specific Referer handling | **Violation** — same |
| `src/core/app/step2/update/app_step2_update_github.rs` | Pattern match `Some("releases")` → `Some("release" \| "releases")` | **Violation** — behavior change |
| `src/core/app/mod_downloads.rs` | Inverted control flow in `load_user_mod_download_source_block`, new `source_default_explicit` field, channel normalization (`releases` → `release`, filter to known channels) | **Violation** — multiple behavior changes |
| `src/core/config/default_mod_downloads.toml` | Switched mod-source forks (BG1UB → Pocket-Plane-Group, Ascension → Gibberlings3, etc.), added branches/packages | Data update, **unrelated to redesign** — belongs in a separate BIO PR |
| `src/core/app/modlist_share.rs` | Added `allow_auto_install` field + new `export_*_with_auto_install` wrapper | **Borderline** — carve-out #5 covers the field; the wrapper function isn't strictly required (the orchestrator could overlay the field instead). Acceptable in practice. |
| `src/core/app/terminal/output.rs` | `pub` → `pub(crate)` on two methods | **Borderline** — tightens visibility (fixing pre-existing warnings) but no listed carve-out covers visibility tightening |
| `src/core/mods/log_file.rs` | Added `is_empty()` method | **Violation** — new method on existing BIO struct, no carve-out covers this |
| `src/core/app/compat/compat_rule_runtime_tests.rs` | Test fixture updates for the `Step2ComponentState` field additions | Consequential to the state_step2 change |

The PR mixes BIO feature work (fork-aware mod-source manifest, deprecated-placeholder propagation, Sentrizeal support, GitHub channel aliasing) into the redesign. The directive's whole point is that the redesign stays isolated. This is the single largest reason not to pull.

## Phase 5 — Home + Install Modlist

Mostly real and functional. Wiring exists for:
- Home filter chips, card list, kebab actions (`Resume`, `Open install folder`, `Rename`, `Reinstall`, `Delete`, `Copy import code`)
- Delete confirm + Reinstall confirm dialogs
- Install Modlist paste / preview / download stages with the 6-tab preview shape
- First-launch empty-registry CTA card
- `game installs detected` block

Concerns:
- The `ShareCode` dialog body still contains placeholder copy: `"Import code is not available yet. Share-code generation lands in Batch 8.2."` (PR `share_paste_code_dialog.rs:12496`) — shipped, but as placeholder text.
- No verification that codes generated by drafts have `allow_auto_install = false` and that preview disables the Install button accordingly (SPEC §4.2 / §13.3). The schema field exists; the gating UI is not visible in the install screen diff.

## Phase 6 — workspace is hollow

This is where the PR breaks. The workspace shell renders but does not drive behavior.

**Step 2 actions are discarded entirely.** PR `src/ui/workspace/workspace_step_router.rs:13003`:

```rust
WorkspaceStep::Step2 => {
    let _ignored_action = crate::ui::step2::page_step2::render(...);
    None
}
```

BIO's `page_step2::render` returns `Option<Step2Action>`; the orchestrator throws it away. Folder picks, scan triggers, prompt-popup opens, update checks — all dead. Step 2 renders but is a static image.

**Step 4 is a placeholder string.** PR `src/ui/workspace/workspace_step_router.rs:13041`:

```rust
fn render_step4_placeholder(...) {
    redesign_box(..., "Step 4 review renderer lands after Step 2/3 embedding is proven.");
}
```

Phase 6 C4 requires an orchestrator-side Step 4 renderer (to replace BIO's `page_step4::render` and avoid the double Save button). Not done.

**Saved modlists cannot be reloaded.** PR `src/ui/workspace/workspace_state_loader.rs:12860`:

```rust
pub fn populate_wizard_state_from_workspace(...) {
    wizard_state.step1.game_install = game_to_step1_value(&registry_entry.game).to_string();
    // Batch 5 is load-only and only maps proven fields. The persisted
    // workspace component refs are not enough to reconstruct Step3ItemState.
    // Step 2/3 reconstruction remains deferred until that mapping is proven.
}
```

Only `step1.game_install` is restored. Open a saved modlist → Step 2/3 are empty → user must re-scan and re-select from scratch every session. The workspace persistence cycle exists structurally but is decorative.

**Every Create button is a no-op.** PR `orchestrator_app.rs:5399`:

```rust
fn handle_create_action(&mut self, action: CreateAction) {
    match action {
        CreateAction::StartNewModlist
        | CreateAction::PasteShareCode
        | CreateAction::LoadDraftRequested => {}
    }
}
```

The Create screen UI exists but clicking "create your own" / "load draft" / "paste import code" does nothing.

## Phase 7 — install runtime is mostly real

Genuine work:
- `src/install_runtime/start_hooks.rs` applies the log policy + EET/single-game flag policy before WeiDU runs. Clean and self-contained.
- `write_import_code_before_install_start` writes `modlist-import-code.txt` with `allow_auto_install = false`, and skips on resume (per Phase 7 H10). Clean.
- `record_install_success_if_needed` flips Installed → in-progress → Installed on clean exit (SPEC §9.2 C3 triple) and regenerates the share code with `allow_auto_install = true`. Clean.
- Rail-nav lock wired in `orchestrator_app.rs:5195`: `rail_locked_tooltip = install_runtime_busy.then_some(INSTALL_NAV_LOCK_TOOLTIP)`. The tooltip is enforced in `nav_rail.rs`.

Gaps:
- **Reinstall does not force `prepare_target_dirs_before_install = true` or skip `DestinationNotEmptyWarning`** per SPEC §3.1. `confirm_reinstall_from_home` routes to the preview screen and `begin_install_preview_accepted` flips state to in-progress, but neither sets overwrite-install mode on the install state. Silent spec violation.
- **No async size computation on a worker thread** per Phase 7 M5. Uses `entry.mod_count` / `entry.component_count` from the registry directly. Often fine in practice; spec violation in spirit.

## Phase 8 — polish work, likely crash culprit

Phase 8 broadly applied carve-out #6 (state-aware theme-token reads) across 24+ BIO UI files. Most are clean color swaps. Three categories of overreach:

1. **`src/ui/step2/format_step2.rs` swaps the font resolution mechanism**, not just colors. Was `egui::TextStyle::Monospace.resolve(ui.style())`, now `egui::FontId::new(REDESIGN_LABEL_FONT_SIZE_PX, redesign_font_mono())`. Carve-out #6 explicitly says "only the color expressions inside each branch may change." Changing font/size is a behavior change.

2. **`src/ui/step5/prompts/prompt_answers_*.rs` modified.** Not in Phase 8's planned scope per `HANDOFF.md`.

3. **`src/ui/step2/page_step2.rs:10079` introduces `apply_redesign_bio_visuals(ui, palette)`** — a new helper that overrides egui visuals globally for the scope. Clever shortcut, but not a listed carve-out.

The user reported the exe started crashing at Phase 8 T8. Top suspects, in order of likelihood:

1. **A `FontFamily::Name("...")` lookup that isn't registered in `install_redesign_fonts`.** The `HANDOFF.md` caveat warns of this exact runtime panic: `FontFamily::Name("X") is not bound to any fonts`. The new `redesign_font_mono()` / `redesign_font_bold()` accessors used across the modified files are precisely where unregistered names would surface.
2. **`apply_redesign_bio_visuals` + BIO's `configure_typography` interacting.** `HANDOFF.md` already calls out that calling BIO's `configure_typography` from orchestrator code wipes the redesign font config. The new visuals override may have triggered this.
3. **`prompt_answers_*` modifications interacting with the embedded terminal lifecycle.**

The PR does not include a `popup_collapse_anchor.rs` wrapper (Phase 8 polish item) either.

## What is worth referencing from the PR

These can inform local Phase 5–7 work without being pulled wholesale:

- **`src/install_runtime/start_hooks.rs` + `flag_policies_log.rs` + `flag_policies_eet.rs` + `flag_policies_single_game.rs`.** ~130 lines total. Logic is correct and self-contained; clean reference for Phase 7 wiring.
- **Post-install state flip** in `orchestrator_app.rs:5754` (`record_install_success_if_needed`): the Installed → in-progress → Installed dance with share-code regeneration matches the spec well.
- **Reinstall preview routing** in `orchestrator_app.rs:5531` (`confirm_reinstall_from_home`): the share-code → preview-stage routing pattern is right.
- **`src/ui/shell/shell_titlebar.rs`** custom traffic-light dots + window controls + drag: solid SPEC §1.2 chrome implementation.
- **`src/ui/install/preview_tabs.rs` + `stage_preview.rs`**: the 6-tab preview shape is largely per-spec.

## Decision matrix

| Approach | Effort | Risk |
|---|---|---|
| **Pull and fix the PR** | Disentangle 12 BIO core file mods (some are real BIO features that belong in BIO main, separately); rebuild Phase 6 workspace persistence + Step 4 + Step 2 action wiring; rebuild Create handlers; debug the Phase 8 crash; verify nothing else is hollow. ~60–80% of original Phase 5–8 work plus disentanglement overhead. | High — silent gaps you don't know about, plus unwinding BIO-core mixing is finicky. |
| **Continue locally** | Implement Phase 5–8 against the existing plan docs. Phases 5/7 can reference the PR for shape. Phase 6 needs to be built right against the plan. | Low — clean baseline; the plan + adversarial review have already paid down most of the unknowns. |

Continuing locally takes longer in clock time but produces a faithful, reviewable codebase. The PR's Phase 6 is the killer: pulling it would ship a workspace that visually looks done but functionally does not persist or restore modlist state, does not handle Step 2 input, and has a placeholder Step 4. Those are the bug classes that surface when someone tries to actually use the app.

## Suggested next moves

1. Stay on local. Commit the Phases 1–4 work to a feature branch so it is not at risk.
2. Dispatch Phase 5 against `plan/phase-05-home-install-paste.md`. Reference the PR's `src/ui/home/*` and `src/ui/install/*` for structural shape, but write fresh against the plan.
3. Before Phase 7 starts, copy `src/install_runtime/start_hooks.rs` + the three `flag_policies_*.rs` from the PR into the local tree as a starting point — they are clean and align with the plan.
4. Build Phase 6 carefully against `plan/phase-06-create-workspace-shell.md`. The workspace state loader is the make-or-break piece; do not defer Step 2/3 reconstruction.
5. Skip the PR's Phase 8 approach. When local Phase 8 lands, stick to the carve-out #6 scope strictly: colors only, no font/visuals overrides. Verify every `FontFamily::Name(...)` is registered.
