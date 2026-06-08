# SPEC — Per-Modlist Download-Source & Installed-Refs Ownership

**Status:** Approved (user decisions locked 2026-06-08). Focused sub-spec; subordinate to `SPEC.md` and the wireframe under the same authority order. The CRITICAL DIRECTIVE in `SPEC.md §1` governs.
**Scope:** Phase-8 follow-on, landing as **PR-1** (the correctness core). The version-mismatch / extracted-version work is explicitly **deferred to a later PR-2** (see §11).

This sub-spec describes **what changes** and **why** for per-modlist ownership of mod download sources **and the installed-refs record** (§3A, added 2026-06-08). The agreed technical decisions (sparse three-tier overlay, ambient active-modlist resolution, the Set-Source destination dropdown, full-resolved-set export, import-into-the-modlist-file, and the symmetric per-modlist installed-refs file) are part of the **what** — they are settled product behavior, not implementation latitude. Line-level edits, file inventories, and carve-out citations are the **plan's** job, not this document's.

---

## 1. Problem & why

A mod's chosen **download source and its pinned exact version** — the `tag` / `commit` / `branch` / `exact_github` selectors inside each source block — are stored in a **single global file**, `mod_downloads_user.toml`, in the OS config directory. That file is shared across every modlist. There is no per-modlist scoping of source *content*, and the file is re-read from disk on every resolution (no cache), so the last write wins globally.

This is a cross-modlist **bleed**. Confirmed in code, it manifests in three places:

1. **Export bakes the wrong version into share codes (primary, user-facing).** The share exporter copies the **entire** global `mod_downloads_user.toml` verbatim into the share code, with no modlist scoping. So whichever version the user last pinned *anywhere* is what ships to recipients — regardless of which modlist is being exported. A user who pins CDTweaks v16 for Modlist A, then pins Master/v19 for Modlist B, then exports Modlist A, ships a code that silently installs **v19** for everyone who uses it — possibly breaking their game.
2. **Manual workspace resolution bleeds.** When the user runs an update-check or download while editing a modlist, resolution reads the global file directly. If the global file currently holds another modlist's pin, this modlist picks it up.
3. **Import clobbers the user's global defaults.** Importing any share code overwrites the importer's global `mod_downloads_user.toml`, destroying their own pinned defaults.

The **same three-way bleed** (verbatim export, global clobber on import, shared resolution) also affects a **second** global file — `mod_installed_refs.toml`, which records the installed source-id + ref per mod. §3A extends this fix to it (user-confirmed 2026-06-08).

**Already correct (not in scope to change):** *which source variant* is selected per mod is **already** per-modlist (persisted in each modlist's `workspace.json`). Only the source *content* — the pin — bleeds. This feature scopes the content, composing with the already-per-modlist selection.

**Why it matters.** Reproducibility is a core product principle (`SPEC.md §1.1`): "Every modlist can be exported as a share code that recreates the entire workspace on another machine." A share code that silently pins the wrong mod version violates that guarantee at its most damaging point — a third party's install.

---

## 2. Goals & non-goals

### Goals (PR-1)

1. A mod's pinned source can be owned at the **modlist level**, independently of the user's global default.
2. A single **three-tier precedence** governs version/source resolution **everywhere a version is resolved** — manual workspace update-check/download, the install pipeline, and export: **modlist override → global user default → app default**.
3. **Export bakes the versions *this modlist* resolves to** (the full resolved set for the modlist's mods), not a verbatim copy of any global file.
4. **Import lands pins in the imported modlist's own file**, leaving the user's global default untouched.
5. The legacy standalone `BIO_legacy` binary behaves **byte-for-byte as today**.
6. The **installed-refs** record (`mod_installed_refs.toml`) is owned per-modlist too (§3A): each modlist records, exports, and imports its own installed source-ids + refs; the legacy binary is unchanged.

### Non-goals (deferred to PR-2 — see §11)

- Detecting and surfacing a mismatch between a mod's **pinned** version and the version **currently downloaded + extracted** in the modlist's mods folder.
- The per-mod "download the pinned version and re-extract into this modlist's mods folder (deleting the old extract first)" remediation action.
- Any new per-modlist tracking of the **extracted** version on disk.

These depend on new per-modlist extracted-state tracking and are a separate plan. PR-1 must not implement them, but must not architecturally preclude them.

---

## 3. The data model — three tiers, sparse overrides

Resolution layers three files, lowest-priority first, **merged field-by-field per mod+source**:

| Tier | File | Role | Priority |
|------|------|------|----------|
| App default | `mod_downloads_default.toml` (ships with the app, config dir) | Built-in source catalog. Unchanged. | Lowest |
| Global user default | `mod_downloads_user.toml` (config dir) | "My default for anything I work on." Unchanged location and role. | Middle |
| **Per-modlist override (NEW)** | `modlists/<id>/mod_downloads_user.toml` | This modlist's own pins. **Same schema** as the global file. | Highest |

- The per-modlist file is **sparse**: it holds only the mods the user has explicitly pinned *for this modlist*. Every other mod falls through to the global tier and then the app-default tier. No duplication of un-overridden mods.
- The merge is **field-wise per source block**: a per-modlist block that sets only the version selector (e.g. `tag = "v16"`) overrides **only** that field and inherits the URL / repo / label / everything else from the lower tiers. This is the natural extension of the loader's existing default→user field-wise overlay, with the per-modlist file applied as a third pass on top.
- The per-modlist file is part of that modlist's owned data under `modlists/<id>/`, alongside `workspace.json`.

---

## 3A. Installed-refs — the second per-modlist file (added 2026-06-08, user-confirmed)

The download pins (§3) are not the only global file that bleeds. **`mod_installed_refs.toml`** (config dir) records, per mod, the **installed source-id** (which variant was installed) and the **installed ref** (the exact commit/tag installed). It is global and exported/imported verbatim, so it bleeds exactly like the pins. User-confirmed 2026-06-08 to bring it into per-modlist ownership.

- **Model — full per-modlist replacement (no overlay).** Unlike the pins (which layer over an app-default catalog), installed-refs are per-install *facts* with no catalog to merge. So `modlists/<id>/mod_installed_refs.toml` **fully owns** the active modlist's installed-refs — there is no three-tier merge. The same ambient active-modlist context (§4) locates it.
- **One seam.** Every load, save, prune, export-read, and import-write of installed-refs routes through a single path resolver. Making that resolver ambient-aware flips the whole file per-modlist at once: a modlist active ⇒ all installed-refs I/O targets `modlists/<id>/mod_installed_refs.toml`; none active (legacy `BIO_legacy`) ⇒ the global file, unchanged. **No read-fallback to global** — an active modlist sees only its own installed-refs (empty until it records its own), which is what guarantees zero bleed.
- **Install-time writes — concurrency requirement.** Installed-refs are written by the install pipeline as it records what got installed, including from the **parallel-extract worker pool** (carve-out #7) — i.e. **off the main thread**. The active-modlist context must therefore stay **pinned to the installing modlist for the whole install** so these writes target the right file; the ambient must not be cleared or switched mid-install. (If the plan finds reading the process-global from a worker thread unsafe, the install captures the target path at start and the workers write to that captured path — same effect.) This is a required correctness invariant.
- **Export.** The exported installed-refs payload reflects **this modlist's** installed state — the source-id map from the modlist's own `selected_source_ids` (already per-modlist) and the refs from its per-modlist `mod_installed_refs.toml` — **not** a verbatim copy of the global file. Gated on an active modlist exactly like the pin export (§6); legacy export byte-identical to today.
- **Import.** The baked installed-refs are written to the **new modlist's** per-modlist file (same ambient redirect), leaving the importer's global file byte-unchanged.
- **Migration (existing modlists).** A modlist installed *before* this feature has its refs in the global file and no per-modlist file yet. With no read-fallback, it starts with empty installed-refs and **rebuilds them on its next install / update-check** (which writes its per-modlist file). This is benign: export version-correctness rides on the download resolved-set (§6), and the exported source-id map is reconstructed from the modlist's per-modlist `selected_source_ids`. The global installed-refs file is left intact (the legacy binary still uses it); it simply stops being read once a modlist is active.
- **Legacy.** No active modlist ⇒ all installed-refs I/O targets the global file exactly as today, byte-for-byte unchanged.

---

## 4. Reads — modlist-aware resolution via an ambient active modlist

Every version/source resolution must apply the three-tier precedence of §3. This includes the manual workspace update-check/download path, the install pipeline, and export (§6).

- Resolution becomes modlist-aware via an **ambient "active modlist"** that the orchestrator sets — fed from the active-modlist identity the orchestrator already tracks. The resolver consults it to locate and apply the per-modlist overlay file.
- **When the ambient is unset, behavior is exactly today's two-tier resolution** (app default → global user). The legacy `BIO_legacy` binary never sets it, so its resolution is unchanged.
- The orchestrator **sets the ambient when a modlist becomes active and clears it when none is** — at minimum: workspace open, install start, and return to Home. The ambient must be refreshed on every modlist switch; a stale ambient would re-introduce the bleed. Keeping it current is the orchestrator's responsibility and a required invariant of this design.

(Decision: ambient over threading an explicit modlist argument through every resolution call site — chosen 2026-06-08 to keep the protected-source edit surface minimal. The correctness obligation is the set/clear discipline above.)

---

## 5. Writes — the Set-Source destination dropdown

The Updates popup's per-mod "Edit Source" affordance becomes a **dropdown** offering two write destinations:

- **"My default"** → writes the **global** `mod_downloads_user.toml` (today's exact behavior and path).
- **"For this modlist"** → writes the **per-modlist** `modlists/<id>/mod_downloads_user.toml`.

- The source-editor flow is **otherwise unchanged**: the same TOML source-block editor opens (pre-filled with the currently resolved source for the chosen destination's tier — see below), and on **Save** it writes the edited block to the destination file the dropdown selected. The destination choice rides from the dropdown (at open) through to the Save.
- **Pre-fill matches the destination's tier.** "My default" pre-fills the value resolved **without** the per-modlist overlay (the true global / app-default value), so editing and saving "My default" never silently promotes a per-modlist pin into the global file. "For this modlist" pre-fills the full three-tier resolved value. Each destination shows what it currently effectively holds.
- Both destinations reuse the **existing source-block writer** (read file → text-block replace/append → validate → write), pointed at the chosen path. No second serializer is introduced.
- Confirmation: writing "For this modlist" must **not** modify the global file, and writing "My default" must **not** modify any per-modlist file.

---

## 6. Export — bake the full resolved set, modlist-scoped

- The exporter must **resolve each mod in the modlist through the three-tier precedence (§3–§4) and bake the resulting source definitions** — the **full resolved set** for this modlist's mods — into the share code. (Decision: full resolved set over only-differs-from-default — chosen 2026-06-08 for correctness robustness.)
- For a modlist export, it must **not** copy any single tier's file verbatim. The exported pins reflect what *this* modlist installs.
- **Scope — active-modlist export only.** This governs export **with an active modlist** (the redesign workspace / install-start path). With **no active modlist** (the legacy `BIO_legacy` export), the exporter preserves today's behavior unchanged per §8 — consistent with §9's rule that every carve-out edit is inert on the unset-ambient path. Concretely, the resolved-set bake is **gated on an active modlist**; with none set, the exporter's current behavior stands. §10.6's "byte-identical" criterion applies to the legacy (no-active-modlist) path; an active-modlist export intentionally produces the resolved-set payload of §6 rather than the legacy verbatim copy.
- The companion **installed-refs** payload the exporter carries is built from **this modlist's** own data (its per-modlist `selected_source_ids` for the source-id map + its per-modlist `mod_installed_refs.toml` for the refs), not a verbatim copy of the global file — see §3A. Gated on an active modlist exactly like the pin export.
- Result: a share code carries exactly the versions this modlist resolves to — independent of the global file's current contents.

---

## 7. Import — land pins in the modlist's file

- Importing a share code writes the baked source pins into the **target (new) modlist's per-modlist file** (`modlists/<id>/mod_downloads_user.toml`), **not** the global `mod_downloads_user.toml`.
- The importer's **global default file is left byte-unchanged**. This eliminates the "paste someone's code, lose your own defaults" bleed (Problem #3) as a direct consequence of per-modlist ownership.
- The imported modlist is thereby self-contained: its pins live with it and resolve at top priority when it is the active modlist.
- The imported **installed-refs** are likewise written to the new modlist's per-modlist `mod_installed_refs.toml` (§3A), leaving the importer's global installed-refs file byte-unchanged.

---

## 8. Behavior preservation & the legacy binary

- **Legacy `BIO_legacy`:** no active modlist is ever set, so the resolver applies only app-default → global (today's two tiers), the source editor writes only the global file, and export/import behave as today. This must be **byte-for-byte unchanged** and is a verification gate.
- **Redesign with no per-modlist override yet:** a modlist that has never used "For this modlist" has no per-modlist file; resolution falls through to global → app default — i.e. current behavior — until the user pins something for it.
- Every protected-source edit this feature requires is **additive and inert on the unset-ambient / no-override path**, preserving the above.

---

## 9. Directive impact — carve-out #16

Every point that resolves, stores, edits, exports, or imports a mod source/version lives in **protected BIO source**. Per-modlist ownership therefore requires editing protected source, which is only legal under an authorized carve-out (`SPEC.md §1` CRITICAL DIRECTIVE). This sub-spec **authorizes carve-out #16 — per-modlist download-source resolution**, with the following tightly-scoped functional surface (the plan enumerates exact files, functions, and line-level edits and cites them per the directive's documentation requirement):

1. A **third overlay pass** in the source loader, keyed off the ambient active-modlist override path, applied on top of the existing default→user merge. The existing two-tier merge structure is preserved; the third pass is additive and skipped when the ambient is unset.
2. A **target-path capability** on the source-block load/save so a write can target the per-modlist file instead of the global file. The global path remains the default when no per-modlist destination is requested.
3. The **ambient active-modlist setter** the loader consults (orchestrator-set; unset for the legacy binary).
4. The **Set-Source destination dropdown** in the Updates popup row, plus carrying the chosen destination through the source-editor open→save (action + editor state).
5. **Export** resolving each modlist mod through the three-tier precedence and baking the full resolved set (replacing the verbatim global-file copy), scoped to the modlist; and constructing the installed-refs payload from this modlist's own data (§3A).
6. **Import** writing the baked pins into the modlist's per-modlist file rather than the global file — and likewise the installed-refs.
7. **The installed-refs path resolver made ambient-aware** (§3A): its single load / save / prune / export-read / import-write seam targets the per-modlist `mod_installed_refs.toml` when a modlist is active and the global file otherwise.
8. **Install-time ambient pinning** so the parallel-extract workers' installed-refs writes target the installing modlist (§3A concurrency requirement); if the plan finds the process-global unsafe to read off-thread, capturing the target path at install start instead.

Each edit must be **additive and behavior-preserving on the unset-ambient / no-override path** (§8). Anything that changes legacy behavior, or that exceeds what is needed for the eight items above, is out of scope and a `SPEC CONFLICT`.

(The directive currently enumerates authorized carve-outs through #15; confirm #16 is the next free number against the live `SPEC.md §1` when formalizing. The carve-out is formalized into `SPEC.md §1` as part of this work's doc-sync.)

---

## 10. Acceptance criteria

1. **Export is modlist-scoped.** Pin CDTweaks v16 "For this modlist" on Modlist A and Master/v19 "For this modlist" on Modlist B; exporting A yields a code carrying **v16**, exporting B yields **v19** — regardless of write order.
2. **"For this modlist" does not touch global.** After a "For this modlist" save, the global `mod_downloads_user.toml` is byte-unchanged; the modlist's per-modlist file contains the new block.
3. **"My default" still writes global.** After a "My default" save, the global file is updated (today's behavior) and no per-modlist file is written.
4. **Precedence + fall-through.** A modlist with no per-modlist override resolves identically to today (global → app default). With an override, the override wins field-wise; un-overridden fields inherit from lower tiers.
5. **Import isolation.** Importing a code into a new modlist writes that modlist's per-modlist file and leaves the global file byte-unchanged.
6. **Legacy unchanged.** `BIO_legacy` source read/write/export/import behavior for `mod_downloads_user.toml` is byte-identical to today (no active modlist set).
7. **Resolution coverage.** Both install-time resolution and manual workspace update-check resolution honor the three-tier precedence for the active modlist.
8. **Installed-refs isolation.** Installing mods under Modlist A records their installed source-id + ref in A's per-modlist `mod_installed_refs.toml`, not the global file; installing a different version under Modlist B does not change A's installed-refs. Exporting A carries A's installed-refs; importing into a new modlist writes that modlist's installed-refs file; the global file is byte-unchanged in both. The installed-refs writes survive the parallel-extract worker pool (ambient pinned for the install). Legacy `BIO_legacy` installed-refs I/O is byte-identical to today.

---

## 11. Deferred — PR-2 (version-mismatch detection)

Out of scope for PR-1, recorded so the plan does not pull it in:

- A "version mismatch" surface that flags mods whose **pinned** source/version differs from what is **currently downloaded + extracted** in this modlist's mods folder.
- A per-mod remediation button: download the pinned version and re-extract it into the modlist's mods folder, deleting the stale extract first (a destructive, single-mod download+extract — data-loss-class, to be designed carefully).
- The new per-modlist tracking of extracted-on-disk versions that mismatch-detection requires.

PR-1 establishes the per-modlist source-of-truth (the pin) that PR-2 will compare against on-disk state. PR-1 must leave that door open but build none of it.
