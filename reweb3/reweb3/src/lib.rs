#![cfg_attr(docsrs, feature(doc_cfg))]

#[cfg(feature = "abi")]
#[cfg_attr(docsrs, doc(cfg(feature = "abi")))]
pub mod abi;

pub mod eip;

pub mod errors;

pub mod primitives;

#[cfg(feature = "signer")]
#[cfg_attr(docsrs, doc(cfg(feature = "signer")))]
pub mod signer;

#[cfg(feature = "wallet")]
#[cfg_attr(docsrs, doc(cfg(feature = "wallet")))]
pub mod wallet;

#[cfg(feature = "clients")]
#[cfg_attr(docsrs, doc(cfg(feature = "clients")))]
pub mod clients;

#[cfg(feature = "rlp")]
#[cfg_attr(docsrs, doc(cfg(feature = "rlp")))]
pub mod rlp;

#[cfg(feature = "hardhat")]
#[cfg_attr(docsrs, doc(cfg(feature = "hardhat")))]
pub mod hardhat;

#[cfg(feature = "macros")]
#[doc(hidden)]
pub mod macros;

/// Reexport runtimes types.
pub mod prelude {
    pub use super::primitives::{balance::TransferOptions, *};

    #[cfg(feature = "abi")]
    pub use super::abi::{from_abi, to_abi};

    #[cfg(feature = "signer")]
    pub use super::signer::SignerWithProvider;

    #[cfg(feature = "clients")]
    pub use super::clients::*;
}

#[cfg(feature = "bind")]
#[cfg_attr(docsrs, doc(cfg(feature = "bind")))]
pub mod bind {
    use std::{fs, path::Path};

    use ethbind::{
        binder::BinderContext,
        mapping::BinderTypeMapping,
        rustgen::write_file,
        typedef::{AbiField, HardhatArtifact},
    };
    use heck::ToSnekCase;
    use quote::{format_ident, quote};

    pub fn bind_hardhat_artifacts<'a, A: IntoIterator<Item = &'a str>, T: AsRef<Path>>(
        artifacts: A,
        output_dir: T,
    ) -> std::io::Result<()> {
        let mut mods = vec![];

        for artifact in artifacts {
            let contract_name = format_ident!(
                "{}",
                bind_hardhat_artifact(artifact, output_dir.as_ref())?.to_snek_case()
            );

            mods.push(quote! {
                pub mod #contract_name;
            });
        }

        fs::write(
            output_dir.as_ref().join("mod.rs"),
            quote! {
                #(#mods)*
            }
            .to_string(),
        )
    }

    /// Create contract mod from hardhat artifact file.
    pub fn bind_hardhat_artifact<T: AsRef<Path>>(
        artifact: &str,
        output_dir: T,
    ) -> std::io::Result<String> {
        let hardhat: HardhatArtifact = serde_json::from_str(artifact).unwrap();

        let cx = BinderContext::new(
            &hardhat.contract_name,
            &hardhat.abi,
            Some(&hardhat.bytecode),
        );

        let mapping: BinderTypeMapping =
            serde_json::from_str(include_str!("../../macros/src/mapping.json")).unwrap();

        if !output_dir.as_ref().exists() {
            fs::create_dir_all(output_dir.as_ref())?;
        }

        let target = output_dir
            .as_ref()
            .join(hardhat.contract_name.to_snek_case() + ".rs");

        write_file(cx, mapping, target)?;

        Ok(hardhat.contract_name)
    }

    /// Create contract mod from solidity abi file.
    pub fn bind_contract<H: AsRef<Path>, M: AsRef<Path>, T: AsRef<Path>>(
        contract_name: &str,
        artifact_path: H,
        output_dir: T,
    ) -> std::io::Result<()> {
        let abi: Vec<AbiField> =
            serde_json::from_slice(fs::read(artifact_path.as_ref())?.as_slice()).unwrap();

        let cx = BinderContext::new(&contract_name, &abi, None);

        let mapping: BinderTypeMapping =
            serde_json::from_str(include_str!("../../macros/src/mapping.json")).unwrap();

        if !output_dir.as_ref().exists() {
            fs::create_dir_all(output_dir.as_ref())?;
        }

        let target = output_dir
            .as_ref()
            .join(contract_name.to_snek_case() + ".rs");

        write_file(cx, mapping, target)
    }
}
