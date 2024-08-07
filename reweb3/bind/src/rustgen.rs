//! This module provides an offical code [`binder`](crate::binder::Binder) implementation for rust code.

use std::{
    cell::RefCell,
    env, fs,
    path::{Path, PathBuf},
    process::Command,
    rc::Rc,
};

use heck::{ToSnekCase, ToUpperCamelCase};
use proc_macro2::{Ident, LexError, TokenStream};
use quote::{format_ident, quote};

use crate::{
    binder::{
        bind, Binder, BinderContext, ConstructorBinder, ContractBinder, EventBinder,
        FunctionBinder, TupleBinder,
    },
    mapping::BinderTypeMapping,
    typedef::{HardhatArtifact, Parameter, StateMutability},
};

/// Error type that raised by the functions in this mod.
#[derive(Debug, thiserror::Error)]
pub enum RustBinderError {
    #[error("`rt_signer_with_provider` type mapping not found")]
    RuntimeSignerWithProviderType,

    #[error("`rt_provider` type mapping not found")]
    RuntimeProviderType,

    #[error("`address` type mapping not found")]
    RuntimeAddressType,

    #[error("`rt_log` type mapping not found")]
    RuntimeLogType,

    #[error("`rt_keccak256` fn mapping not found")]
    RuntimeKeccak256Fn,

    #[error("`rt_h256` type mapping not found")]
    RuntimeH256Type,

    #[error("`rt_abi_encode` type mapping not found")]
    RuntimeAbiEncodeFn,

    #[error("`rt_abi_decode` type mapping not found")]
    RuntimeAbiDecodeFn,

    #[error("`rt_transfer_ops` type mapping not found")]
    RuntimeTransferOps,

    #[error("parse mapping type `{0}` failed: {1}")]
    ParseMappingType(String, String),

    #[error("`{0}` type mapping not found")]
    RuntimeTypeNotFound(String),
}

/// The code generator for rust language.
pub struct RustBinder(Rc<RefCell<BinderTypeMapping>>);

impl From<BinderTypeMapping> for RustBinder {
    fn from(value: BinderTypeMapping) -> Self {
        Self(Rc::new(RefCell::new(value)))
    }
}

impl RustBinder {
    /// Create new `RustBinder` to generate rust binding code.
    pub fn new(mapping: BinderTypeMapping) -> Self {
        mapping.into()
    }
}

impl Binder for RustBinder {
    type Error = RustBinderError;
    type ContractBinder = RustContractBinder;

    fn prepare(
        &mut self,
        _cx: &crate::binder::BinderContext<'_>,
        contract_name: &str,
    ) -> Result<Self::ContractBinder, Self::Error> {
        Ok(RustContractBinder::new(contract_name, self.0.clone()))
    }
}

struct RustContractBinderContext {
    contract_name: String,
    mapping: Rc<RefCell<BinderTypeMapping>>,
    funcs: Vec<TokenStream>,
    tuples: Vec<TokenStream>,
    events: Vec<TokenStream>,
    errors: Vec<TokenStream>,
}

impl RustContractBinderContext {
    fn new(contract_name: &str, mapping: Rc<RefCell<BinderTypeMapping>>) -> Self {
        Self {
            contract_name: contract_name.to_owned(),
            mapping,
            funcs: Default::default(),
            tuples: Default::default(),
            errors: Default::default(),
            events: Default::default(),
        }
    }
}

/// An individual contract generator
pub struct RustContractBinder {
    context: Rc<RefCell<RustContractBinderContext>>,
}

impl RustContractBinder {
    fn new(contract_name: &str, mapping: Rc<RefCell<BinderTypeMapping>>) -> Self {
        Self {
            context: Rc::new(RefCell::new(RustContractBinderContext::new(
                contract_name,
                mapping,
            ))),
        }
    }
}

impl ContractBinder for RustContractBinder {
    type Error = RustBinderError;

    type ConstructorBinder = RustConstructorBinder;

    type FunctionBinder = RustFunctionBinder;

    type EventBinder = RustEventBinder;

    type ErrorBinder = RustEventBinder;

    type TupleBinder = RustTupleBinder;

    fn bind_constructor(
        &mut self,
        _cx: &crate::binder::BinderContext<'_>,
        signature: &str,
        state: &crate::typedef::StateMutability,
    ) -> Result<Self::ConstructorBinder, Self::Error> {
        Ok(RustConstructorBinder::new(
            signature.to_owned(),
            state.clone(),
            self.context.clone(),
        ))
    }

    fn bind_function(
        &mut self,
        _cx: &crate::binder::BinderContext<'_>,
        fn_name: &str,
        signature: &str,
        state: &crate::typedef::StateMutability,
    ) -> Result<Self::FunctionBinder, Self::Error> {
        Ok(RustFunctionBinder::new(
            fn_name.to_owned(),
            signature.to_owned(),
            state.clone(),
            self.context.clone(),
        ))
    }

    fn bind_receiver(
        &mut self,
        _cx: &crate::binder::BinderContext<'_>,
        _state: &crate::typedef::StateMutability,
    ) -> Result<(), Self::Error> {
        Ok(())
    }

    fn bind_fallback(
        &mut self,
        _cx: &crate::binder::BinderContext<'_>,
        _state: &crate::typedef::StateMutability,
    ) -> Result<(), Self::Error> {
        Ok(())
    }

    fn bind_event(
        &mut self,
        _cx: &crate::binder::BinderContext<'_>,
        name: &str,
        signature: Option<&str>,
    ) -> Result<Self::EventBinder, Self::Error> {
        Ok(RustEventBinder::new(
            name.to_owned(),
            signature.map(str::to_owned),
            self.context.clone(),
            false,
        ))
    }

    fn bind_error(
        &mut self,
        _cx: &crate::binder::BinderContext<'_>,
        name: &str,
        signature: &str,
    ) -> Result<Self::ErrorBinder, Self::Error> {
        Ok(RustEventBinder::new(
            name.to_owned(),
            Some(signature.to_owned()),
            self.context.clone(),
            true,
        ))
    }

    fn bind_tuple(
        &mut self,
        _cx: &BinderContext<'_>,
        name: &str,
    ) -> Result<Self::TupleBinder, Self::Error> {
        Ok(RustTupleBinder::new(
            normalised_tuple_name(name),
            self.context.clone(),
        ))
    }

    fn finialize(
        &mut self,
        _cx: &crate::binder::BinderContext<'_>,
    ) -> Result<proc_macro2::TokenStream, Self::Error> {
        let contract_name = format_ident!(
            "{}",
            self.context
                .borrow()
                .contract_name
                .clone()
                .to_upper_camel_case()
        );

        let address = rt_type_mapping(
            &mut self.context.borrow_mut().mapping.borrow_mut(),
            "address",
        )?
        .ok_or(RustBinderError::RuntimeAddressType)?;

        let fns = self
            .context
            .borrow_mut()
            .funcs
            .drain(..)
            .collect::<Vec<_>>();

        let tuples = self
            .context
            .borrow_mut()
            .tuples
            .drain(..)
            .collect::<Vec<_>>();

        let events = self
            .context
            .borrow_mut()
            .events
            .drain(..)
            .collect::<Vec<_>>();

        let errors = self
            .context
            .borrow_mut()
            .errors
            .drain(..)
            .collect::<Vec<_>>();

        let stream = quote! {

            pub mod tuples {
                #(#tuples)*
            }

            pub mod events {
                #(#events)*
            }

            pub mod errors {
                #(#errors)*
            }



            pub struct #contract_name<C> {
                address: #address,
                client: C,
            }

            impl<C> #contract_name<C> {
                pub fn new(client: C, address: #address) -> Self {
                    Self {
                        address,client,
                    }
                }
            }

            impl<C> #contract_name<C> {
                #(#fns)*
            }
        };

        Ok(stream)
    }
}

fn normalised_tuple_name(name: &str) -> String {
    let name = if name.starts_with("struct") {
        &name[6..]
    } else {
        name
    };

    name.to_upper_camel_case()
}

fn param_type_mapping(
    mapping: &mut BinderTypeMapping,
    param: &Parameter,
) -> Result<TokenStream, RustBinderError> {
    if let Some(rt_typte) = mapping.abi_type_mapping(&param.r#type) {
        rt_typte.parse().map_err(|err: LexError| {
            RustBinderError::ParseMappingType(rt_typte.to_owned(), err.to_string())
        })
    } else if let Some(internal_type) = param.internal_type.as_deref() {
        ("tuples::".to_owned() + normalised_tuple_name(internal_type).as_str())
            .parse()
            .map_err(|err: LexError| {
                RustBinderError::ParseMappingType(internal_type.to_owned(), err.to_string())
            })
    } else {
        return Err(RustBinderError::RuntimeTypeNotFound(
            param.r#type.to_string(),
        ));
    }
}

fn rt_type_mapping(
    mapping: &mut BinderTypeMapping,
    type_name: &str,
) -> Result<Option<TokenStream>, RustBinderError> {
    if let Some(rt_type) = mapping.rt_type_mapping(type_name) {
        rt_type
            .parse()
            .map_err(|err: LexError| {
                RustBinderError::ParseMappingType(rt_type.to_owned(), err.to_string())
            })
            .map(|v| Some(v))
    } else {
        Ok(None)
    }
}

fn to_generic_param_ident(index: usize, param: &Parameter) -> Ident {
    if param.name.is_empty() {
        format_ident!("P{}", index)
    } else {
        format_ident!("{}", param.name.to_upper_camel_case())
    }
}

fn to_var_ident(index: usize, param: &Parameter) -> Ident {
    if param.name.is_empty() {
        format_ident!("p{}", index)
    } else {
        format_ident!("{}", param.name.to_snek_case())
    }
}

pub struct RustConstructorBinder {
    #[allow(unused)]
    state: StateMutability,
    context: Rc<RefCell<RustContractBinderContext>>,
    signature: String,
    where_list: Vec<TokenStream>,
    generic_param_list: Vec<TokenStream>,
    param_list: Vec<TokenStream>,
    try_into_list: Vec<TokenStream>,
    param_encode_list: Vec<TokenStream>,
}

impl RustConstructorBinder {
    fn new(
        signature: String,
        state: StateMutability,
        context: Rc<RefCell<RustContractBinderContext>>,
    ) -> Self {
        Self {
            state,
            context,
            signature,
            param_list: Default::default(),
            generic_param_list: Default::default(),
            where_list: Default::default(),
            try_into_list: Default::default(),
            param_encode_list: Default::default(),
        }
    }
}

impl ConstructorBinder for RustConstructorBinder {
    type Error = RustBinderError;

    fn bind_input(
        &mut self,
        _cx: &crate::binder::BinderContext<'_>,
        index: usize,
        param: &crate::typedef::Parameter,
    ) -> Result<(), Self::Error> {
        let generic_param_ident = to_generic_param_ident(index, param);

        let var_ident = to_var_ident(index, param);

        let rt_ident =
            param_type_mapping(&mut self.context.borrow_mut().mapping.borrow_mut(), param)?;

        self.generic_param_list.push(quote!(#generic_param_ident));

        self.param_list
            .push(quote!(#var_ident: #generic_param_ident));

        self.where_list.push(quote! {
            #generic_param_ident: TryInto<#rt_ident>,
            #generic_param_ident::Error: std::fmt::Debug + Send + 'static
        });

        self.try_into_list.push(quote! {
            let #var_ident = #var_ident.try_into().map_err(|err|std::io::Error::new(std::io::ErrorKind::InvalidInput, format!("{:#?}",err)))?
        });

        self.param_encode_list.push(quote! {#var_ident});

        Ok(())
    }

    fn finialize(&mut self, cx: &crate::binder::BinderContext<'_>) -> Result<(), Self::Error> {
        let bytecode = match cx.bytecode() {
            Some(bytecode) => bytecode,
            // bytecode not found.
            None => {
                log::warn!(
                    "{}: bytecode not found, skip generate deploy fn.",
                    self.context.borrow().contract_name
                );
                return Ok(());
            }
        };

        let signer = rt_type_mapping(
            &mut self.context.borrow_mut().mapping.borrow_mut(),
            "rt_signer_with_provider",
        )?
        .ok_or(RustBinderError::RuntimeSignerWithProviderType)?;

        let abi_encode = rt_type_mapping(
            &mut self.context.borrow_mut().mapping.borrow_mut(),
            "rt_abi_encode",
        )?
        .ok_or(RustBinderError::RuntimeAbiEncodeFn)?;

        let transfer_ops = rt_type_mapping(
            &mut self.context.borrow_mut().mapping.borrow_mut(),
            "rt_transfer_ops",
        )?
        .ok_or(RustBinderError::RuntimeTransferOps)?;

        let h256 = rt_type_mapping(
            &mut self.context.borrow_mut().mapping.borrow_mut(),
            "rt_h256",
        )?
        .ok_or(RustBinderError::RuntimeH256Type)?;

        let signature = &self.signature;
        let generic_param_list = &self.generic_param_list;
        let param_list: &Vec<TokenStream> = &self.param_list;
        let where_list = &self.where_list;
        let try_into_list = &self.try_into_list;
        let param_encode_list: &Vec<TokenStream> = &self.param_encode_list;

        let fn_stream = quote! {
            /// Deploy contract with provided client.
            pub async fn deploy<#(#generic_param_list,)* Ops>(client: C, #(#param_list,)* transfer_ops: Ops) -> std::io::Result<#h256>
            where
                C: #signer + Send + Unpin,
                Ops: TryInto<#transfer_ops>,
                Ops:: Error: std::fmt::Debug + Send + 'static,
                #(#where_list,)*
            {
                let transfer_ops = transfer_ops.try_into().map_err(|err|std::io::Error::new(std::io::ErrorKind::InvalidInput, format!("{:#?}",err)))?;

                #(#try_into_list;)*

                let params = #abi_encode((#(#param_encode_list,)*))
                    .map_err(|err|std::io::Error::new(std::io::ErrorKind::InvalidInput, format!("{:#?}",err)))?;

                let hash = client.deploy_contract(#bytecode, #signature, params, transfer_ops).await
                    .map_err(|err|std::io::Error::new(std::io::ErrorKind::Other, format!("{:#?}",err)))?;

                Ok(hash)
            }
        };

        self.context.borrow_mut().funcs.push(fn_stream);

        Ok(())
    }
}

pub struct RustFunctionBinder {
    fn_name: String,
    state: StateMutability,
    context: Rc<RefCell<RustContractBinderContext>>,
    signature: String,
    where_list: Vec<TokenStream>,
    generic_param_list: Vec<TokenStream>,
    param_list: Vec<TokenStream>,
    return_param_list: Vec<TokenStream>,
    try_into_list: Vec<TokenStream>,
    param_encode_list: Vec<TokenStream>,
}

impl RustFunctionBinder {
    fn new(
        fn_name: String,
        signature: String,
        state: StateMutability,
        context: Rc<RefCell<RustContractBinderContext>>,
    ) -> Self {
        Self {
            fn_name,
            signature,
            state,
            context,
            param_list: Default::default(),
            generic_param_list: Default::default(),
            where_list: Default::default(),
            try_into_list: Default::default(),
            param_encode_list: Default::default(),
            return_param_list: Default::default(),
        }
    }
}

impl FunctionBinder for RustFunctionBinder {
    type Error = RustBinderError;

    fn bind_input(
        &mut self,
        _cx: &crate::binder::BinderContext<'_>,
        index: usize,
        param: &crate::typedef::Parameter,
    ) -> Result<(), Self::Error> {
        let generic_param_ident = to_generic_param_ident(index, param);

        let var_ident = to_var_ident(index, param);

        let rt_ident =
            param_type_mapping(&mut self.context.borrow_mut().mapping.borrow_mut(), param)?;

        self.generic_param_list.push(quote!(#generic_param_ident));

        self.param_list
            .push(quote!(#var_ident: #generic_param_ident));

        self.where_list.push(quote! {
            #generic_param_ident: TryInto<#rt_ident>,
            #generic_param_ident::Error: std::fmt::Debug + Send + 'static
        });

        self.try_into_list.push(quote! {
            let #var_ident = #var_ident.try_into().map_err(|err|std::io::Error::new(std::io::ErrorKind::InvalidInput, format!("{:#?}",err)))?
        });

        self.param_encode_list.push(quote! {#var_ident});

        Ok(())
    }

    fn bind_output(
        &mut self,
        _cx: &crate::binder::BinderContext<'_>,
        _: usize,
        param: &crate::typedef::Parameter,
    ) -> Result<(), Self::Error> {
        match self.state {
            // only `pure` or `view` function need to generate return param list.
            StateMutability::Pure | StateMutability::View => {}
            _ => {
                return Ok(());
            }
        }

        let rt_ident =
            param_type_mapping(&mut self.context.borrow_mut().mapping.borrow_mut(), param)?;

        self.return_param_list.push(quote! {#rt_ident});

        Ok(())
    }

    fn finialize(&mut self, _cx: &crate::binder::BinderContext<'_>) -> Result<(), Self::Error> {
        let abi_encode = rt_type_mapping(
            &mut self.context.borrow_mut().mapping.borrow_mut(),
            "rt_abi_encode",
        )?
        .ok_or(RustBinderError::RuntimeAbiEncodeFn)?;

        let abi_decode = rt_type_mapping(
            &mut self.context.borrow_mut().mapping.borrow_mut(),
            "rt_abi_decode",
        )?
        .ok_or(RustBinderError::RuntimeAbiDecodeFn)?;

        let transfer_ops = rt_type_mapping(
            &mut self.context.borrow_mut().mapping.borrow_mut(),
            "rt_transfer_ops",
        )?
        .ok_or(RustBinderError::RuntimeTransferOps)?;

        let h256 = rt_type_mapping(
            &mut self.context.borrow_mut().mapping.borrow_mut(),
            "rt_h256",
        )?
        .ok_or(RustBinderError::RuntimeH256Type)?;

        let signer = rt_type_mapping(
            &mut self.context.borrow_mut().mapping.borrow_mut(),
            "rt_signer_with_provider",
        )?
        .ok_or(RustBinderError::RuntimeSignerWithProviderType)?;

        let provider = rt_type_mapping(
            &mut self.context.borrow_mut().mapping.borrow_mut(),
            "rt_provider",
        )?
        .ok_or(RustBinderError::RuntimeProviderType)?;

        let signature = &self.signature;
        let fn_name = format_ident!("{}", &self.fn_name.to_snek_case());
        let generic_param_list = &self.generic_param_list;
        let param_list: &Vec<TokenStream> = &self.param_list;
        let where_list = &self.where_list;
        let try_into_list = &self.try_into_list;
        let param_encode_list: &Vec<TokenStream> = &self.param_encode_list;
        let return_param_list = &self.return_param_list;

        let fn_stream = match self.state {
            StateMutability::Pure | StateMutability::View => {
                quote! {
                    pub async fn #fn_name<#(#generic_param_list,)*>(&self, #(#param_list,)*) -> std::io::Result<(#(#return_param_list,)*)>
                    where
                        C: #provider + Sync + Unpin,
                        #(#where_list,)*
                    {
                        #(#try_into_list;)*

                        let params = #abi_encode((#(#param_encode_list,)*))
                            .map_err(|err|std::io::Error::new(std::io::ErrorKind::InvalidInput, format!("{:#?}",err)))?;

                         let call_result = self.client.call_contract(#signature, &self.address, params).await
                            .map_err(|err|std::io::Error::new(std::io::ErrorKind::Other, format!("{:#?}",err)))?;

                         Ok(#abi_decode(call_result)
                            .map_err(|err|std::io::Error::new(std::io::ErrorKind::InvalidInput, format!("{:#?}",err)))?)
                    }
                }
            }

            StateMutability::Nonpayable | StateMutability::Payable => {
                quote! {
                    pub async fn #fn_name<#(#generic_param_list,)* Ops>(&self, #(#param_list,)* transfer_ops: Ops) -> std::io::Result<#h256>
                    where
                        C: #signer + Sync + Unpin,
                        Ops: TryInto<#transfer_ops>,
                        Ops:: Error: std::fmt::Debug + Send + 'static,
                        #(#where_list,)*
                    {

                        let transfer_ops = transfer_ops.try_into().map_err(|err|std::io::Error::new(std::io::ErrorKind::InvalidInput, format!("{:#?}",err)))?;

                        #(#try_into_list;)*

                        let params = #abi_encode((#(#param_encode_list,)*))
                            .map_err(|err|std::io::Error::new(std::io::ErrorKind::InvalidInput, format!("{:#?}",err)))?;

                         Ok(self.client.call_contract_with_transaction(#signature, &self.address, params, transfer_ops).await
                            .map_err(|err|std::io::Error::new(std::io::ErrorKind::Other, format!("{:#?}",err)))?)
                    }
                }
            }
        };

        self.context.borrow_mut().funcs.push(fn_stream);

        Ok(())
    }
}

pub struct RustTupleBinder {
    tuple_name: String,
    context: Rc<RefCell<RustContractBinderContext>>,
    field_list: Vec<TokenStream>,
}

impl RustTupleBinder {
    fn new(tuple_name: String, context: Rc<RefCell<RustContractBinderContext>>) -> Self {
        Self {
            tuple_name,
            context,
            field_list: Default::default(),
        }
    }
}

impl TupleBinder for RustTupleBinder {
    type Error = RustBinderError;

    fn bind_input(
        &mut self,
        _cx: &BinderContext<'_>,
        _index: usize,
        param: &Parameter,
    ) -> Result<(), Self::Error> {
        let rt_ident =
            param_type_mapping(&mut self.context.borrow_mut().mapping.borrow_mut(), param)?;

        let field_name: Ident = format_ident!("{}", param.name.to_snek_case());

        self.field_list.push(quote! {
            pub #field_name: #rt_ident
        });

        Ok(())
    }

    fn finialize(&mut self, _cx: &BinderContext<'_>) -> Result<(), Self::Error> {
        let tuple_name = format_ident!("{}", self.tuple_name);

        let field_list = self.field_list.as_slice();

        let stream = quote! {
            #[derive(serde::Serialize,serde::Deserialize)]
            pub struct #tuple_name {
                #(#field_list,)*
            }
        };

        self.context.borrow_mut().tuples.push(stream);

        Ok(())
    }
}

pub struct RustEventBinder {
    name: String,
    signature: Option<String>,
    context: Rc<RefCell<RustContractBinderContext>>,
    field_list: Vec<TokenStream>,
    field_name_list: Vec<TokenStream>,
    topic_decode_list: Vec<TokenStream>,
    data_decode_list: Vec<TokenStream>,
    data_decode_type_list: Vec<TokenStream>,
    topic_index: usize,
    is_error: bool,
}

impl RustEventBinder {
    fn new(
        name: String,
        signature: Option<String>,
        context: Rc<RefCell<RustContractBinderContext>>,
        is_error: bool,
    ) -> Self {
        Self {
            topic_index: if signature.is_some() { 1 } else { 0 },
            is_error,
            name,
            signature,
            context,
            field_list: Default::default(),
            field_name_list: Default::default(),
            topic_decode_list: Default::default(),
            data_decode_list: Default::default(),
            data_decode_type_list: Default::default(),
        }
    }
}

impl EventBinder for RustEventBinder {
    type Error = RustBinderError;

    fn bind_input(
        &mut self,
        _cx: &crate::binder::BinderContext<'_>,
        _index: usize,
        param: &crate::typedef::Parameter,
    ) -> Result<(), Self::Error> {
        let rt_ident =
            param_type_mapping(&mut self.context.borrow_mut().mapping.borrow_mut(), param)?;

        let field_name: Ident = format_ident!("{}", param.name.to_snek_case());

        let abi_decode = rt_type_mapping(
            &mut self.context.borrow_mut().mapping.borrow_mut(),
            "rt_abi_decode",
        )?
        .ok_or(RustBinderError::RuntimeAbiDecodeFn)?;

        let topic_index = self.topic_index;

        self.field_list.push(quote! {
            pub #field_name: #rt_ident
        });

        self.field_name_list.push(quote! {#field_name});

        if param.indexed {
            self.topic_index += 1;
            self.topic_decode_list.push(quote! {

                if !(log.topics.len() > #topic_index) {
                    return Err(std::io::Error::new(std::io::ErrorKind::Other, format!("decode log failed: topic out of range")));
                }

                let #field_name: #rt_ident = #abi_decode(&log.topics[#topic_index])
                    .map_err(|err|std::io::Error::new(std::io::ErrorKind::Other, format!("{:#?}",err)))?;
            });
        } else {
            self.data_decode_list.push(quote! {#field_name});
            self.data_decode_type_list.push(quote! {#rt_ident});
        }

        Ok(())
    }

    fn finialize(&mut self, _cx: &crate::binder::BinderContext<'_>) -> Result<(), Self::Error> {
        let tuple_name = format_ident!("{}", self.name);

        let log = rt_type_mapping(
            &mut self.context.borrow_mut().mapping.borrow_mut(),
            "rt_log",
        )?
        .ok_or(RustBinderError::RuntimeLogType)?;

        let keccak256 = rt_type_mapping(
            &mut self.context.borrow_mut().mapping.borrow_mut(),
            "rt_keccak256",
        )?
        .ok_or(RustBinderError::RuntimeKeccak256Fn)?;

        let field_list = self.field_list.as_slice();
        let field_name_list = self.field_name_list.as_slice();
        let topic_decode_list = self.topic_decode_list.as_slice();
        let data_decode_list = self.data_decode_list.as_slice();
        let data_decode_type_list = self.data_decode_type_list.as_slice();
        let abi_decode = rt_type_mapping(
            &mut self.context.borrow_mut().mapping.borrow_mut(),
            "rt_abi_decode",
        )?
        .ok_or(RustBinderError::RuntimeAbiDecodeFn)?;

        let signature = if let Some(signature) = self.signature.as_deref() {
            quote! {
                impl #tuple_name {
                    pub fn signature() -> &'static str {
                        #signature
                    }
                }
            }
        } else {
            quote! {}
        };

        let check_signature = if self.signature.is_some() {
            quote! {
                if log.topics.is_empty() {
                    return Ok(None);
                }
                if #keccak256(Self::signature()) != log.topics[0] {
                    return Ok(None);
                }
            }
        } else {
            quote! {}
        };

        let stream = quote! {
            pub struct #tuple_name {
                #(#field_list,)*
            }

            #signature

            impl #tuple_name {
                pub fn from_log(log: #log) -> std::io::Result<Option<Self>> {

                    #check_signature

                    #(#topic_decode_list)*
                    let (#(#data_decode_list,)*): (#(#data_decode_type_list,)*) = #abi_decode(log.data)
                        .map_err(|err|std::io::Error::new(std::io::ErrorKind::Other, format!("{:#?}",err)))?;

                    Ok(Some(#tuple_name {
                         #(#field_name_list,)*
                    }))
                }
            }
        };

        if self.is_error {
            self.context.borrow_mut().errors.push(stream);
        } else {
            self.context.borrow_mut().events.push(stream);
        }

        Ok(())
    }
}

/// A utility tool to generate rust bind code and write to file.
pub fn write_file<'a, P: AsRef<Path>>(
    cx: BinderContext<'a>,
    mapping: BinderTypeMapping,
    path: P,
) -> std::io::Result<()> {
    let binder = RustBinder::new(mapping);

    let stream = bind(&cx, binder)
        .map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, err.to_string()))?;

    fs::write(path.as_ref(), stream.to_string())?;

    // try format the generated file.
    let rust_fmt_path =
        PathBuf::from(env::var("CARGO_HOME").expect("Get CARGO_HOME")).join("bin/rustfmt");

    let mut child = Command::new(&rust_fmt_path)
        .args(["--edition", "2021", path.as_ref().to_str().unwrap()])
        .spawn()?;

    child.wait()?;

    Ok(())
}

/// Load the hardhat artifact and generate bind code file with `path`.
pub fn bind_hardhat_artifact<H: AsRef<Path>, M: AsRef<Path>, T: AsRef<Path>>(
    artifact_path: H,
    mapping_path: M,
    target_dir: T,
) -> std::io::Result<()> {
    let hardhat: HardhatArtifact =
        serde_json::from_slice(fs::read(artifact_path.as_ref())?.as_slice()).unwrap();

    let cx = BinderContext::new(
        &hardhat.contract_name,
        &hardhat.abi,
        Some(&hardhat.bytecode),
    );

    let mapping: BinderTypeMapping =
        serde_json::from_slice(fs::read(mapping_path.as_ref())?.as_slice()).unwrap();

    if !target_dir.as_ref().exists() {
        fs::create_dir_all(target_dir.as_ref())?;
    }

    let target = target_dir
        .as_ref()
        .join(hardhat.contract_name.to_snek_case() + ".rs");

    write_file(cx, mapping, target)
}
