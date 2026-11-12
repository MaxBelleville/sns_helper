//Models and shared values essentially anything that mutiple parts of the project can use

use proc_macro::TokenStream;

use syn::spanned::Spanned;
use syn::{DeriveInput, parse_macro_input};
mod math;

#[proc_macro_derive(FieldMath, attributes(skip_field))]
pub fn derive_make_field_math(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let fields_result: Result<Vec<&syn::Field>, syn::Error> = extract_named_fields(&input);
    if let Err(error) = fields_result {
        return error.to_compile_error().into();
    }
    let fields = fields_result.unwrap();

    math::make_field(&input, &fields)
}

#[proc_macro_derive(IterableFields, attributes(skip_field))]
pub fn derive_make_iterable_field(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let fields_result: Result<Vec<&syn::Field>, syn::Error> = extract_named_fields(&input);
    if let Err(error) = fields_result {
        return error.to_compile_error().into();
    }
    let fields = fields_result.unwrap();
    match math::make_iterable(&input, &fields) {
        Ok(expansion) => expansion,
        Err(error) => error.to_compile_error().into(),
    }
}

fn extract_named_fields(input: &syn::DeriveInput) -> syn::Result<Vec<&syn::Field>> {
    match &input.data {
        syn::Data::Struct(s) => match &s.fields {
            syn::Fields::Named(named) => Ok(named.named.iter().collect()),
            _ => Err(syn::Error::new(
                input.span(),
                "Only named-field structs are supported",
            )),
        },
        _ => Err(syn::Error::new(input.span(), "Only structs are supported")),
    }
}
