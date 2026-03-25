// lib.rs
// Copyright 2026 Patrick Meade.
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

use proc_macro::TokenStream;
use quote::quote;
use syn::{
    Data, DeriveInput, Error, Fields, FieldsNamed, Type, parse_macro_input, spanned::Spanned,
};

#[proc_macro_derive(CheckTemplate)]
pub fn derive_check_template(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    match expand_check_template(&input) {
        Ok(tokens) => tokens.into(),
        Err(error) => error.to_compile_error().into(),
    }
}

fn expand_check_template(input: &DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let fields = named_fields(input)?;
    let mut errors: Option<Error> = None;

    require_field(fields, "current_path", "String", &mut errors);
    require_field(fields, "locale", "Locale", &mut errors);

    if let Some(errors) = errors {
        return Err(errors);
    }

    let ident = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    Ok(quote! {
        impl #impl_generics crate::template::HasTemplateBase for #ident #ty_generics #where_clause {
            fn current_path(&self) -> &str {
                &self.current_path
            }

            fn locale(&self) -> &crate::web::Locale {
                &self.locale
            }
        }
    })
}

fn named_fields(input: &DeriveInput) -> syn::Result<&FieldsNamed> {
    match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => Ok(fields),
            Fields::Unnamed(fields) => Err(Error::new_spanned(
                fields,
                "CheckTemplate only supports structs with named fields",
            )),
            Fields::Unit => Err(Error::new_spanned(
                input,
                "CheckTemplate only supports structs with named fields",
            )),
        },
        _ => Err(Error::new_spanned(
            input,
            "CheckTemplate only supports structs with named fields",
        )),
    }
}

fn require_field(
    fields: &FieldsNamed,
    field_name: &str,
    expected_type: &str,
    errors: &mut Option<Error>,
) {
    let field = fields.named.iter().find(|field| {
        field
            .ident
            .as_ref()
            .is_some_and(|ident| ident == field_name)
    });

    match field {
        Some(field) if type_matches(&field.ty, expected_type) => {}
        Some(field) => push_error(
            errors,
            Error::new(
                field.ty.span(),
                format!(
                    "CheckTemplate requires `{field_name}: {expected_type}` on the template struct"
                ),
            ),
        ),
        None => push_error(
            errors,
            Error::new(
                fields.span(),
                format!(
                    "CheckTemplate requires a `{field_name}: {expected_type}` field on the template struct"
                ),
            ),
        ),
    }
}

fn type_matches(ty: &Type, expected: &str) -> bool {
    match ty {
        Type::Path(type_path) if type_path.qself.is_none() => type_path
            .path
            .segments
            .last()
            .is_some_and(|segment| segment.ident == expected),
        _ => false,
    }
}

fn push_error(errors: &mut Option<Error>, error: Error) {
    if let Some(existing) = errors {
        existing.combine(error);
    } else {
        *errors = Some(error);
    }
}
