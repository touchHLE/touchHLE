/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use syn::{parse::ParseBuffer, spanned::Spanned, Expr, ExprArray};

/// Used to consume a parse buffer. Useful for skipping tokens inside a group
pub(crate) fn consume_parse_buffer(buffer: ParseBuffer) {
    buffer
        .step(|cursor| {
            let mut rest = *cursor;
            while let Some((_, next)) = rest.token_tree() {
                rest = next;
            }
            Ok(((), rest))
        })
        .unwrap()
}

pub(crate) fn extract_array_reference(expr: &Expr) -> syn::Result<&ExprArray> {
    match expr {
        Expr::Reference(array_ref) => match &*array_ref.expr {
            Expr::Array(array) => Ok(array),
            _ => Err(syn::Error::new(
                array_ref.expr.span(),
                "Expected an array expression",
            )),
        },
        _ => Err(syn::Error::new(
            expr.span(),
            "Expected a reference to an array expression",
        )),
    }
}
