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
#[derive(Debug, Default)]
pub struct SampledNoise2d {
    noise: Vec<f32>,
    width: usize,
    height: usize,
    cursor: usize,
}
impl SampledNoise2d {
    pub fn new(noise: Vec<f32>, width: usize, height: usize) -> Self {
        let expected_len = width
            .checked_mul(height)
            .expect("sampled noise dimensions overflow usize");
        assert_eq!(
            noise.len(),
            expected_len,
            "noise length does not match dimensions"
        );
        Self {
            noise,
            width,
            height,
            cursor: 0,
        }
    }
    pub fn get(&self, x_idx: usize, y_idx: usize) -> f32 {
        assert!(
            x_idx < self.width && y_idx < self.height,
            "noise coordinate out of bounds"
        );
        self.noise[y_idx * self.width + x_idx]
    }
    pub fn get_vec2(&self, idx: UVec2) -> f32 {
        self.get(idx.x as usize, idx.y as usize)
    }
    pub fn get_full_noise(&self) -> &[f32] {
        &self.noise
    }
    pub fn len(&self) -> usize {
        self.noise.len()
    }
}
impl Iterator for SampledNoise2d {
    type Item = f32;
    fn next(&mut self) -> Option<Self::Item> {
        let value = self.noise.get(self.cursor).copied();
        if value.is_some() {
            self.cursor += 1;
        }
        value
    }
}

pub struct SampledNoise3d {
    noise: Vec<f32>,
    width: usize,
    height: usize,
    depth: usize,
    cursor: usize,
}
impl SampledNoise3d {
    pub fn new(noise: Vec<f32>, width: usize, height: usize, depth: usize) -> Self {
        let expected_len = width
            .checked_mul(height)
            .and_then(|area| area.checked_mul(depth))
            .expect("sampled noise dimensions overflow usize");
        assert_eq!(
            noise.len(),
            expected_len,
            "noise length does not match dimensions"
        );
        Self {
            noise,
            width,
            height,
            depth,
            cursor: 0,
        }
    }
    pub fn get(&self, x_idx: usize, y_idx: usize, z_idx: usize) -> f32 {
        assert!(
            x_idx < self.width && y_idx < self.height && z_idx < self.depth,
            "noise coordinate out of bounds"
        );
        self.noise[z_idx * self.width * self.height + y_idx * self.width + x_idx]
    }
    pub fn get_vec3(&self, idx: UVec3) -> f32 {
        self.get(idx.x as usize, idx.y as usize, idx.z as usize)
    }
    pub fn get_full_noise(&self) -> &[f32] {
        &self.noise
    }
    pub fn len(&self) -> usize {
        self.noise.len()
    }
}

impl Iterator for SampledNoise3d {
    type Item = f32;
    fn next(&mut self) -> Option<Self::Item> {
        let value = self.noise.get(self.cursor).copied();
        if value.is_some() {
            self.cursor += 1;
        }
        value
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
    ) -> SampledNoise2d;
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
    ) -> SampledNoise3d;
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
    ) -> SampledNoise2d
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
    pub fn sample_vec2<K: NoiseKind2d>(&self, pos: Vec2, size: UVec2, seed: i32) -> SampledNoise2d
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
    ) -> SampledNoise3d
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
    pub fn sample_vec3<K: NoiseKind3d>(&self, pos: Vec3, size: UVec3, seed: i32) -> SampledNoise3d
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
        ) -> SampledNoise2d {
            let values = (0..(w * h))
                .map(|i| {
                    let ix = i % w;
                    let iy = i / w;
                    self.sample_point_2d(x + ix as f32, y + iy as f32, seed, settings)
                })
                .collect();
            SampledNoise2d::new(normalize(values, settings.range), w, h)
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
        ) -> SampledNoise3d {
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
            SampledNoise3d::new(normalize(values, settings.range), w, h, d)
        }
    }

    #[test]
    fn point_samples_are_raw_and_bulk_samples_use_backend_range() {
        let settings = NoiseSettings::new(2.0, 4, 0.5, 2.0);
        let builder = NoiseSampler::new(ConstantNoise, settings);

        assert_eq!(builder.sample_point_2d::<Fbm2D>(1.0, 2.0, 5), 10.0);
        assert_eq!(builder.sample_point_3d::<Fbm3D>(1.0, 2.0, 3.0, 5), 13.0);
        let mut field_2d = builder.sample_2d::<Fbm2D>(1.0, 2.0, 2, 2, 5);
        assert_eq!(field_2d.nth(0), Some(-1.0));
        assert_eq!(field_2d.last(), Some(1.0));
        let mut field_3d = builder.sample_3d::<Fbm3D>(1.0, 2.0, 3.0, 2, 1, 1, 5);
        assert_eq!(field_3d.nth(0), Some(-1.0));
        assert_eq!(field_3d.last(), Some(1.0));
    }
}
