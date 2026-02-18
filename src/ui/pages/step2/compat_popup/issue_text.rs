// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

mod copy;
mod explain;
mod helpers;
mod kind;

pub(super) use copy::format_issue_for_copy;
pub(super) use explain::{issue_reason, issue_verdict, issue_what_to_do, issue_why_this_appears};
pub(super) use kind::human_kind;
