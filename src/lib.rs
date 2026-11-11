mod math;
#[cfg(feature = "simdnoise")]
mod simdnoise;
mod structs;

mod noise;

#[cfg(feature = "simdnoise")]
pub(crate) use simdnoise::SimdNoiseBackend;

pub mod prelude {
    #[cfg(feature = "simdnoise")]
    pub use crate::SimdNoiseBackend;
    pub use crate::math::*;
    pub use crate::noise::*;
    pub use crate::structs::*;
    pub use macros::*;
}
