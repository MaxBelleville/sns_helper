use macros::IterableFields;

use super::structs::{Vec2, Vec3};

/// Parameters shared by noise implementations.
#[derive(Clone, Copy, Debug, PartialEq, IterableFields)]
pub struct NoiseSettings {
    pub frequency: f32,
    pub octaves: u8,
    pub gain: f32,
    pub lacunarity: f32,
    /// Multiplier applied to the raw sample returned by the backend.
    pub mult: f32,
}

impl NoiseSettings {
    pub const fn new(frequency: f32, octaves: u8, gain: f32, lacunarity: f32, mult: f32) -> Self {
        Self {
            frequency,
            octaves,
            gain,
            lacunarity,
            mult,
        }
    }
}

/// Marker for noise algorithms that can be sampled in two dimensions.
pub trait NoiseKind2d {}
/// Marker for noise algorithms that can be sampled in three dimensions.
pub trait NoiseKind3d {}

macro_rules! noise_kinds {
    ($(($name:ident, $dimension:ident)),+ $(,)?) => {
        $(
        pub struct $name;
        impl $dimension for $name {}
        )+
    };
}

noise_kinds!(
    (Cell2D, NoiseKind2d),
    (Cell3D, NoiseKind3d),
    (TwoCell2D, NoiseKind2d),
    (TwoCell3D, NoiseKind3d),
    (Fbm2D, NoiseKind2d),
    (Fbm3D, NoiseKind3d),
    (Ridge2D, NoiseKind2d),
    (Ridge3D, NoiseKind3d),
    (Turbulence2D, NoiseKind2d),
    (Turbulence3D, NoiseKind3d),
    (Gradient2D, NoiseKind2d),
    (Gradient3D, NoiseKind3d),
);

/// A backend that can sample two-dimensional noise families.
/// Implement this for a local adapter type to isolate the external noise crate.
pub trait Noise2d<K: NoiseKind2d> {
    fn sample_2d(&self, x: f32, y: f32, seed: i32, settings: &NoiseSettings) -> f32;
}

/// A backend that can sample three-dimensional noise families.
pub trait Noise3d<K: NoiseKind3d> {
    fn sample_3d(&self, x: f32, y: f32, z: f32, seed: i32, settings: &NoiseSettings) -> f32;
}

/// Typed noise sampler. Dimension-specific methods are available only when
/// `T` implements the corresponding sampling trait.
pub struct NoiseSampler<T> {
    backend: T,
    settings: NoiseSettings,
}

impl<T> NoiseSampler<T> {
    pub const fn new(backend: T, settings: NoiseSettings) -> Self {
        Self { backend, settings }
    }

    pub fn backend(&self) -> &T {
        &self.backend
    }

    pub const fn settings(&self) -> &NoiseSettings {
        &self.settings
    }
}

impl<T> NoiseSampler<T> {
    #[inline]
    pub fn sample_2d<K: NoiseKind2d>(&self, x: f32, y: f32, seed: i32) -> f32
    where
        T: Noise2d<K>,
    {
        <T as Noise2d<K>>::sample_2d(&self.backend, x, y, seed, &self.settings) * self.settings.mult
    }

    #[inline]
    pub fn sample_vec2<K: NoiseKind2d>(&self, vec: Vec2, seed: i32) -> f32
    where
        T: Noise2d<K>,
    {
        <T as Noise2d<K>>::sample_2d(&self.backend, vec.x, vec.y, seed, &self.settings)
            * self.settings.mult
    }

    #[inline]
    pub fn sample_3d<K: NoiseKind3d>(&self, x: f32, y: f32, z: f32, seed: i32) -> f32
    where
        T: Noise3d<K>,
    {
        <T as Noise3d<K>>::sample_3d(&self.backend, x, y, z, seed, &self.settings)
            * self.settings.mult
    }

    #[inline]
    pub fn sample_vec3<K: NoiseKind3d>(&self, vec: Vec3, seed: i32) -> f32
    where
        T: Noise3d<K>,
    {
        <T as Noise3d<K>>::sample_3d(&self.backend, vec.x, vec.y, vec.z, seed, &self.settings)
            * self.settings.mult
    }
}

#[cfg(test)]
mod noise_tests {
    use super::*;

    struct ConstantNoise;

    impl Noise2d<Fbm2D> for ConstantNoise {
        fn sample_2d(&self, x: f32, y: f32, seed: i32, settings: &NoiseSettings) -> f32 {
            x + y + seed as f32 + settings.frequency
        }
    }

    impl Noise3d<Fbm3D> for ConstantNoise {
        fn sample_3d(&self, x: f32, y: f32, z: f32, seed: i32, settings: &NoiseSettings) -> f32 {
            x + y + z + seed as f32 + settings.frequency
        }
    }

    #[test]
    fn sample_with_multi() {
        let settings = NoiseSettings::new(2.0, 4, 0.5, 2.0, 3.0);
        let builder = NoiseSampler::new(ConstantNoise, settings);

        assert_eq!(builder.sample_2d::<Fbm2D>(1.0, 2.0, 5), 30.0);
        assert_eq!(builder.sample_3d::<Fbm3D>(1.0, 2.0, 3.0, 5), 39.0);
    }
}
