# src/

Entry point and the three top-level module trees.

## Entry point flow

`main.rs` (45 lines):
1. Parses `Cli` (from `core/cli/args.rs`).
2. Defaults to `Command::Gui` when no subcommand is given.
3. `setup::init` configures `tracing` (from `core/logging/`).
4. Converts CLI to `AppCommandConfig` (`core/config/options.rs`).
5. Dispatches:
   - `Gui` → `ui::run(dev_mode)` (eframe loop).
   - everything else → `app::dispatch::run(&command)` (headless).

`main.rs` re-exports core modules with `#[path = "core/<x>/mod.rs"] mod <x>;` so the rest of the crate refers to them as `crate::app`, `crate::cli`, etc. — i.e. the `core/` layer in disk does not appear in module paths.

## Subtrees

| Dir | Role | Tier-2 doc |
|-----|------|------------|
| `core/` | Pure logic: wizard state, install runner, scanners, compat engine, parsers, CLI args. | `core/CLAUDE.md` |
| `ui/` | egui rendering: `WizardApp`, frame chrome, per-step pages. | `ui/CLAUDE.md` |
| `settings/` | `bio_settings.json` model + load/save. | `settings/CLAUDE.md` |
