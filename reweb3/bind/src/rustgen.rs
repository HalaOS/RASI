//! This module provides an offical code [`binder`](crate::binder::Binder) implementation for rust code.

use std::{cell::RefCell, rc::Rc};

use heck::{ToSnekCase, ToUpperCamelCase};
use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};

use crate::{
    binder::{Binder, ConstructorBinder, ContractBinder, ErrorBinder, EventBinder, FunctionBinder},
    mapping::BinderTypeMapping,
    typedef::{Parameter, StateMutability},
};

/// Error type that raised by the functions in this mod.
#[derive(Debug, thiserror::Error)]
pub enum RustBinderError {
    #[error("`rt_signer_with_provider` type mapping not found")]
    RuntimeClientType,

    #[error("`rt_h256` type mapping not found")]
    RuntimeH256Type,

    #[error("`rt_abi_encode` type mapping not found")]
    RuntimeAbiEncodeFn,

    #[error("`rt_abi_decode` type mapping not found")]
    RuntimeAbiDecodeFn,

    #[error("`rt_transfer_ops` type mapping not found")]
    RuntimeTransferOps,
}

/// The code generator for rust language.
pub struct RustBinder(Rc<RefCell<BinderTypeMapping>>);

impl From<BinderTypeMapping> for RustBinder {
    fn from(value: BinderTypeMapping) -> Self {
        Self(Rc::new(RefCell::new(value)))
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
}

impl RustContractBinderContext {
    fn new(contract_name: &str, mapping: Rc<RefCell<BinderTypeMapping>>) -> Self {
        Self {
            contract_name: contract_name.to_owned(),
            mapping,
            funcs: Default::default(),
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

    type ErrorBinder = RustErrorBinder;

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
        todo!()
    }

    fn bind_fallback(
        &mut self,
        _cx: &crate::binder::BinderContext<'_>,
        _state: &crate::typedef::StateMutability,
    ) -> Result<(), Self::Error> {
        todo!()
    }

    fn bind_event(
        &mut self,
        _cx: &crate::binder::BinderContext<'_>,
        name: &str,
        anonymous: bool,
    ) -> Result<Self::EventBinder, Self::Error> {
        Ok(RustEventBinder {
            name: name.to_owned(),
            anonymous,
        })
    }

    fn bind_error(
        &mut self,
        _cx: &crate::binder::BinderContext<'_>,
        name: &str,
    ) -> Result<Self::ErrorBinder, Self::Error> {
        Ok(RustErrorBinder {
            name: name.to_owned(),
        })
    }

    fn bind_deploy(
        &mut self,
        _cx: &crate::binder::BinderContext<'_>,
        _bytecode: &str,
    ) -> Result<(), Self::Error> {
        todo!()
    }

    fn finialize(
        &mut self,
        _cx: &crate::binder::BinderContext<'_>,
    ) -> Result<proc_macro2::TokenStream, Self::Error> {
        todo!()
    }
}

fn to_rt_ident(mapping: &mut BinderTypeMapping, param: &Parameter) -> Ident {
    if let Some(rt_typte) = mapping.abi_type_mapping(&param.r#type) {
        format_ident!("{}", rt_typte)
    } else {
        format_ident!(
            "{}",
            param
                .internal_type
                .as_deref()
                .expect("expect tuple name")
                .to_upper_camel_case()
        )
    }
}

fn to_generic_param_ident(param: &Parameter) -> Ident {
    format_ident!("{}", param.name.to_upper_camel_case())
}

fn to_var_ident(param: &Parameter) -> Ident {
    format_ident!("{}", param.name.to_snek_case())
}

pub struct RustConstructorBinder {
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
        _index: usize,
        param: &crate::typedef::Parameter,
    ) -> Result<(), Self::Error> {
        let generic_param_ident = to_generic_param_ident(param);

        let var_ident = to_var_ident(param);

        let rt_ident = to_rt_ident(&mut self.context.borrow_mut().mapping.borrow_mut(), param);

        self.generic_param_list.push(quote!(#generic_param_ident));

        self.param_list
            .push(quote!(#var_ident: #generic_param_ident));

        self.where_list.push(quote! {
            #generic_param_ident: TryInto<#rt_ident>,
            #generic_param_ident::Error: Debug + Send + 'static,
        });

        self.try_into_list.push(quote! {
            let $var_ident = $var_ident.try_into().map_error(|err|std::io::Error::new(std::io::ErrorKind::InvalidInput, format!("{}",err)))?
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

        let signer_with_provider = format_ident!(
            "{}",
            self.context
                .borrow_mut()
                .mapping
                .borrow_mut()
                .rt_type_mapping("rt_signer_with_provider")
                .ok_or(RustBinderError::RuntimeClientType)?
        );

        let abi_encode = format_ident!(
            "{}",
            self.context
                .borrow_mut()
                .mapping
                .borrow_mut()
                .rt_type_mapping("rt_abi_encode")
                .ok_or(RustBinderError::RuntimeAbiEncodeFn)?
        );

        let transfer_ops = format_ident!(
            "{}",
            self.context
                .borrow_mut()
                .mapping
                .borrow_mut()
                .rt_type_mapping("rt_transfer_ops")
                .ok_or(RustBinderError::RuntimeTransferOps)?
        );

        let signature = &self.signature;
        let generic_param_list = &self.generic_param_list;
        let param_list: &Vec<TokenStream> = &self.param_list;
        let where_list = &self.where_list;
        let try_into_list = &self.try_into_list;
        let param_encode_list: &Vec<TokenStream> = &self.param_encode_list;

        let fn_stream = quote! {
            /// Deploy contract with provided client.
            pub async fn deploy<C, #(#generic_param_list,)* Ops>(client: C, #(#param_list,)* transfer_ops: Ops) -> std::io::Result<Self>
            where
                C: TryInto<#signer_with_provider>,
                C:: Error: Debug + Send + 'static,
                Ops: TryInto<#transfer_ops>,
                Ops:: Error: Debug + Send + 'static,
                #(#where_list,)*
            {
                let client = client.try_into().map_err(|err|std::io::Error::new(std::io::ErrorKind::InvalidInput, format!("{}",err)))?;

                let transfer_ops = transfer_ops.try_into().map_err(|err|std::io::Error::new(std::io::ErrorKind::InvalidInput, format!("{}",err)))?;

                #(#try_into_list;)*

                let params = #abi_encode((#(#param_encode_list,)*))
                    .map_err(|err|std::io::Error::new(std::io::ErrorKind::InvalidInput, format!("{}",err)))?;

                let address = client.deploy(#bytecode, #signature, params, transfer_ops).await
                    .map_err(|err|std::io::Error::new(std::io::ErrorKind::Other, format!("{}",err)))?;

                Ok(Self::new(client, address))
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
        _index: usize,
        param: &crate::typedef::Parameter,
    ) -> Result<(), Self::Error> {
        let generic_param_ident = to_generic_param_ident(param);

        let var_ident = to_var_ident(param);

        let rt_ident = to_rt_ident(&mut self.context.borrow_mut().mapping.borrow_mut(), param);

        self.generic_param_list.push(quote!(#generic_param_ident));

        self.param_list
            .push(quote!(#var_ident: #generic_param_ident));

        self.where_list.push(quote! {
            #generic_param_ident: TryInto<#rt_ident>,
            #generic_param_ident::Error: Debug + Send + 'static,
        });

        self.try_into_list.push(quote! {
            let $var_ident = $var_ident.try_into().map_error(|err|std::io::Error::new(std::io::ErrorKind::InvalidInput, format!("{}",err)))?
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

        let rt_ident = to_rt_ident(&mut self.context.borrow_mut().mapping.borrow_mut(), param);

        self.return_param_list.push(quote! {#rt_ident});

        Ok(())
    }

    fn finialize(&mut self, _cx: &crate::binder::BinderContext<'_>) -> Result<(), Self::Error> {
        let abi_encode = format_ident!(
            "{}",
            self.context
                .borrow_mut()
                .mapping
                .borrow_mut()
                .rt_type_mapping("rt_abi_encode")
                .ok_or(RustBinderError::RuntimeAbiEncodeFn)?
        );

        let abi_decode = format_ident!(
            "{}",
            self.context
                .borrow_mut()
                .mapping
                .borrow_mut()
                .rt_type_mapping("rt_abi_encode")
                .ok_or(RustBinderError::RuntimeAbiEncodeFn)?
        );

        let h256 = format_ident!(
            "{}",
            self.context
                .borrow_mut()
                .mapping
                .borrow_mut()
                .rt_type_mapping("rt_h256")
                .ok_or(RustBinderError::RuntimeH256Type)?
        );

        let transfer_ops = format_ident!(
            "{}",
            self.context
                .borrow_mut()
                .mapping
                .borrow_mut()
                .rt_type_mapping("rt_transfer_ops")
                .ok_or(RustBinderError::RuntimeTransferOps)?
        );

        let signature = &self.signature;
        let fn_name = format_ident!("{}", &self.fn_name);
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
                        C:: Error: Debug + Send + 'static,
                        #(#where_list,)*
                    {
                        #(#try_into_list;)*

                        let params = #abi_encode((#(#param_encode_list,)*))
                            .map_err(|err|std::io::Error::new(std::io::ErrorKind::InvalidInput, format!("{}",err)))?;

                         let call_result = self.client.call(#signature, &self.address, params).await
                            .map_err(|err|std::io::Error::new(std::io::ErrorKind::Other, format!("{}",err)))?;

                         Ok(#abi_decode(call_result)
                            .map_err(|err|std::io::Error::new(std::io::ErrorKind::InvalidInput, format!("{}",err)))?)
                    }
                }
            }

            StateMutability::Nonpayable | StateMutability::Payable => {
                quote! {
                    pub async fn #fn_name<#(#generic_param_list,)* Ops>(&self, #(#param_list,)* transfer_ops: Ops) -> std::io::Result<#h256>
                    where
                        C:: Error: Debug + Send + 'static,
                        Ops: TryInto<#transfer_ops>,
                        Ops:: Error: Debug + Send + 'static,
                        #(#where_list,)*
                    {

                        let transfer_ops = transfer_ops.try_into().map_err(|err|std::io::Error::new(std::io::ErrorKind::InvalidInput, format!("{}",err)))?;

                        #(#try_into_list;)*

                        let params = #abi_encode((#(#param_encode_list,)*))
                            .map_err(|err|std::io::Error::new(std::io::ErrorKind::InvalidInput, format!("{}",err)))?;

                         Ok(self.client.sign_and_send_transaction(#signature, &self.address, params, transfer_ops).await
                            .map_err(|err|std::io::Error::new(std::io::ErrorKind::Other, format!("{}",err)))?)
                    }
                }
            }
        };

        self.context.borrow_mut().funcs.push(fn_stream);

        Ok(())
    }
}

pub struct RustEventBinder {
    name: String,
    anonymous: bool,
}

impl EventBinder for RustEventBinder {
    type Error = RustBinderError;

    fn bind_input(
        &mut self,
        _cx: &crate::binder::BinderContext<'_>,
        index: usize,
        parameter: &crate::typedef::Parameter,
    ) -> Result<(), Self::Error> {
        todo!()
    }

    fn finialize(&mut self, cx: &crate::binder::BinderContext<'_>) -> Result<(), Self::Error> {
        todo!()
    }
}

pub struct RustErrorBinder {
    name: String,
}

impl ErrorBinder for RustErrorBinder {
    type Error = RustBinderError;

    fn bind_input(
        &mut self,
        _cx: &crate::binder::BinderContext<'_>,
        index: usize,
        parameter: &crate::typedef::Parameter,
    ) -> Result<(), Self::Error> {
        todo!()
    }

    fn finialize(&mut self, cx: &crate::binder::BinderContext<'_>) -> Result<(), Self::Error> {
        todo!()
    }
}
