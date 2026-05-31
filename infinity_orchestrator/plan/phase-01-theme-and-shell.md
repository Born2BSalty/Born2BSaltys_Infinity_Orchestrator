# Phase 1 — Library/binary split + theme tokens + fonts + shell modules + new binary entry

**Status:** SHIPPED — full audit record: [`archive/phase-01-theme-and-shell.md`](archive/phase-01-theme-and-shell.md).

**Commit anchors:** rolled in with the early redesign baseline (`375342f` "Infinity Orchestrator redesign baseline", `562161b` "Overhaul"); precise per-task commit map not separately captured in `revision-log.md` — see archive for task-level detail.

**What shipped:** the SPEC §1 CRITICAL DIRECTIVE carve-out #3 structural split (`src/lib.rs` declaring the full BIO module tree as `pub mod`, slim ~12-line `src/main.rs` shim, the new `src/bin/infinity_orchestrator.rs` binary entry, plus `[lib]` + `[[bin]] name = "BIO"` blocks in `Cargo.toml`) — mechanical and behavior-preserving, no logic changed. Plus the visual foundation that every later phase compiles against: Poppins (300/500/700) + FiraCode Nerd embedded fonts, the redesign's light + dark theme tokens via additive `pub fn` accessors in `src/ui/shared/redesign_tokens.rs`, and the shell-chrome modules (`shell_chrome` / `shell_titlebar` / `shell_statusbar`) — created but not yet invoked (Phase 2 wires them around `OrchestratorApp`). Both binaries (`BIO_legacy`, `BIO`) coexist and build clean.

**Task IDs (full detail in archive):** P1.T0, P1.T1, P1.T2, P1.T3, P1.T4, P1.T5, P1.T6, P1.T7, P1.T8, P1.T9.
