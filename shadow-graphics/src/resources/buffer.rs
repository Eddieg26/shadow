use crate::core::{Color, RenderDevice, RenderQueue};
use encase::ShaderType;
use glam::Vec4;
use std::num::NonZeroUsize;
use wgpu::util::DeviceExt;

#[derive(Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum VertexAttribute {
    Float,
    Int,
    UInt,
    Color,
    Vec2F,
    Vec3F,
    Vec4F,
    Vec2U,
    Vec3U,
    Vec4U,
    Vec2I,
    Vec3I,
    Vec4I,
}

impl VertexAttribute {
    pub fn size(&self) -> usize {
        match self {
            VertexAttribute::Float | VertexAttribute::UInt | VertexAttribute::Int => 4,
            VertexAttribute::Vec2F | VertexAttribute::Vec2U | VertexAttribute::Vec2I => 8,
            VertexAttribute::Vec3F | VertexAttribute::Vec3U | VertexAttribute::Vec3I => 12,
            VertexAttribute::Vec4F
            | VertexAttribute::Vec4U
            | VertexAttribute::Vec4I
            | VertexAttribute::Color => 16,
        }
    }
}

impl Into<wgpu::VertexFormat> for VertexAttribute {
    fn into(self) -> wgpu::VertexFormat {
        match self {
            VertexAttribute::Float => wgpu::VertexFormat::Float32,
            VertexAttribute::Vec2F => wgpu::VertexFormat::Float32x2,
            VertexAttribute::Vec3F => wgpu::VertexFormat::Float32x3,
            VertexAttribute::Vec4F => wgpu::VertexFormat::Float32x4,
            VertexAttribute::UInt => wgpu::VertexFormat::Uint32,
            VertexAttribute::Vec2U => wgpu::VertexFormat::Uint32x2,
            VertexAttribute::Vec3U => wgpu::VertexFormat::Uint32x3,
            VertexAttribute::Vec4U => wgpu::VertexFormat::Uint32x4,
            VertexAttribute::Int => wgpu::VertexFormat::Sint32,
            VertexAttribute::Vec2I => wgpu::VertexFormat::Sint32x2,
            VertexAttribute::Vec3I => wgpu::VertexFormat::Sint32x3,
            VertexAttribute::Vec4I => wgpu::VertexFormat::Sint32x4,
            VertexAttribute::Color => wgpu::VertexFormat::Float64x4,
        }
    }
}

pub trait Vertex: bytemuck::Pod + 'static {
    fn attributes() -> &'static [VertexAttribute];
}

#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
pub struct BasicVertex {
    pub position: Vec4,
    pub color: Color,
}

impl Vertex for BasicVertex {
    fn attributes() -> &'static [VertexAttribute] {
        &[VertexAttribute::Vec4F, VertexAttribute::Color]
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum IndexFormat {
    U16,
    U32,
}

pub trait VertexIndex: bytemuck::Pod + 'static {
    fn format() -> IndexFormat;
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

pub struct VertexBuffer<V: Vertex> {
    buffer: Option<wgpu::Buffer>,
    vertices: Vec<V>,
    len: NonZeroUsize,
    flags: BufferFlags,
}

impl<V: Vertex> VertexBuffer<V> {
    pub fn create(vertices: &[V], flags: BufferFlags) -> Option<Self> {
        let len = NonZeroUsize::new(vertices.len())?;

        Some(Self {
            buffer: None,
            vertices: vertices.to_vec(),
            len,
            flags,
        })
    }

    pub fn buffer(&self) -> Option<&wgpu::Buffer> {
        self.buffer.as_ref()
    }

    pub fn vertices(&self) -> &[V] {
        &self.vertices
    }

    pub fn vertices_mut(&mut self) -> &mut Vec<V> {
        &mut self.vertices
    }

    pub fn len(&self) -> usize {
        self.len.get()
    }

    pub fn push(&mut self, vertex: V) {
        self.vertices.push(vertex);
    }

    pub fn append(&mut self, vertices: &[V]) {
        self.vertices.extend_from_slice(vertices);
    }

    pub fn clear(&mut self) {
        self.vertices.clear();
    }

    pub fn commit(&mut self, device: &RenderDevice, queue: &RenderQueue) {
        let buffer_size = self.buffer.as_ref().map_or(0, |buffer| buffer.size());
        let size = (self.vertices.len() * std::mem::size_of::<V>()) as u64;

        if size > 0 && buffer_size != size {
            self.buffer = Some(
                device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: None,
                    contents: bytemuck::cast_slice(&self.vertices),
                    usage: self.flags.usages(BufferKind::Vertex),
                }),
            );
        }

        match (&self.buffer, self.flags.is_write()) {
            (Some(buffer), true) => {
                queue.write_buffer(buffer, 0, bytemuck::cast_slice(&self.vertices))
            }
            _ => (),
        }
    }
}

pub struct IndexBuffer<V: VertexIndex> {
    buffer: Option<wgpu::Buffer>,
    indices: Vec<V>,
    len: NonZeroUsize,
    flags: BufferFlags,
}

impl<V: VertexIndex> IndexBuffer<V> {
    pub fn create(indices: &[V], flags: BufferFlags) -> Option<Self> {
        let len = NonZeroUsize::new(indices.len())?;

        Some(Self {
            buffer: None,
            indices: indices.to_vec(),
            len,
            flags,
        })
    }

    pub fn buffer(&self) -> Option<&wgpu::Buffer> {
        self.buffer.as_ref()
    }

    pub fn indices(&self) -> &[V] {
        &self.indices
    }

    pub fn indices_mut(&mut self) -> &mut Vec<V> {
        &mut self.indices
    }

    pub fn len(&self) -> usize {
        self.len.get()
    }

    pub fn push(&mut self, index: V) {
        self.indices.push(index);
    }

    pub fn append(&mut self, indices: &[V]) {
        self.indices.extend_from_slice(indices);
    }

    pub fn clear(&mut self) {
        self.indices.clear();
    }

    pub fn commit(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
        let buffer_size = self.buffer.as_ref().map_or(0, |buffer| buffer.size());
        let size = (self.indices.len() * std::mem::size_of::<V>()) as u64;

        if size > 0 && buffer_size != size {
            self.buffer = Some(
                device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: None,
                    contents: bytemuck::cast_slice(&self.indices),
                    usage: self.flags.usages(BufferKind::Index),
                }),
            );
        }

        match (&self.buffer, self.flags.is_write()) {
            (Some(buffer), true) => {
                queue.write_buffer(buffer, 0, bytemuck::cast_slice(&self.indices))
            }
            _ => (),
        }
    }
}

impl<V: VertexIndex> std::ops::Deref for IndexBuffer<V> {
    type Target = Option<wgpu::Buffer>;

    fn deref(&self) -> &Self::Target {
        &self.buffer
    }
}

pub struct UniformBuffer<T: ShaderType> {
    value: T,
    buffer: Option<wgpu::Buffer>,
    flags: BufferFlags,
    dirty: bool,
}

impl<T: ShaderType + bytemuck::Pod> UniformBuffer<T> {
    pub fn create(value: T, flags: BufferFlags) -> Self {
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

    pub fn commit(&mut self, device: &RenderDevice, queue: &RenderQueue) {
        if let None = &self.buffer {
            self.buffer = Some(
                device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: None,
                    contents: bytemuck::bytes_of(&self.value),
                    usage: self.flags.usages(BufferKind::Uniform),
                }),
            );
        }

        match (self.dirty, self.flags().is_write(), &self.buffer) {
            (true, true, Some(buffer)) => {
                queue.write_buffer(buffer, 0, bytemuck::bytes_of(&self.value));
                self.dirty = false;
            }
            _ => (),
        }
    }
}

impl<T: ShaderType> std::ops::Deref for UniformBuffer<T> {
    type Target = Option<wgpu::Buffer>;

    fn deref(&self) -> &Self::Target {
        &self.buffer
    }
}

pub struct UniformBufferArray<T: ShaderType> {
    values: Vec<T>,
    buffer: Option<wgpu::Buffer>,
    usage: wgpu::BufferUsages,
    last_len: usize,
    changed: bool,
}

impl<T: ShaderType + bytemuck::Pod> UniformBufferArray<T> {
    pub fn create(device: &wgpu::Device, values: &[T], flags: BufferFlags) -> Self {
        let usage = flags.usages(BufferKind::Uniform);
        let buffer = match values.is_empty() {
            true => None,
            false => Some(
                device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: None,
                    contents: bytemuck::cast_slice(values),
                    usage,
                }),
            ),
        };

        Self {
            values: values.to_vec(),
            buffer,
            usage,
            last_len: values.len(),
            changed: false,
        }
    }

    pub fn buffer(&self) -> Option<&wgpu::Buffer> {
        self.buffer.as_ref()
    }

    pub fn values(&self) -> &[T] {
        &self.values
    }

    pub fn usage(&self) -> wgpu::BufferUsages {
        self.usage
    }

    pub fn binding(&self) -> Option<wgpu::BindingResource> {
        let binding = self.buffer.as_ref()?.as_entire_buffer_binding();
        Some(wgpu::BindingResource::Buffer(binding))
    }

    pub fn push(&mut self, value: T) {
        self.changed = true;
        self.last_len = self.values.len();
        self.values.push(value);
    }

    pub fn update(&mut self, values: &[T]) {
        self.changed = true;
        self.last_len = values.len();
        self.values = values.to_vec()
    }

    pub fn clear(&mut self) {
        self.changed = true;
        self.last_len = self.values.len();
        self.values.clear();
    }

    pub fn commit(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
        let buffer_size = self.buffer.as_ref().map_or(0, |buffer| buffer.size());
        let size = (self.values.len() * std::mem::size_of::<T>()) as u64;

        if self.changed || (size > 0 && buffer_size != size) {
            self.buffer = Some(
                device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: None,
                    contents: bytemuck::cast_slice(&self.values),
                    usage: self.usage,
                }),
            );
            self.changed = false;
        }

        if let Some(buffer) = &self.buffer {
            queue.write_buffer(buffer, 0, bytemuck::cast_slice(&self.values));
        }
    }
}

impl<T: ShaderType> std::ops::Deref for UniformBufferArray<T> {
    type Target = Option<wgpu::Buffer>;

    fn deref(&self) -> &Self::Target {
        &self.buffer
    }
}

pub struct StorageBuffer<T: ShaderType> {
    value: T,
    buffer: Option<wgpu::Buffer>,
    flags: BufferFlags,
    dirty: bool,
}

impl<T: ShaderType + bytemuck::Pod> StorageBuffer<T> {
    pub fn create(value: T, flags: BufferFlags) -> Self {
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

    pub fn commit(&mut self, device: &RenderDevice, queue: &RenderQueue) {
        if let None = &self.buffer {
            self.buffer = Some(
                device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: None,
                    contents: bytemuck::bytes_of(&self.value),
                    usage: self.flags.usages(BufferKind::Uniform),
                }),
            );
        }

        match (self.dirty, self.flags().is_write(), &self.buffer) {
            (true, true, Some(buffer)) => {
                queue.write_buffer(buffer, 0, bytemuck::bytes_of(&self.value));
                self.dirty = false;
            }
            _ => (),
        }
    }
}

impl<T: ShaderType> std::ops::Deref for StorageBuffer<T> {
    type Target = Option<wgpu::Buffer>;

    fn deref(&self) -> &Self::Target {
        &self.buffer
    }
}

pub struct StorageBufferArray<T: ShaderType> {
    values: Vec<T>,
    buffer: Option<wgpu::Buffer>,
    usage: wgpu::BufferUsages,
    last_len: usize,
    changed: bool,
}

impl<T: ShaderType + bytemuck::Pod> StorageBufferArray<T> {
    pub fn create(device: &wgpu::Device, values: &[T], flags: BufferFlags) -> Self {
        let usage = flags.usages(BufferKind::Storage);
        let buffer = match values.is_empty() {
            true => None,
            false => Some(
                device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: None,
                    contents: bytemuck::cast_slice(values),
                    usage,
                }),
            ),
        };

        Self {
            values: values.to_vec(),
            buffer,
            usage,
            last_len: values.len(),
            changed: false,
        }
    }

    pub fn buffer(&self) -> Option<&wgpu::Buffer> {
        self.buffer.as_ref()
    }

    pub fn values(&self) -> &[T] {
        &self.values
    }

    pub fn usage(&self) -> wgpu::BufferUsages {
        self.usage
    }

    pub fn binding(&self) -> Option<wgpu::BindingResource> {
        let binding = self.buffer.as_ref()?.as_entire_buffer_binding();
        Some(wgpu::BindingResource::Buffer(binding))
    }

    pub fn push(&mut self, value: T) {
        self.changed = true;
        self.last_len = self.values.len();
        self.values.push(value);
    }

    pub fn update(&mut self, values: &[T]) {
        self.changed = true;
        self.last_len = values.len();
        self.values = values.to_vec()
    }

    pub fn clear(&mut self) {
        self.changed = true;
        self.last_len = self.values.len();
        self.values.clear();
    }

    pub fn commit(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
        let buffer_size = self.buffer.as_ref().map_or(0, |buffer| buffer.size());
        let size = (self.values.len() * std::mem::size_of::<T>()) as u64;

        if self.changed || (size > 0 && buffer_size != size) {
            self.buffer = Some(
                device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: None,
                    contents: bytemuck::cast_slice(&self.values),
                    usage: self.usage,
                }),
            );
            self.changed = false;
        }

        if let Some(buffer) = &self.buffer {
            queue.write_buffer(buffer, 0, bytemuck::cast_slice(&self.values));
        }
    }
}

impl<T: ShaderType> std::ops::Deref for StorageBufferArray<T> {
    type Target = Option<wgpu::Buffer>;

    fn deref(&self) -> &Self::Target {
        &self.buffer
    }
}
