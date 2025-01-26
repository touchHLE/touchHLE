/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::parsing::consume_parse_buffer;
use core::fmt;

use syn::{
    parse::{Parse, ParseStream},
    Ident, Result,
};

pub(crate) struct ObjcClasses {
    pub(crate) classes: Vec<ObjcClass>,
}

impl Parse for ObjcClasses {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut classes = Vec::new();
        let _content;
        syn::parenthesized!(_content in input);
        consume_parse_buffer(_content);

        _ = input.parse::<syn::Token![;]>()?; // Skip the  macro: (env, this, _cmd);

        while !input.is_empty() {
            let class = input.parse()?;
            classes.push(class);
        }

        Ok(Self { classes })
    }
}

pub(crate) struct ObjcClass {
    pub(crate) class_ident: Ident,
    #[allow(dead_code)]
    parent_name: Option<Ident>,
    pub(crate) class_methods: Vec<ObjcMethod>,
}

impl Parse for ObjcClass {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut class_methods = Vec::new();
        _ = input.parse::<syn::Token![@]>()?;
        _ = input.parse::<Ident>()?; // Skip: @ implementation

        let class_name = input.parse()?;
        let parent_name = if input.peek(syn::Token![:]) {
            _ = input.parse::<syn::Token![:]>()?;
            Some(input.parse()?)
        } else {
            None
        };

        while input.peek(syn::Token![-]) || input.peek(syn::Token![+]) {
            let class_method = input.parse()?;
            class_methods.push(class_method);
        }

        _ = input.parse::<syn::Token![@]>()?;
        _ = input.parse::<Ident>()?; /* end */
        Ok(Self {
            class_ident: class_name,
            parent_name,
            class_methods,
        })
    }
}

#[derive(PartialEq, Eq, Hash)]
enum MethodType {
    Class,
    Instance,
}

impl fmt::Display for MethodType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MethodType::Class => write!(f, "+"),
            MethodType::Instance => write!(f, "-"),
        }
    }
}

#[derive(PartialEq, Eq, Hash)]
pub(crate) struct ObjcMethod {
    pub(crate) identity: Ident,
    arguments: Vec<Ident>,
    method_type: MethodType,
    is_variadic: bool,
}

impl Parse for ObjcMethod {
    fn parse(input: ParseStream) -> Result<Self> {
        let method_type = if input.peek(syn::Token![+]) {
            _ = input.parse::<syn::Token![+]>()?;
            MethodType::Class
        } else {
            _ = input.parse::<syn::Token![-]>()?;
            MethodType::Instance
        };

        let _method_type;
        syn::parenthesized!(_method_type in input);
        consume_parse_buffer(_method_type);

        let name = input.parse()?;

        let mut arguments = Vec::new();

        while input.peek(syn::Token![:]) {
            _ = input.parse::<syn::Token![:]>()?;
            let _arg_type;
            syn::parenthesized!(_arg_type in input);
            consume_parse_buffer(_arg_type);

            _ = input.parse::<Ident>()?;

            if input.peek(Ident) {
                let arg_name = input.parse()?;
                arguments.push(arg_name);
            }
        }

        // Handle variadic arguments
        let is_variadic = if input.peek(syn::Token![,]) {
            _ = input.parse::<syn::Token![,]>()?;
            _ = input.parse::<syn::Token![.]>()?;
            _ = input.parse::<syn::Token![.]>()?;
            _ = input.parse::<syn::Token![.]>()?;
            _ = input.parse::<Ident>()?;
            true
        } else {
            false
        };

        let _implementation;
        syn::braced!(_implementation in input);
        consume_parse_buffer(_implementation);

        Ok(Self {
            identity: name,
            arguments,
            method_type,
            is_variadic,
        })
    }
}
