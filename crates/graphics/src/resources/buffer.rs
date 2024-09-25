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
    data: Vec<u8>,
    buffer: Option<wgpu::Buffer>,
    flags: BufferFlags,
    element_size: usize,
    dirty: bool,
    _marker: std::marker::PhantomData<T>,
}

impl<T: BufferData> UniformBufferArray<T> {
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

    pub fn push(&mut self, value: T) {
        self.data.extend(bytemuck::bytes_of(&value));
        self.dirty = true;
    }

    pub fn len(&self) -> usize {
        self.data.len() * self.element_size
    }

    pub fn set(&mut self, index: usize, value: T) {
        let offset = index * self.element_size;
        if offset > self.data.len() - offset {
            panic!("Index out of bounds.")
        }

        unsafe {
            let dst = self.data.as_mut_ptr().wrapping_add(offset);
            let src = bytemuck::bytes_of(&value);
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
                usage: self.flags.usages(BufferKind::Uniform),
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
