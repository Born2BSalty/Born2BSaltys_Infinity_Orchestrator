# src/settings/

`bio_settings.json` persistence (the only thing in this directory). Two files:

| File | Role |
|------|------|
| `model.rs` | `Step1Settings` (every Step 1 field — paths, toggles, numeric inputs) + `AppSettings` wrapper with `exe_fingerprint`. Serde-compatible with `#[serde(default)]` so adding fields stays backward-compatible. |
| `store.rs` | `SettingsStore` — load from / save to OS config dir (`%APPDATA%\bio` / `~/.config/bio` / `~/Library/Application Support/bio`). Best-effort save on `WizardApp` drop. |

## Where it's wired

- **Loaded** in `ui/app_bootstrap.rs::initialize` at startup.
- **Saved** opportunistically in `app/app_update_cycle::persist_step1_if_needed` (only when Step 1 actually changed) and on app drop (`ui/app_lifecycle.rs`).
- **Default values** for paths come from `core/platform_defaults.rs` (per-OS `weidu` / `mod_installer` binary lookups).

When adding a new Step 1 field: add to `Step1Settings` in `model.rs` (with a default), surface it in `core/app/state/state_step1.rs` if it influences validation, then add the UI control in `ui/step1/content_step1.rs`.
