// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::path::PathBuf;

fn run_check(root: &std::path::Path) {
    match bio::gallery_feed::folder::check_folder(root) {
        Ok(count) => println!("ok: {count} entries"),
        Err(errors) => {
            for error in &errors {
                eprintln!("{error}");
            }
            std::process::exit(1);
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if let (Some("check"), Some(root)) = (args.get(1).map(String::as_str), args.get(2)) {
        run_check(&PathBuf::from(root));
    } else {
        eprintln!("gallery-index check <folder>");
        std::process::exit(2);
    }
}
