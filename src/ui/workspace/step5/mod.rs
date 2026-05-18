// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// `bio::ui::workspace::step5` — the Step-5 install-runtime **chrome** that
// wraps BIO's existing embedded Step-5 panel inside the workspace.
//
// Per SPEC §9 + the CRITICAL DIRECTIVE: BIO's Step-5 install runtime — the
// install runner, the embedded terminal, the WeiDU child process, prompt
// detection + auto-answer, the diagnostics writer, the Actions/Diagnostics
// menus, the Prompt Answers window — stays **exactly** as today's BIO. This
// tree is net-new redesign chrome that **calls
// `bio::ui::step5::page_step5::render` directly** (the verified public
// top-level renderer) and composes the new chrome rows *around* it (per H9,
// the success-banner row + post-install action row sit immediately ABOVE
// the embedded panel — visually adjacent to BIO's Install button at the
// top of that panel). BIO's Step-5 module tree is never reached into and
// never edited.
//
// **Run 1 (Step-5 runtime spine + workspace chrome) scope.** This run
// implements ONLY the chrome scaffold:
//   - `page_workspace_step5` → renders the (empty pre-install)
//     success-banner row slot + the (empty pre-install) post-install
//     action row slot ABOVE the embedded panel, then calls
//     `page_step5::render` with the OrchestratorApp Step-5 fields
//     (`terminal = step5_terminal.as_mut()` = `None` pre-install → BIO's
//     pre-install panel: Command card, Summary card, console box, prompt
//     input), then dispatches the returned `Option<Step5Action>`. For
//     `StartInstall`, Run 1 ONLY sets the install-clicked marker that
//     drives P7.T8's `← Previous` lock; the real install-start hook is a
//     clearly-commented Run-2 (P7.T3) placeholder.
//   - `success_banner` / `post_install_actions` → the C3-gated rows; they
//     render **nothing** this run (the C3 clean-exit triple is false — no
//     install has run). Their behavior lands in Run 3 (P7.T4 / P7.T5).
//   - `share_paste_code_dialog` / `state_workspace_step5` → minimal stubs
//     (the dialog's behavior is Run 3 / P7.T7; `state_workspace_step5`
//     carries the Run-1 install-clicked marker + the Run-3 fields declared
//     inert, mirroring the established staged-field pattern).
//
// SPEC: §9.1, §9.2, §9.3, §10.3, §13.13 (H9 positioning).

pub mod page_workspace_step5;
pub mod post_install_actions;
pub mod share_paste_code_dialog;
pub mod state_workspace_step5;
pub mod success_banner;
