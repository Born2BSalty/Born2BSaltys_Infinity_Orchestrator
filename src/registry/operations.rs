// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// `operations` — registry CRUD entry points (`create_modlist`,
// `rename_modlist`, `delete_modlist`, `flip_to_installed`, etc.).
//
// **Phase 3 stub.** The module is declared here so visibility is stable across
// phases; Phase 5 populates it with real CRUD functions. Per H6 these
// operations are plain `pub fn` taking `&mut ModlistRegistry +
// &RegistryStore` — no write-guard wrapper.
//
// SPEC: §13.1.
