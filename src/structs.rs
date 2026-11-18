#[cfg(not(feature = "bevy"))]
use serde::Deserialize;

#[cfg(feature = "bevy")]
pub use bevy_math::prelude::*;

#[cfg(not(feature = "bevy"))]
#[derive(Debug, Default, Deserialize)]
pub struct IVec3 {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}
#[cfg(not(feature = "bevy"))]
impl IVec3 {
    pub fn new(x: i32, y: i32, z: i32) -> Self {
        Self { x, y, z }
    }
}
#[cfg(not(feature = "bevy"))]
#[derive(Debug, Default, Deserialize)]
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
#[derive(Debug, Default, Deserialize)]
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
#[derive(Debug, Default, Deserialize)]
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
