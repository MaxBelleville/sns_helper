use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{DeriveInput, Expr, Field, Lit, Type, Visibility};

#[derive(Clone, Copy, Default)]
struct Constraint {
    min: Option<f64>,
    max: Option<f64>,
    is_finite: bool,
}

impl Constraint {
    fn merge(&mut self, other: Self) {
        if let Some(min) = other.min {
            self.min = Some(self.min.map_or(min, |current| current.max(min)));
        }
        if let Some(max) = other.max {
            self.max = Some(self.max.map_or(max, |current| current.min(max)));
        }
        self.is_finite |= other.is_finite;
    }

    fn is_empty(self) -> bool {
        self.min.is_none() && self.max.is_none() && !self.is_finite
    }
}

#[derive(Clone, Copy)]
enum NumericKind {
    F32,
    I32,
}

pub fn make_validate(input: &DeriveInput, fields: &[&Field]) -> syn::Result<TokenStream> {
    let name = &input.ident;
    let struct_constraint = parse_constraints(&input.attrs)?;
    let mut checks = Vec::new();
    let mut setters = Vec::new();

    for field in fields {
        if is_skipped(field) {
            continue;
        }

        let field_constraint = parse_constraints(&field.attrs)?;
        if field_constraint.is_empty() && struct_constraint.is_empty() {
            continue;
        }

        let Some(kind) = numeric_kind(&field.ty) else {
            if field_constraint.is_empty() {
                continue;
            }
            return Err(syn::Error::new_spanned(
                field,
                "field-level `constraint` attributes are supported only on f32 and i32 fields",
            ));
        };
        let ident = field
            .ident
            .as_ref()
            .expect("named fields validated by derive entrypoint");
        let mut constraint = struct_constraint;
        constraint.merge(field_constraint);

        let min = match kind {
            NumericKind::F32 => cast_f32_bound(constraint.min, field)?,
            NumericKind::I32 => cast_i32_min(constraint.min, field)?.map(f64::from),
        };
        let max = match kind {
            NumericKind::F32 => cast_f32_bound(constraint.max, field)?,
            NumericKind::I32 => cast_i32_max(constraint.max, field)?.map(f64::from),
        };
        if let (Some(min), Some(max)) = (min, max) {
            if min > max {
                return Err(syn::Error::new_spanned(
                    field,
                    "effective constraint minimum exceeds maximum",
                ));
            }
        }

        let min_check = min.map(|min| match kind {
            NumericKind::F32 => quote!(!(self.#ident >= (#min as f32))),
            NumericKind::I32 => quote!(!(self.#ident >= (#min as i32))),
        });
        let max_check = max.map(|max| match kind {
            NumericKind::F32 => quote!(!(self.#ident <= (#max as f32))),
            NumericKind::I32 => quote!(!(self.#ident <= (#max as i32))),
        });
        let finite = constraint.is_finite && matches!(kind, NumericKind::F32);
        let finite_check = if finite {
            quote!(!self.#ident.is_finite())
        } else {
            quote!(false)
        };
        let any_checks = min_check
            .into_iter()
            .chain(max_check)
            .chain(std::iter::once(finite_check));
        let rules = format_rules(min, max, finite);
        checks.push(quote! {
            if false #(|| #any_checks)* {
                issues.push(format!(
                    "field `{}` violates constraints ({})",
                    stringify!(#ident),
                    #rules
                ));
            }
        });

        let vis: &Visibility = &field.vis;
        let setter = format_ident!("{}_clamped", ident);
        let (setter_type, setter_logic) = match kind {
            NumericKind::F32 => {
                let min_expr = min.map(|value| quote!(value = value.max(#value as f32);));
                let max_expr = max.map(|value| quote!(value = value.min(#value as f32);));
                (
                    quote!(f32),
                    quote! {
                        if value.is_nan() {
                            return Err(format!("field `{}` cannot be set to NaN", stringify!(#ident)));
                        }
                        #min_expr
                        #max_expr
                        if !value.is_finite() {
                            return Err(format!("field `{}` cannot be clamped to a finite value", stringify!(#ident)));
                        }
                    },
                )
            }
            NumericKind::I32 => {
                let min_expr = min.map(|bound| quote!(value = value.max(#bound as i32);));
                let max_expr = max.map(|bound| quote!(value = value.min(#bound as i32);));
                (quote!(i32), quote! { #min_expr #max_expr })
            }
        };
        setters.push(quote! {
            #vis fn #setter(&mut self, mut value: #setter_type) -> Result<(), String> {
                #setter_logic
                self.#ident = value;
                Ok(())
            }
        });
    }

    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    Ok(TokenStream::from(quote! {
        impl #impl_generics #name #ty_generics #where_clause {
            pub fn validate(&self) -> Result<(), Vec<String>> {
                let mut issues = Vec::<String>::new();
                #(#checks)*
                if issues.is_empty() { Ok(()) } else { Err(issues) }
            }

            #(#setters)*
        }
    }))
}

fn numeric_kind(ty: &Type) -> Option<NumericKind> {
    let Type::Path(path) = ty else { return None };
    let segment = path.path.segments.last()?;
    match segment.ident.to_string().as_str() {
        "f32" => Some(NumericKind::F32),
        "i32" => Some(NumericKind::I32),
        _ => None,
    }
}

fn is_skipped(field: &Field) -> bool {
    field
        .attrs
        .iter()
        .any(|attr| attr.path().is_ident("skip_field"))
}

fn parse_constraints(attrs: &[syn::Attribute]) -> syn::Result<Constraint> {
    let mut result = Constraint::default();
    let mut seen_attribute = false;
    for attr in attrs
        .iter()
        .filter(|attr| attr.path().is_ident("constraint"))
    {
        if seen_attribute {
            return Err(syn::Error::new_spanned(
                attr,
                "use one `constraint` attribute per item",
            ));
        }
        seen_attribute = true;
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("is_finite") {
                result.is_finite = true;
                return Ok(());
            }
            if meta.path.is_ident("min") {
                if result.min.is_some() {
                    return Err(meta.error("duplicate `min` constraint"));
                }
                result.min = Some(parse_number(&meta.value()?.parse::<Expr>()?)?);
                return Ok(());
            }
            if meta.path.is_ident("max") {
                if result.max.is_some() {
                    return Err(meta.error("duplicate `max` constraint"));
                }
                result.max = Some(parse_number(&meta.value()?.parse::<Expr>()?)?);
                return Ok(());
            }
            Err(meta.error("supported constraints are `min`, `max`, and `is_finite`"))
        })?;
    }
    for value in [result.min, result.max].into_iter().flatten() {
        if !value.is_finite() {
            return Err(syn::Error::new(
                proc_macro2::Span::call_site(),
                "constraint bounds must be finite numeric literals",
            ));
        }
    }
    Ok(result)
}

fn parse_number(expr: &Expr) -> syn::Result<f64> {
    match expr {
        Expr::Lit(expr) => match &expr.lit {
            Lit::Float(value) => value.base10_parse(),
            Lit::Int(value) => value.base10_parse(),
            _ => Err(syn::Error::new_spanned(
                expr,
                "constraint bounds must be numeric literals",
            )),
        },
        Expr::Unary(expr) if matches!(expr.op, syn::UnOp::Neg(_)) => Ok(-parse_number(&expr.expr)?),
        _ => Err(syn::Error::new_spanned(
            expr,
            "constraint bounds must be numeric literals",
        )),
    }
}

fn cast_f32_bound(bound: Option<f64>, field: &Field) -> syn::Result<Option<f64>> {
    bound
        .map(|value| {
            let value = value as f32;
            if value.is_finite() {
                Ok(f64::from(value))
            } else {
                Err(syn::Error::new_spanned(
                    field,
                    "constraint bound is outside the f32 range",
                ))
            }
        })
        .transpose()
}

fn cast_i32_min(bound: Option<f64>, field: &Field) -> syn::Result<Option<i32>> {
    cast_i32_bound(bound.map(f64::ceil), field)
}

fn cast_i32_max(bound: Option<f64>, field: &Field) -> syn::Result<Option<i32>> {
    cast_i32_bound(bound.map(f64::floor), field)
}

fn cast_i32_bound(bound: Option<f64>, field: &Field) -> syn::Result<Option<i32>> {
    bound
        .map(|value| {
            if value < i32::MIN as f64 || value > i32::MAX as f64 {
                Err(syn::Error::new_spanned(
                    field,
                    "constraint bound is outside the i32 range",
                ))
            } else {
                Ok(value as i32)
            }
        })
        .transpose()
}

fn format_rules(min: Option<f64>, max: Option<f64>, finite: bool) -> String {
    let mut rules = Vec::new();
    if let Some(min) = min {
        rules.push(format!("min = {min}"));
    }
    if let Some(max) = max {
        rules.push(format!("max = {max}"));
    }
    if finite {
        rules.push("is_finite".to_owned());
    }
    rules.join(", ")
}

#[cfg(test)]
mod tests {
    use super::make_validate;
    use syn::{Data, DeriveInput, Fields, parse_quote};

    fn fields(input: &DeriveInput) -> Vec<&syn::Field> {
        let Data::Struct(data) = &input.data else {
            panic!("test input must be a struct");
        };
        let Fields::Named(fields) = &data.fields else {
            panic!("test input must have named fields");
        };
        fields.named.iter().collect()
    }

    #[test]
    fn rejects_unknown_constraint_properties() {
        let input: DeriveInput = parse_quote! {
            #[constraint(positive)]
            struct Invalid { value: f32 }
        };
        assert!(make_validate(&input, &fields(&input)).is_err());
    }

    #[test]
    fn rejects_contradictory_effective_bounds() {
        let input: DeriveInput = parse_quote! {
            #[constraint(min = 2.0, max = 1.0)]
            struct Invalid { value: f32 }
        };
        assert!(make_validate(&input, &fields(&input)).is_err());
    }

    #[test]
    fn rejects_field_constraints_for_unsupported_types() {
        let input: DeriveInput = parse_quote! {
            struct Invalid { #[constraint(min = 0.0)] value: String }
        };
        assert!(make_validate(&input, &fields(&input)).is_err());
    }
}
