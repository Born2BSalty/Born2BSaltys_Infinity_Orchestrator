// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

mod cleanup;
mod clone_ops;
mod keys;
mod repair;
mod visibility;

pub use cleanup::{merge_adjacent_same_mod_blocks, prune_empty_parent_blocks};
pub use clone_ops::clone_parent_empty_block;
pub use keys::step3_item_key;
pub use repair::repair_orphan_children;
pub use visibility::{block_indices, count_children_in_block, visible_indices};
