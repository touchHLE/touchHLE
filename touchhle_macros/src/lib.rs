/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::collections::HashSet;

use quote::ToTokens;

use syn::{parse2, spanned::Spanned, Expr};

use proc_macro::TokenStream;

use std::sync::{LazyLock, Mutex};

mod c_function_expr;
mod objc_expr;
mod parsing;

use c_function_expr::CFunction;
use objc_expr::ObjcClasses;
use parsing::extract_array_reference;

static EXPORTED_FUNCTIONS: LazyLock<Mutex<HashSet<String>>> =
    LazyLock::new(|| Mutex::new(HashSet::new()));

static EXPORTED_CLASSES: LazyLock<Mutex<HashSet<String>>> =
    LazyLock::new(|| Mutex::new(HashSet::new()));

/// Validates that no duplicate paths exist in a function or class list.
///
/// This attribute macro processes a constant array of path expressions and ensures
/// each path appears only once. If a duplicate is found, a compile error is generated.
///
/// # Arguments
/// * `_` - Unused attribute arguments
/// * `item` - TokenStream containing the const array of paths to validate
///
/// # Returns
/// * The original TokenStream if validation passes, or a compile error if duplicates are found
///
/// # Example
/// ```ignore
/// #[no_duplicate_paths]
/// const FUNCTION_LISTS: &[super::FunctionExports] = &[
///     "libc::clocale::FUNCTIONS",
///     "libc:ctype::FUNCTIONS"
/// ];
/// ```
#[proc_macro_attribute]
pub fn no_duplicate_paths(_: TokenStream, item: TokenStream) -> TokenStream {
    let function_list = syn::parse_macro_input!(item as syn::ItemConst);

    let mut function_paths = HashSet::new();

    let functions = match extract_array_reference(&function_list.expr) {
        Ok(functions) => functions,
        Err(err) => return err.to_compile_error().into(),
    };

    for function in &functions.elems {
        let function_path = match function {
            Expr::Path(path) => path,
            _ => {
                return syn::Error::new(function.span(), "Expected a path expression")
                    .into_compile_error()
                    .into()
            }
        };

        if !function_paths.insert(function_path) {
            return syn::Error::new(function.span(), "Duplicate path export")
                .into_compile_error()
                .into();
        }
    }

    function_list.to_token_stream().into()
}

/// Validates function exports to ensure no duplicate function names exist across the codebase.
///
/// This attribute macro processes a constant array of function macros and maintains a global
/// registry of exported function names. If a function name is exported multiple times across
/// different modules, a compile error is generated.
///
/// # Arguments
/// * `_` - Unused attribute arguments
/// * `item` - TokenStream containing the const array of function macros to validate
///
/// # Returns
/// * The original TokenStream if validation passes, or a compile error if duplicates are found
///
/// # Example
/// ```ignore
/// #[validate_function_exports]
/// const FUNCTIONS: FunctionExports= &[
///     export_c_func!(example_function()),
///     export_c_func!(another_function(_,_))
/// ];
/// ```
#[proc_macro_attribute]
pub fn validate_function_exports(_: TokenStream, item: TokenStream) -> TokenStream {
    let macro_list = syn::parse_macro_input!(item as syn::ItemConst);

    let function_macros = match extract_array_reference(&macro_list.expr) {
        Ok(functions) => functions,
        Err(err) => return err.to_compile_error().into(),
    };

    let mut parsed_c_functions = Vec::new();

    for function_macro in &function_macros.elems {
        let function_def = match function_macro {
            Expr::Macro(function_def) => function_def,
            _ => {
                return syn::Error::new(function_macro.span(), "Expected an macro expression")
                    .to_compile_error()
                    .into()
            }
        };
        match parse2::<CFunction>(function_def.mac.tokens.clone()) {
            Ok(c_function) => parsed_c_functions.push(c_function),
            Err(err) => return err.to_compile_error().into(),
        };
    }

    let mut exported_functions = EXPORTED_FUNCTIONS
        .lock()
        .expect("Failed to acquire lock on EXPORTED_FUNCTIONS");

    for c_function in parsed_c_functions {
        if !exported_functions.insert(c_function.to_string()) {
            return syn::Error::new(
                c_function
                    .function_alias
                    .map_or(c_function.function_ident.span(), |alias| alias.span()),
                "Duplicate function export",
            )
            .into_compile_error()
            .into();
        }
    }

    macro_list.to_token_stream().into()
}

/// Validates class exports to ensure no duplicate class or method names exist.
///
/// This attribute macro processes Objective-C class definitions and ensures that:
/// 1. No class name is exported multiple times across the codebase
/// 2. Within each class, no method name is duplicated
///
/// # Arguments
/// * `_` - Unused attribute arguments
/// * `item` - TokenStream containing the class definitions to validate
///
/// # Returns
/// * The original TokenStream if validation passes, or a compile error if duplicates are found
///
/// # Example
/// ```ignore
/// #[validate_class_exports]
/// const CLASSES: ClassExports = objc_classes! {
///
/// (env, this, _cmd);
///
/// @implementation ExampleClass: ExampleSuper
///
/// - (()) testMethod {
///
/// }
///
/// @end
/// };
/// ```
#[proc_macro_attribute]
pub fn validate_class_exports(_: TokenStream, item: TokenStream) -> TokenStream {
    let function_list = syn::parse_macro_input!(item as syn::ItemConst);

    let class_export = match &*function_list.expr {
        Expr::Macro(class_export) => class_export,
        _ => {
            return syn::Error::new(function_list.expr.span(), "Expected a macro expression")
                .to_compile_error()
                .into()
        }
    };

    let tokens = class_export.mac.tokens.clone();
    let classes = match parse2::<ObjcClasses>(tokens) {
        Ok(classes) => classes,
        Err(err) => return err.to_compile_error().into(),
    };

    let mut class_names = EXPORTED_CLASSES
        .lock()
        .expect("Failed to acquire lock on EXPORTED_CLASSES");

    for class in classes.classes {
        let class_name = class.class_ident.to_string();
        if !class_names.insert(class_name) {
            return syn::Error::new(class.class_ident.span(), "Duplicate class export")
                .into_compile_error()
                .into();
        }

        let mut methods = HashSet::new();
        for method in &class.class_methods {
            if !methods.insert(method) {
                return syn::Error::new(method.identity.span(), "Duplicate method export")
                    .into_compile_error()
                    .into();
            }
        }
    }

    function_list.to_token_stream().into()
}
