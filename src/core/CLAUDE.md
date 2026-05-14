# src/core/

Pure logic — no egui imports. Re-exposed at crate root via `#[path]` aliases in `main.rs` (this directory's name does not appear in module paths).

## Map

| Dir / file | Role |
|------------|------|
| `app/` | Wizard state machine, step flows, compat engine, embedded terminal, modlist sharing. **Heaviest subtree** — see `app/CLAUDE.md`. |
| `cli/args.rs` | `clap` definitions: `Cli`, `Command` (Gui/Normal/Eet/Scan), `CommonOptions`. |
| `config/options.rs` | Converts parsed `Cli` → `AppCommandConfig` (the typed command the rest of the app consumes). |
| `config/default_*.toml` + `user_*.toml` | Default & user-overridable compat rules and mod-download manifests, embedded via `include_str!`. |
| `install/` | WeiDU/`mod_installer` execution. `weidu_exec.rs` runs the binary; `weidu_scan*.rs` runs read-only scans; `step5_command_*.rs` builds the install command. `plan.rs` + `runner.rs` are the CLI-mode (non-GUI) install entry. |
| `mods/` | Disk-side mod model: `component.rs`, `discovery.rs` (find TP2s), `log_file.rs` (parse `weidu.log`). |
| `parser/` | TP2 parsing — `weidu_component_line.rs`, `weidu_version.rs`, `prompt_eval_expr*` (TP2 conditional language), `compat_dependency_expr.rs`, and the vendored `lapdu/` TP2 grammar parser bridge. |
| `logging/setup.rs` | `tracing-subscriber` init from `--log-level`. |
| `platform_defaults.rs` | OS-specific default paths (where to look for `weidu`, `mod_installer`, etc.). |

## Headless vs GUI dispatch

- `app/dispatch.rs` is the headless entry — called from `main.rs` for all non-GUI subcommands. It fans out to `app/normal.rs`, `app/eet.rs`, `app/scan_components.rs`, `app/scan_languages.rs`.
- GUI never goes through `dispatch.rs`; it goes straight to `ui::run`.
