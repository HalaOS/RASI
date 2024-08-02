use crate::bind::{Contract, File};
use heck::ToUpperCamelCase;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};

#[derive(Debug, Default)]
#[allow(unused)]
pub(crate) struct ContractGenerator {
    pub(crate) contract_name: String,
    fn_token_streams: Vec<TokenStream>,
    event_token_streams: Vec<TokenStream>,
    error_token_streams: Vec<TokenStream>,
}

impl ContractGenerator {
    pub(crate) fn new(contract_name: &str) -> Self {
        Self {
            contract_name: contract_name.to_owned(),
            ..Default::default()
        }
    }

    pub(crate) fn add_fn_token_stream(&mut self, token_stream: TokenStream) {
        self.fn_token_streams.push(token_stream);
    }

    pub(crate) fn add_event_token_stream(&mut self, token_stream: TokenStream) {
        self.event_token_streams.push(token_stream);
    }

    pub(crate) fn finalize(
        &self,
        rt_client: &TokenStream,
        rt_address: &TokenStream,
    ) -> anyhow::Result<Contract> {
        let fn_token_streams = &self.fn_token_streams;
        let event_token_streams = &self.event_token_streams;
        // let error_token_streams = &self.error_token_streams;

        let ident = format_ident!("{}", &self.contract_name.to_upper_camel_case());

        let token_stream = quote! {
            pub struct #ident{
                pub client: #rt_client,
                pub address: #rt_address,
            }

            impl #ident {
                #(#fn_token_streams)*
            }

            #(#event_token_streams)*
        };

        Ok(Contract {
            files: vec![File {
                name: self.contract_name.clone(),
                data: token_stream.to_string(),
            }],
        })
    }
}
