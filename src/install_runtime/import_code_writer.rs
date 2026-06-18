// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::fs;
use std::io;
use std::path::Path;

pub const IMPORT_CODE_FILENAME: &str = "modlist-import-code.txt";

pub fn write_modlist_import_code_txt(
    install_destination: &Path,
    share_code: &str,
) -> io::Result<()> {
    fs::create_dir_all(install_destination)?;
    let path = install_destination.join(IMPORT_CODE_FILENAME);

    fs::write(path, share_code.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dest(tag: &str) -> std::path::PathBuf {
        let mut p = std::env::temp_dir();
        p.push(format!(
            "bio_import_code_test_{}_{}_{}",
            tag,
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        p
    }

    #[test]
    fn writes_file_into_a_not_yet_existing_destination() {
        let dest = temp_dest("fresh");
        assert!(!dest.exists(), "precondition: destination does not exist");
        write_modlist_import_code_txt(&dest, "BIO-MODLIST-V1:abc").expect("write");
        let written =
            fs::read_to_string(dest.join(IMPORT_CODE_FILENAME)).expect("read back the file");
        assert_eq!(written, "BIO-MODLIST-V1:abc");
        let _ = fs::remove_dir_all(&dest);
    }

    #[test]
    fn overwrites_an_existing_file() {
        let dest = temp_dest("overwrite");
        fs::create_dir_all(&dest).unwrap();
        let path = dest.join(IMPORT_CODE_FILENAME);
        fs::write(&path, "OLD-CODE-from-a-cancelled-attempt").unwrap();

        write_modlist_import_code_txt(&dest, "BIO-MODLIST-V1:new").expect("overwrite");
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            "BIO-MODLIST-V1:new",
            "the previous (cancelled-attempt) file must be discarded"
        );
        let _ = fs::remove_dir_all(&dest);
    }

    #[test]
    fn idempotent_when_destination_already_exists() {
        let dest = temp_dest("idem");
        fs::create_dir_all(&dest).unwrap();
        write_modlist_import_code_txt(&dest, "X").expect("first write");
        write_modlist_import_code_txt(&dest, "X").expect("second write — create_dir_all is Ok");
        let _ = fs::remove_dir_all(&dest);
    }
}
