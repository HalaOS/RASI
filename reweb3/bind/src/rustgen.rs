//! This module provides an offical code [`binder`](crate::binder::Binder) implementation for rust code.

use std::rc::Rc;

use crate::{
    binder::{Binder, ConstructorBinder, ContractBinder, ErrorBinder, EventBinder, FunctionBinder},
    mapping::BinderTypeMapping,
    typedef::StateMutability,
};

/// Error type that raised by the functions in this mod.
#[derive(Debug, thiserror::Error)]
pub enum RustBinderError {}

/// The code generator for rust language.
pub struct RustBinder(Rc<BinderTypeMapping>);

impl From<BinderTypeMapping> for RustBinder {
    fn from(value: BinderTypeMapping) -> Self {
        Self(Rc::new(value))
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

/// An individual contract generator
pub struct RustContractBinder {
    contract_name: String,
    mapping: Rc<BinderTypeMapping>,
}

impl RustContractBinder {
    fn new(contract_name: &str, mapping: Rc<BinderTypeMapping>) -> Self {
        Self {
            contract_name: contract_name.to_string(),
            mapping,
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
        state: &crate::typedef::StateMutability,
    ) -> Result<Self::ConstructorBinder, Self::Error> {
        Ok(RustConstructorBinder {
            state: state.clone(),
        })
    }

    fn bind_function(
        &mut self,
        _cx: &crate::binder::BinderContext<'_>,
        state: &crate::typedef::StateMutability,
    ) -> Result<Self::FunctionBinder, Self::Error> {
        Ok(RustFunctionBinder {
            state: state.clone(),
        })
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

pub struct RustConstructorBinder {
    state: StateMutability,
}

impl ConstructorBinder for RustConstructorBinder {
    type Error = RustBinderError;

    fn bind_input(
        &mut self,
        _cx: &crate::binder::BinderContext<'_>,
        index: usize,
        parameter: &crate::typedef::Parameter,
    ) -> Result<(), Self::Error> {
        todo!()
    }

    fn finialize(
        &mut self,
        cx: &crate::binder::BinderContext<'_>,
    ) -> Result<proc_macro2::TokenStream, Self::Error> {
        todo!()
    }
}

pub struct RustFunctionBinder {
    state: StateMutability,
}

impl FunctionBinder for RustFunctionBinder {
    type Error = RustBinderError;

    fn bind_input(
        &mut self,
        _cx: &crate::binder::BinderContext<'_>,
        index: usize,
        parameter: &crate::typedef::Parameter,
    ) -> Result<(), Self::Error> {
        todo!()
    }

    fn bind_output(
        &mut self,
        _cx: &crate::binder::BinderContext<'_>,
        index: usize,
        parameter: &crate::typedef::Parameter,
    ) -> Result<(), Self::Error> {
        todo!()
    }

    fn finialize(
        &mut self,
        cx: &crate::binder::BinderContext<'_>,
    ) -> Result<proc_macro2::TokenStream, Self::Error> {
        todo!()
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

    fn finialize(
        &mut self,
        cx: &crate::binder::BinderContext<'_>,
    ) -> Result<proc_macro2::TokenStream, Self::Error> {
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

    fn finialize(
        &mut self,
        cx: &crate::binder::BinderContext<'_>,
    ) -> Result<proc_macro2::TokenStream, Self::Error> {
        todo!()
    }
}
