use std::{env, fs::read_to_string, path::PathBuf};

use ethbind::{
    binder::{bind, BinderContext},
    mapping::BinderTypeMapping,
    rustgen::RustBinder,
    typedef::{AbiField, HardhatArtifact},
};
use heck::ToSnekCase;
use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{parse::Parse, parse_macro_input, Ident, LitStr, Token};

struct Contract {
    pub contract_name: Option<String>,
    pub abi_data: String,
}

impl Parse for Contract {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let contract_name: Option<Ident> = input.parse()?;

        if contract_name.is_some() {
            input.parse::<Token!(,)>()?;
        }

        let abi_data: LitStr = input.parse()?;

        Ok(Self {
            contract_name: contract_name.map(|c| c.to_string()),

            abi_data: abi_data.value(),
        })
    }
}

fn load_json_file(path: &str) -> String {
    let dir = env::var("CARGO_MANIFEST_DIR").expect("Find CARGO_MANIFEST_DIR");

    let path = PathBuf::from(dir).join(path);

    read_to_string(path.clone()).expect(&format!("Read json file: {:?}", path))
}

#[proc_macro]
pub fn contract(item: TokenStream) -> TokenStream {
    let contract = parse_macro_input!(item as Contract);

    let mapping: BinderTypeMapping =
        serde_json::from_str(include_str!("./mapping.json")).expect("Parse mapping file.");

    let abi_data = load_json_file(&contract.abi_data);

    if let Some(contract_name) = contract.contract_name {
        let abi: Vec<AbiField> = serde_json::from_str(&abi_data).expect("Load abi data");

        let cx = BinderContext::new(&contract_name, &abi, None);

        let token_stream = bind(&cx, RustBinder::new(mapping)).expect("Generate rust code");

        let mod_name = format_ident!("{}", contract_name.to_snek_case());

        quote! {
            pub mod #mod_name {
                #token_stream
            }
        }
        .into()
    } else {
        let abi: HardhatArtifact = serde_json::from_str(&abi_data).expect("Load abi data");

        let cx = BinderContext::new(&abi.contract_name, &abi.abi, Some(&abi.bytecode));

        let mod_name = format_ident!("{}", abi.contract_name.to_snek_case());

        let token_stream = bind(&cx, RustBinder::new(mapping)).expect("Generate rust code");

        quote! {
            pub mod #mod_name {
                #token_stream
            }
        }
        .into()
    }
}
