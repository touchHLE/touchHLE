/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::parsing::consume_parse_buffer;
use std::fmt;
use syn::{
    parse::{Parse, ParseStream},
    Ident, LitStr, Result,
};

pub(crate) struct CFunction {
    pub(crate) function_ident: Ident,
    pub(crate) function_alias: Option<LitStr>,
}

impl Parse for CFunction {
    fn parse(input: ParseStream) -> Result<Self> {
        let function_alias = if input.peek(LitStr) {
            let alias = input.parse::<LitStr>()?;
            _ = input.parse::<syn::Token![,]>()?;
            Some(alias)
        } else {
            None
        };
        let function_ident = input.parse()?;
        let args;
        syn::parenthesized!(args in input);
        consume_parse_buffer(args);

        Ok(Self {
            function_ident,
            function_alias,
        })
    }
}

impl fmt::Display for CFunction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            self.function_alias
                .as_ref()
                .map(|a| a.value())
                .unwrap_or(self.function_ident.to_string())
        )
    }
}
