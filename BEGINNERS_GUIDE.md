# Born2BSalty's Infinity Orchestrator (BIO)    (Work in progress!!)
## Beginner's Guide (No Coding Needed)

This guide is for people who only use the released app (BIO.exe / app build), not source code.

## What BIO Does

BIO helps you:
1. Point to your game/tools/mod folders
2. Scan mods and pick components
3. Reorder selected components safely
4. Validate compatibility
5. Run install with logs and prompt handling

## Troubleshooting

Before troubleshooting, run BIO in dev mode so diagnostics can be exported and shared.

- Windows (cmd)
1. Open the folder where BIO.exe is.
2. Click the address bar, type cmd, then press Enter.
3. In the black window, run:
    BIO.exe -d gui

- Linux/macOS
1. Open Terminal.
2. Go to the BIO folder:
    cd "/path/to/BIO/folder"
3. Run:
    ./BIO -d gui

## First-Time Setup (Quick Start)



## Where BIO Saves Your Settings

BIO saves your settings in your user app-data location (so they persist between runs):
- Windows: %APPDATA%
- macOS: ~/Library/Application Support
- Linux: ~/.config (or app-equivalent)

You do **not** need source folders to use BIO.


## OS Notes

## Windows
- Run BIO.exe.
- Use full paths like D:\Modding\....
- Typical binaries: weidu.exe and your installer exe.

## macOS
- Use the macOS BIO build.
- Make binaries executable first (chmod +x).
- Use full paths like /Users/<you>/....

## Linux
- Use the Linux BIO build.
- Make binaries executable first (chmod +x).
- Use full paths like /home/<you>/....

## Step 1: Setup (How to Use It)



## Flags (how to decide)
- Skip installed: 
- Abort on warnings: 
- Strict matching: 
- Download missing mods: 
- Overwrite mod folder: 


## Step 2: Scan and Select (How to Use It)

- Scan Mods Folder
- Cancel Scan

## Compatibility pills
You may see:
- Conflict
- Missing dependency
- Warning
- Conditional
- Game mismatch

Click a pill to open detailed explanation.

## Details panel
Shows selected item info such as:
- Component ID/version
- TP2 source
- Paths/readme/web
- Compatibility reason/source/related target


## Step 3: Reorder and Resolve (How to Use It)


## Core actions
- Drag/drop components
- Undo/Redo
- Collapse/Expand
- Revalidate

## Right-click on component row
- Uncheck In Step 2
- Set @wlb-inputs...
- Edit Prompt JSON...
- Clear Prompt Data

Set @wlb-inputs...
- Set comma-separated scripted answers for that component.
- Example: 126,,a,129,,a,,y

Edit Prompt JSON...
- Advanced manual entry editor.

Clear Prompt Data
- Removes saved prompt entry for that component.


## Step 4: Review (How to Use It)

Use Step 4 to verify:
- Selected components
- Final order
- Generated install plan/source

If anything looks wrong, go back to Step 2 or Step 3.

## Step 5: Install, Logs, Diagnostics

## Main usage
- Start install
- Watch console
- Handle prompts
- Cancel/Force Cancel when needed

## Prompt Answers window
Shows remembered prompt entries and lets you edit them.

## How to send diagnostics to support (important)

If you want help, do this exactly:

1. Go to your BIO folder in File Explorer.
2. Click the address bar, type cmd, and press Enter.
3. Command Prompt opens already in the BIO folder.
4. Run:

- BIO.exe -d gui

- Set RUST_LOG=DEBUG (or TRACE if requested)

4. Run the same install flow again until the issue happens.
5. In Step 5, export diagnostics (or collect the generated diagnostics folder).
6. Zip the full diagnostics folder.
7. Send the zip plus a short report:

- what failed
- what you expected
- what happened instead
- game mode
- mod/component involved

That gives enough data to troubleshoot quickly.

## Auto-Answer Basics

BIO can auto-answer using:
1. Component sequence (@wlb-inputs)
2. Saved prompt memory entries

If auto-answer fails when questions are to long:
- Increase delay settings
- Verify component has valid @wlb-inputs
- Confirm component is actually in current run

## EET Path Questions (Most Common Support Issue)

When running EET, installer may ask for path input in console during pre-EET flow.

Important:
- Enter the **actual game folder path** the question asks for.
- Do not enter mod folder path.
- Do not enter log folder path.

Typical case example:
- Prompt asks for BG:EE+SoD path → enter your BGEE game install directory.
    BGEE Game Folder: D:\SteamLibrary\steamapps\common\Baldur's Gate Enhanced Edition

If your install uses flags/modes that trigger extra EET pre-checks (for example combinations like -n/-p in your installer flow), then your path choice on EET question changes!
    Source BGEE Folder (-p): = then BGEE Game Folder:
        Pre-EET Directory: = empty folder on your desired location to install the BGEE game
    Source BG2EE Folder (-n): = then BG2EE Game Folder:
        New EET Directory: = empty folder on your desired location to install the BG2EE game
        
        ## Common Problems and Fixes