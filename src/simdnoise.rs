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
            fn sample_2d(&self, x: f32, y: f32, seed: i32, settings: &NoiseSettings) -> f32 {
                let mut builder = NoiseBuilder::$constructor(x, 1, y, 1);
                builder.configure(settings, seed);
                builder.generate().0[0]
            }
        }
    };
}

macro_rules! impl_noise_3d {
    ($kind:ty, $constructor:ident) => {
        impl Noise3d<$kind> for SimdNoiseBackend {
            fn sample_3d(
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
        let settings = NoiseSettings::new(0.05, 3, 0.5, 2.0, 1.0);
        let backend = SimdNoiseBackend;
        let builder = NoiseSampler::new(backend, settings);

        assert!(builder.sample_2d::<Cell2D>(1.0, 2.0, 42).is_finite());
        assert!(builder.sample_3d::<Cell3D>(1.0, 2.0, 3.0, 42).is_finite());
        assert!(builder.sample_2d::<TwoCell2D>(1.0, 2.0, 42).is_finite());
        assert!(
            builder
                .sample_3d::<TwoCell3D>(1.0, 2.0, 3.0, 42)
                .is_finite()
        );

        assert!(builder.sample_2d::<Fbm2D>(1.0, 2.0, 42).is_finite());
        assert!(builder.sample_3d::<Fbm3D>(1.0, 2.0, 3.0, 42).is_finite());
        assert!(builder.sample_2d::<Ridge2D>(1.0, 2.0, 42).is_finite());
        assert!(builder.sample_3d::<Ridge3D>(1.0, 2.0, 3.0, 42).is_finite());
        assert!(builder.sample_2d::<Turbulence2D>(1.0, 2.0, 42).is_finite());
        assert!(
            builder
                .sample_3d::<Turbulence3D>(1.0, 2.0, 3.0, 42)
                .is_finite()
        );
        assert!(builder.sample_2d::<Gradient2D>(1.0, 2.0, 42).is_finite());
        assert!(
            builder
                .sample_3d::<Gradient3D>(1.0, 2.0, 3.0, 42)
                .is_finite()
        );
    }
}
