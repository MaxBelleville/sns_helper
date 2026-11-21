use crate::noise::*;
use simdnoise::{
    Cellular2Settings, CellularSettings, FbmSettings, GradientSettings, NoiseBuilder,
    RidgeSettings, TurbulenceSettings,
};

/// The built-in simdnoise implementation for the generic noise interface.
#[derive(Clone, Copy, Debug, Default)]
pub struct SimdNoiseBackend;

trait ConfigureSettings {
    fn configure(&mut self, settings: &NoiseSettings, seed: i32);
}

impl ConfigureSettings for CellularSettings {
    fn configure(&mut self, settings: &NoiseSettings, seed: i32) {
        self.with_seed(seed).with_freq(settings.frequency);
    }
}

impl ConfigureSettings for Cellular2Settings {
    fn configure(&mut self, settings: &NoiseSettings, seed: i32) {
        self.with_seed(seed).with_freq(settings.frequency);
    }
}

impl ConfigureSettings for FbmSettings {
    fn configure(&mut self, settings: &NoiseSettings, seed: i32) {
        self.with_seed(seed)
            .with_freq(settings.frequency)
            .with_octaves(settings.octaves)
            .with_gain(settings.gain)
            .with_lacunarity(settings.lacunarity);
    }
}

impl ConfigureSettings for RidgeSettings {
    fn configure(&mut self, settings: &NoiseSettings, seed: i32) {
        self.with_seed(seed)
            .with_freq(settings.frequency)
            .with_octaves(settings.octaves)
            .with_gain(settings.gain)
            .with_lacunarity(settings.lacunarity);
    }
}

impl ConfigureSettings for TurbulenceSettings {
    fn configure(&mut self, settings: &NoiseSettings, seed: i32) {
        self.with_seed(seed)
            .with_freq(settings.frequency)
            .with_octaves(settings.octaves)
            .with_gain(settings.gain)
            .with_lacunarity(settings.lacunarity);
    }
}

impl ConfigureSettings for GradientSettings {
    fn configure(&mut self, settings: &NoiseSettings, seed: i32) {
        self.with_seed(seed).with_freq(settings.frequency);
    }
}

macro_rules! impl_noise_2d {
    ($kind:ty, $constructor:ident) => {
        impl Noise2d<$kind> for SimdNoiseBackend {
            fn sample_point_2d(&self, x: f32, y: f32, seed: i32, settings: &NoiseSettings) -> f32 {
                let mut builder = NoiseBuilder::$constructor(x, 1, y, 1);
                builder.configure(settings, seed);
                builder.generate().0[0]
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
                let mut builder = NoiseBuilder::$constructor(x, w, y, h);
                builder.configure(settings, seed);
                builder.generate_scaled(settings.range.x, settings.range.y)
            }
        }
    };
}

macro_rules! impl_noise_3d {
    ($kind:ty, $constructor:ident) => {
        impl Noise3d<$kind> for SimdNoiseBackend {
            fn sample_point_3d(
                &self,
                x: f32,
                y: f32,
                z: f32,
                seed: i32,
                settings: &NoiseSettings,
            ) -> f32 {
                let mut builder = NoiseBuilder::$constructor(x, 1, y, 1, z, 1);
                builder.configure(settings, seed);
                builder.generate().0[0]
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
                let mut builder = NoiseBuilder::$constructor(x, w, y, h, z, d);
                builder.configure(settings, seed);
                builder.generate_scaled(settings.range.x, settings.range.y)
            }
        }
    };
}

impl_noise_2d!(Cell2D, cellular_2d_offset);
impl_noise_3d!(Cell3D, cellular_3d_offset);
impl_noise_2d!(TwoCell2D, cellular2_2d_offset);
impl_noise_3d!(TwoCell3D, cellular2_3d_offset);
impl_noise_2d!(Fbm2D, fbm_2d_offset);
impl_noise_3d!(Fbm3D, fbm_3d_offset);
impl_noise_2d!(Ridge2D, ridge_2d_offset);
impl_noise_3d!(Ridge3D, ridge_3d_offset);
impl_noise_2d!(Turbulence2D, turbulence_2d_offset);
impl_noise_3d!(Turbulence3D, turbulence_3d_offset);
impl_noise_2d!(Gradient2D, gradient_2d_offset);
impl_noise_3d!(Gradient3D, gradient_3d_offset);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::noise::NoiseSampler;

    #[test]
    fn simdnoise_backend_every_type() {
        let settings =
            NoiseSettings::new_range(0.05, 3, 0.5, 2.0, crate::structs::Vec2::new(0.0, 1.0));
        let backend = SimdNoiseBackend;
        let builder = NoiseSampler::new(backend, settings);

        assert!(builder.sample_point_2d::<Cell2D>(1.0, 2.0, 42).is_finite());
        assert!(
            builder
                .sample_point_3d::<Cell3D>(1.0, 2.0, 3.0, 42)
                .is_finite()
        );
        assert!(
            builder
                .sample_point_2d::<TwoCell2D>(1.0, 2.0, 42)
                .is_finite()
        );
        assert!(
            builder
                .sample_point_3d::<TwoCell3D>(1.0, 2.0, 3.0, 42)
                .is_finite()
        );

        assert!(builder.sample_point_2d::<Fbm2D>(1.0, 2.0, 42).is_finite());
        assert!(
            builder
                .sample_point_3d::<Fbm3D>(1.0, 2.0, 3.0, 42)
                .is_finite()
        );
        assert!(builder.sample_point_2d::<Ridge2D>(1.0, 2.0, 42).is_finite());
        assert!(
            builder
                .sample_point_3d::<Ridge3D>(1.0, 2.0, 3.0, 42)
                .is_finite()
        );
        assert!(
            builder
                .sample_point_2d::<Turbulence2D>(1.0, 2.0, 42)
                .is_finite()
        );
        assert!(
            builder
                .sample_point_3d::<Turbulence3D>(1.0, 2.0, 3.0, 42)
                .is_finite()
        );
        assert!(
            builder
                .sample_point_2d::<Gradient2D>(1.0, 2.0, 42)
                .is_finite()
        );
        assert!(
            builder
                .sample_point_3d::<Gradient3D>(1.0, 2.0, 3.0, 42)
                .is_finite()
        );

        let field_2d = builder.sample_2d::<Fbm2D>(1.0, 2.0, 4, 3, 42);
        let field_3d = builder.sample_3d::<Fbm3D>(1.0, 2.0, 3.0, 4, 3, 2, 42);
        assert_eq!(field_2d.len(), 12);
        assert_eq!(field_3d.len(), 24);
        assert!(field_2d.iter().all(|v| v.is_finite()));
        assert!(field_3d.iter().all(|v| v.is_finite()));

        let vec_field_2d = builder.sample_vec2::<Fbm2D>(
            crate::structs::Vec2::new(1.0, 2.0),
            crate::structs::UVec2 { x: 4, y: 3 },
            42,
        );
        let vec_field_3d = builder.sample_vec3::<Fbm3D>(
            crate::structs::Vec3::new(1.0, 2.0, 3.0),
            crate::structs::UVec3 { x: 4, y: 3, z: 2 },
            42,
        );
        assert_eq!(vec_field_2d.len(), 12);
        assert_eq!(vec_field_3d.len(), 24);
    }

    #[test]
    fn bulk_range_is_applied_by_backend_but_points_remain_raw() {
        let base_settings =
            NoiseSettings::new_range(0.05, 3, 0.5, 2.0, crate::structs::Vec2::new(0.0, 1.0));
        let scaled_settings =
            NoiseSettings::new_range(0.05, 3, 0.5, 2.0, crate::structs::Vec2::new(-2.0, 2.0));
        let base = NoiseSampler::new(SimdNoiseBackend, base_settings);
        let scaled = NoiseSampler::new(SimdNoiseBackend, scaled_settings);

        let point_2d = base.sample_point_2d::<Fbm2D>(1.0, 2.0, 42);
        let point_3d = base.sample_point_3d::<Fbm3D>(1.0, 2.0, 3.0, 42);
        assert_eq!(scaled.sample_point_2d::<Fbm2D>(1.0, 2.0, 42), point_2d);
        assert_eq!(scaled.sample_point_3d::<Fbm3D>(1.0, 2.0, 3.0, 42), point_3d);

        let field_2d = base.sample_2d::<Fbm2D>(1.0, 2.0, 3, 2, 42);
        let scaled_field_2d = scaled.sample_2d::<Fbm2D>(1.0, 2.0, 3, 2, 42);
        assert!(
            field_2d
                .iter()
                .zip(scaled_field_2d)
                .all(|(a, b)| (b - (a * 4.0 - 2.0)).abs() < 1e-5)
        );

        let field_3d = base.sample_3d::<Fbm3D>(1.0, 2.0, 3.0, 3, 2, 2, 42);
        let scaled_field_3d = scaled.sample_3d::<Fbm3D>(1.0, 2.0, 3.0, 3, 2, 2, 42);
        assert!(
            field_3d
                .iter()
                .zip(scaled_field_3d)
                .all(|(a, b)| (b - (a * 4.0 - 2.0)).abs() < 1e-5)
        );
    }
}
