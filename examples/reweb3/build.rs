use reweb3::bind;

fn main() {
    println!("cargo:rerun-if-changed=abi.json");

    bind::bind_hardhat_artifacts([include_str!("./abi.json")], "src/contracts")
        .expect("bind hardhat artifact");
}
