use std::{env, fs, path::PathBuf, process::Command};

use sha3::{Digest, Keccak256};

use crate::bind::Contract;

/// The trait to support `Rust` language formatting
pub trait RustPretty {
    /// Invoke this `fn` to perform the `formatting codes action`
    fn pretty(&mut self) -> anyhow::Result<()>;
}

impl RustPretty for Contract {
    fn pretty(&mut self) -> anyhow::Result<()> {
        let rust_fmt_path =
            PathBuf::from(env::var("CARGO_HOME").expect("Get CARGO_HOME")).join("bin/rustfmt");

        for file in &mut self.files {
            let temp_file_name = format!(
                "{:x}",
                Keccak256::new()
                    .chain_update(file.data.as_bytes())
                    .finalize()
            );

            let path = env::temp_dir().join(temp_file_name);

            fs::write(&path, file.data.as_bytes())?;

            // Call rustfmt to fmt tmp file
            let mut child = Command::new(&rust_fmt_path)
                .args(["--edition", "2021", path.to_str().unwrap()])
                .spawn()?;

            child.wait()?;

            let formated = fs::read_to_string(path).expect("Read formated codes");

            file.data = formated;
        }

        Ok(())
    }
}

impl RustPretty for Vec<Contract> {
    fn pretty(&mut self) -> anyhow::Result<()> {
        for c in self {
            c.pretty()?;
        }

        Ok(())
    }
}
