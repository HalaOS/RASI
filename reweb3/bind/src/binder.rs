//! This mod provides various types and utilities for code generation.

use std::collections::HashMap;

use proc_macro2::TokenStream;

use crate::typedef::{AbiField, Parameter, StateMutability};

/// The trait that the error returned by binder traits must implement.
pub trait BinderError: std::error::Error + Send + Sync + 'static {}

impl<T> BinderError for T where T: std::error::Error + Send + Sync + 'static {}

/// Context data to invoke [`bind`] function.
pub struct BinderContext<'a> {
    contract_name: &'a str,
    fields: &'a [AbiField],
    bytecode: Option<&'a str>,
    components: HashMap<String, Vec<Parameter>>,
}

impl<'a> BinderContext<'a> {
    /// Create a new `BinderContext` object.
    pub fn new(contract_name: &'a str, fields: &'a [AbiField], bytecode: Option<&'a str>) -> Self {
        let components = HashMap::new();

        let mut cx = BinderContext {
            contract_name,
            fields,
            bytecode,
            components,
        };

        cx.index_components();

        cx
    }

    fn index_components(&mut self) {
        for field in self.fields {
            match field {
                AbiField::Function(function) => {
                    for parameter in &function.inputs {
                        self.index_component_of_parameter(parameter);
                    }

                    for parameter in &function.outputs {
                        self.index_component_of_parameter(parameter);
                    }
                }
                AbiField::Constructor(constructor) => {
                    for parameter in &constructor.inputs {
                        self.index_component_of_parameter(parameter);
                    }
                }
                AbiField::Event(field) => {
                    for parameter in &field.inputs {
                        self.index_component_of_parameter(parameter);
                    }
                }
                AbiField::Error(field) => {
                    for parameter in &field.inputs {
                        self.index_component_of_parameter(parameter);
                    }
                }
                _ => {}
            }
        }
    }

    fn index_component_of_parameter(&mut self, parameter: &Parameter) {
        if let Some(components) = &parameter.components {
            let internal_type = parameter
                .internal_type
                .as_deref()
                .expect("internal type is null");

            if !self.components.contains_key(internal_type) {
                self.components
                    .insert(internal_type.to_owned(), components.clone());

                // recursively index component.
                for parameter in components {
                    self.index_component_of_parameter(parameter);
                }
            }
        }
    }

    /// Returns the bytecode string of compiled contract.
    pub fn bytecode(&self) -> Option<&str> {
        self.bytecode
    }
}

/// A binder is the specific-language code generator of solidity abi.
pub trait Binder {
    type Error: BinderError;

    type ContractBinder: ContractBinder<Error = Self::Error>;

    /// Start a new process of contract code generation.
    fn prepare(
        &mut self,
        cx: &BinderContext<'_>,
        contract_name: &str,
    ) -> Result<Self::ContractBinder, Self::Error>;
}

/// A trait object returns by [`prepare`](Binder::prepare) function.
pub trait ContractBinder {
    type Error: BinderError;

    type ConstructorBinder: ConstructorBinder<Error = Self::Error>;
    type FunctionBinder: FunctionBinder<Error = Self::Error>;
    type EventBinder: EventBinder<Error = Self::Error>;
    type ErrorBinder: EventBinder<Error = Self::Error>;
    type TupleBinder: TupleBinder<Error = Self::Error>;

    fn bind_tuple(
        &mut self,
        cx: &BinderContext<'_>,
        name: &str,
    ) -> Result<Self::TupleBinder, Self::Error>;

    /// This function is called to generate contract `contructor` function.
    fn bind_constructor(
        &mut self,
        cx: &BinderContext<'_>,
        signature: &str,
        state: &StateMutability,
    ) -> Result<Self::ConstructorBinder, Self::Error>;

    /// This function is called to generate contract function.
    fn bind_function(
        &mut self,
        cx: &BinderContext<'_>,
        fn_name: &str,
        signature: &str,
        state: &StateMutability,
    ) -> Result<Self::FunctionBinder, Self::Error>;

    /// This function is called to generate contract `receiver` function.
    fn bind_receiver(
        &mut self,
        cx: &BinderContext<'_>,
        state: &StateMutability,
    ) -> Result<(), Self::Error>;

    /// This function is called to generate contract `fallback` function.
    fn bind_fallback(
        &mut self,
        cx: &BinderContext<'_>,
        state: &StateMutability,
    ) -> Result<(), Self::Error>;

    /// Calling this function generates event handling related code.
    fn bind_event(
        &mut self,
        cx: &BinderContext<'_>,
        name: &str,
        signature: Option<&str>,
    ) -> Result<Self::EventBinder, Self::Error>;

    /// Calling this function generates error handling related code.
    fn bind_error(
        &mut self,
        cx: &BinderContext<'_>,
        name: &str,
        signature: &str,
    ) -> Result<Self::ErrorBinder, Self::Error>;

    /// This function is called to clean up resources after the code generation process is end.
    fn finialize(&mut self, cx: &BinderContext<'_>) -> Result<TokenStream, Self::Error>;
}

/// A trait object returns by [`bind_constructor`](ContractBinder::bind_constructor) function.
pub trait ConstructorBinder {
    type Error: BinderError;

    /// This function is called to generate parameter list of the function's.
    fn bind_input(
        &mut self,
        cx: &BinderContext<'_>,
        index: usize,
        parameter: &Parameter,
    ) -> Result<(), Self::Error>;

    /// This function is called to clean up resources after the code generation process is end.
    fn finialize(&mut self, cx: &BinderContext<'_>) -> Result<(), Self::Error>;
}

/// A trait object returns by [`bind_function`](ContractBinder::bind_function) function.
pub trait FunctionBinder {
    type Error: BinderError;

    /// This function is called to generate parameter list of the function's.
    fn bind_input(
        &mut self,
        cx: &BinderContext<'_>,
        index: usize,
        parameter: &Parameter,
    ) -> Result<(), Self::Error>;

    /// This function is called to generate output parameter list of the function's.
    fn bind_output(
        &mut self,
        cx: &BinderContext<'_>,
        index: usize,
        parameter: &Parameter,
    ) -> Result<(), Self::Error>;

    /// This function is called to clean up resources after the code generation process is end.
    fn finialize(&mut self, cx: &BinderContext<'_>) -> Result<(), Self::Error>;
}

/// A trait object returns by [`bind_event`](ContractBinder::bind_event) function.
pub trait EventBinder {
    type Error: BinderError;

    fn bind_input(
        &mut self,
        cx: &BinderContext<'_>,
        index: usize,
        parameter: &Parameter,
    ) -> Result<(), Self::Error>;

    /// This function is called to clean up resources after the code generation process is end.
    fn finialize(&mut self, cx: &BinderContext<'_>) -> Result<(), Self::Error>;
}

/// A trait object returns by [`bind_tuple`](ContractBinder::bind_tuple) function.
pub trait TupleBinder {
    type Error: BinderError;

    fn bind_input(
        &mut self,
        cx: &BinderContext<'_>,
        index: usize,
        parameter: &Parameter,
    ) -> Result<(), Self::Error>;

    /// This function is called to clean up resources after the code generation process is end.
    fn finialize(&mut self, cx: &BinderContext<'_>) -> Result<(), Self::Error>;
}

/// Invoke code generation with `context data`.
///
/// On success, returns the [`TokenStream`] of generated codes.
pub fn bind<'a, B: Binder>(cx: &BinderContext<'a>, mut binder: B) -> Result<TokenStream, B::Error> {
    let mut contract = binder.prepare(cx, &cx.contract_name)?;

    for (name, fields) in &cx.components {
        let mut binder = contract.bind_tuple(cx, name)?;

        for (index, parameter) in fields.iter().enumerate() {
            binder.bind_input(cx, index, parameter)?;
        }

        binder.finialize(cx)?;
    }

    for field in cx.fields {
        match field {
            AbiField::Function(function) => {
                let mut binder = contract.bind_function(
                    cx,
                    &function.name,
                    function.signature().as_str(),
                    &function.state_mutability,
                )?;

                for (index, parameter) in function.inputs.iter().enumerate() {
                    binder.bind_input(cx, index, parameter)?;
                }

                for (index, parameter) in function.outputs.iter().enumerate() {
                    binder.bind_output(cx, index, parameter)?;
                }

                binder.finialize(cx)?;
            }
            AbiField::Constructor(constructor) => {
                let mut binder = contract.bind_constructor(
                    cx,
                    constructor.signature().as_str(),
                    &constructor.state_mutability,
                )?;

                for (index, parameter) in constructor.inputs.iter().enumerate() {
                    binder.bind_input(cx, index, parameter)?;
                }

                binder.finialize(cx)?;
            }
            AbiField::Receive(receiver) => {
                contract.bind_receiver(cx, &receiver.state_mutability)?;
            }
            AbiField::Fallback(fallback) => {
                contract.bind_receiver(cx, &fallback.state_mutability)?;
            }
            AbiField::Event(event) => {
                let signature = if event.anonymous {
                    None
                } else {
                    Some(event.signature())
                };

                let mut binder = contract.bind_event(cx, &event.name, signature.as_deref())?;

                for (index, parameter) in event.inputs.iter().enumerate() {
                    binder.bind_input(cx, index, parameter)?;
                }

                binder.finialize(cx)?;
            }
            AbiField::Error(e) => {
                let mut binder = contract.bind_error(cx, &e.name, e.signature().as_str())?;

                for (index, parameter) in e.inputs.iter().enumerate() {
                    binder.bind_input(cx, index, parameter)?;
                }

                binder.finialize(cx)?;
            }
        }
    }

    Ok(contract.finialize(cx)?)
}

#[cfg(test)]
mod tests {

    use crate::typedef::HardhatArtifact;

    use super::BinderContext;

    #[test]
    fn binder_context_new() {
        let hardhat: HardhatArtifact = serde_json::from_str(include_str!("./abi.json")).unwrap();

        BinderContext::new(
            &hardhat.contract_name,
            &hardhat.abi,
            Some(&hardhat.bytecode),
        );
    }
}
