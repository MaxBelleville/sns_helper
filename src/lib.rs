mod math;
#[cfg(feature = "simdnoise")]
mod simdnoise;
mod structs;

mod noise;

pub mod prelude {
    #[cfg(feature = "simdnoise")]
    pub use crate::simdnoise::SimdNoiseBackend;
    pub use crate::math::*;
    pub use crate::noise::*;
    pub use crate::structs::*;
    pub use macros::*;
}
