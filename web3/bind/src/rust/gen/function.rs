use crate::{
    abi::{Parameter, Type},
    bind::RuntimeBinder,
};
use heck::ToSnakeCase;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use super::RustGenerator;

impl RustGenerator {
    /// Convert `params` to generic list
    pub(crate) fn to_param_list<R: RuntimeBinder>(
        &self,
        _runtime_binder: &mut R,
        params: &[Parameter],
    ) -> anyhow::Result<Vec<TokenStream>> {
        let mut token_streams = vec![];

        for (index, param) in params.iter().enumerate() {
            let type_ident = format_ident!("P{}", index);

            let var_ident = if param.name != "" {
                format_ident!("{}", param.name.to_snake_case())
            } else {
                format_ident!("p{}", index)
            };

            token_streams.push(quote!(#var_ident: #type_ident));
        }

        Ok(token_streams)
    }

    /// Convert fn param list to fn generic list
    pub(crate) fn to_generic_list<R: RuntimeBinder>(
        &self,
        _runtime_binder: &mut R,
        params: &[Parameter],
    ) -> anyhow::Result<Vec<TokenStream>> {
        let mut token_streams = vec![];

        for (index, _) in params.iter().enumerate() {
            let type_ident = format_ident!("P{}", index);

            token_streams.push(quote!(#type_ident));
        }

        Ok(token_streams)
    }

    /// Convert fn param list to fn where clause list
    pub(crate) fn to_where_clause_list<R: RuntimeBinder>(
        &self,
        runtime_binder: &mut R,
        params: &[Parameter],
    ) -> anyhow::Result<Vec<TokenStream>> {
        let mut token_streams = vec![];

        for (index, param) in params.iter().enumerate() {
            let type_ident = format_ident!("P{}", index);

            let try_into_type = self.to_rust_type(runtime_binder, param)?;

            token_streams.push(quote!(#type_ident: TryInto<#try_into_type>, #type_ident::Error: std::error::Error + Sync + Send + 'static));
        }

        Ok(token_streams)
    }

    /// Convert fn param list to try_into statement
    pub(crate) fn to_try_into_list<R: RuntimeBinder>(
        &self,
        _runtime_binder: &mut R,
        params: &[Parameter],
    ) -> anyhow::Result<Vec<TokenStream>> {
        let mut token_streams = vec![];

        for (index, param) in params.iter().enumerate() {
            let var_ident = if param.name != "" {
                format_ident!("{}", param.name.to_snake_case())
            } else {
                format_ident!("p{}", index)
            };

            token_streams.push(quote!(let #var_ident = #var_ident.try_into()?));
        }

        Ok(token_streams)
    }

    /// Convert fn param list to rlp encode statement
    pub(crate) fn to_abi_encode_list<R: RuntimeBinder>(
        &self,
        _runtime_binder: &mut R,
        params: &[Parameter],
    ) -> anyhow::Result<TokenStream> {
        let mut token_streams = vec![];

        for (index, param) in params.iter().enumerate() {
            let param_ident = if param.name != "" {
                format_ident!("{}", param.name.to_snake_case())
            } else {
                format_ident!("p{}", index)
            };

            token_streams.push(quote!(#param_ident));
        }

        if token_streams.len() > 1 {
            Ok(quote! {
                (#(#token_streams,)*)
            })
        } else if token_streams.len() == 1 {
            Ok(quote! {
                #(#token_streams,)*
            })
        } else {
            Ok(quote! {
                ()
            })
        }
    }

    #[allow(unused)]
    pub(crate) fn to_rust_type<R: RuntimeBinder>(
        &self,
        runtime_binder: &mut R,
        param: &Parameter,
    ) -> anyhow::Result<TokenStream> {
        if let Some(runtime_type) = runtime_binder.to_runtime_type(&param.r#type)? {
            let runtime_type: TokenStream = runtime_type
                .parse()
                .map_err(|err| anyhow::format_err!("{}", err))?;

            return Ok(runtime_type);
        } else {
            let components = param
                .components
                .as_ref()
                .expect("Tuple parameter's components field is None");

            let mut tuple_token_streams = vec![];

            for c in components {
                tuple_token_streams.push(self.to_rust_type(runtime_binder, c)?);
            }

            return Ok(quote!((#(#tuple_token_streams,)*)));
        }
    }

    pub(crate) fn to_outputs_type<R: RuntimeBinder>(
        &self,
        runtime_binder: &mut R,
        outputs: &[Parameter],
    ) -> anyhow::Result<TokenStream> {
        let mut tuple_token_streams = vec![];

        for param in outputs {
            tuple_token_streams.push(self.to_rust_type(runtime_binder, param)?);
        }

        if outputs.len() <= 1 {
            Ok(quote!(#(#tuple_token_streams)*))
        } else {
            Ok(quote!((#(#tuple_token_streams,)*)))
        }
    }

    #[allow(unused)]
    pub(crate) fn to_abi_encode<R: RuntimeBinder>(
        &self,
        runtime_binder: &mut R,
        var_name: &str,
        r#type: &Type,
        components: &[Parameter],
    ) -> anyhow::Result<TokenStream> {
        if let Some(_) = runtime_binder.to_runtime_type(r#type)? {
            let var_ident: TokenStream =
                var_name.parse().map_err(|e| anyhow::format_err!("{}", e))?;

            return Ok(quote!(outputs.abi_encode(&#var_ident)?;));
        } else {
            let mut tuple_token_streams = vec![];

            static NULL: Vec<Parameter> = vec![];

            for (index, c) in components.iter().enumerate() {
                tuple_token_streams.push(self.to_abi_encode(
                    runtime_binder,
                    &format!("{}.{}", var_name, index),
                    &c.r#type,
                    c.components.as_ref().unwrap_or(&NULL).as_slice(),
                )?);
            }

            return Ok(quote! {
                outputs.abi_start_encode_tuple()?;
                #(#tuple_token_streams)*
                outputs.abi_end_encode_tuple()?;
            });
        }
    }
}
