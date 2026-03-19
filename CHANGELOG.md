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
- Test PR rules.

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
