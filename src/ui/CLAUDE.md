# src/ui/

egui rendering layer. Owns `WizardApp` (the `eframe::App` impl) and per-step page renderers. No business logic — calls into `crate::app::*` (which is `core/app/`) for state mutation and into `crate::app::app_step*_flow` for orchestration.

## Top-level files

| File / dir | Role |
|------------|------|
| `run.rs` | `pub fn run(dev_mode)` — eframe entry point called from `main.rs`. Configures typography + visuals, builds `WizardApp`. |
| `app.rs` | The `WizardApp` struct — owns `WizardState`, `SettingsStore`, all background-thread `Receiver`s (scan, GitHub OAuth, update check/download/extract, install prep), the `EmbeddedTerminal`, and Step 5 console view state. |
| `app_bootstrap.rs` | Initial settings load + startup path validation. |
| `app_lifecycle.rs` | Save-on-drop, shutdown. |
| `app_methods.rs` | Misc handlers wired to `app/app_nav_actions.rs`. |
| `app_nav_ui.rs` | Top nav (Reset/Back/Next/Exit) + Step 1 clean-confirm + Step 4 save-error popup. Thin wrapper around `crate::app::app_nav` queries. |
| `app_step2_log.rs`, `app_step2_router.rs` | Step 2 user-action handlers (call into `crate::app::app_step2_*_flow`). |
| `app_update_loop.rs` | The `eframe::App::update` body — polls all background channels via `app_update_cycle`, dispatches the current step's `page::render`, renders shared popups, handles repaint scheduling. |
| `layout.rs` | Global pixel constants (window size, gaps, button sizes). |
| `frame/` | Native window setup: `frame_window.rs` (`native_options`, title), `state_app.rs` (build), `update_app.rs` (visuals + nav button rendering), `action_app.rs` (`launch_gui`). |
| `shared/` | Cross-step concerns: `theme_global.rs`, `typography_global.rs`, `tooltip_global.rs`, `layout_tokens_global.rs`. |
| `step1/` … `step5/` | Per-step page rendering. **See "Step page pattern" below.** |

## Step page pattern (applies to step1–step5)

Each step folder follows the same naming convention. To find a thing for step `N`:

| File | What lives there |
|------|------------------|
| `page_stepN.rs` | `pub fn render(ui, state, dev_mode, exe_fingerprint) -> Option<StepNAction>`. The dispatch entry called from `app_update_loop.rs`. |
| `frame_stepN.rs` | Top/bottom chrome of the page (`render_top` / `render_bottom`). |
| `content_stepN.rs` | Main body (often split further — e.g. `step5/content/content_*` for cancel UI, dev header, install row). |
| `action_stepN.rs` | The `StepNAction` enum — every user intent that bubbles back to the wizard. |
| `state_stepN.rs` | UI-only state (display flags, scroll positions) — distinct from the wizard state in `crate::app::state::StepNState`. |
| `service_stepN*.rs` | Pure helpers used by the page (formatting, validation, list ops). |
| `format_stepN.rs` | Text formatting helpers. |

Larger steps (Step 2 and Step 5) split content into `tree/`, `details/`, `compat/`, `prompt/`, `services/`, `toolbar/`, `update_check/`, `content/`, `prompts/`, `services/`, `status/` subdirs — each subdir mirrors a logical UI region. These subdirs are flattened into `crate::ui::stepN::*` via `#[path]` in the step's `mod.rs`.

## Step-specific notes

- **Step 1 (`step1/`)**: Path/binary configuration. `github_auth_popup_step1.rs` handles GitHub OAuth flow. `archive_backup_step1.rs` handles "clean install" archive backup.
- **Step 2 (`step2/`)**: Heaviest UI. `tree/` renders the component tree, `details/` the right-side selected-component pane, `compat/` the issue popups + window, `update_check/` the update-checker popup, `prompt/` the prompt-summary popup.
- **Step 3 (`step3/`)**: Drag-reorder flow — `state_drag_step3.rs` + `service_drag_ops_step3.rs` own the drag mechanic. `block_selection_step3.rs` is multi-select.
- **Step 4 (`step4/`)**: Smallest. Review + save. `service_step4.rs` handles the save action.
- **Step 5 (`step5/`)**: Install runtime. `content/` is the install header + cancel controls; `prompts/` is the auto-answer table; `status/` is the bar/console/input row/phase indicator; `top_panels_step5.rs` is the upper console panel; `menus_step5.rs` is dev menus.
