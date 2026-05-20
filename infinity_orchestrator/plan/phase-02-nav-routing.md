# Phase 2 — `OrchestratorApp` + left-rail navigation + destination routing

**Status:** SHIPPED — full audit record: [`archive/phase-02-nav-routing.md`](archive/phase-02-nav-routing.md).

**Commit anchors:** rolled in with the foundational redesign work alongside Phase 1; precise per-task commit map not separately captured in `revision-log.md` — see archive for task-level detail.

**What shipped:** the standalone `OrchestratorApp` `eframe::App` impl living inside the library crate at `bio::ui::orchestrator::orchestrator_app::OrchestratorApp` (per Phase 1's carve-out #3 split), owning its own `WizardState`, its own `SettingsStore`, its own background-thread receivers, and its own destination router. The Phase 1 shell chrome (titlebar + statusbar) is wired around it; the persistent left rail renders the brand mark + four nav items (Home / Install / Create / Settings) + bottom status dot per SPEC §2.1, plus the `Workspace { modlist_id }` destination as a stub. Wholly independent of `WizardApp` — the two coexist as separate `eframe::App` impls. The four primary destinations and `Workspace` are stub pages in this phase (real content arrives in Phases 4-6).

**Task IDs (full detail in archive):** P2.T1, P2.T2, P2.T3, P2.T4, P2.T5, P2.T6, P2.T7, P2.T8.
