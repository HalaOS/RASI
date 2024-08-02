mod gen;
pub use gen::*;

mod pretty;
pub use pretty::*;

mod token_stream;
pub use token_stream::*;

use crate::bind::{Executor, JsonRuntimeBinder};

pub type BindingBuilder = crate::bind::BindingBuilder<Executor<RustGenerator, JsonRuntimeBinder>>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gen() {
        _ = pretty_env_logger::try_init();

        let runtime_binder: JsonRuntimeBinder = include_str!("../../../macros/tests/mapping.json")
            .parse()
            .expect("Parse mapping");

        let mut contracts = BindingBuilder::new((RustGenerator::default(), runtime_binder))
            .bind_hardhat(include_str!("../../../macros/tests/abi.json"))
            .finalize()
            .expect("Generate data");

        contracts.pretty().expect("Pretty");

        // contracts.save_to("./");
    }
}
