//Models and shared values essentially anything that mutiple parts of the project can use

use proc_macro::TokenStream;

use syn::spanned::Spanned;
use syn::{DeriveInput, Field, Type, parse_macro_input};
mod iterable;
mod math;
mod validate;

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
    match iterable::make_iterable(&input, &fields) {
        Ok(expansion) => expansion,
        Err(error) => error.to_compile_error().into(),
    }
}

#[proc_macro_derive(FieldValidate, attributes(constraint, skip_field))]
pub fn derive_validate(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let fields = match extract_named_fields(&input) {
        Ok(fields) => fields,
        Err(error) => return error.to_compile_error().into(),
    };
    match validate::make_validate(&input, &fields) {
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

fn field_is_skipped(field: &Field) -> bool {
    field
        .attrs
        .iter()
        .any(|attr| attr.path().is_ident("skip_field"))
}

#[derive(Clone, Copy)]
enum FieldKind {
    Scalar,
    Vec,
    Option,
}

fn container_item_type(ty: &Type) -> syn::Result<Option<(FieldKind, Type)>> {
    let Type::Path(path) = ty else {
        if matches!(ty, Type::Array(_)) {
            return Err(syn::Error::new_spanned(
                ty,
                "IterableFields does not support array containers yet",
            ));
        }
        return Ok(None);
    };
    let Some(segment) = path.path.segments.last() else {
        return Ok(None);
    };
    let kind = match segment.ident.to_string().as_str() {
        "Vec" => FieldKind::Vec,
        "Option" => FieldKind::Option,
        _ => return Ok(None),
    };
    let syn::PathArguments::AngleBracketed(args) = &segment.arguments else {
        return Err(syn::Error::new_spanned(ty, "expected Vec<T> or Option<T>"));
    };
    let mut types = args.args.iter().filter_map(|arg| match arg {
        syn::GenericArgument::Type(ty) => Some(ty.clone()),
        _ => None,
    });
    let Some(item_ty) = types.next() else {
        return Err(syn::Error::new_spanned(ty, "expected one type argument"));
    };
    if types.next().is_some() {
        return Err(syn::Error::new_spanned(ty, "expected one type argument"));
    }
    Ok(Some((kind, item_ty)))
}
