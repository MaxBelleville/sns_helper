#[cfg(not(feature = "bevy"))]
use serde::Deserialize;

#[cfg(feature = "bevy")]
pub use bevy_math::prelude::*;

#[cfg(not(feature = "bevy"))]
#[derive(Debug, Default, Deserialize, Clone, Copy)]
pub struct IVec3 {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}
#[cfg(not(feature = "bevy"))]
impl IVec3 {
    pub const fn new(x: i32, y: i32, z: i32) -> Self {
        Self { x, y, z }
    }
    pub const X: Self = Self::new(1, 0, 0);
    pub const Y: Self = Self::new(0, 1, 0);
    pub const Z: Self = Self::new(0, 0, 1);
    pub const NEG_X: Self = Self::new(-1, 0, 0);
    pub const NEG_Y: Self = Self::new(0, -1, 0);
    pub const NEG_Z: Self = Self::new(0, 0, -1);
}
#[cfg(not(feature = "bevy"))]
#[derive(Debug, Default, Deserialize, Clone, Copy)]
pub struct IVec2 {
    pub x: i32,
    pub y: i32,
}
#[cfg(not(feature = "bevy"))]
impl IVec2 {
    pub fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
}
#[cfg(not(feature = "bevy"))]
#[derive(Debug, Default, Deserialize, Clone, Copy)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}
#[cfg(not(feature = "bevy"))]
impl Vec3 {
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }
}
#[cfg(not(feature = "bevy"))]
#[derive(Debug, Default, Deserialize, Clone, Copy)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}
#[cfg(not(feature = "bevy"))]
impl Vec2 {
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

/// The six face directions of a cubic voxel, in the fixed ordinal order the compute
/// shader also enumerates them. `dir` is the neighbor offset and `normal` the outward
/// normal (both in cell-space); corners are the four CCW-from-outside quad corners
/// relative to the voxel's minimum corner, triangulated as (0, 1, 2) (0, 2, 3).
pub const CUBE_FACES: [CubeFace; 6] = [
    // +x: cross((b - a), (c - a)) = (1, 0, 0)
    CubeFace {
        dir: IVec3::X,
        normal: [1.0, 0.0, 0.0],
        corners: [[1, 0, 1], [1, 0, 0], [1, 1, 0], [1, 1, 1]],
    },
    // -x
    CubeFace {
        dir: IVec3::NEG_X,
        normal: [-1.0, 0.0, 0.0],
        corners: [[0, 0, 0], [0, 0, 1], [0, 1, 1], [0, 1, 0]],
    },
    // +y
    CubeFace {
        dir: IVec3::Y,
        normal: [0.0, 1.0, 0.0],
        corners: [[0, 1, 1], [1, 1, 1], [1, 1, 0], [0, 1, 0]],
    },
    // -y
    CubeFace {
        dir: IVec3::NEG_Y,
        normal: [0.0, -1.0, 0.0],
        corners: [[0, 0, 0], [1, 0, 0], [1, 0, 1], [0, 0, 1]],
    },
    // +z
    CubeFace {
        dir: IVec3::Z,
        normal: [0.0, 0.0, 1.0],
        corners: [[0, 0, 1], [1, 0, 1], [1, 1, 1], [0, 1, 1]],
    },
    // -z
    CubeFace {
        dir: IVec3::NEG_Z,
        normal: [0.0, 0.0, -1.0],
        corners: [[1, 0, 0], [0, 0, 0], [0, 1, 0], [1, 1, 0]],
    },
];

/// One exposed-face direction of a cubic voxel.
#[derive(Clone, Copy, Debug)]
pub struct CubeFace {
    pub dir: IVec3,
    pub normal: [f32; 3],
    pub corners: [[u32; 3]; 4],
}

pub struct LinearAllocator<T> {
    storage: Vec<T>,
}

impl<T> LinearAllocator<T> {
    pub fn new(capacity: usize) -> Self {
        Self {
            storage: Vec::with_capacity(capacity),
        }
    }

    pub fn allocate(&mut self, value: T) -> usize {
        let index = self.storage.len();
        self.storage.push(value);
        index
    }

    pub fn get(&self, index: usize) -> &T {
        &self.storage[index]
    }

    pub fn get_mut(&mut self, index: usize) -> &mut T {
        &mut self.storage[index]
    }

    pub fn clear(&mut self) {
        self.storage.clear();
    }
    pub fn slice(&self, start: usize, count: usize) -> &[T] {
        &self.storage[start..start + count]
    }
    pub fn slice_range(&self, from: usize, to: usize) -> &[T] {
        &self.storage[from..to]
    }

    pub fn as_slice(&self) -> &[T] {
        &self.storage
    }
}

#[derive(Default, Clone, Copy)]
pub struct Range<T> {
    start: T,
    end: T,
}
impl<T> Range<T>
where
    T: Clone,
{
    pub fn new(start: T, end: T) -> Self {
        Self { start, end }
    }
    pub fn set_range(&mut self, start: T, end: T) {
        self.start = start;
        self.end = end;
    }

    pub fn set_start(&mut self, start: T) {
        self.start = start;
    }
    pub fn set_end(&mut self, end: T) {
        self.end = end;
    }
    pub fn get_range(&self) -> (T, T) {
        (self.start.clone(), self.end.clone())
    }
    pub fn start(&self) -> T {
        self.start.clone()
    }
    pub fn end(&self) -> T {
        self.end.clone()
    }
}
