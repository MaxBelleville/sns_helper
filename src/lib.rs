mod math;
mod noise;
#[cfg(feature = "simdnoise")]
mod simdnoise;
mod structs;
mod validation;

extern crate self as sns_helper;

/// Iterates over same-typed fields of a derived struct.
///
/// `iter` combines scalar fields, vector elements, and present optional values.
/// `try_iter` keeps optional fields as `None` and wraps other values in `Some`.
pub trait IterableField<T> {
    fn iter(&self) -> std::vec::IntoIter<&T>;
    fn iter_mut(&mut self) -> std::vec::IntoIter<&mut T>;
    fn try_iter(&self) -> std::vec::IntoIter<Option<&T>>;
    fn try_iter_mut(&mut self) -> std::vec::IntoIter<Option<&mut T>>;
}

pub mod prelude {
    pub use crate::IterableField;
    pub use crate::math::*;
    pub use crate::noise::*;
    #[cfg(feature = "simdnoise")]
    pub use crate::simdnoise::SimdNoiseBackend;
    pub use crate::structs::*;
    pub use crate::validation::*;
    pub use macros::*;
}

#[cfg(test)]
mod validation_derive_tests {
    use super::IterableField;
    use macros::{FieldValidate, IterableFields};

    #[derive(FieldValidate)]
    #[constraint(min = 0.0, max = 1.0, is_finite)]
    #[allow(dead_code)]
    struct ColorValues {
        pub r: f32,
        #[constraint(min = 0.25)]
        g: f32,
        b: f32,
        #[skip_field]
        ignored: f32,
        label: String,
    }

    #[derive(FieldValidate)]
    #[constraint(min = 0.5, max = 10.9)]
    struct IntegerValues {
        value: i32,
        #[constraint(is_finite)]
        also_value: i32,
    }

    #[derive(IterableFields)]
    struct IterableExample {
        first: f32,
        values: Vec<f32>,
        optional: Option<f32>,
    }

    #[test]
    fn validates_struct_and_field_constraints_while_skipping_fields() {
        let valid = ColorValues {
            r: 0.5,
            g: 0.25,
            b: 1.0,
            ignored: 4.0,
            label: "ignored type".into(),
        };
        assert_eq!(valid.validate(), Ok(()));

        let invalid = ColorValues {
            r: f32::INFINITY,
            g: 0.1,
            b: 1.2,
            ignored: f32::NAN,
            label: String::new(),
        };
        let errors = invalid.validate().unwrap_err();
        assert_eq!(errors.len(), 3);
        assert!(errors[0].contains("r"));
        assert!(errors[1].contains("g"));
        assert!(errors[2].contains("b"));
        assert!(errors[1].contains("min = 0.25"));
    }

    #[test]
    fn clamped_setters_handle_floats_and_integer_rounding() {
        let mut color = ColorValues {
            r: 0.5,
            g: 0.5,
            b: 0.5,
            ignored: 0.0,
            label: String::new(),
        };
        color.r_clamped(2.0).unwrap();
        color.g_clamped(-2.0).unwrap();
        assert_eq!((color.r, color.g), (1.0, 0.25));
        assert!(color.r_clamped(f32::NAN).is_err());
        color.r_clamped(f32::INFINITY).unwrap();
        assert_eq!(color.r, 1.0);

        let mut integers = IntegerValues {
            value: 5,
            also_value: 8,
        };
        integers.value_clamped(-5).unwrap();
        integers.also_value_clamped(20).unwrap();
        assert_eq!((integers.value, integers.also_value), (1, 10));
        assert_eq!(integers.validate(), Ok(()));
        let invalid_integers = IntegerValues {
            value: 0,
            also_value: 11,
        };
        assert_eq!(invalid_integers.validate().unwrap_err().len(), 2);
    }

    #[test]
    fn validate_derive_does_not_affect_iterable_fields() {
        let value = IterableExample {
            first: 1.0,
            values: vec![2.0],
            optional: Some(3.0),
        };
        assert_eq!(
            IterableField::<f32>::iter(&value)
                .copied()
                .collect::<Vec<_>>(),
            [1.0, 2.0, 3.0]
        );
    }
}
