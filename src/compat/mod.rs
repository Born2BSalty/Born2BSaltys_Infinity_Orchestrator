// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

pub mod model;
pub mod tp2_parse;
pub mod validator;

pub use model::Tp2Metadata;
pub use tp2_parse::parse_tp2_rules;
pub use validator::{CompatValidator, SelectedComponent};
