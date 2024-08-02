use proc_macro2::TokenStream;

use crate::bind::Contract;

pub trait ToTokenStream {
    fn to_token_streams(&self) -> anyhow::Result<Vec<TokenStream>>;
}

impl ToTokenStream for Vec<Contract> {
    fn to_token_streams(&self) -> anyhow::Result<Vec<TokenStream>> {
        let mut streams = vec![];
        for c in self {
            streams.append(&mut c.to_token_streams()?);
        }

        Ok(streams)
    }
}

impl ToTokenStream for Contract {
    fn to_token_streams(&self) -> anyhow::Result<Vec<TokenStream>> {
        let mut streams = vec![];

        for file in &self.files {
            let token_stream: TokenStream = file
                .data
                .parse()
                .map_err(|error| anyhow::format_err!("{}", error))?;

            streams.push(token_stream);
        }

        Ok(streams)
    }
}
