// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

pub const DEFAULT_TOOLTIP_MAX_WIDTH: f32 = 1400.0;

pub const COPY: &str = "Copy";
pub const OPEN: &str = "Open";
pub const SHOW_PARSED_PROMPTS: &str = "Show parsed prompts.";

pub const STEP1_GAME_INSTALL: &str = "Select target mode: BGEE, BG2EE, or EET.";
pub const STEP1_CUSTOM_SCAN_DEPTH: &str =
    "Maximum folder depth under Mods Folder to search for TP2 files.";
pub const STEP1_TIMEOUT_PER_MOD: &str = "What it does: Sets the timeout for the whole install run from start to finish.\n\
Example: If your whole mod list needs 5 hours, set this to 18000 seconds.\n\
How to use it: Enable this and enter the number of seconds to allow.\n\
Default when off: 3600 seconds (1 hour).";
pub const STEP1_AUTO_ANSWER_INITIAL_DELAY: &str = "Base wait before first auto-answer on a prompt. Increase this for large prompt lists so answers are not sent too early.";
pub const STEP1_AUTO_ANSWER_POST_SEND_DELAY: &str =
    "Wait between auto-answer sends and fallback checks.";
pub const STEP1_TICK_DEV: &str = "Polling interval in ms for installer output.";
pub const STEP1_WEIDU_LOG_MODE: &str =
    "Enable/disable WeiDU logging flags (autolog/logapp/log-extern/log component).";
pub const STEP1_PROMPT_REQUIRED_SOUND: &str =
    "Play a bell once when installer is waiting for input.";
pub const STEP1_DOWNLOAD_ARCHIVE: &str =
    "Store downloaded mod archive files in the configured Mods Archive folder.";
pub const STEP1_PROMPT_CONTEXT_LOOKBACK: &str =
    "Keep this many prior output lines for prompt detection and context.";
pub const STEP1_PREPARE_TARGET_DIRS: &str =
    "BIO prepares target directories before run (backup or clean, based on next option).";
pub const STEP1_BACKUP_TARGET_DIRS: &str = "If target dir has files, move it to a timestamped backup folder and recreate an empty target before copy.";
pub const STEP1_SKIP_INSTALLED: &str = "Skip components already present in WeiDU logs.";
pub const STEP1_CHECK_LAST_INSTALLED: &str = "Use strict last-installed validation.";
pub const STEP1_CLONE_BGEE_PRE_EET: &str = "Copy from Source BGEE Folder into Pre-EET Directory.";
pub const STEP1_ABORT_ON_WARNINGS: &str = "Abort install when warnings are encountered.";
pub const STEP1_STRICT_MATCHING: &str = "Require strict component/version matching.";
pub const STEP1_CLONE_BG2EE_EET: &str = "Copy from Source BG2EE Folder into New EET Directory.";
pub const STEP1_DOWNLOAD_MISSING: &str = "Prompt for download URI when a mod is missing.";
pub const STEP1_OVERWRITE_MOD_FOLDER: &str = "Force copy mod folders even when they already exist.";
pub const STEP1_CLONE_SOURCE_TARGET: &str = "Copy from Source Game Folder into Generate Directory.";

pub const STEP2_SUBTITLE: &str = "Select the components you want BIO to install.";
pub const STEP2_SEARCH: &str = "Filter the tree by mod name, component text, TP2, or id.";
pub const STEP2_SCAN: &str = "Scan the configured Mods Folder and build the mod/component tree.";
pub const STEP2_CANCEL_SCAN: &str = "Cancel the active scan.";
pub const STEP2_CLEAR_ALL: &str = "Uncheck all components in the current tab.";
pub const STEP2_SELECT_VISIBLE: &str = "Check all filter-matching components in the current tab.";
pub const STEP2_COLLAPSE_ALL: &str = "Collapse all parent mods in the tree.";
pub const STEP2_EXPAND_ALL: &str = "Expand all parent mods in the tree.";
pub const STEP2_JUMP_SELECTED: &str = "Scroll to the currently selected row in the tree.";
pub const STEP2_MODS_COMPONENTS: &str =
    "Active game tab controls which component list and log-apply action are used.";
pub const STEP2_SELECT_BGEE_LOG: &str = "Read BGEE WeiDU log and tick matching components.";
pub const STEP2_SELECT_BG2EE_LOG: &str = "Read BG2EE WeiDU log and tick matching components.";

pub const STEP3_EXPORT_DIAGNOSTICS: &str = "Export diagnostics from current state.";
pub const STEP3_EXPAND_ALL: &str = "Expand all parent blocks.";
pub const STEP3_COLLAPSE_ALL: &str = "Collapse all parent blocks.";
pub const STEP3_REDO: &str = "Redo the most recent undone reorder.";
pub const STEP3_UNDO: &str = "Undo the most recent reorder.";
pub const STEP3_LOCK_PARENT: &str = "Lock/unlock this parent block for drag operations.";
pub const STEP3_DRAG_PARENT: &str = "Drag to move parent block";
pub const STEP3_DRAG_ROW: &str = "Drag to reorder";

pub const STEP4_SAVE_WEIDU_LOG: &str = "Write weidu.log file(s) from the current install order.";

pub const STEP5_FORCE_CANCEL: &str = "Immediate stop. May leave game/mod state unrecoverable.";
pub const STEP5_CANCEL_INSTALL: &str = "Request cancel. Confirmation required.";
pub const STEP5_START_INSTALL: &str = "Start installer with current configuration.";
pub const STEP5_DEV_MODE_DIAG_REQUIRED: &str = "Dev mode requires diagnostics: enable Full Debug + Raw Output and set RUST_LOG to DEBUG or TRACE.";
pub const STEP5_GENERAL_OUTPUT: &str = "Show full output (no filtering).";
pub const STEP5_IMPORTANT_ONLY: &str = "Show only important lines (warn/error/fatal/prompts).";
pub const STEP5_INSTALLED_ONLY: &str = "Show only installation progress lines.";
pub const STEP5_AUTO_SCROLL: &str = "Follow new output automatically.";
pub const STEP5_PROMPT_ANSWERS: &str = "Manage saved auto-answer entries.";
pub const STEP5_CAPTURE_PROMPT: &str = "Create/update entry for currently detected prompt key.";
pub const STEP5_COPY_ERROR_BLOCK: &str = "Copy recent error/fatal lines from console output.";
