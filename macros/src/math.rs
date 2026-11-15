use proc_macro::TokenStream;

use super::field_is_skipped;
use quote::quote;
use syn::{DeriveInput, Field};

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
