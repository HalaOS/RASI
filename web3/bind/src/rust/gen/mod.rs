use proc_macro2::TokenStream;
/// The rust language generator for `Ethbind`
#[derive(Debug, Default)]
pub struct RustGenerator {
    contracts: Vec<ContractGenerator>,
}

impl RustGenerator {
    /// Push new contract generator to back end of generation list
    pub(crate) fn new_contract(&mut self, name: &str) {
        self.contracts.push(ContractGenerator::new(name))
    }

    /// Returns contract generator at back edn of generation list.
    pub(crate) fn current_contract(&mut self) -> &mut ContractGenerator {
        self.contracts.last_mut().expect("Call new_contract first")
    }

    pub(crate) fn to_runtime_type_token_stream<R: RuntimeBinder>(
        &self,
        runtime_binder: &mut R,
        name: &str,
    ) -> anyhow::Result<TokenStream> {
        Ok(runtime_binder
            .get(name)?
            .parse()
            .map_err(|e| anyhow::format_err!("{}", e))?)
    }
}

mod generator;

mod function;

mod contract;
use contract::*;

mod event;

use crate::bind::RuntimeBinder;
