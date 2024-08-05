use std::path::PathBuf;

use ethbind::rustgen::bind_hardhat_artifact;

#[test]
fn gen_rust_code() {
    let root_dir: PathBuf = env!("CARGO_MANIFEST_DIR").parse().unwrap();

    let tmp_dir: PathBuf = env!("CARGO_TARGET_TMPDIR").parse().unwrap();

    bind_hardhat_artifact(
        root_dir.join("src/abi.json"),
        root_dir.join("tests/mapping.json"),
        tmp_dir,
    )
    .unwrap();
}
