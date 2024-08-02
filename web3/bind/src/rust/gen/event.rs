use crate::{abi::Parameter, bind::RuntimeBinder};
use heck::ToSnakeCase;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use super::RustGenerator;

impl RustGenerator {
    pub(crate) fn to_event_field_list<R: RuntimeBinder>(
        &self,
        runtime_binder: &mut R,
        params: &[Parameter],
    ) -> anyhow::Result<Vec<TokenStream>> {
        let mut token_streams = vec![];

        for (index, param) in params.iter().enumerate() {
            let type_ident = self.to_rust_type(runtime_binder, param)?;

            let var_ident = if param.name != "" {
                format_ident!("{}", param.name.to_snake_case())
            } else {
                format_ident!("p{}", index)
            };

            token_streams.push(quote!(#var_ident: #type_ident));
        }

        Ok(token_streams)
    }
}
