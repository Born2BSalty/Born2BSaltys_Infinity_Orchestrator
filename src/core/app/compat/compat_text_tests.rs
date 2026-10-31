use super::{decode_compat_text, read_compat_text};

#[test]
fn preserves_utf8_and_strips_only_initial_bom() {
    for text in ["", "ASCII\nnext", "Café 日本語\n", "x\u{feff}y"] {
        assert_eq!(decode_compat_text(text.as_bytes()).expect("UTF8"), text);
        let mut bytes = vec![0xef, 0xbb, 0xbf];
        bytes.extend_from_slice(text.as_bytes());
        assert_eq!(decode_compat_text(&bytes).expect("UTF8 BOM"), text);
    }
}

#[test]
fn preserves_each_legacy_byte_and_physical_lines() {
    assert_eq!(
        decode_compat_text(b"BEGIN ~\x80\xe9~\r\nNEXT\n").expect("legacy"),
        "BEGIN ~\u{80}é~\r\nNEXT\n"
    );
    assert_eq!(
        decode_compat_text(&[0xef, 0xbb, 0xbf, 0xe9]).expect("malformed UTF8 BOM fallback"),
        "é"
    );
}

#[test]
fn decodes_both_explicit_utf16_orders_strictly() {
    let text = "BEGIN ~Café 😀~\r\nNEXT\n";
    for little in [true, false] {
        let mut bytes = if little {
            vec![0xff, 0xfe]
        } else {
            vec![0xfe, 0xff]
        };
        for unit in text.encode_utf16() {
            bytes.extend_from_slice(&if little {
                unit.to_le_bytes()
            } else {
                unit.to_be_bytes()
            });
        }
        assert_eq!(decode_compat_text(&bytes).expect("UTF16"), text);
    }
}

#[test]
fn rejects_malformed_declared_unicode() {
    for bytes in [
        &[0xff, 0xfe, 0x41][..],
        &[0xfe, 0xff, 0xd8, 0],
        &[0xff, 0xfe, 0, 0],
        &[0, 0, 0xfe, 0xff],
    ] {
        assert!(decode_compat_text(bytes).is_err(), "{bytes:?}");
    }
    let missing = std::env::temp_dir()
        .join(format!("bio-no-such-compat-{}", std::process::id()))
        .join("missing.tp2");
    assert!(read_compat_text(&missing).is_err());
}

#[test]
fn preserves_binary_literals_comments_and_inline_payloads() {
    for bytes in [
        b"~f9\x83\x10G\0\0t%~".as_slice(),
        b"// NUL\0 control\x10\n",
        b"<<<<<<<< payload\n\0\x10\n>>>>>>>>\n",
    ] {
        let expected = bytes.iter().copied().map(char::from).collect::<String>();
        assert_eq!(
            decode_compat_text(bytes).expect("preserved payload"),
            expected
        );
    }
    assert_eq!(
        decode_compat_text(b"valid UTF8\0\x10").expect("valid UTF8 controls"),
        "valid UTF8\0\u{10}"
    );
}
