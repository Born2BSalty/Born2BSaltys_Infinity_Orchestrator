// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// `bio::ui::workspace::step2` — the orchestrator-side **Step 2 C4 chrome
// wrapper** (P6.T2c). The direct analogue of `step4/` (P6.T2b): a net-new
// redesign chrome that reuses **only** BIO's tree / details / popup
// sub-renderers. BIO's `page_step2` / `frame_step2` are **not** called.
//
// Why net-new chrome (per the 2026-05-16 SPEC-CONFLICT resolution in
// `plan/overview.md`): the wireframe's Step 2 (`screens.jsx` ~L2786-2920) is
// structurally different from BIO's `frame_step2` — full-width `flex`
// search, **no** "Restart App With Diagnostics", Details pane
// hidden-by-default (SPEC §6). Carve-out #6 is colour-only and cannot
// restructure BIO's `frame_step2` toolbar; the CRITICAL DIRECTIVE forbids
// editing it. So the chrome is net-new; only the heavy interaction
// surfaces (tree / details / compat / prompt) are reused read-only via
// BIO's public rect-parameterized sub-renderers (directive decision-order
// step 1).
//
//   - `workspace_step2`  → the top-level Step-2 renderer + layout-rect
//                           owner (the #4 fix: the orchestrator owns the
//                           split/footer rects so the panel never bleeds
//                           into the workspace nav bar).
//   - `step2_search`     → the net-new full-width `flex` search row + the
//                           wireframe `Rescan Mods Folder` button +
//                           (dev-only) scan-folder affordance.
//   - `step2_tab_row`    → the net-new redesign tab row (GameTabs +
//                           log/updates buttons + live compat / prompt
//                           Pills + count Label + Kebab). Replaces BIO's
//                           `content_step2::render_controls` +
//                           `render_tabs` (the wireframe has no BIO
//                           controls row — `screens.jsx:2807-2852`).
//   - `step2_dev_scan`   → the dev-only "scan an arbitrary folder"
//                           affordance (test enablement; absent in normal
//                           mode — pre-Phase-7 there is no per-install
//                           extracted-mods folder, SPEC §13.12a).
//   - `step2_rescan_reconcile` → the rescan-reconcile logic (SPEC §6.3, the
//                           #2 fix): snapshot the selection at scan-trigger
//                           time, re-apply it onto the freshly-scanned mod
//                           set on scan-completion (preserving
//                           `selected_order`), drop only absent
//                           mods/components + surface the missing-mods
//                           warning. Net-new orchestrator logic (BIO has no
//                           reusable rescan-preserves-selection mechanism);
//                           BIO read-only.
//   - `step2_log_confirm` → the Select-via-WeiDU-Log destructive-confirm
//                           strings (SPEC §6.10 + wireframe `askWeiduImport`,
//                           `screens.jsx:2778-2784`). Owns only the
//                           wireframe-verbatim title/body/label; the modal
//                           itself is the shared `ConfirmDialog` Home reuses.
//
// SPEC: §6, §6.10, §1 (decision order; carve-out boundary), §13.12a, §2.2;
//       wireframe `screens.jsx:2786-2880`.

pub mod step2_dev_scan;
pub mod step2_log_confirm;
pub mod step2_rescan_reconcile;
pub mod step2_search;
pub mod step2_tab_row;
pub mod workspace_step2;
