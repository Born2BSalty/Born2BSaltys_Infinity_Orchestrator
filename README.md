# Born2BSalty's Infinity Orchestrator (BIO) - BGEE / BG2EE / EET / IWD WeiDU Mod Installer

BIO is a WeiDU mod installer and install-order orchestrator for BGEE, BG2EE, EET / And IWD.

If you are looking for a Baldur's Gate mod manager, BGEE WeiDU installer, or EET mod installer, this project is built for that.

## Supported Targets

- BGEE
- BG2EE
- EET
- IWDEE

For IWD-in-EET style setups, use the EET workflow. Those mods install into the EET/BG2EE target, not into a normal IWDEE install.

## Community & Support

Join the BIO Discord for:

- installation help
- test builds
- bug reports
- modlist sharing
- development discussion

[![Discord](https://img.shields.io/badge/Discord-Join%20BIO-5865F2?logo=discord&logoColor=white)](https://discord.gg/QqZtzvHDG5)

## What BIO Does

- Scans WeiDU mods and TP2 components.
- Builds modlists from selected components.
- Checks compatibility, dependencies, conflicts, game-target rules, and install-order problems.
- Supports BGEE, BG2EE, EET, and IWDEE workflows.
- Lets users save, load, and share BIO modlists.
- Guides users through installing a BIO modlist.
- Runs installs with live output, prompt handling, and diagnostics.
- Helps support troubleshooting through exported diagnostic bundles.
  
## Quick Start (Normal Users)

1. Download the latest BIO release for your system.
2. Download WeiDU v249 from https://github.com/WeiDUorg/weidu/releases/tag/v249.00

   WeiDU v249 is currently recommended because it has had the fewest reported issues with BIO.
   
4. Extract the download fully before running BIO.
5. Launch `BIO.exe`.
6. Open `Settings` and configure the required game/tool paths.
7. Use `Create` to scan mods and build a BIO modlist.
8. Use `Install` to install an existing BIO modlist or shared BIO code.
9. If something fails, export diagnostics and share them in the BIO Discord.

## Diagnostic Mode

If you are testing BIO or reporting a bug, enable Diagnostic Mode from inside BIO.

1. Open `Settings`.
2. Go to the `General` tab.
3. Enable `Diagnostic Mode`.

Diagnostic Mode provides extra logging and support information that can help track down bugs.

## Wizard Overview
![BIO screenshot 0](docs/images/Home.png)
![BIO screenshot 1](docs/images/Install.png)
![BIO screenshot 2](docs/images/Summary.png)
![BIO screenshot 3](docs/images/Install%20Weidu%20Preview.png)
![BIO screenshot 4](docs/images/Install%20Weidu%202%20Preview.png)
![BIO screenshot 5](docs/images/Install%20Download%20Preview.png)
![BIO screenshot 6](docs/images/Install%20Refs%20Preview.png)
![BIO screenshot 7](docs/images/Install%20Mod%20Config.png)
![BIO screenshot 8](docs/images/Download%20and%20Extract.png)
![BIO screenshot 9](docs/images/Install%20step%205%20screen.png)
![BIO screenshot 10](docs/images/Installation%20Preview.png)
![BIO screenshot 11](docs/images/Installation%20Preview%202.png)
![BIO screenshot 12](docs/images/Create%20Start.png)
![BIO screenshot 13](docs/images/Create%20Scanned%20Preview.png)
![BIO screenshot 14](docs/images/Create%20Step%203%20preview.png)
![BIO screenshot 15](docs/images/Create%20step%204%20preview.png)
![BIO screenshot 16](docs/images/Settings%201.png)
![BIO screenshot 17](docs/images/Settings%202.png)
![BIO screenshot 18](docs/images/Settings%203.png)
![BIO screenshot 19](docs/images/Settings%204%20.png)
![BIO screenshot 20](docs/images/Settings%205.png)

## Main Workflows

### Home

The Home screen is the starting point for BIO. From here you can create a new modlist,install an existing one, continue previous work, or open settings.

### Create

Create is for building a BIO modlist.

Use Create to:

- choose the target game mode
- scan a folder of extracted WeiDU mods
- inspect available TP2 components
- select components for install
- review compatibility warnings and mismatches
- reorder selected components
- save or share the finished BIO modlist

### Install

Install is for using an existing BIO modlist.

Use Install to:

- load a BIO modlist or share code
- review the install summary
- check required WeiDU references
- configure install options
- download and extract supported mod archives
- run the install
- watch live install output
- export diagnostics if support is needed

### Settings

Settings is where BIO paths and behavior are configured.

Use Settings to configure:

- game folders
- WeiDU path
- mod installer path
- download behavior
- install behavior
- prompt handling
- diagnostics and support options

## Compatibility Checks

BIO checks TP2 and install-order data to help catch problems before install.

BIO can report:

- missing dependencies
- conflicts
- game mismatches
- install-order warnings
- conditional compatibility cases
- EET phase or target issues

A mismatch means BIO believes a component does not match the selected target game or install context. For example, a component requiring GAME_IS ~eet~ should be installed in an EET workflow, not a normal IWDEE workflow.

## WeiDU and Mod Installer

BIO does not replace WeiDU. BIO orchestrates WeiDU-based installs and helps users build, validate, and run modlists.

Depending on the release and setup, BIO may require paths to external tools such as:

- WeiDU
- mod_installer

Configure these in Settings if BIO asks for them.

## Diagnostics for Support

When reporting a problem:

1. Reproduce the issue.
2. Export diagnostics from BIO.
3. Send the diagnostics folder in the BIO Discord.
4. Include a short explanation of:
  - what you expected
  - what happened instead
  - which mod/component failed
  - which game mode you selected

Diagnostics help support identify modlist, path, compatibility, and install-output problems faster.

## App Data

BIO stores user settings and support files in the normal per-user app data location for your operating system.

Common files may include:

- bio_settings.json
- prompt_answers.json
- compatibility rule files
- diagnostics exports

## Build From Source

Source users need:

- Rust stable
- system build tools required by Rust dependencies
- Java/JDK if parser generation is required by the build

Build:

cargo build --release

The built BIO executable will be under:

target/release/

## Media

Videos and additional previews will be added later.

## License and Ownership

- License: GNU GPL v3.0 or later
- Maintainer/Owner: Born2BSalty
- Ownership and attribution details: see NOTICE
