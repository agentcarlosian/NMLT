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
