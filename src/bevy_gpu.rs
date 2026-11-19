//! Optional adapter from the backend-agnostic GPU API to Bevy's render device and queue.

use crate::gpu::{GpuBufferHandle, GpuBufferUsage, GpuCompletion, GpuDevice, GpuError};
use bevy_render::render_resource::{
    Buffer, BufferDescriptor, BufferUsages, CommandEncoderDescriptor, PollType,
};
use bevy_render::renderer::{RenderDevice, RenderQueue};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

static NEXT_HANDLE: AtomicU64 = AtomicU64::new(1);

struct RegisteredBuffer {
    buffer: Buffer,
    size_bytes: usize,
    usage: BufferUsages,
}

/// A [`GpuDevice`] implementation backed by Bevy's render device and queue.
///
/// Construct this in the render world with its `RenderDevice` and `RenderQueue` resources:
///
/// ```ignore
/// let gpu = BevyGpuDevice::new((*render_device).clone(), (*render_queue).clone());
/// let gpu_buffer = GpuBuffer::<u32>::new(&gpu, 256, usage)?;
/// let render_buffer = gpu.buffer(gpu_buffer.handle())?;
/// ```
/// Handles returned by this adapter are IDs into a synchronized resource registry and should
/// only be passed back to the same adapter instance.
///
/// Completion polling is available on native and WebGPU targets. `wait` blocks through Bevy's
/// device polling on native targets; on WebGPU it returns
/// [`GpuError::BlockingWaitUnsupported`] until the queue callback has completed, since blocking
/// the browser event loop would prevent that callback from running.
pub struct BevyGpuDevice {
    render_device: RenderDevice,
    render_queue: RenderQueue,
    buffers: Mutex<HashMap<u64, RegisteredBuffer>>,
}

impl BevyGpuDevice {
    pub fn new(render_device: RenderDevice, render_queue: RenderQueue) -> Self {
        Self {
            render_device,
            render_queue,
            buffers: Mutex::new(HashMap::new()),
        }
    }

    /// Clones a Bevy buffer resource for use in render bindings or command encoders.
    /// The returned clone shares the same underlying GPU allocation.
    pub fn buffer(&self, handle: GpuBufferHandle) -> Result<Buffer, GpuError> {
        let buffers = self
            .buffers
            .lock()
            .map_err(|_| GpuError::Backend("GPU buffer registry lock poisoned".into()))?;
        buffers
            .get(&handle.raw())
            .map(|registered| registered.buffer.clone())
            .ok_or_else(|| GpuError::Backend(format!("unknown GPU buffer handle {}", handle.raw())))
    }

    fn completion(&self) -> Box<dyn GpuCompletion> {
        let complete = Arc::new(AtomicBool::new(false));
        let callback_flag = Arc::clone(&complete);
        self.render_queue
            .on_submitted_work_done(move || callback_flag.store(true, Ordering::Release));
        Box::new(BevyGpuCompletion {
            render_device: self.render_device.clone(),
            complete,
        })
    }
}

struct BevyGpuCompletion {
    render_device: RenderDevice,
    complete: Arc<AtomicBool>,
}

impl GpuCompletion for BevyGpuCompletion {
    fn is_complete(&self) -> Result<bool, GpuError> {
        if self.complete.load(Ordering::Acquire) {
            return Ok(true);
        }
        self.render_device
            .poll(PollType::Poll)
            .map_err(|error| GpuError::Backend(error.to_string()))?;
        Ok(self.complete.load(Ordering::Acquire))
    }

    fn wait(&self) -> Result<(), GpuError> {
        if self.complete.load(Ordering::Acquire) {
            return Ok(());
        }
        #[cfg(target_arch = "wasm32")]
        {
            Err(GpuError::BlockingWaitUnsupported)
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.render_device
                .poll(PollType::Wait {
                    submission_index: None,
                    timeout: None,
                })
                .map_err(|error| GpuError::Backend(error.to_string()))?;
            if self.complete.load(Ordering::Acquire) {
                Ok(())
            } else {
                Err(GpuError::Backend(
                    "GPU poll returned before completion callback ran".into(),
                ))
            }
        }
    }
}

impl GpuDevice for BevyGpuDevice {
    fn create_buffer(
        &self,
        size_bytes: usize,
        usage: GpuBufferUsage,
    ) -> Result<GpuBufferHandle, GpuError> {
        if size_bytes == 0 {
            return Err(GpuError::Backend(
                "Bevy GPU buffers must have non-zero size".into(),
            ));
        }
        let size = u64::try_from(size_bytes).map_err(|_| GpuError::CapacityOverflow)?;
        let bevy_usage = convert_usage(usage)?;
        let max_buffer_size = self.render_device.limits().max_buffer_size;
        if size > max_buffer_size {
            return Err(GpuError::Backend(format!(
                "buffer size {size} exceeds Bevy device limit {max_buffer_size}"
            )));
        }

        let buffer = self.render_device.create_buffer(&BufferDescriptor {
            label: Some("sns_helper GPU buffer"),
            size,
            usage: bevy_usage,
            mapped_at_creation: false,
        });
        let id = allocate_handle_id()?;
        let mut buffers = self
            .buffers
            .lock()
            .map_err(|_| GpuError::Backend("GPU buffer registry lock poisoned".into()))?;
        buffers.insert(
            id,
            RegisteredBuffer {
                buffer,
                size_bytes,
                usage: bevy_usage,
            },
        );
        Ok(GpuBufferHandle::new(id))
    }

    fn destroy_buffer(&self, handle: GpuBufferHandle) -> Result<(), GpuError> {
        let removed = self
            .buffers
            .lock()
            .map_err(|_| GpuError::Backend("GPU buffer registry lock poisoned".into()))?
            .remove(&handle.raw());
        removed
            .map(drop)
            .ok_or_else(|| GpuError::Backend(format!("unknown GPU buffer handle {}", handle.raw())))
    }

    fn upload_buffer(
        &self,
        handle: GpuBufferHandle,
        offset_bytes: usize,
        data: &[u8],
    ) -> Result<Box<dyn GpuCompletion>, GpuError> {
        validate_copy_alignment(offset_bytes, data.len())?;
        let offset = u64::try_from(offset_bytes).map_err(|_| GpuError::CapacityOverflow)?;
        let buffers = self
            .buffers
            .lock()
            .map_err(|_| GpuError::Backend("GPU buffer registry lock poisoned".into()))?;
        let registered = buffers.get(&handle.raw()).ok_or_else(|| {
            GpuError::Backend(format!("unknown GPU buffer handle {}", handle.raw()))
        })?;
        if !registered.usage.contains(BufferUsages::COPY_DST) {
            return Err(GpuError::MissingUsage("transfer_dst"));
        }
        let end = offset_bytes
            .checked_add(data.len())
            .ok_or(GpuError::CapacityOverflow)?;
        if end > registered.size_bytes {
            return Err(GpuError::RangeOutOfBounds {
                start: offset_bytes,
                end,
                capacity: registered.size_bytes,
            });
        }
        if data.is_empty() {
            return Ok(Box::new(ImmediateCompletion));
        }
        self.render_queue
            .write_buffer(&registered.buffer, offset, data);
        self.render_queue.submit([]);
        Ok(self.completion())
    }

    fn copy_buffer(
        &self,
        src: GpuBufferHandle,
        src_offset_bytes: usize,
        dst: GpuBufferHandle,
        dst_offset_bytes: usize,
        size_bytes: usize,
    ) -> Result<Box<dyn GpuCompletion>, GpuError> {
        validate_copy_alignment(src_offset_bytes, size_bytes)?;
        validate_copy_alignment(dst_offset_bytes, size_bytes)?;
        let src_end = src_offset_bytes
            .checked_add(size_bytes)
            .ok_or(GpuError::CapacityOverflow)?;
        let dst_end = dst_offset_bytes
            .checked_add(size_bytes)
            .ok_or(GpuError::CapacityOverflow)?;
        let buffers = self
            .buffers
            .lock()
            .map_err(|_| GpuError::Backend("GPU buffer registry lock poisoned".into()))?;
        let source = buffers
            .get(&src.raw())
            .ok_or_else(|| GpuError::Backend(format!("unknown GPU buffer handle {}", src.raw())))?;
        let destination = buffers
            .get(&dst.raw())
            .ok_or_else(|| GpuError::Backend(format!("unknown GPU buffer handle {}", dst.raw())))?;
        if !source.usage.contains(BufferUsages::COPY_SRC) {
            return Err(GpuError::MissingUsage("transfer_src"));
        }
        if !destination.usage.contains(BufferUsages::COPY_DST) {
            return Err(GpuError::MissingUsage("transfer_dst"));
        }
        validate_byte_range(src_offset_bytes, src_end, source.size_bytes)?;
        validate_byte_range(dst_offset_bytes, dst_end, destination.size_bytes)?;
        if size_bytes == 0 {
            return Ok(Box::new(ImmediateCompletion));
        }

        let mut encoder = self
            .render_device
            .create_command_encoder(&CommandEncoderDescriptor {
                label: Some("sns_helper GPU buffer copy"),
            });
        if src.raw() == dst.raw() {
            if src_offset_bytes < dst_end && dst_offset_bytes < src_end {
                return Err(GpuError::OverlappingCopy {
                    src_start: src_offset_bytes,
                    dst_start: dst_offset_bytes,
                    len: size_bytes,
                });
            }
            let size = u64::try_from(size_bytes).map_err(|_| GpuError::CapacityOverflow)?;
            let temporary = self.render_device.create_buffer(&BufferDescriptor {
                label: Some("sns_helper same-buffer copy staging"),
                size,
                usage: BufferUsages::COPY_SRC | BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            encoder.copy_buffer_to_buffer(
                &source.buffer,
                u64::try_from(src_offset_bytes).map_err(|_| GpuError::CapacityOverflow)?,
                &temporary,
                0,
                size,
            );
            encoder.copy_buffer_to_buffer(
                &temporary,
                0,
                &destination.buffer,
                u64::try_from(dst_offset_bytes).map_err(|_| GpuError::CapacityOverflow)?,
                size,
            );
            self.render_queue.submit([encoder.finish()]);
            drop(temporary);
        } else {
            encoder.copy_buffer_to_buffer(
                &source.buffer,
                u64::try_from(src_offset_bytes).map_err(|_| GpuError::CapacityOverflow)?,
                &destination.buffer,
                u64::try_from(dst_offset_bytes).map_err(|_| GpuError::CapacityOverflow)?,
                u64::try_from(size_bytes).map_err(|_| GpuError::CapacityOverflow)?,
            );
            self.render_queue.submit([encoder.finish()]);
        }
        drop(buffers);
        Ok(self.completion())
    }
}

struct ImmediateCompletion;
impl GpuCompletion for ImmediateCompletion {
    fn is_complete(&self) -> Result<bool, GpuError> {
        Ok(true)
    }
    fn wait(&self) -> Result<(), GpuError> {
        Ok(())
    }
}

fn validate_copy_alignment(offset: usize, size: usize) -> Result<(), GpuError> {
    const ALIGNMENT: usize = 4;
    if offset % ALIGNMENT != 0 || size % ALIGNMENT != 0 {
        return Err(GpuError::Backend(format!(
            "Bevy GPU buffer offsets and sizes must be multiples of {ALIGNMENT} bytes"
        )));
    }
    Ok(())
}

fn allocate_handle_id() -> Result<u64, GpuError> {
    let mut current = NEXT_HANDLE.load(Ordering::Relaxed);
    loop {
        let next = current.checked_add(1).ok_or(GpuError::CapacityOverflow)?;
        match NEXT_HANDLE.compare_exchange_weak(current, next, Ordering::Relaxed, Ordering::Relaxed)
        {
            Ok(_) => return Ok(current),
            Err(actual) => current = actual,
        }
    }
}

fn convert_usage(usage: GpuBufferUsage) -> Result<BufferUsages, GpuError> {
    let mut converted = BufferUsages::empty();
    if usage.storage {
        converted |= BufferUsages::STORAGE;
    }
    if usage.transfer_src {
        converted |= BufferUsages::COPY_SRC;
    }
    if usage.transfer_dst {
        converted |= BufferUsages::COPY_DST;
    }
    if converted.is_empty() {
        return Err(GpuError::Backend(
            "Bevy GPU buffers require at least one usage flag".into(),
        ));
    }
    Ok(converted)
}

fn validate_byte_range(start: usize, end: usize, capacity: usize) -> Result<(), GpuError> {
    if end > capacity {
        return Err(GpuError::RangeOutOfBounds {
            start,
            end,
            capacity,
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_wgpu_copy_alignment() {
        assert!(validate_copy_alignment(0, 4).is_ok());
        assert!(validate_copy_alignment(4, 8).is_ok());
        assert!(validate_copy_alignment(2, 4).is_err());
        assert!(validate_copy_alignment(0, 2).is_err());
    }

    #[test]
    fn maps_generic_usages_to_bevy_buffer_usages() {
        let usages = convert_usage(GpuBufferUsage {
            storage: true,
            transfer_src: true,
            transfer_dst: true,
        })
        .unwrap();
        assert!(usages.contains(BufferUsages::STORAGE));
        assert!(usages.contains(BufferUsages::COPY_SRC));
        assert!(usages.contains(BufferUsages::COPY_DST));
        assert!(convert_usage(GpuBufferUsage::default()).is_err());
    }

    #[test]
    fn validates_byte_ranges_without_overflow() {
        assert!(validate_byte_range(8, 16, 16).is_ok());
        assert!(matches!(
            validate_byte_range(8, 17, 16),
            Err(GpuError::RangeOutOfBounds { .. })
        ));
        assert!(matches!(8usize.checked_add(usize::MAX), None));
    }
}
