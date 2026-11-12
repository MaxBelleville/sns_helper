use proc_macro::TokenStream;
use std::collections::HashMap;

use quote::quote;
use syn::{DeriveInput, Field, Type};

pub fn field_is_skipped(field: &Field) -> bool {
    field
        .attrs
        .iter()
        .any(|attr| attr.path().is_ident("skip_field"))
}

pub fn make_field(input: &DeriveInput, fields: &[&Field]) -> TokenStream {
    let name = &input.ident;
    let active_fields: Vec<_> = fields
        .iter()
        .copied()
        .filter(|f| !field_is_skipped(f))
        .collect();
    let skipped_fields: Vec<_> = fields
        .iter()
        .copied()
        .filter(|f| field_is_skipped(f))
        .collect();
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    let add_fields = active_fields.iter().map(|f| {
        let ident = f.ident.as_ref().unwrap();
        quote!(#ident: self.#ident + rhs.#ident)
    });

    let div_fields = active_fields.iter().map(|f| {
        let ident = f.ident.as_ref().unwrap();
        quote!(#ident: self.#ident / rhs.#ident)
    });

    let mul_fields = active_fields.iter().map(|f| {
        let ident = f.ident.as_ref().unwrap();
        quote!(#ident: self.#ident * rhs.#ident)
    });

    let sub_fields = active_fields.iter().map(|f| {
        let ident = f.ident.as_ref().unwrap();
        quote!(#ident: self.#ident - rhs.#ident)
    });

    let gaussian_terms = active_fields.iter().map(|f| {
        let ident = f.ident.as_ref().unwrap();
        quote! {
            sum += ((self.#ident - center.#ident) / width.#ident).powi(2);
        }
    });

    let min_fields = active_fields.iter().map(|f| {
        let ident = f.ident.as_ref().unwrap();
        quote!(#ident: self.#ident.min(rhs.#ident))
    });

    let clamp_fields = active_fields.iter().map(|f| {
        let ident = f.ident.as_ref().unwrap();
        quote!(#ident: self.#ident.clamp(-rhs.#ident,rhs.#ident))
    });

    let lerp_fields = active_fields.iter().map(|f| {
        let ident = f.ident.as_ref().unwrap();
        quote!(#ident: self.#ident + t * (other.#ident - self.#ident))
    });

    let add_skipped = skipped_fields.iter().map(|f| f.ident.as_ref().unwrap());
    let sub_skipped = skipped_fields.iter().map(|f| f.ident.as_ref().unwrap());
    let mul_skipped = skipped_fields.iter().map(|f| f.ident.as_ref().unwrap());
    let div_skipped = skipped_fields.iter().map(|f| f.ident.as_ref().unwrap());
    let min_skipped = skipped_fields.iter().map(|f| f.ident.as_ref().unwrap());
    let clamp_skipped = skipped_fields.iter().map(|f| f.ident.as_ref().unwrap());
    let lerp_skipped = skipped_fields.iter().map(|f| f.ident.as_ref().unwrap());
    let lerp_clone_bounds = skipped_fields.iter().map(|f| &f.ty);
    let lerp_where = if skipped_fields.is_empty() {
        quote!()
    } else {
        quote!(where #(#lerp_clone_bounds: Clone),*)
    };

    let expand = quote! {
        impl #impl_generics std::ops::Add for #name #ty_generics #where_clause {
            type Output = Self;
            fn add(self, rhs: Self) -> Self {
                Self { #(#add_fields,)* #(#add_skipped: self.#add_skipped),* }
            }
        }

        impl #impl_generics std::ops::Sub for #name #ty_generics #where_clause {
            type Output = Self;
            fn sub(self, rhs: Self) -> Self {
                Self { #(#sub_fields,)* #(#sub_skipped: self.#sub_skipped),* }
            }
        }


        impl #impl_generics std::ops::Mul for #name #ty_generics #where_clause {
            type Output = Self;
            fn mul(self, rhs: Self) -> Self {
                Self { #(#mul_fields,)* #(#mul_skipped: self.#mul_skipped),* }
            }
        }

        impl #impl_generics std::ops::Div for #name #ty_generics #where_clause {
            type Output = Self;
            fn div(self, rhs: Self) -> Self {
                Self { #(#div_fields,)* #(#div_skipped: self.#div_skipped),* }
            }
        }

        impl #impl_generics #name #ty_generics #where_clause {
            pub fn gaussian(self, center: &Self, width: &Self) -> f32 {
                let mut sum = 0.0;
                #(#gaussian_terms)*
                (-sum).exp()
            }

            pub fn min(self, rhs: Self) -> Self {
                 Self { #(#min_fields,)* #(#min_skipped: self.#min_skipped),* }
            }

            pub fn clamp(self, rhs: Self) -> Self {
                 Self { #(#clamp_fields,)* #(#clamp_skipped: self.#clamp_skipped),* }
            }


            pub fn lerp(&self, other: &Self, t: f32) -> Self #lerp_where {
                Self { #(#lerp_fields,)* #(#lerp_skipped: self.#lerp_skipped.clone()),* }
            }
        }
    };
    TokenStream::from(expand)
}

/// Generate one `IterableField<T>` implementation for every distinct scalar or
/// container element type found among the non-skipped fields.
pub fn make_iterable(input: &DeriveInput, fields: &[&Field]) -> syn::Result<TokenStream> {
    let name = &input.ident;
    let mut grouped: HashMap<String, (Type, Vec<(FieldKind, &Field)>)> = HashMap::new();

    for field in fields
        .iter()
        .copied()
        .filter(|field| !field_is_skipped(field))
    {
        let ty = field.ty.clone();
        let (kind, item_ty) = match container_item_type(&ty)? {
            Some((kind, item_ty)) => (kind, item_ty),
            None => (FieldKind::Scalar, ty.clone()),
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
