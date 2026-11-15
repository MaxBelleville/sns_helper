use proc_macro::TokenStream;
use std::collections::HashMap;

use super::{FieldKind, container_item_type, field_is_skipped};
use quote::quote;
use syn::{DeriveInput, Field, Type};

pub fn make_iterable(input: &DeriveInput, fields: &[&Field]) -> syn::Result<TokenStream> {
    let name = &input.ident;
    let mut grouped: HashMap<String, (Type, Vec<(FieldKind, &Field)>)> = HashMap::new();

    for field in fields
        .iter()
        .copied()
        .filter(|field| !field_is_skipped(field))
    {
        let (kind, item_ty) = match container_item_type(&field.ty)? {
            Some((kind, item_ty)) => (kind, item_ty),
            None => (FieldKind::Scalar, field.ty.clone()),
        };
        let key = quote!(#item_ty).to_string();
        let entry = grouped.entry(key).or_insert_with(|| (item_ty, Vec::new()));
        entry.1.push((kind, field));
    }

    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    let implementations = grouped.into_values().map(|(item_ty, fields)| {
        let try_refs = fields.iter().map(|(kind, field)| {
            let id = field.ident.as_ref().unwrap();
            match kind {
                FieldKind::Scalar => quote!(values.push(Some(&self.#id));),
                FieldKind::Vec => quote!(values.extend(self.#id.iter().map(Some));),
                FieldKind::Option => quote!(values.push(self.#id.as_ref());),
            }
        });
        let try_muts = fields.iter().map(|(kind, field)| {
            let id = field.ident.as_ref().unwrap();
            match kind {
                FieldKind::Scalar => quote!(values.push(Some(&mut self.#id));),
                FieldKind::Vec => quote!(values.extend(self.#id.iter_mut().map(Some));),
                FieldKind::Option => quote!(values.push(self.#id.as_mut());),
            }
        });
        quote! {
            impl #impl_generics ::sns_helper::IterableField<#item_ty> for #name #ty_generics #where_clause {
                fn iter(&self) -> std::vec::IntoIter<&#item_ty> {
                    <Self as ::sns_helper::IterableField<#item_ty>>::try_iter(self)
                        .flatten().collect::<Vec<_>>().into_iter()
                }
                fn iter_mut(&mut self) -> std::vec::IntoIter<&mut #item_ty> {
                    <Self as ::sns_helper::IterableField<#item_ty>>::try_iter_mut(self)
                        .flatten().collect::<Vec<_>>().into_iter()
                }
                fn try_iter(&self) -> std::vec::IntoIter<Option<&#item_ty>> {
                    let mut values: Vec<Option<&#item_ty>> = Vec::new();
                    #(#try_refs)*
                    values.into_iter()
                }
                fn try_iter_mut(&mut self) -> std::vec::IntoIter<Option<&mut #item_ty>> {
                    let mut values: Vec<Option<&mut #item_ty>> = Vec::new();
                    #(#try_muts)*
                    values.into_iter()
                }
            }
        }
    });

    Ok(TokenStream::from(quote!(#(#implementations)*)))
}
