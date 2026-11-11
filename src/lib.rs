mod math;
#[cfg(feature = "simdnoise")]
mod simdnoise_backend;
mod structs;

mod noise;

#[cfg(feature = "simdnoise")]
pub use simdnoise_backend::SimdNoiseBackend;

pub mod prelude {
    #[cfg(feature = "simdnoise")]
    pub use crate::SimdNoiseBackend;
    pub use crate::math::*;
    pub use crate::noise::*;
    pub use crate::structs::*;
    pub use macros::*;
}
