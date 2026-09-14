use nmlt_runtime::{identity, sha256};
use std::fs;
use std::path::PathBuf;

fn directory() -> PathBuf {
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!(
        "../../target/tool-identity-{}-{stamp}",
        std::process::id()
    ));
    fs::create_dir_all(path.join("lib")).unwrap();
    path
}
#[test]
fn streamed_hashes_match_existing_identity_and_detect_tree_changes() {
    let root = directory();
    let data: Vec<_> = (0..131_083).map(|i| (i % 251) as u8).collect();
    fs::write(root.join("lib/tool"), &data).unwrap();
    assert_eq!(
        identity::file(&root.join("lib/tool"), 200_000).unwrap(),
        (data.len() as u64, sha256(&data))
    );
    assert!(identity::file(&root.join("lib/tool"), 100).is_err());
    let before = identity::tree(&root, &["lib"]).unwrap();
    fs::write(root.join("lib/tool"), b"changed").unwrap();
    assert_ne!(identity::tree(&root, &["lib"]).unwrap(), before);
    assert!(identity::tree(&root, &["../"]).is_err());
}

#[test]
fn sha256_known_answers_and_stream_boundaries_remain_byte_exact() {
    let root = directory();
    let path = root.join("lib/vector");
    for (bytes, expected) in [
        (
            b"".as_slice(),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
        ),
        (
            b"abc".as_slice(),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
        ),
        (
            b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq".as_slice(),
            "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1",
        ),
    ] {
        fs::write(&path, bytes).unwrap();
        assert_eq!(sha256(bytes), expected);
        assert_eq!(
            identity::file(&path, bytes.len() as u64).unwrap(),
            (bytes.len() as u64, expected.to_owned())
        );
    }
    for length in [55, 56, 63, 64, 65, 65_535, 65_536, 65_537, 131_083] {
        let bytes: Vec<u8> = (0..length).map(|i| (i % 251) as u8).collect();
        fs::write(&path, &bytes).unwrap();
        let (observed, digest) = identity::file(&path, length as u64).unwrap();
        assert_eq!(observed, length as u64);
        // The existing implementation is independent of the sha2 crate.
        assert_eq!(digest, sha256(&bytes));
        assert_eq!(digest.len(), 64);
        assert!(
            digest
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        );
    }
}
