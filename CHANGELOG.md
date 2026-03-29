## Beta 15
- Reworked compatibility handling and messaging:
- FORBID_COMPONENT now distinguishes install-order issues from real conflicts
- mutual exclusions are clearer
- included/mismatch wording is improved
- Added richer Step 2 compatibility UX:
- toolbar badges
- compat popup filters
- compat Next navigation
- better jump behavior and popup persistence
- Improved Step 2 details and scanning:
- Details frame copy buttons
- Details TP2 folder shortcut
- cleaner TP2 block extraction
- better TP2 grouping/order handling for mods like TNT
- Added TP2 deprecated detection and improved dependency/path handling:
- deprecated now overrides mismatch
- FILE_EXISTS checks behave as missing dependencies
- Improved parser and diagnostics:
- better prompt extraction for affected mods
- more complete diagnostics exports and triage files

## Beta 14
- Improved Step 2 and Step 3 compatibility UX:
- toolbar issue badges, popup filters, Next navigation, and better jump behavior
- Fixed stale and misleading compatibility states in Step 2:
- Clear All / Select Visible now refresh correctly
- missing_dep only shows for checked components
- FILE_EXISTS checks now classify as missing dependencies
- Reworked TP2 compatibility handling:
- better ENGINE_IS mismatch handling
- FORBID_COMPONENT now behaves as install-order logic where appropriate
- clearer mutual-exclusion reporting for true impossible combinations
- Step 2 conflicts no longer auto-grey or auto-uncheck components, so users can resolve them manually
- Improved Step 3 ordering behavior:
- dragging full mod selections now keeps headers with their components
- Step 2 checkbox order now syncs back into Step 3 correctly
- Added better TP2 details in Step 2:
- copy buttons for Component Block and WeiDU Line
- cleaner component-block extraction for large mods like EET
- Added TP2 DEPRECATED scanning:
- deprecated components now override mismatch
- they are shown as deprecated, greyed out/uncheckable, but still selectable for details
- deprecated status is now component-only and no longer shown on main mod headers

## Beta 13
- Fixed several component ordering issues so BIO matches manual WeiDU TP2 order more closely.
- Improved TP2 label/order matching for mods using `BEGIN @...`, custom setup TRA files, and WeiDU version-suffixed labels.
- Increased scan worker cap from `8` to `16`.
- Fixed false `GAME_INCLUDES "bg2"` mismatches on `BG2EE` and `EET`.
- Improved Step 3 compatibility handling:
- dedicated validator path
- row compat pills
- automatic revalidation after drag-drop and uncheck actions
- Removed redundant `Revalidate` buttons from Step 2 and Step 3.
- Improved Step 3 instructions and compatibility filter UI.
- Cleaned up orphaned compat code and reduced duplicated formatting helpers.

## Beta 12
- Fixed several Step 2 / Step 3 compatibility misreads for complex WeiDU TP2 rules.
- Improved Stratagems / SCS compatibility handling, including order rules, mutual exclusivity, and Item Revisions-related checks.
- Fixed stale or duplicated Step 2 conflict pills after changing selections.
- Improved jump-to-conflict so BIO now expands collapsed mod/group headers to reveal the target component.
- Improved compatibility popups in Step 2 / Step 3:
- cleaner layout
- scroll support for large entries
- collapsible `Component block`
- Improved dependency and order reporting, including better handling of OR requirements.
- Improved `Path requirement` reporting to show the actual missing file/folder BIO checked.
- Fixed several mod grouping / ordering issues so component trees match WeiDU more closely.
- Added a Step 1 install language dropdown that applies across scan and install flow.

## Beta 11
- Fixed Step 3 compatibility jump so related-target navigation now goes to the exact related component instead of the wrong component in the same mod.
- Fixed Step 2 mod-header compatibility pills
- Fixed Step 2 so shared-package nested TP2s like `EET_gui` and `EET_end` now scan from the correct working directory instead of showing undefined strings.
- Step 2 now shows shared subcomponent option groups as collapsible headers.
- Step 2 now hides cosmetic/customization-only subcomponent branches from the main component list.
- Step 2 no longer shows main mod-header pills for game mismatches or included components.
- Step 2 TP2 compatibility parsing now ignores block comments.
- Fixed BG1UB Restored Elfsong Tavern Movie on BGEE.
- Step 2 now catches negated `GAME_IS` rules like `!GAME_IS ~bgee eet~` and greys those components out correctly.
- Step 2 now catches `GAME_INCLUDES` rules like `GAME_INCLUDES ~tob~` and greys those components out correctly.
- Fixed BG1UB Angelo Notices Shar-teel so it now shows as already included on BGEE.
- Step 2 now greys out deprecated TP2 components and shows them with a Deprecated pill.
- Step 2 now correctly handles components with multiple TP2 `GAME_IS` restrictions.
- Fixed strict cargo clippy warnings

## Beta 10
- Reworked diagnostics so WeiDU logs are now exported by where they actually came from, instead of vague source/saved folders.
- Diagnostics now separates logs from game folders, WeiDU log folders, selected WeiDU log files, and Step 4 saved WeiDU logs.
- EET diagnostics now also checks for `WeiDU-BGEE.log` where relevant.

## Beta 9
- Fixed Step 2 mod discovery so category folders inside the selected mods directory no longer collapse multiple mods into one entry.
- Fixed Step 2 mod discovery so nested support TP2 files stay grouped under their parent mod instead of appearing as separate mods.

## Beta 8
- Initial Beta 8 notes.
- Step 1 no longer disables `-s Skip installed` when target-dir preparation is enabled.
- Fixed TP2 game validation so `REQUIRE_PREDICATE NOT (GAME_IS ~...~)` is no longer misread as a positive game restriction.
- Reset Wizard State on Step 2 now also deletes `bio_scan_cache.json` and `prompt_answers.json`.
- Step 2 now greys out simple `DIRECTORY_EXISTS` components like BGGO, but no longer pre-disables `FILE_EXISTS` cases such as generated install-time files.
- Step 2 no longer treats classic-engine `GAME_IS` targets like `BGT` and `ToB` as automatically valid for `EET`.
- Step 2 component ordering now follows TP2 BEGIN file order instead of --list-components order
- Diagnostics now keep `raw_output_*` and `bio_full_debug_*` inside the current `run_*` folder, so exporting diagnostics preserves the active run and only old runs are pruned.
- Save Console Log now also writes `console_*` into the current `run_*` folder instead of the top-level diagnostics folder.

## Beta 7
- Dev-mode diagnostics now show which BIO build wrote the scan cache, so it is easier to spot reports coming from old cached scans.
- Dev-mode diagnostics now export the full raw Lapdu parser JSON for each scanned TP2 under parser_raw/.
- Dev-mode diagnostics now split copied WeiDU logs into source_logs and saved_logs, so it is clearer which logs BIO used as input and which ones came from Step 4 save.
- Dev-mode diagnostics now clear old run folders and old debug logs before a new run, so people do not send a folder full of stale logs.
- Dev-mode diagnostics now stop showing cache build mismatch as false when the scan did not actually use cache.
- Clarified the Step 1 timeout tooltip so it explains this timeout is for the whole install run, not each individual component.
- Step 2 prompt rendering no longer shows info prefaces as fake standalone yes/no prompts before the real question.
- Step 3 now keeps your draft install order if you go back to Step 2 and come back.
- Step 3 now shows PROMPT pills too, so you can still open prompt details while sorting.
- Step 2 prompt evaluation now keeps prompts that depend on unresolved variables, which fixes cases like MultiKits dropping interactive prompts.
- Saved prompt answers are now stored with Auto off by default. You have to enable them yourself in Step 5 if you want BIO to reuse them.
- Added a Step 3 hint so it is more obvious that rows support right-click actions.
