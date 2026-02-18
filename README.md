# Born2BSalty's Infinity Orchestrator (BIO)

Born2BSalty's Infinity Orchestrator (BIO) is a Rust desktop wizard for building and running WeiDU-based installs for Infinity Engine Enhanced Edition setups:

- `BGEE`
- `BG2EE`
- `EET`

It scans TP2 components, lets you select/reorder install sets, validates compatibility, and runs `mod_installer` with live console control.

---

## 1) What It Does

### Main workflow (GUI)

The GUI is a 5-step wizard:

1. **Step 1: Setup**
- Configure game mode, folders, tools, and install flags.
- Configure path checks and optional target prep behavior.

2. **Step 2: Scan + Select**
- Scan mods folder for TP2 components.
- Select components for BGEE/BG2EE tabs.
- Apply WeiDU log selection.
- View compatibility pills and details.

3. **Step 3: Order + Validate**
- Reorder selected components.
- Run compatibility validation on ordered set.
- Inspect compatibility modal.

4. **Step 4: Preview/Save**
- Review generated install order / log lines.
- Save/export effective `weidu.log` style output.

5. **Step 5: Install + Console**
- Run install.
- Live console, prompt input bar, auto-answer support.
- Cancel/force-cancel controls.
- Diagnostics/export actions.

---

## 2) Key Features

### Scan and selection

- Fast TP2 component scan from mods directory.
- BGEE/BG2EE tabbed selection for EET workflows.
- Search/filter, bulk select/clear, expand/collapse support.
- Apply selection from existing `weidu.log` sources.

### Compatibility checks

- TP2-driven checks (require/forbid/game predicates/conditional patterns).
- Optional UI compatibility overrides via `step2_compat_rules.toml`.
- Step 2 conflict pills and detailed compatibility popup.
- Step 3 compatibility modal for ordered-set validation.

### Installer run + terminal

- Embedded terminal output from child process.
- `General`, `Important only`, and `Installed only` views.
- Prompt detection and input handling.
- Optional auto-answer using:
  - inline `@wlb-inputs` tags in log lines
  - saved Prompt Answers JSON fallback
- Graceful/force cancel controls.

### Diagnostics and logging

- Export diagnostics bundle to local `diagnostics/`.
- Console save/open actions.
- Optional raw output + BIO debug logs (dev-oriented).

---

## 3) Requirements

- Cross-platform runtime target (Windows/Linux/macOS), with platform-compatible binaries.
- Rust toolchain for building (`cargo`, stable).
- External tools you provide in Step 1:
  - `mod_installer` (`.exe` on Windows)
  - `weidu` (`.exe` on Windows)

---

## 4) Build and Run

From project root:

```bash
cargo build --release
```

Run GUI (default):

```bash
./target/release/BIO.exe
```

Linux/macOS binary name:

```bash
./target/release/BIO
```

Or explicit:

```bash
./target/release/BIO.exe gui
```

Dev mode GUI:

```bash
./target/release/BIO.exe -d gui
```

---

## 5) CLI Commands (Non-GUI)

Binary supports subcommands:

- `gui`
- `normal`
- `eet`
- `scan components`
- `scan languages`

Examples:

```bash
BIO.exe scan components --game-directory "D:\\Games\\BG2EE" --mod-directories "D:\\Modding\\Mods Folder"
```

```bash
BIO scan components --game-directory "/games/BG2EE" --mod-directories "/mods"
```

```bash
BIO.exe scan languages --mod-directories "D:\\Modding\\Mods Folder"
```

```bash
BIO.exe normal --log-file "D:\\Logs\\BG2\\weidu.log" --game-directory "D:\\Games\\BG2EE"
```

```bash
BIO.exe eet --bg1-game-directory "D:\\Games\\BGEE" --bg1-log-file "D:\\Logs\\BG1\\weidu.log" --bg2-game-directory "D:\\Games\\BG2EE" --bg2-log-file "D:\\Logs\\BG2\\weidu.log"
```

---

## 6) Important Files and Paths

### Project-local files (working directory)

- `bio_settings.json`
  - persisted app settings (step/path/options state)
- `diagnostics/`
  - exported diagnostics and saved console logs
  - raw output / BIO debug logs when enabled

### App config paths (per-user)

#### Prompt answers

`prompt_answers.json` is stored at:

- Windows: `%APPDATA%\\bio\\prompt_answers.json`
- Linux: `~/.config/bio/prompt_answers.json`
- macOS: `~/Library/Application Support/bio/prompt_answers.json`

#### Step 2 compatibility rules

`step2_compat_rules.toml` is stored at:

- Windows: `%APPDATA%\\bio\\step2_compat_rules.toml`
- Linux: `~/.config/bio/step2_compat_rules.toml`
- macOS: `~/Library/Application Support/bio/step2_compat_rules.toml`
- fallback: `config/step2_compat_rules.toml`

---

## 7) `@wlb-inputs` (Prompt Auto-Input)

You can append scripted answers to a WeiDU log line:

```text
// @wlb-inputs: y,1,,n
```

Rules:

- answers are consumed left-to-right
- `,,` means blank answer (press Enter)
- keep exact marker with colon: `@wlb-inputs:`

Examples:

```text
~EET\EET.TP2~ #0 #0 // EET core (resource importation): v14.0 // @wlb-inputs: y
```

```text
~EET\EET.TP2~ #0 #0 // EET core (resource importation): v14.0 // @wlb-inputs: D:\My Games\BG2
```

```text
~VIENXAY\VIENXAY.TP2~ #0 #0 // Vienxay NPC for BG1EE: 1.67 // @wlb-inputs: 1,2
```

```text
~SOMEMOD\SETUP-SOMEMOD.TP2~ #0 #10 // Confirm install: v1.0 // @wlb-inputs: y,n,a,c
```

```text
~ANOTHERMOD\SETUP-ANOTHERMOD.TP2~ #0 #0 // Optional prompt: v2.0 // @wlb-inputs: 1,,y
```

---

## 8) Step 1 Flags (Practical Summary)

- `-s` Skip installed
- `-c` Check last installed
- `-a` Abort on warnings
- `-x` Strict matching
- `--download` Download missing mods
- `-o` Overwrite mod folder

Directory cloning modes:

- `-p` Clone BGEE -> Pre-EET target (source unchanged)
- `-n` Clone BG2EE -> EET target (source unchanged)
- `-g` Clone source game -> target directory (source unchanged)

---

## 9) Compatibility Semantics

BIO shows different issue classes:

- **Missing dependency** (`REQ_MISSING`)
- **Conflict** (`FORBID_HIT`, conflict-like rules)
- **Game mismatch** (`GAME_MISMATCH`)
- **Conditional patch** (`CONDITIONAL`)
- **Order warning** (`ORDER_WARN`)

For EET mode:

- tabs are selection buckets (BGEE phase / BG2EE phase)
- game-mode checks are validated in EET context where applicable

---

## 10) Diagnostics Workflow (Support-Friendly)

When reporting an issue:

1. Reproduce in Step 5.
2. Enable diagnostic logging options if needed.
3. Export diagnostics from Step 5 menu.
4. Share:
- `diagnostics/bio_diag_<ts>.txt`
- related `diagnostics/console_<ts>.log`
- raw/debug logs (if enabled)

---

## 11) Troubleshooting

### A) Auto-input does not send

Check:

- `=== Loaded N @wlb-inputs token(s) ===` appears in Step 5 console.
- Prompt actually reached `User Input required`.
- Marker syntax is exact: `// @wlb-inputs:`
- Answers align with real prompt sequence.

If still failing, export diagnostics and include the exact prompt block.

### B) Step 2 shows unexpected conflict/mismatch

- Click pill -> inspect `Reason`, `Source`, `Rule detail`.
- Check `step2_compat_rules.toml` for custom overrides.
- Revalidate after selection/order changes.

### C) Force cancel freezes or is slow

- current implementation is non-blocking for tree-kill call, but process teardown timing still depends on child process behavior.

### D) Parsing/selection from WeiDU log seems wrong

- confirm source log path in Step 1 / Step 4.
- ensure saved log is the one actually used at install time.

---

## 12) Development Notes

- Project is structured by focused modules under `src/ui/...`.
- GUI uses `eframe/egui`.
- Compatibility and TP2 parsing in `src/compat`.
- Terminal/process handling in `src/ui/terminal`.

Recommended dev loop:

```bash
cargo build --release
# run exe, reproduce, export diagnostics, patch, repeat
```

---

## 13) License / Ownership

- License: GNU GPL v3.0 or later (`LICENSE`).
- Creator/Maintainer: `Born2BSalty`.
- Project ownership notice is documented in `NOTICE`.

If you fork or redistribute this project, keep attribution and license terms intact.
