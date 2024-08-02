use crate::{abi::{Constructor, Error, Event, Function, StateMutability}, bind::{Contract, Generator, RuntimeBinder}};
use heck::{ToSnakeCase, ToUpperCamelCase};
use quote::{format_ident, quote};

use super::RustGenerator;

impl Generator for RustGenerator {
    fn begin<R: RuntimeBinder>(
        &mut self,
        _runtime_binder: &mut R,
        name: &str,
    ) -> anyhow::Result<()> {
        self.new_contract(name);

        Ok(())
    }

    fn end<R: RuntimeBinder>(
        &mut self,
        _runtime_binder: &mut R,
        _name: &str,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    fn finalize<R: RuntimeBinder>(
        self,
        runtime_binder: &mut R,
    ) -> anyhow::Result<Vec<Contract>> {
        let client_type = self.to_runtime_type_token_stream(runtime_binder, "rt_client")?;
        let adress = self.to_runtime_type_token_stream(runtime_binder, "address")?;

        let mut contracts = vec![];

        for c in &self.contracts {
            contracts.push(c.finalize(&client_type, &adress)?);
        }

        Ok(contracts)
    }

    fn generate_deploy<R: RuntimeBinder>(
        &mut self,
        runtime_binder: &mut R,
        contructor: &Constructor,
        deploy_bytes: &str,
    ) -> anyhow::Result<()> {
        let client_type = self.to_runtime_type_token_stream(runtime_binder, "rt_client")?;

        let opts_type = self.to_runtime_type_token_stream(runtime_binder, "rt_opts")?;

        let abi_encode = self.to_runtime_type_token_stream(runtime_binder, "rt_abi_serialize")?;

        let error_type = self.to_runtime_type_token_stream(runtime_binder, "rt_error")?;

        let generic_list = self.to_generic_list(runtime_binder, &contructor.inputs)?;

        let param_list = self.to_param_list(runtime_binder, &contructor.inputs)?;

        let where_clause_list = self.to_where_clause_list(runtime_binder, &contructor.inputs)?;

        let try_into_list = self.to_try_into_list(runtime_binder, &contructor.inputs)?;

        let abi_encode_list = self.to_abi_encode_list(runtime_binder, &contructor.inputs)?;

        let fn_signature = contructor.signature();

        self.current_contract().add_fn_token_stream(quote! {
            pub async fn deploy_with<C, #(#generic_list,)* Ops>(client: C, #(#param_list,)* ops: Ops) -> std::result::Result<Self,#error_type>
            where C: TryInto<#client_type>, C::Error: std::error::Error + Sync + Send + 'static,
            Ops: TryInto<#opts_type>, Ops::Error: std::error::Error + Sync + Send + 'static,
            #(#where_clause_list,)*
            {
                let mut client = client.try_into()?;
                #(#try_into_list;)*
                let ops = ops.try_into()?;

                let outputs = #abi_encode(&#abi_encode_list)?;

                let address = client.deploy_contract(#fn_signature, outputs,#deploy_bytes,ops).await?;

                Ok(Self{ client, address })
            }

            pub async fn deploy<C, #(#generic_list,)* Ops>(client: C, #(#param_list,)*) -> std::result::Result<Self,#error_type>
            where C: TryInto<#client_type>, C::Error: std::error::Error + Sync + Send + 'static,
            #(#where_clause_list,)*
            {
                let mut client = client.try_into()?;
                #(#try_into_list;)*
            
                let outputs = #abi_encode(&#abi_encode_list)?;

                let address = client.deploy_contract(#fn_signature, outputs,#deploy_bytes, Default::default()).await?;

                Ok(Self{ client, address })
            }
        });

        Ok(())
    }

    fn generate_error<R: RuntimeBinder>(
        &mut self,
        _runtime_binder: &mut R,
        _error: &Error,
    ) -> anyhow::Result<()> {
        // Skip generate error binding code
        Ok(())
    }

    fn generate_event<R: RuntimeBinder>(
        &mut self,
        runtime_binder: &mut R,
        event: &Event,
    ) -> anyhow::Result<()> {
        log::trace!("generate event {}", event.name);

        let event_field_list = self.to_event_field_list(runtime_binder, &event.inputs)?;

        let serialize_derive_macro =
            self.to_runtime_type_token_stream(runtime_binder, "rt_serialize_derive")?;

        let deserialize_derive_macro =
            self.to_runtime_type_token_stream(runtime_binder, "rt_deserialize_derive")?;

        let event_ident = format_ident!(
            "{}{}",
            self.current_contract().contract_name.to_upper_camel_case(),
            event.name.to_upper_camel_case()
        );

        let abi_json = serde_json::to_string(event)?;

        self.current_contract().add_event_token_stream(quote! {
            #[derive(#serialize_derive_macro,#deserialize_derive_macro)]
            pub struct #event_ident {
                #(#event_field_list,)*
            }

            impl #event_ident {
                pub fn abi_json() -> &'static str {
                    #abi_json
                }
            }
        });
    
        Ok(())
    }

    fn generate_fn<R: RuntimeBinder>(
        &mut self,
        runtime_binder: &mut R,
        function: &Function,
    ) -> anyhow::Result<()> {
        log::trace!("genearte fn {}", function.name);

        let opts_type = self.to_runtime_type_token_stream(runtime_binder, "rt_opts")?;

        let error_type = self.to_runtime_type_token_stream(runtime_binder, "rt_error")?;

        let abi_decode = self.to_runtime_type_token_stream(runtime_binder, "rt_abi_deserialize")?;

        let abi_encode = self.to_runtime_type_token_stream(runtime_binder, "rt_abi_serialize")?;

        let receipt_type = self.to_runtime_type_token_stream(runtime_binder, "rt_receipt")?;

        let generic_list = self.to_generic_list(runtime_binder, &function.inputs)?;

        let param_list = self.to_param_list(runtime_binder, &function.inputs)?;

        let where_clause_list = self.to_where_clause_list(runtime_binder, &function.inputs)?;

        let try_into_list = self.to_try_into_list(runtime_binder, &function.inputs)?;

        let abi_encode_list = self.to_abi_encode_list(runtime_binder, &function.inputs)?;

        let outputs_type = self.to_outputs_type(runtime_binder, &function.outputs)?;

        let fn_ident = format_ident!("{}", function.name.to_snake_case());
        let fn_with_ident = format_ident!("{}_with", function.name.to_snake_case());

        let send_transaction = match function.state_mutability {
            StateMutability::Pure | StateMutability::View => false,
            _ => true,
        };

        let fn_signature = function.signature();

        if send_transaction {
            self.current_contract().add_fn_token_stream(quote! {
                pub async fn #fn_with_ident<Ops, #(#generic_list,)* >(&self, #(#param_list,)* ops: Ops) -> std::result::Result<#receipt_type,#error_type>
                where Ops: TryInto<#opts_type>, Ops::Error: std::error::Error + Sync + Send + 'static, #(#where_clause_list,)*
                {
                    #(#try_into_list;)*
                    let ops = ops.try_into()?;

                    let outputs = #abi_encode(&#abi_encode_list)?;

                    self.client.send_raw_transaction(#fn_signature, &self.address, outputs,ops).await
                }

                pub async fn #fn_ident<#(#generic_list,)* >(&self, #(#param_list,)*) -> std::result::Result<#receipt_type,#error_type>
                where #(#where_clause_list,)*
                {
                    #(#try_into_list;)*
                  
                    let outputs = #abi_encode(&#abi_encode_list)?;

                    self.client.send_raw_transaction(#fn_signature, &self.address, outputs, Default::default()).await
                }
            });
        } else {
            self.current_contract().add_fn_token_stream(quote! {
                pub async fn #fn_ident<#(#generic_list,)* >(&self, #(#param_list,)*) -> std::result::Result<#outputs_type,#error_type>
                where #(#where_clause_list,)*
                {
                    #(#try_into_list;)*

                    let outputs = #abi_encode(&#abi_encode_list)?;

                    let inputs = self.client.eth_call(#fn_signature, &self.address, outputs).await?;

                    Ok(#abi_decode(inputs)?)
                }
            });
        }

        Ok(())
    }
}