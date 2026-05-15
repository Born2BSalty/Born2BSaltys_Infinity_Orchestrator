# Spec Authority

The **user-approved spec for the project currently in flight is the source of truth for every behavior decision.** Any agent doing planning, implementation, or review treats it as gospel. A finding — from analysis, review, QA, or your own judgment — that contradicts the spec MUST be escalated to the user, never silently applied.

This doctrine is **project-agnostic**. It does not assume any particular spec file or project. The specific artifacts that constitute "the spec" for whatever work is active right now are named in the **Current project** section at the bottom of this file — that section is swapped when the active project changes; everything above it stays.

## What "the spec" is

"The spec" is not one fixed file. For the active project it is the set of approved artifacts identified in **Current project** below (and/or in your dispatch brief), evaluated in a priority order where higher wins on conflict:

1. **Project hard-directives** — any non-negotiable constraint the project's spec pins down (e.g., "only these N edits to existing code are allowed"). The hardest constraint.
2. **The approved spec document** — the rest of the product spec for the active project.
3. **The canonical UI reference** (wireframe / mockups / design source), if the project has one — wins over spec prose on visual / copy / spacing / pixel detail.
4. **The plan / work-order docs** — each task traces to a spec section.
5. Pre-existing / legacy behavior — fallback only when 1-4 are silent.

A handoff or state doc, if the project has one, records current state, caveats, and intentional deviations already approved by the user — treat those as settled, not as drift to re-fix.

## Why

The spec and UI reference encode user-approved intent an agent cannot reconstruct from code alone. Silent overrides cause churn and have broken behavior before: code that "looks wrong" locally is often a deliberate, spec-encoded user decision. When the code matches the spec, that is not a defect to fix — it is intent to preserve.

## What counts as a SPEC CONFLICT

Acting on the finding would:

- Remove or change behavior the spec or UI reference specifies must exist.
- Violate a project hard-directive (e.g., touch code the directive puts off-limits).
- Change a user-facing interaction, data shape, or file format the spec pins down.
- Contradict the spec section a task traces to, or an intentional deviation already recorded by the user.

**Gap vs conflict:** if the spec is silent on a detail and you must exercise judgment *within* spec intent, that is a **gap** — resolve it by reading intent. If you are unsure whether something is a conflict, **assume it is** and escalate.

## Escalation protocol

1. **Do not silently apply the change.** Do not remove the conflicting code or "fix" spec-specified behavior.
2. **Label it with the exact phrase `SPEC CONFLICT`** so it is detectable, in a dedicated section *before* any other findings (reviewers: before Critical Issues).
3. State it as: **Spec claim** (what the spec / UI ref / directive says, with citation) · **Finding** (what the code does or what you'd change) · **Implication** (why it matters). Do not recommend the "fix" in the body — describe, cite, request confirmation.
4. **Stop and wait for the user's decision:** (a) finding is right → amend the spec; (b) spec is right → reject the finding; (c) re-scope.
5. Once the user decides, record it (commit message / handoff note) so it is not re-litigated.

## The only path to amend the spec

If the user decides the spec must change: edit the spec document (and the UI reference and/or plan + handoff doc) so they stay in sync, written **as if that were always the intent** — no decision-tree narrative in the spec body (substantive revision rationale belongs in a revision log if the project keeps one). Then continue from the earliest affected task. No task may silently mutate the spec or act as if it says something it does not.

## Scope-growth resistance

If achieving a task genuinely requires behavior a project hard-directive does not permit, that is **not** a license to do it — it is a `SPEC CONFLICT` requiring explicit user approval to relax/extend the directive. **Resist.** Prefer the spec's prescribed escape hatch (e.g., a net-new component over editing protected code). Surface the request; never sneak it in.

## Bias rule

When in doubt, preserve spec-specified behavior and escalate. A false escalation costs one user turn; a silent override costs broken behavior or a directive violation. Measure twice, cut once.

---

## Current project

> Swap this section when the active project changes. Everything above is project-agnostic and stays.

**Project:** Infinity Orchestrator — the redesign of the `bio` Rust crate into a multi-modlist workspace app, alongside the preserved legacy `BIO` binary.

**Spec artifacts, in the priority order above:**

1. **Project hard-directive:** `infinity_orchestrator/SPEC.md` §1 CRITICAL DIRECTIVE — the **six authorized carve-outs**. Any edit to existing BIO source outside those six is disallowed; everything else is a net-new file/sibling.
2. **Spec:** `infinity_orchestrator/SPEC.md`.
3. **Canonical UI reference:** `infinity_orchestrator/wireframe-preview/` — wins over SPEC prose on UI / UX / layout / copy / spacing / pixel values.
4. **Plan / work order:** `infinity_orchestrator/plan/phase-NN-*.md` (per-phase task lists); `infinity_orchestrator/plan/overview.md` holds the revision log.
5. **State / caveats / settled deviations:** `infinity_orchestrator/HANDOFF.md`.

**Verification for this project:** `cargo build --bin infinity_orchestrator --release`; `cargo test --lib` when lib/serde code changed; `cargo build --bin BIO --release` to confirm the legacy binary still compiles. Windows footgun: a linker `Access is denied. (os error 5)` means the running `infinity_orchestrator.exe` holds the file lock — report it, don't work around it destructively.
