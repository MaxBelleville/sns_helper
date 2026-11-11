//Models and shared values essentially anything that mutiple parts of the project can use

use proc_macro::TokenStream;

use syn::spanned::Spanned;
use syn::{DeriveInput, parse_macro_input};
mod math;

#[proc_macro_derive(FieldMath)]
pub fn derive_make_field_math(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    let fields_result: Result<Vec<&syn::Field>, syn::Error> = extract_named_fields(&input);
    if let Err(error) = fields_result {
        return error.to_compile_error().into();
    }
    let fields = fields_result.unwrap();

    let active_fields = fields.iter().filter(|f| !math::field_is_skipped(f));

    math::make_field(name, active_fields.collect())
}

#[proc_macro_derive(Iterable, attributes(iterable))]
pub fn derive_make_iterable(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    let trait_name = math::gather_iterable_trait(&input);
    let fields_result: Result<Vec<&syn::Field>, syn::Error> = extract_named_fields(&input);

    if let Err(error) = fields_result {
        return error.to_compile_error().into();
    }
    let fields = fields_result.unwrap();

    let active_fields = fields.iter().filter(|f| !math::field_is_skipped(f));

    math::make_iterable(name, trait_name, active_fields.collect())
}

#[proc_macro_derive(IterableMut, attributes(iterable))]
pub fn derive_make_iterable_mut(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    let trait_name = math::gather_iterable_trait(&input);
    let fields_result: Result<Vec<&syn::Field>, syn::Error> = extract_named_fields(&input);

    if let Err(error) = fields_result {
        return error.to_compile_error().into();
    }
    let fields = fields_result.unwrap();

    let active_fields = fields.iter().filter(|f| !math::field_is_skipped(f));

    math::make_iterable_mut(name, trait_name, active_fields.collect())
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
