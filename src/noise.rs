use macros::IterableFields;

use super::structs::{UVec2, UVec3, Vec2, Vec3};

/// Parameters shared by noise implementations.
#[derive(Clone, Copy, Debug, PartialEq, Default, IterableFields)]
pub struct NoiseSettings {
    pub frequency: f32,
    pub octaves: u8,
    pub gain: f32,
    pub lacunarity: f32,
    /// Target minimum and maximum for backend-generated fields.
    /// Point-sampling methods return raw backend values and ignore this range.
    pub range: Vec2,
}

impl NoiseSettings {
    pub fn new(frequency: f32, octaves: u8, gain: f32, lacunarity: f32) -> Self {
        Self {
            frequency,
            octaves,
            gain,
            lacunarity,
            range: Vec2::new(-1.0, 1.0),
        }
    }
    pub fn new_range(frequency: f32, octaves: u8, gain: f32, lacunarity: f32, range: Vec2) -> Self {
        Self {
            frequency,
            octaves,
            gain,
            lacunarity,
            range,
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
    fn sample_point_2d(&self, x: f32, y: f32, seed: i32, settings: &NoiseSettings) -> f32;
    fn sample_2d(
        &self,
        x: f32,
        y: f32,
        w: usize,
        h: usize,
        seed: i32,
        settings: &NoiseSettings,
    ) -> Vec<f32>;
}

/// A backend that can sample three-dimensional noise families.
pub trait Noise3d<K: NoiseKind3d> {
    fn sample_point_3d(&self, x: f32, y: f32, z: f32, seed: i32, settings: &NoiseSettings) -> f32;
    fn sample_3d(
        &self,
        x: f32,
        y: f32,
        z: f32,
        w: usize,
        h: usize,
        d: usize,
        seed: i32,
        settings: &NoiseSettings,
    ) -> Vec<f32>;
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
    pub fn sample_point_2d<K: NoiseKind2d>(&self, x: f32, y: f32, seed: i32) -> f32
    where
        T: Noise2d<K>,
    {
        <T as Noise2d<K>>::sample_point_2d(&self.backend, x, y, seed, &self.settings)
    }

    #[inline]
    pub fn sample_2d<K: NoiseKind2d>(
        &self,
        x: f32,
        y: f32,
        w: usize,
        h: usize,
        seed: i32,
    ) -> Vec<f32>
    where
        T: Noise2d<K>,
    {
        <T as Noise2d<K>>::sample_2d(&self.backend, x, y, w, h, seed, &self.settings)
    }

    #[inline]
    pub fn sample_point_vec2<K: NoiseKind2d>(&self, pos: Vec2, seed: i32) -> f32
    where
        T: Noise2d<K>,
    {
        <T as Noise2d<K>>::sample_point_2d(&self.backend, pos.x, pos.y, seed, &self.settings)
    }
    #[inline]
    pub fn sample_vec2<K: NoiseKind2d>(&self, pos: Vec2, size: UVec2, seed: i32) -> Vec<f32>
    where
        T: Noise2d<K>,
    {
        <T as Noise2d<K>>::sample_2d(
            &self.backend,
            pos.x,
            pos.y,
            size.x as usize,
            size.y as usize,
            seed,
            &self.settings,
        )
    }

    #[inline]
    pub fn sample_point_3d<K: NoiseKind3d>(&self, x: f32, y: f32, z: f32, seed: i32) -> f32
    where
        T: Noise3d<K>,
    {
        <T as Noise3d<K>>::sample_point_3d(&self.backend, x, y, z, seed, &self.settings)
    }
    #[inline]
    pub fn sample_3d<K: NoiseKind3d>(
        &self,
        x: f32,
        y: f32,
        z: f32,
        w: usize,
        h: usize,
        d: usize,
        seed: i32,
    ) -> Vec<f32>
    where
        T: Noise3d<K>,
    {
        <T as Noise3d<K>>::sample_3d(&self.backend, x, y, z, w, h, d, seed, &self.settings)
    }

    #[inline]
    pub fn sample_point_vec3<K: NoiseKind3d>(&self, pos: Vec3, seed: i32) -> f32
    where
        T: Noise3d<K>,
    {
        <T as Noise3d<K>>::sample_point_3d(&self.backend, pos.x, pos.y, pos.z, seed, &self.settings)
    }
    #[inline]
    pub fn sample_vec3<K: NoiseKind3d>(&self, pos: Vec3, size: UVec3, seed: i32) -> Vec<f32>
    where
        T: Noise3d<K>,
    {
        <T as Noise3d<K>>::sample_3d(
            &self.backend,
            pos.x,
            pos.y,
            pos.z,
            size.x as usize,
            size.y as usize,
            size.z as usize,
            seed,
            &self.settings,
        )
    }
}

#[cfg(test)]
mod noise_tests {
    use super::*;

    fn normalize(values: Vec<f32>, range: Vec2) -> Vec<f32> {
        let Some((&min, &max)) = values
            .iter()
            .min_by(|a, b| a.total_cmp(b))
            .zip(values.iter().max_by(|a, b| a.total_cmp(b)))
        else {
            return values;
        };
        if min == max {
            return vec![range.x; values.len()];
        }
        values
            .into_iter()
            .map(|value| range.x + (value - min) / (max - min) * (range.y - range.x))
            .collect()
    }

    struct ConstantNoise;

    impl Noise2d<Fbm2D> for ConstantNoise {
        fn sample_point_2d(&self, x: f32, y: f32, seed: i32, settings: &NoiseSettings) -> f32 {
            x + y + seed as f32 + settings.frequency
        }

        fn sample_2d(
            &self,
            x: f32,
            y: f32,
            w: usize,
            h: usize,
            seed: i32,
            settings: &NoiseSettings,
        ) -> Vec<f32> {
            let values = (0..(w * h))
                .map(|i| {
                    let ix = i % w;
                    let iy = i / w;
                    self.sample_point_2d(x + ix as f32, y + iy as f32, seed, settings)
                })
                .collect();
            normalize(values, settings.range)
        }
    }

    impl Noise3d<Fbm3D> for ConstantNoise {
        fn sample_point_3d(
            &self,
            x: f32,
            y: f32,
            z: f32,
            seed: i32,
            _settings: &NoiseSettings,
        ) -> f32 {
            x + y + z + seed as f32 + _settings.frequency
        }

        fn sample_3d(
            &self,
            x: f32,
            y: f32,
            z: f32,
            w: usize,
            h: usize,
            d: usize,
            seed: i32,
            settings: &NoiseSettings,
        ) -> Vec<f32> {
            let values = (0..(w * h * d))
                .map(|i| {
                    let ix = i % w;
                    let iy = (i / w) % h;
                    let iz = i / (w * h);
                    self.sample_point_3d(
                        x + ix as f32,
                        y + iy as f32,
                        z + iz as f32,
                        seed,
                        settings,
                    )
                })
                .collect();
            normalize(values, settings.range)
        }
    }

    #[test]
    fn point_samples_are_raw_and_bulk_samples_use_backend_range() {
        let settings = NoiseSettings::new(2.0, 4, 0.5, 2.0);
        let builder = NoiseSampler::new(ConstantNoise, settings);

        assert_eq!(builder.sample_point_2d::<Fbm2D>(1.0, 2.0, 5), 10.0);
        assert_eq!(builder.sample_point_3d::<Fbm3D>(1.0, 2.0, 3.0, 5), 13.0);
        let field_2d = builder.sample_2d::<Fbm2D>(1.0, 2.0, 2, 2, 5);
        assert_eq!(field_2d.first(), Some(&-1.0));
        assert_eq!(field_2d.last(), Some(&1.0));
        let field_3d = builder.sample_3d::<Fbm3D>(1.0, 2.0, 3.0, 2, 1, 1, 5);
        assert_eq!(field_3d.first(), Some(&-1.0));
        assert_eq!(field_3d.last(), Some(&1.0));
    }
}
