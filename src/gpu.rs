use bytemuck::NoUninit;
use std::marker::PhantomData;
use std::mem::size_of;
use std::ops::Range as StdRange;

/// Owns a backend buffer handle. This type intentionally does not implement `Clone` or `Copy`.
pub struct GpuBuffer<T> {
    /// Backend-specific handle (VkBuffer, CUdeviceptr, wgpu::Buffer, etc.)
    handle: GpuBufferHandle,

    /// Number of elements allocated on GPU
    capacity: usize,

    /// Usage flags (storage, transfer, etc.)
    usage: GpuBufferUsage,

    _marker: PhantomData<T>,
}

#[derive(Clone, Copy, Default, Debug, PartialEq, Eq, Hash)]
pub struct GpuBufferHandle {
    raw: u64, // opaque backend handle
}
impl GpuBufferHandle {
    pub fn new(raw: u64) -> Self {
        Self { raw }
    }

    pub fn raw(&self) -> u64 {
        self.raw
    }
}

#[derive(Clone, Copy, Default)]
pub struct GpuBufferUsage {
    pub storage: bool,
    pub transfer_src: bool,
    pub transfer_dst: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GpuError {
    CapacityOverflow,
    RangeOutOfBounds {
        start: usize,
        end: usize,
        capacity: usize,
    },
    InvalidRange {
        start: usize,
        end: usize,
    },
    OverlappingCopy {
        src_start: usize,
        dst_start: usize,
        len: usize,
    },
    MissingUsage(&'static str),
    BlockingWaitUnsupported,
    Backend(String),
}

impl std::fmt::Display for GpuError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CapacityOverflow => write!(f, "GPU buffer byte size overflow"),
            Self::RangeOutOfBounds {
                start,
                end,
                capacity,
            } => write!(
                f,
                "GPU element range {start}..{end} exceeds capacity {capacity}"
            ),
            Self::InvalidRange { start, end } => {
                write!(f, "invalid GPU element range {start}..{end}")
            }
            Self::OverlappingCopy {
                src_start,
                dst_start,
                len,
            } => write!(
                f,
                "same-buffer copy ranges starting at {src_start} and {dst_start} with length {len} overlap",
            ),
            Self::MissingUsage(usage) => write!(f, "GPU buffer is missing {usage} usage"),
            Self::BlockingWaitUnsupported => {
                write!(
                    f,
                    "blocking GPU completion waits are unsupported on this target"
                )
            }
            Self::Backend(message) => write!(f, "GPU backend error: {message}"),
        }
    }
}
impl std::error::Error for GpuError {}

/// Completion status for a submitted upload or copy.
///
/// Implementations may report errors that occur after successful submission.
pub trait GpuCompletion: Send {
    fn is_complete(&self) -> Result<bool, GpuError>;
    fn wait(&self) -> Result<(), GpuError>;
}

/// A validated half-open range of buffer elements.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GpuRange(StdRange<usize>);
impl GpuRange {
    pub fn new(range: StdRange<usize>, capacity: usize) -> Result<Self, GpuError> {
        if range.start > range.end {
            return Err(GpuError::InvalidRange {
                start: range.start,
                end: range.end,
            });
        }
        if range.end > capacity {
            return Err(GpuError::RangeOutOfBounds {
                start: range.start,
                end: range.end,
                capacity,
            });
        }
        Ok(Self(range))
    }
    pub fn len(&self) -> usize {
        self.0.end - self.0.start
    }
    pub fn is_empty(&self) -> bool {
        self.0.start == self.0.end
    }
    pub fn start(&self) -> usize {
        self.0.start
    }
    pub fn end(&self) -> usize {
        self.0.end
    }
}

/// Backend adapter for GPU buffers. This trait has no Bevy dependency: a Bevy render-world
/// resource can implement it by mapping handles to backend resources and wrapping queue
/// submissions in [`GpuCompletion`] tokens.
pub trait GpuDevice {
    fn create_buffer(
        &self,
        size_bytes: usize,
        usage: GpuBufferUsage,
    ) -> Result<GpuBufferHandle, GpuError>;
    /// Must be safe immediately after submissions that reference this handle. Asynchronous
    /// backends defer actual reclamation until those submissions finish.
    fn destroy_buffer(&self, handle: GpuBufferHandle) -> Result<(), GpuError>;
    /// Backend offsets and data lengths are bytes.
    fn upload_buffer(
        &self,
        handle: GpuBufferHandle,
        offset_bytes: usize,
        data: &[u8],
    ) -> Result<Box<dyn GpuCompletion>, GpuError>;
    /// Backend offsets and size are bytes. Same-buffer overlapping copies must be rejected by
    /// the backend unless that backend explicitly supports them.
    fn copy_buffer(
        &self,
        src: GpuBufferHandle,
        src_offset_bytes: usize,
        dst: GpuBufferHandle,
        dst_offset_bytes: usize,
        size_bytes: usize,
    ) -> Result<Box<dyn GpuCompletion>, GpuError>;

    /// Retires a buffer handle for deferred destruction. Backends may keep the underlying
    /// allocation alive until [`Self::flush_retired_buffers`] or until GPU work referencing it
    /// completes. This reduces allocation churn when buffers are grown and replaced during a frame.
    fn retire_buffer(&self, handle: GpuBufferHandle) -> Result<(), GpuError> {
        self.destroy_buffer(handle)
    }

    /// Reclaims or pools buffers previously passed to [`Self::retire_buffer`]. Call at a safe point
    /// where no submitted GPU work still references the retired buffers, such as after a frame
    /// boundary. Implementations may keep the allocations for reuse by later `create_buffer` calls.
    fn flush_retired_buffers(&self) -> Result<(), GpuError> {
        Ok(())
    }
}

impl<T> GpuBuffer<T> {
    pub fn new(
        device: &dyn GpuDevice,
        capacity: usize,
        usage: GpuBufferUsage,
    ) -> Result<Self, GpuError> {
        let size_bytes = capacity
            .checked_mul(size_of::<T>())
            .ok_or(GpuError::CapacityOverflow)?;
        let handle = device.create_buffer(size_bytes, usage)?;
        Ok(Self {
            handle,
            capacity,
            usage,
            _marker: PhantomData,
        })
    }
    pub fn destroy(self, device: &dyn GpuDevice) -> Result<(), GpuError> {
        device.destroy_buffer(self.handle)
    }
    pub fn capacity(&self) -> usize {
        self.capacity
    }
    pub fn handle(&self) -> GpuBufferHandle {
        self.handle
    }

    /// Copies elements from `source` to this destination buffer. Offsets and lengths are elements.
    pub fn copy_from(
        &self,
        device: &dyn GpuDevice,
        source: &GpuBuffer<T>,
        src_offset: usize,
        dst_offset: usize,
        len: usize,
    ) -> Result<Box<dyn GpuCompletion>, GpuError> {
        if !source.usage.transfer_src {
            return Err(GpuError::MissingUsage("transfer_src"));
        }
        if !self.usage.transfer_dst {
            return Err(GpuError::MissingUsage("transfer_dst"));
        }
        let src_end = src_offset
            .checked_add(len)
            .ok_or(GpuError::CapacityOverflow)?;
        let dst_end = dst_offset
            .checked_add(len)
            .ok_or(GpuError::CapacityOverflow)?;
        let src = GpuRange::new(src_offset..src_end, source.capacity)?;
        let dst = GpuRange::new(dst_offset..dst_end, self.capacity)?;
        if source.handle.raw() == self.handle.raw()
            && src.start() < dst.end()
            && dst.start() < src.end()
        {
            return Err(GpuError::OverlappingCopy {
                src_start: src_offset,
                dst_start: dst_offset,
                len,
            });
        }
        let element_size = size_of::<T>();
        let src_bytes = src
            .start()
            .checked_mul(element_size)
            .ok_or(GpuError::CapacityOverflow)?;
        let dst_bytes = dst
            .start()
            .checked_mul(element_size)
            .ok_or(GpuError::CapacityOverflow)?;
        let size_bytes = src
            .len()
            .checked_mul(element_size)
            .ok_or(GpuError::CapacityOverflow)?;
        device.copy_buffer(source.handle, src_bytes, self.handle, dst_bytes, size_bytes)
    }
    /// Alias for [`GpuBuffer::copy_from`].
    pub fn copy_range_from(
        &self,
        device: &dyn GpuDevice,
        source: &GpuBuffer<T>,
        src_offset: usize,
        dst_offset: usize,
        len: usize,
    ) -> Result<Box<dyn GpuCompletion>, GpuError> {
        self.copy_from(device, source, src_offset, dst_offset, len)
    }
    pub fn copy_range(
        &self,
        device: &dyn GpuDevice,
        src_offset: usize,
        dst_offset: usize,
        len: usize,
    ) -> Result<Box<dyn GpuCompletion>, GpuError> {
        self.copy_from(device, self, src_offset, dst_offset, len)
    }
    /// Uploads elements at `offset`. Buffer offsets and lengths are elements.
    pub fn upload_range(
        &self,
        device: &dyn GpuDevice,
        offset: usize,
        data: &[T],
    ) -> Result<Box<dyn GpuCompletion>, GpuError>
    where
        T: NoUninit,
    {
        let end = offset
            .checked_add(data.len())
            .ok_or(GpuError::CapacityOverflow)?;
        GpuRange::new(offset..end, self.capacity)?;
        let byte_offset = offset
            .checked_mul(size_of::<T>())
            .ok_or(GpuError::CapacityOverflow)?;
        data.len()
            .checked_mul(size_of::<T>())
            .ok_or(GpuError::CapacityOverflow)?;
        device.upload_buffer(self.handle, byte_offset, bytemuck::cast_slice(data))
    }
    /// Grows storage, returning the copy completion token when existing data was copied.
    ///
    /// The previous buffer handle is retired through [`GpuDevice::retire_buffer`]; call
    /// [`GpuDevice::flush_retired_buffers`] at a safe point after dependent GPU work completes.
    /// The allocation is sized exactly to `new_capacity`; use [`GpuBuffer::ensure_capacity`] when
    /// geometric headroom is preferred to reduce reallocation frequency.
    pub fn grow(
        &mut self,
        device: &dyn GpuDevice,
        new_capacity: usize,
        copy_existing: bool,
    ) -> Result<Option<Box<dyn GpuCompletion>>, GpuError> {
        if new_capacity <= self.capacity {
            return Err(GpuError::InvalidRange {
                start: self.capacity,
                end: new_capacity,
            });
        }
        let new_size = new_capacity
            .checked_mul(size_of::<T>())
            .ok_or(GpuError::CapacityOverflow)?;
        let new_handle = device.create_buffer(new_size, self.usage)?;
        let completion = if copy_existing {
            if !self.usage.transfer_src || !self.usage.transfer_dst {
                let _ = device.destroy_buffer(new_handle);
                return Err(GpuError::MissingUsage(if !self.usage.transfer_src {
                    "transfer_src"
                } else {
                    "transfer_dst"
                }));
            }
            let copy_bytes = match self.capacity.checked_mul(size_of::<T>()) {
                Some(size) => size,
                None => {
                    let _ = device.destroy_buffer(new_handle);
                    return Err(GpuError::CapacityOverflow);
                }
            };
            match device.copy_buffer(self.handle, 0, new_handle, 0, copy_bytes) {
                Ok(token) => Some(token),
                Err(error) => {
                    let _ = device.destroy_buffer(new_handle);
                    return Err(error);
                }
            }
        } else {
            None
        };
        if let Err(error) = device.retire_buffer(self.handle) {
            let _ = device.destroy_buffer(new_handle);
            return Err(error);
        }
        self.handle = new_handle;
        self.capacity = new_capacity;
        Ok(completion)
    }

    /// Ensures the buffer can hold at least `min_capacity` elements, growing geometrically when
    /// needed. This reduces allocation churn compared to growing to the exact required size, and
    /// is the recommended way to size buffers that may grow repeatedly during a frame.
    ///
    /// Returns the copy completion token when existing data was copied into a newly allocated
    /// buffer. The previous buffer handle is retired through [`GpuDevice::retire_buffer`]; call
    /// [`GpuDevice::flush_retired_buffers`] at a safe point after dependent GPU work completes.
    pub fn ensure_capacity(
        &mut self,
        device: &dyn GpuDevice,
        min_capacity: usize,
        copy_existing: bool,
    ) -> Result<Option<Box<dyn GpuCompletion>>, GpuError> {
        if min_capacity <= self.capacity {
            return Ok(None);
        }
        let grown = geometric_growth_capacity(self.capacity, min_capacity);
        self.grow(device, grown, copy_existing)
    }

    /// Like [`GpuBuffer::ensure_capacity`], but rejects growth beyond `max_capacity`. This gives
    /// callers a hard VRAM budget for a single buffer, which is useful when a workload (such as
    /// blocky voxel meshing) can otherwise request very large slabs.
    pub fn ensure_capacity_within(
        &mut self,
        device: &dyn GpuDevice,
        min_capacity: usize,
        max_capacity: usize,
        copy_existing: bool,
    ) -> Result<Option<Box<dyn GpuCompletion>>, GpuError> {
        if min_capacity <= self.capacity {
            return Ok(None);
        }
        if min_capacity > max_capacity || max_capacity <= self.capacity {
            return Err(GpuError::CapacityOverflow);
        }
        let grown = geometric_growth_capacity(self.capacity, min_capacity).min(max_capacity);
        self.grow(device, grown, copy_existing)
    }
}

fn geometric_growth_capacity(current: usize, min_capacity: usize) -> usize {
    let doubled = current.saturating_mul(2).max(1);
    doubled.max(min_capacity)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    struct Immediate;

    impl GpuCompletion for Immediate {
        fn is_complete(&self) -> Result<bool, GpuError> {
            Ok(true)
        }

        fn wait(&self) -> Result<(), GpuError> {
            Ok(())
        }
    }

    #[derive(Default)]
    struct MockState {
        next: u64,
        buffers: HashMap<u64, usize>,
        retired: Vec<u64>,
        destroyed: Vec<u64>,
        copies: Vec<(u64, usize, u64, usize, usize)>,
    }

    #[derive(Clone, Default)]
    struct MockDevice {
        state: Arc<Mutex<MockState>>,
    }

    impl MockDevice {
        fn retired(&self) -> Vec<u64> {
            self.state.lock().unwrap().retired.clone()
        }

        fn destroyed(&self) -> Vec<u64> {
            self.state.lock().unwrap().destroyed.clone()
        }
    }

    impl GpuDevice for MockDevice {
        fn create_buffer(
            &self,
            size_bytes: usize,
            _usage: GpuBufferUsage,
        ) -> Result<GpuBufferHandle, GpuError> {
            let mut state = self.state.lock().unwrap();
            state.next += 1;
            let id = state.next;
            state.buffers.insert(id, size_bytes);
            Ok(GpuBufferHandle::new(id))
        }

        fn destroy_buffer(&self, handle: GpuBufferHandle) -> Result<(), GpuError> {
            let mut state = self.state.lock().unwrap();
            state
                .buffers
                .remove(&handle.raw())
                .ok_or_else(|| GpuError::Backend("unknown".into()))?;
            state.destroyed.push(handle.raw());
            Ok(())
        }

        fn upload_buffer(
            &self,
            _handle: GpuBufferHandle,
            _offset_bytes: usize,
            _data: &[u8],
        ) -> Result<Box<dyn GpuCompletion>, GpuError> {
            Ok(Box::new(Immediate))
        }

        fn copy_buffer(
            &self,
            src: GpuBufferHandle,
            src_offset_bytes: usize,
            dst: GpuBufferHandle,
            dst_offset_bytes: usize,
            size_bytes: usize,
        ) -> Result<Box<dyn GpuCompletion>, GpuError> {
            let mut state = self.state.lock().unwrap();
            state.copies.push((
                src.raw(),
                src_offset_bytes,
                dst.raw(),
                dst_offset_bytes,
                size_bytes,
            ));
            Ok(Box::new(Immediate))
        }

        fn retire_buffer(&self, handle: GpuBufferHandle) -> Result<(), GpuError> {
            let mut state = self.state.lock().unwrap();
            state
                .buffers
                .remove(&handle.raw())
                .ok_or_else(|| GpuError::Backend("unknown".into()))?;
            state.retired.push(handle.raw());
            Ok(())
        }

        fn flush_retired_buffers(&self) -> Result<(), GpuError> {
            let mut state = self.state.lock().unwrap();
            let drained: Vec<_> = state.retired.drain(..).collect();
            state.destroyed.extend(drained);
            Ok(())
        }
    }

    fn test_usage() -> GpuBufferUsage {
        GpuBufferUsage {
            storage: true,
            transfer_src: true,
            transfer_dst: true,
        }
    }

    #[test]
    fn grow_uses_requested_capacity_and_retires_old_buffer() {
        let device = MockDevice::default();
        let mut buffer = GpuBuffer::<u32>::new(&device, 4, test_usage()).unwrap();
        let old = buffer.handle();

        buffer.grow(&device, 5, true).unwrap();

        assert_eq!(buffer.capacity(), 5);
        assert_ne!(buffer.handle(), old);
        assert_eq!(device.retired(), vec![old.raw()]);
        assert!(device.destroyed().is_empty());
        device.flush_retired_buffers().unwrap();
        assert_eq!(device.destroyed(), vec![old.raw()]);
    }

    #[test]
    fn ensure_capacity_grows_geometrically_and_noops_when_sufficient() {
        let device = MockDevice::default();
        let mut buffer = GpuBuffer::<u32>::new(&device, 4, test_usage()).unwrap();

        let none = buffer.ensure_capacity(&device, 4, true).unwrap();
        assert!(none.is_none());
        assert_eq!(buffer.capacity(), 4);

        buffer.ensure_capacity(&device, 5, true).unwrap();
        assert_eq!(buffer.capacity(), 8);

        let none = buffer.ensure_capacity(&device, 8, true).unwrap();
        assert!(none.is_none());
        assert_eq!(buffer.capacity(), 8);
    }

    #[test]
    fn geometric_growth_caps_to_at_least_requested() {
        assert_eq!(geometric_growth_capacity(0, 3), 3);
        assert_eq!(geometric_growth_capacity(4, 5), 8);
        assert_eq!(geometric_growth_capacity(8, 32), 32);
    }

    #[test]
    fn ensure_capacity_within_respects_budget() {
        let device = MockDevice::default();
        let mut buffer = GpuBuffer::<u32>::new(&device, 4, test_usage()).unwrap();

        buffer
            .ensure_capacity_within(&device, 5, 6, true)
            .unwrap();
        assert_eq!(buffer.capacity(), 6);

        assert!(buffer
            .ensure_capacity_within(&device, 7, 6, true)
            .is_err());
        assert_eq!(buffer.capacity(), 6);
    }
}
