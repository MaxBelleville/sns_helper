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
