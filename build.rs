// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::env;
use std::fs;
use std::path::Path;

fn write_gallery_snapshot() {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR is set");
    let out_dir = env::var("OUT_DIR").expect("OUT_DIR is set");
    let gallery_dir = Path::new(&manifest_dir).join("gallery");

    println!("cargo:rerun-if-changed=gallery");

    let mut lines = vec!["pub(crate) static SNAPSHOT_FILES: &[(&str, &[u8])] = &[".to_string()];

    let mut subfolders: Vec<_> = fs::read_dir(&gallery_dir)
        .expect("gallery directory is readable")
        .filter_map(Result::ok)
        .filter(|dir_entry| dir_entry.path().is_dir())
        .filter(|dir_entry| !dir_entry.file_name().to_string_lossy().starts_with('.'))
        .collect();
    subfolders.sort_by_key(std::fs::DirEntry::file_name);

    for dir_entry in subfolders {
        let id = dir_entry.file_name().to_string_lossy().into_owned();
        let folder_path = dir_entry.path();

        for file_name in ["entry.json", "modlist.biolist", "cover.png"] {
            let file_path = folder_path.join(file_name);
            if !file_path.is_file() {
                continue;
            }
            println!("cargo:rerun-if-changed={}", file_path.display());
            let absolute_path = file_path.canonicalize().unwrap_or_else(|err| {
                panic!("failed to canonicalize {}: {err}", file_path.display())
            });
            let absolute_path_text = absolute_path.to_string_lossy().into_owned();
            assert!(
                !absolute_path_text.contains('"'),
                "gallery file path contains a double quote: {absolute_path_text}"
            );
            lines.push(format!(
                "    (\"{id}/{file_name}\", include_bytes!(r\"{absolute_path_text}\")),"
            ));
        }
    }

    lines.push("];".to_string());

    let snapshot_path = Path::new(&out_dir).join("gallery_snapshot.rs");
    fs::write(&snapshot_path, lines.join("\n") + "\n")
        .unwrap_or_else(|err| panic!("failed to write {}: {err}", snapshot_path.display()));
}

fn main() {
    #[cfg(target_os = "windows")]
    {
        println!("cargo:rerun-if-changed=assets/icon.ico");
        let mut res = winres::WindowsResource::new();
        res.set_icon("assets/icon.ico");
        if let Err(err) = res.compile() {
            panic!("failed to compile windows resources: {err}");
        }
    }

    write_gallery_snapshot();
}
