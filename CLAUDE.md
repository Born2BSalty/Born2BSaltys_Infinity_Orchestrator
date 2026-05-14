# Born2BSalty's Infinity Orchestrator (BIO)

A Rust desktop app + CLI for installing WeiDU mods for the Infinity Engine Enhanced Edition games (BGEE, BG2EE, EET). Built on `eframe`/`egui` with an embedded terminal for live install console.

- Crate name: `bio`, binary `BIO` (see `Cargo.toml`).
- Entry point: `src/main.rs` — parses CLI, then either launches GUI or runs a non-GUI subcommand.
- License: GPL-3.0-or-later. README.md has the user-facing setup walkthrough.

## Top-level layout

| Path | What's here |
|------|-------------|
| `src/` | All Rust source. See `src/CLAUDE.md`. |
| `assets/` | Windows `.ico` / `.png` (compiled into the exe by `build.rs`). |
| `docs/images/` | README screenshots. |
| `third_party/egui_term/` | Patched `egui_term` crate (see `[patch.crates-io]` in `Cargo.toml`). |
| `vendor/lapdu-parser-rust-master/` | Vendored TP2 parser used via `lapdu_parser_rust` dep. |
| `.github/workflows/` | CI. |
| `CHANGELOG.md`, `BEGINNERS_GUIDE.md`, `Downloads GUIDE.md` | User docs. |

## Architecture in one paragraph

A 5-step wizard (Step 1: setup → Step 2: scan/select → Step 3: reorder/resolve → Step 4: review → Step 5: install). Logic lives in `src/core/`, egui rendering lives in `src/ui/`. Both have parallel `step1`–`step5` subtrees: `src/core/app/state/state_stepN.rs` defines the data, `src/ui/stepN/page_stepN.rs` renders it, `src/ui/stepN/action_stepN.rs` enumerates user intents that bubble back up. Settings persist as `bio_settings.json` via `src/settings/`. Compatibility rules (TOML) live in `src/core/config/` and `src/core/app/compat/`.

## Build & run

- `cargo build --release` → `target/release/BIO` (or `BIO.exe` on Windows).
- Default is GUI; non-GUI subcommands: `gui`, `normal`, `eet`, `scan components`, `scan languages` (see `src/core/cli/args.rs`).
- Dev mode: `BIO -d gui` (exposes diagnostics export, extra logging).

## Where things land at runtime

- `bio_settings.json` + `prompt_answers.json` + `step2_compat_rules.toml` → OS-specific config dir (`%APPDATA%\bio` / `~/.config/bio` / `~/Library/Application Support/bio`).
- `diagnostics/run_<timestamp>/` → working directory (export bundle from Step 5).
