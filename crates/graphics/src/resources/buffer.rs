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
    label: Option<&'static str>,
    buffer: Option<wgpu::Buffer>,
    data: Vec<u8>,
    flags: BufferFlags,
    size: usize,
    dirty: bool,
}

impl VertexBuffer {
    pub fn new<V: Vertex>(vertices: &[V], flags: BufferFlags) -> Self {
        Self {
            label: None,
            buffer: None,
            data: bytemuck::cast_slice(vertices).to_vec(),
            flags,
            size: std::mem::size_of::<V>(),
            dirty: false,
        }
    }

    pub fn with_label(mut self, label: &'static str) -> Self {
        self.label = Some(label);
        self
    }

    pub fn label(&self) -> Option<&'static str> {
        self.label
    }

    pub fn buffer(&self) -> Option<&wgpu::Buffer> {
        self.buffer.as_ref()
    }

    pub fn data(&self) -> &[u8] {
        &self.data
    }

    pub fn flags(&self) -> BufferFlags {
        self.flags
    }

    pub fn size(&self) -> usize {
        self.size
    }

    pub fn dirty(&self) -> bool {
        self.dirty
    }

    pub fn len(&self) -> usize {
        self.data.len() / self.size
    }

    pub fn set<V: Vertex>(&mut self, vertices: &[V]) {
        self.data = bytemuck::cast_slice(vertices).to_vec();
        self.dirty = true;
    }

    pub fn clear(&mut self) {
        self.data.clear();
        self.dirty = true;
    }

    pub fn create(&mut self, device: &RenderDevice) -> bool {
        let buffer_size = self.buffer.as_ref().map_or(0, |buffer| buffer.size());
        let size = self.data.len() as u64;

        if size > 0 && (self.dirty || buffer_size != size) {
            let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: self.label,
                contents: &self.data,
                usage: self.flags.usages(BufferKind::Vertex),
            });

            self.buffer = Some(buffer);
            self.dirty = false;
            true
        } else {
            false
        }
    }

    pub fn commit(&mut self, device: &RenderDevice, queue: &RenderQueue) {
        if !self.create(device) && self.flags.is_write() {
            self.buffer.as_ref().map(|buffer| {
                queue.write_buffer(buffer, 0, &self.data[..self.data.len()]);
            });
        }
    }
}

pub struct IndexBuffer {
    buffer: Option<wgpu::Buffer>,
    indices: Indices,
    flags: BufferFlags,
    dirty: bool,
}

impl IndexBuffer {
    pub fn new(indices: Indices, flags: BufferFlags) -> Self {
        Self {
            buffer: None,
            indices: indices,
            flags,
            dirty: false,
        }
    }

    pub fn buffer(&self) -> Option<&wgpu::Buffer> {
        self.buffer.as_ref()
    }

    pub fn indices(&self) -> &Indices {
        &self.indices
    }

    pub fn indices_mut(&mut self) -> &mut Indices {
        &mut self.indices
    }

    pub fn push(&mut self, vertex: u32) {
        match &mut self.indices {
            Indices::U16(v) => v.push(vertex as u16),
            Indices::U32(v) => v.push(vertex),
        }

        self.dirty = true;
    }

    pub fn append(&mut self, indices: Indices) {
        match (&mut self.indices, indices) {
            (Indices::U16(v1), Indices::U16(v2)) => v1.extend_from_slice(&v2),
            (Indices::U32(v1), Indices::U32(v2)) => v1.extend_from_slice(&v2),
            _ => (),
        }

        self.dirty = true;
    }

    pub fn set(&mut self, indices: Indices) {
        self.indices = indices;
        self.dirty = true;
    }

    pub fn len(&self) -> usize {
        match &self.indices {
            Indices::U16(v) => v.len(),
            Indices::U32(v) => v.len(),
        }
    }

    pub fn clear(&mut self) {
        match &mut self.indices {
            Indices::U16(v) => v.clear(),
            Indices::U32(v) => v.clear(),
        }

        self.dirty = true;
    }

    pub fn create(&mut self, device: &RenderDevice) -> bool {
        let buffer_size = self.buffer.as_ref().map_or(0, |buffer| buffer.size());
        let size = match &self.indices {
            Indices::U16(v) => (v.len() * std::mem::size_of::<u16>()) as u64,
            Indices::U32(v) => (v.len() * std::mem::size_of::<u32>()) as u64,
        };

        if size > 0 && (self.dirty || buffer_size != size) {
            let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: None,
                contents: self.indices.bytes(),
                usage: self.flags.usages(BufferKind::Index),
            });

            self.buffer = Some(buffer);
            self.dirty = false;
            true
        } else {
            false
        }
    }

    pub fn commit(&mut self, device: &RenderDevice, queue: &RenderQueue) {
        if !self.create(device) && self.flags.is_write() {
            self.buffer.as_ref().map(|buffer| {
                queue.write_buffer(buffer, 0, &self.indices.bytes());
            });
        }
    }
}

pub struct UniformBuffer<T: BufferData> {
    value: T,
    buffer: Option<wgpu::Buffer>,
    flags: BufferFlags,
    dirty: bool,
}

impl<T: BufferData> UniformBuffer<T> {
    pub fn new(value: T, flags: BufferFlags) -> Self {
        Self {
            value,
            buffer: None,
            flags,
            dirty: false,
        }
    }

    pub fn buffer(&self) -> Option<&wgpu::Buffer> {
        self.buffer.as_ref()
    }

    pub fn value(&self) -> T {
        self.value
    }

    pub fn flags(&self) -> BufferFlags {
        self.flags
    }

    pub fn binding(&self) -> Option<wgpu::BindingResource> {
        let binding = self.buffer.as_ref()?.as_entire_buffer_binding();
        Some(wgpu::BindingResource::Buffer(binding))
    }

    pub fn update(&mut self, value: T) {
        self.dirty = true;
        self.value = value;
    }

    pub fn create(&mut self, device: &RenderDevice) -> bool {
        let buffer_size = self.buffer.as_ref().map_or(0, |buffer| buffer.size());
        let size = std::mem::size_of::<T>() as u64;

        if size > 0 && (self.dirty || buffer_size != size) {
            let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: None,
                contents: bytemuck::bytes_of(&self.value),
                usage: self.flags.usages(BufferKind::Uniform),
            });

            self.buffer = Some(buffer);
            self.dirty = false;
            true
        } else {
            false
        }
    }

    pub fn commit(&mut self, device: &RenderDevice, queue: &RenderQueue) {
        if (!self.create(device)) && self.flags.is_write() {
            self.buffer.as_ref().map(|buffer| {
                queue.write_buffer(buffer, 0, bytemuck::bytes_of(&self.value));
            });
        }
    }
}

pub struct UniformBufferArray<T: BufferData> {
    values: Vec<T>,
    buffer: Option<wgpu::Buffer>,
    flags: BufferFlags,
    dirty: bool,
}

impl<T: BufferData> UniformBufferArray<T> {
    pub fn new(flags: BufferFlags) -> Self {
        Self {
            flags,
            values: vec![],
            buffer: None,
            dirty: false,
        }
    }

    pub fn buffer(&self) -> Option<&wgpu::Buffer> {
        self.buffer.as_ref()
    }

    pub fn values(&self) -> &[T] {
        &self.values
    }

    pub fn flags(&self) -> BufferFlags {
        self.flags
    }

    pub fn len(&self) -> usize {
        self.values.len()
    }

    pub fn binding(&self) -> Option<wgpu::BindingResource> {
        Some(self.buffer.as_ref()?.as_entire_binding())
    }

    pub fn binding_offset(&self, index: u64) -> Option<wgpu::BindingResource> {
        let buffer = self.buffer.as_ref()?;
        let size = std::mem::size_of::<T>() as u64;
        let binding = wgpu::BufferBinding {
            buffer,
            offset: index * size,
            size: NonZero::<u64>::new(size),
        };

        Some(wgpu::BindingResource::Buffer(binding))
    }

    pub fn push(&mut self, value: T) {
        self.dirty = true;
        self.values.push(value);
    }

    pub fn update(&mut self, index: usize, value: T) {
        if index >= self.values.len() {
            panic!("Index out of bounds");
        }

        self.dirty = true;
        self.values[index] = value;
    }

    pub fn append(&mut self, values: &[T]) {
        self.dirty = true;
        self.values.extend_from_slice(values);
    }

    pub fn clear(&mut self) {
        self.dirty = true;
        self.values.clear();
    }

    pub fn create(&mut self, device: &RenderDevice) -> bool {
        let buffer_size = self.buffer.as_ref().map_or(0, |buffer| buffer.size());
        let size = (self.values.len() * std::mem::size_of::<T>()) as u64;

        if size > 0 && (self.dirty || buffer_size != size) {
            let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: None,
                contents: bytemuck::cast_slice(&self.values),
                usage: self.flags.usages(BufferKind::Uniform),
            });

            self.buffer = Some(buffer);
            self.dirty = false;
            true
        } else {
            false
        }
    }

    pub fn commit(&mut self, device: &RenderDevice, queue: &RenderQueue) {
        if (!self.create(device) || self.dirty) && self.flags.is_write() {
            self.buffer.as_ref().map(|buffer| {
                queue.write_buffer(buffer, 0, bytemuck::cast_slice(&self.values));
            });
        }
    }
}

pub struct StorageBuffer<T: BufferData> {
    value: T,
    buffer: Option<wgpu::Buffer>,
    flags: BufferFlags,
    dirty: bool,
}

impl<T: BufferData> StorageBuffer<T> {
    pub fn new(value: T, flags: BufferFlags) -> Self {
        Self {
            value,
            buffer: None,
            flags,
            dirty: false,
        }
    }

    pub fn buffer(&self) -> Option<&wgpu::Buffer> {
        self.buffer.as_ref()
    }

    pub fn value(&self) -> T {
        self.value
    }

    pub fn flags(&self) -> BufferFlags {
        self.flags
    }

    pub fn binding(&self) -> Option<wgpu::BindingResource> {
        let binding = self.buffer.as_ref()?.as_entire_buffer_binding();
        Some(wgpu::BindingResource::Buffer(binding))
    }

    pub fn update(&mut self, value: T) {
        self.dirty = true;
        self.value = value;
    }

    pub fn create(&mut self, device: &RenderDevice) -> bool {
        let buffer_size = self.buffer.as_ref().map_or(0, |buffer| buffer.size());
        let size = std::mem::size_of::<T>() as u64;

        if size > 0 && buffer_size != size {
            let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: None,
                contents: bytemuck::bytes_of(&self.value),
                usage: self.flags.usages(BufferKind::Uniform),
            });

            self.buffer = Some(buffer);
            self.dirty = false;
            true
        } else {
            false
        }
    }

    pub fn commit(&mut self, device: &RenderDevice, queue: &RenderQueue) {
        if (!self.create(device) || self.dirty) && self.flags.is_write() {
            self.buffer.as_ref().map(|buffer| {
                queue.write_buffer(buffer, 0, bytemuck::bytes_of(&self.value));
            });
        }
    }
}

pub struct StorageBufferArray<T: BufferData> {
    values: Vec<T>,
    buffer: Option<wgpu::Buffer>,
    flags: BufferFlags,
    dirty: bool,
}

impl<T: BufferData> StorageBufferArray<T> {
    pub fn new(flags: BufferFlags) -> Self {
        Self {
            flags,
            values: vec![],
            buffer: None,
            dirty: false,
        }
    }

    pub fn buffer(&self) -> Option<&wgpu::Buffer> {
        self.buffer.as_ref()
    }

    pub fn values(&self) -> &[T] {
        &self.values
    }

    pub fn flags(&self) -> BufferFlags {
        self.flags
    }

    pub fn len(&self) -> usize {
        self.values.len()
    }

    pub fn binding(&self) -> Option<wgpu::BindingResource> {
        let binding = self.buffer.as_ref()?.as_entire_buffer_binding();
        Some(wgpu::BindingResource::Buffer(binding))
    }

    pub fn push(&mut self, value: T) {
        self.dirty = true;
        self.values.push(value);
    }

    pub fn update(&mut self, index: usize, value: T) {
        if index >= self.values.len() {
            panic!("Index out of bounds");
        }

        self.dirty = true;
        self.values[index] = value;
    }

    pub fn append(&mut self, values: &[T]) {
        self.dirty = true;
        self.values.extend_from_slice(values);
    }

    pub fn clear(&mut self) {
        self.dirty = true;
        self.values.clear();
    }

    pub fn create(&mut self, device: &RenderDevice) -> bool {
        let buffer_size = self.buffer.as_ref().map_or(0, |buffer| buffer.size());
        let size = (self.values.len() * std::mem::size_of::<T>()) as u64;

        if self.dirty || (size > 0 && buffer_size != size) {
            let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: None,
                contents: bytemuck::cast_slice(&self.values),
                usage: self.flags.usages(BufferKind::Storage),
            });

            self.buffer = Some(buffer);
            self.dirty = false;
            true
        } else {
            false
        }
    }

    pub fn commit(&mut self, device: &RenderDevice, queue: &RenderQueue) {
        if (!self.create(device) || self.dirty) && self.flags.is_write() {
            self.buffer.as_ref().map(|buffer| {
                queue.write_buffer(buffer, 0, bytemuck::cast_slice(&self.values));
            });
        }
    }
}
