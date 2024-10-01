use std::num::NonZero;

use crate::core::{RenderDevice, RenderQueue};
use wgpu::util::DeviceExt;

pub trait BufferData: bytemuck::Pod {}

impl<T: bytemuck::Pod> BufferData for T {}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub enum Indices {
    U16(Vec<u16>),
    U32(Vec<u32>),
}

impl Indices {
    pub fn len(&self) -> usize {
        match self {
            Indices::U16(v) => v.len(),
            Indices::U32(v) => v.len(),
        }
    }

    pub fn size(&self) -> usize {
        match self {
            Indices::U16(v) => std::mem::size_of::<u16>() * v.len(),
            Indices::U32(v) => std::mem::size_of::<u32>() * v.len(),
        }
    }

    pub fn bytes(&self) -> &[u8] {
        match self {
            Indices::U16(v) => bytemuck::cast_slice(&v[..]),
            Indices::U32(v) => bytemuck::cast_slice(&v[..]),
        }
    }

    pub fn is_empty(&self) -> bool {
        match self {
            Indices::U16(v) => v.is_empty(),
            Indices::U32(v) => v.is_empty(),
        }
    }

    pub fn extend(&mut self, indices: Indices) {
        match (self, indices) {
            (Indices::U16(v1), Indices::U16(v2)) => v1.extend_from_slice(&v2),
            (Indices::U32(v1), Indices::U32(v2)) => v1.extend_from_slice(&v2),
            _ => (),
        }
    }

    pub fn format(&self) -> wgpu::IndexFormat {
        match self {
            Indices::U16(_) => wgpu::IndexFormat::Uint16,
            Indices::U32(_) => wgpu::IndexFormat::Uint32,
        }
    }
}

#[derive(Clone, Copy)]
pub enum BufferKind {
    Index,
    Vertex,
    Uniform,
    Storage,
}

bitflags::bitflags! {
    #[repr(transparent)]
    #[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
    pub struct BufferFlags: u8 {
        const MAP_READ = 1 << 0;
        const MAP_WRITE = 1 << 1;
        const COPY_SRC = 1 << 2;
        const COPY_DST = 1 << 3;
        const INDIRECT = 1 << 4;
        const QUERY_RESOLVE = 1 << 5;
    }
}

impl BufferFlags {
    pub fn usages(&self, kind: BufferKind) -> wgpu::BufferUsages {
        let mut usages = wgpu::BufferUsages::empty();

        if self.contains(Self::MAP_READ) {
            usages |= wgpu::BufferUsages::MAP_READ;
        }

        if self.contains(Self::MAP_WRITE) {
            usages |= wgpu::BufferUsages::MAP_WRITE;
        }

        if self.contains(Self::COPY_SRC) {
            usages |= wgpu::BufferUsages::COPY_SRC;
        }

        if self.contains(Self::COPY_DST) {
            usages |= wgpu::BufferUsages::COPY_DST;
        }

        if self.contains(Self::INDIRECT) {
            usages |= wgpu::BufferUsages::INDIRECT;
        }

        if self.contains(Self::QUERY_RESOLVE) {
            usages |= wgpu::BufferUsages::QUERY_RESOLVE;
        }

        match kind {
            BufferKind::Index => usages |= wgpu::BufferUsages::INDEX,
            BufferKind::Vertex => usages |= wgpu::BufferUsages::VERTEX,
            BufferKind::Uniform => usages |= wgpu::BufferUsages::UNIFORM,
            BufferKind::Storage => usages |= wgpu::BufferUsages::STORAGE,
        }

        usages
    }

    pub fn is_write(&self) -> bool {
        self.contains(Self::MAP_WRITE) || self.contains(Self::COPY_DST)
    }
}

impl serde::Serialize for BufferFlags {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_u8(self.bits())
    }
}

impl<'de> serde::Deserialize<'de> for BufferFlags {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Ok(BufferFlags::from_bits_truncate(u8::deserialize(
            deserializer,
        )?))
    }
}

pub trait Vertex: bytemuck::Pod + bytemuck::Zeroable + 'static {}

pub struct VertexBuffer {
    buffer: wgpu::Buffer,
    flags: BufferFlags,
    len: u64,
}

impl VertexBuffer {
    pub fn new<V: Vertex>(device: &RenderDevice, vertices: &[V], flags: BufferFlags) -> Self {
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(vertices),
            usage: flags.usages(BufferKind::Vertex),
        });

        let len = vertices.len() as u64;

        Self { buffer, flags, len }
    }

    pub fn id(&self) -> wgpu::Id<wgpu::Buffer> {
        self.buffer.global_id()
    }

    pub fn buffer(&self) -> &wgpu::Buffer {
        &self.buffer
    }

    pub fn flags(&self) -> BufferFlags {
        self.flags
    }

    pub fn len(&self) -> u64 {
        self.len
    }

    pub fn update<V: Vertex>(
        &mut self,
        device: &RenderDevice,
        queue: &RenderQueue,
        offset: u64,
        vertices: &[V],
    ) {
        if self.flags.is_write() {
            let len = vertices.len() as u64;
            if offset + len > self.len {
                self.buffer = device.create_buffer(&wgpu::BufferDescriptor {
                    label: None,
                    size: len * std::mem::size_of::<V>() as u64,
                    usage: self.flags.usages(BufferKind::Vertex),
                    mapped_at_creation: false,
                });
                self.len = len + offset
            }

            queue.write_buffer(&self.buffer, offset, bytemuck::cast_slice(vertices));
        }
    }
}

impl std::ops::Deref for VertexBuffer {
    type Target = wgpu::Buffer;

    fn deref(&self) -> &Self::Target {
        &self.buffer
    }
}

pub struct IndexBuffer {
    buffer: wgpu::Buffer,
    flags: BufferFlags,
    format: wgpu::IndexFormat,
    len: u64,
}

impl IndexBuffer {
    pub fn new(device: &RenderDevice, indices: Indices, flags: BufferFlags) -> Self {
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: indices.bytes(),
            usage: flags.usages(BufferKind::Index),
        });

        Self {
            buffer,
            flags,
            len: indices.len() as u64,
            format: indices.format(),
        }
    }

    pub fn id(&self) -> wgpu::Id<wgpu::Buffer> {
        self.buffer.global_id()
    }

    pub fn buffer(&self) -> &wgpu::Buffer {
        &self.buffer
    }

    pub fn flags(&self) -> BufferFlags {
        self.flags
    }

    pub fn format(&self) -> wgpu::IndexFormat {
        self.format
    }

    pub fn len(&self) -> u64 {
        self.len
    }

    pub fn update(
        &mut self,
        device: &RenderDevice,
        queue: &RenderQueue,
        offset: u64,
        indices: &Indices,
    ) {
        if self.flags.is_write() {
            let len = indices.len() as u64;
            if offset + len > self.len {
                self.buffer = device.create_buffer(&wgpu::BufferDescriptor {
                    label: None,
                    size: indices.size() as u64,
                    usage: self.flags.usages(BufferKind::Index),
                    mapped_at_creation: false,
                });
                self.len = len + offset
            }

            queue.write_buffer(&self.buffer, offset, indices.bytes());
        }
    }
}

impl std::ops::Deref for IndexBuffer {
    type Target = wgpu::Buffer;

    fn deref(&self) -> &Self::Target {
        &self.buffer
    }
}

pub struct UniformBuffer<T: BufferData> {
    value: T,
    buffer: wgpu::Buffer,
    flags: BufferFlags,
    size: NonZero<u64>,
}

impl<T: BufferData> UniformBuffer<T> {
    pub fn new(device: &RenderDevice, value: T, flags: BufferFlags) -> Self {
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::bytes_of(&value),
            usage: flags.usages(BufferKind::Uniform),
        });

        Self {
            buffer,
            value,
            flags,
            size: NonZero::new(std::mem::size_of::<T>() as u64).unwrap(),
        }
    }

    pub fn value(&self) -> &T {
        &self.value
    }

    pub fn id(&self) -> wgpu::Id<wgpu::Buffer> {
        self.buffer.global_id()
    }

    pub fn buffer(&self) -> &wgpu::Buffer {
        &self.buffer
    }

    pub fn flags(&self) -> BufferFlags {
        self.flags
    }

    pub fn size(&self) -> NonZero<u64> {
        self.size
    }

    pub fn binding(&self) -> wgpu::BindingResource {
        self.buffer.as_entire_binding()
    }

    pub fn update(&mut self, queue: &RenderQueue, value: T) {
        self.value = value;
        let bytes = bytemuck::bytes_of(&self.value);
        queue.write_buffer(&self.buffer, 0, bytes);
    }
}

pub struct StorageBuffer<T: BufferData> {
    value: T,
    buffer: wgpu::Buffer,
    flags: BufferFlags,
}

impl<T: BufferData> StorageBuffer<T> {
    pub fn new(device: &RenderDevice, value: T, flags: BufferFlags) -> Self {
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::bytes_of(&value),
            usage: flags.usages(BufferKind::Storage),
        });

        Self {
            buffer,
            value,
            flags,
        }
    }

    pub fn value(&self) -> &T {
        &self.value
    }

    pub fn id(&self) -> wgpu::Id<wgpu::Buffer> {
        self.buffer.global_id()
    }

    pub fn buffer(&self) -> &wgpu::Buffer {
        &self.buffer
    }

    pub fn flags(&self) -> BufferFlags {
        self.flags
    }

    pub fn binding(&self) -> wgpu::BindingResource {
        self.buffer.as_entire_binding()
    }

    pub fn update(&mut self, queue: &RenderQueue, value: T) {
        if self.flags.is_write() {
            self.value = value;
            queue.write_buffer(&self.buffer, 0, bytemuck::bytes_of(&self.value));
        }
    }
}

pub struct UniformBufferArray<T: BufferData> {
    data: Vec<T>,
    buffer: wgpu::Buffer,
    flags: BufferFlags,
    element_size: usize,
    offset: usize,
    len: usize,
    dirty: bool,
}

impl<T: BufferData> UniformBufferArray<T> {
    pub fn new(device: &RenderDevice, flags: BufferFlags) -> Self {
        let alignment = device.limits().min_uniform_buffer_offset_alignment as usize;
        let layout = std::alloc::Layout::new::<T>().align_to(alignment).unwrap();
        let size = layout.align();
        let buffer = Self::create_buffer(device, &vec![0; alignment], flags);

        Self {
            data: vec![],
            buffer,
            flags,
            element_size: size,
            offset: layout.align() / size,
            len: 0,
            dirty: false,
        }
    }

    pub fn with_amount(device: &RenderDevice, flags: BufferFlags, amount: usize) -> Self {
        let alignment = device.limits().min_uniform_buffer_offset_alignment as usize;
        let layout = std::alloc::Layout::new::<T>().align_to(alignment).unwrap();
        let size = layout.align();
        let max_amount = device.limits().max_uniform_buffer_binding_size as usize / size;
        let amount = amount.min(max_amount);
        let offset = layout.align() / layout.size();

        let data = vec![T::zeroed(); amount];
        let buffer = Self::create_buffer(device, bytemuck::cast_slice(&data), flags);

        Self {
            data,
            buffer,
            flags,
            element_size: size,
            offset,
            len: 0,
            dirty: false,
        }
    }

    pub fn buffer(&self) -> &wgpu::Buffer {
        &self.buffer
    }

    pub fn flags(&self) -> BufferFlags {
        self.flags
    }

    pub fn element_size(&self) -> usize {
        self.element_size
    }

    pub fn dirty(&self) -> bool {
        self.dirty
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    pub fn push(&mut self, value: T) {
        self.data.push(value);
        self.len += 1;
        self.dirty = true;
    }

    pub fn reserve(&mut self, additional: usize) {
        self.data.extend((0..additional).map(|_| T::zeroed()));
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn set(&mut self, index: usize, value: T) {
        self.data[index * self.offset] = value;
        self.len = self.len.max(index + 1);
        self.dirty = true;
    }

    pub fn clear(&mut self) {
        bytemuck::fill_zeroes(&mut self.data);
        self.len = 0;
        self.dirty = true;
    }

    pub fn binding(&self) -> wgpu::BindingResource {
        self.buffer.as_entire_binding()
    }

    fn create_buffer(device: &RenderDevice, data: &[u8], flags: BufferFlags) -> wgpu::Buffer {
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: data,
            usage: flags.usages(BufferKind::Uniform),
        })
    }

    pub fn commit(&mut self, device: &RenderDevice, queue: &RenderQueue) {
        let buffer_cap = self.buffer.size();
        let size = (self.len * self.element_size) as u64;

        if buffer_cap < size {
            self.buffer = Self::create_buffer(device, bytemuck::cast_slice(&self.data), self.flags);
            self.dirty = false;
        } else if self.dirty {
            queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(&self.data));
        }
    }
}

pub struct StorageBufferArray<T: BufferData> {
    data: Vec<u8>,
    buffer: Option<wgpu::Buffer>,
    flags: BufferFlags,
    element_size: usize,
    dirty: bool,
    _marker: std::marker::PhantomData<T>,
}

impl<T: BufferData> StorageBufferArray<T> {
    pub fn new(flags: BufferFlags) -> Self {
        Self {
            data: vec![],
            buffer: None,
            flags,
            element_size: std::mem::size_of::<T>(),
            dirty: false,
            _marker: Default::default(),
        }
    }

    pub fn buffer(&self) -> Option<&wgpu::Buffer> {
        self.buffer.as_ref()
    }

    pub fn flags(&self) -> BufferFlags {
        self.flags
    }

    pub fn element_size(&self) -> usize {
        self.element_size
    }

    pub fn dirty(&self) -> bool {
        self.dirty
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    pub fn push(&mut self, value: &T) {
        self.data.extend(bytemuck::bytes_of(value));
        self.dirty = true;
    }

    pub fn len(&self) -> usize {
        self.data.len() * self.element_size
    }

    pub fn set(&mut self, index: usize, value: &T) {
        let offset = index * self.element_size;
        if offset > self.data.len() - offset {
            panic!("Index out of bounds.")
        }

        unsafe {
            let dst = self.data.as_mut_ptr().wrapping_add(offset);
            let src = bytemuck::bytes_of(value);
            std::ptr::copy_nonoverlapping(src.as_ptr(), dst, src.len());
        }

        self.dirty = true;
    }

    pub fn clear(&mut self) {
        self.data.clear();
        self.dirty = true;
    }

    pub fn binding(&self) -> Option<wgpu::BindingResource> {
        Some(self.buffer.as_ref()?.as_entire_binding())
    }

    pub fn commit(&mut self, device: &RenderDevice, queue: &RenderQueue) {
        let buffer_cap = self.buffer.as_ref().map(|a| a.size()).unwrap_or(0);
        let size = self.data.len() as u64;

        if buffer_cap < size || ((self.dirty || self.buffer.is_none()) && size > 0) {
            let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: None,
                contents: &self.data,
                usage: self.flags.usages(BufferKind::Storage),
            });

            self.buffer = Some(buffer);
            self.dirty = false;
        } else if let Some(buffer) = self.buffer() {
            queue.write_buffer(buffer, 0, &self.data);
        } else if size == 0 {
            self.buffer = None;
        }
    }
}

pub struct BatchIndex<T: BufferData> {
    index: usize,
    offset: u32,
    _marker: std::marker::PhantomData<T>,
}

impl<T: BufferData> BatchIndex<T> {
    pub fn new(index: usize, offset: u32) -> Self {
        Self {
            index,
            offset,
            _marker: Default::default(),
        }
    }

    pub fn index(&self) -> usize {
        self.index
    }

    pub fn offset(&self) -> u32 {
        self.offset
    }
}

impl<T: BufferData> Clone for BatchIndex<T> {
    fn clone(&self) -> Self {
        Self::new(self.index, self.offset)
    }
}
impl<T: BufferData> Copy for BatchIndex<T> {}

pub struct BatchedUniformBuffer<T: BufferData> {
    data: Vec<u8>,
    buffers: Vec<wgpu::Buffer>,
    flags: BufferFlags,
    alignment: usize,
    batch_size: usize,
    _marker: std::marker::PhantomData<T>,
}

impl<T: BufferData> BatchedUniformBuffer<T> {
    pub fn new(device: &RenderDevice, flags: BufferFlags) -> Self {
        let alignment = device.limits().min_uniform_buffer_offset_alignment as usize;
        let layout = std::alloc::Layout::new::<T>().align_to(alignment).unwrap();
        let alignment = layout.align();
        let batch_size = device.limits().max_uniform_buffer_binding_size as usize;

        Self {
            data: vec![],
            buffers: vec![],
            flags,
            alignment,
            batch_size,
            _marker: Default::default(),
        }
    }

    pub fn buffer(&self, index: usize) -> &wgpu::Buffer {
        &self.buffers[index]
    }

    pub fn flags(&self) -> BufferFlags {
        self.flags
    }

    pub fn stride(&self) -> usize {
        self.alignment
    }

    pub fn batch_size(&self) -> usize {
        self.batch_size
    }

    pub fn buffer_count(&self) -> usize {
        self.buffers.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    pub fn get(&self, index: BatchIndex<T>) -> Option<&T> {
        let offset = index.index * self.batch_size + index.offset as usize;
        let bytes = std::mem::size_of::<T>();
        let end = offset + bytes;

        if end > self.data.len() {
            return None;
        }

        let bytes = &self.data[offset..end];
        Some(bytemuck::from_bytes(bytes))
    }

    pub fn get_mut(&mut self, index: BatchIndex<T>) -> Option<&mut T> {
        let offset = index.index * self.batch_size + index.offset as usize;
        let bytes = std::mem::size_of::<T>();
        let end = offset + bytes;

        if end > self.data.len() {
            return None;
        }

        let bytes = &mut self.data[offset..end];
        Some(bytemuck::from_bytes_mut(bytes))
    }

    pub fn push(&mut self, value: T) -> BatchIndex<T> {
        self.data.resize(self.data.len() + self.alignment, 0);
        let offset = self.data.len() - self.alignment;
        let bytes = bytemuck::bytes_of(&value);
        self.data[offset..offset + bytes.len()].copy_from_slice(bytes);

        let batch_index = offset / self.batch_size;
        let dynamic_offset = offset % self.batch_size;

        BatchIndex::new(batch_index, dynamic_offset as u32)
    }

    pub fn commit(&mut self, device: &RenderDevice, queue: &RenderQueue) {
        let max_buffers = match self.data.is_empty() {
            true => 0,
            false => (self.data.len() / self.batch_size) + 1,
        };

        let buffers_needed = max_buffers.max(self.buffers.len()) - self.buffers.len();

        for _ in 0..buffers_needed {
            let buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: None,
                size: self.batch_size as u64,
                usage: self.flags.usages(BufferKind::Uniform),
                mapped_at_creation: false,
            });
            self.buffers.push(buffer);
        }

        for (index, chunk) in self.data.chunks(self.batch_size).enumerate() {
            queue.write_buffer(&self.buffers[index], 0, chunk);
        }

        self.data.clear();
    }

    pub fn reset(&mut self) {
        self.data.clear();
        self.buffers.clear();
    }
}

pub struct ArrayBuffer<T: BufferData> {
    data: Vec<T>,
    kind: BufferKind,
    flags: BufferFlags,
    buffer: Option<wgpu::Buffer>,
    dirty: bool,
}

impl<T: BufferData> ArrayBuffer<T> {
    pub fn new(kind: BufferKind, flags: BufferFlags) -> Self {
        Self {
            data: vec![],
            kind,
            flags,
            buffer: None,
            dirty: false,
        }
    }

    pub fn buffer(&self) -> Option<&wgpu::Buffer> {
        self.buffer.as_ref()
    }

    pub fn flags(&self) -> BufferFlags {
        self.flags
    }

    pub fn kind(&self) -> BufferKind {
        self.kind
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn push(&mut self, value: T) {
        self.data.push(value);
        self.dirty = true;
    }

    pub fn set(&mut self, index: usize, value: T) {
        self.data[index] = value;
        self.dirty = true;
    }

    pub fn resize(&mut self, len: usize, value: T) {
        self.data.resize(len, value);
        self.dirty = true;
    }

    pub fn extend(&mut self, values: &[T]) -> usize {
        let offset = self.data.len() * std::mem::size_of::<T>();
        self.data.extend_from_slice(values);
        self.dirty = true;
        offset
    }

    pub fn clear(&mut self) {
        self.data.clear();
    }

    pub fn commit(&mut self, device: &RenderDevice, queue: &RenderQueue) {
        let buffer_cap = self.buffer.as_ref().map(|a| a.size()).unwrap_or(0);
        let size = (self.data.len() * std::mem::size_of::<T>()) as u64;

        if buffer_cap < size || ((self.dirty || self.buffer.is_none()) && size > 0) {
            let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: None,
                contents: bytemuck::cast_slice(&self.data),
                usage: self.flags.usages(self.kind),
            });

            self.buffer = Some(buffer);
            self.dirty = false;
        } else if let Some(buffer) = self.buffer() {
            queue.write_buffer(buffer, 0, bytemuck::cast_slice(&self.data));
        } else if size == 0 {
            self.buffer = None;
        }
    }
}
