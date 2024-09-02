use crate::core::{RenderDevice, RenderQueue, VertexLayout};
use encase::ShaderType;
use wgpu::util::DeviceExt;

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

    pub fn data(&self, index: usize) -> &[u8] {
        match self {
            Indices::U16(v) => bytemuck::bytes_of(&v[index]),
            Indices::U32(v) => bytemuck::bytes_of(&v[index]),
        }
    }

    pub fn bytes(&self) -> &[u8] {
        match self {
            Indices::U16(v) => bytemuck::cast_slice(&v[0..v.len()]),
            Indices::U32(v) => bytemuck::cast_slice(&v[0..v.len()]),
        }
    }

    pub fn is_empty(&self) -> bool {
        match self {
            Indices::U16(v) => v.is_empty(),
            Indices::U32(v) => v.is_empty(),
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

pub struct VertexBuffer<V> {
    buffer: Option<wgpu::Buffer>,
    vertices: Vec<V>,
    flags: BufferFlags,
    layout: VertexLayout,
    dirty: bool,
}

impl<V: bytemuck::Pod> VertexBuffer<V> {
    pub fn create(vertices: Vec<V>, layout: VertexLayout, flags: BufferFlags) -> Self {
        Self {
            buffer: None,
            vertices,
            flags,
            layout,
            dirty: false,
        }
    }

    pub fn buffer(&self) -> Option<&wgpu::Buffer> {
        self.buffer.as_ref()
    }

    pub fn vertices(&self) -> &[V] {
        &self.vertices
    }

    pub fn vertices_mut(&mut self) -> &mut Vec<V> {
        self.dirty = true;
        &mut self.vertices
    }

    pub fn layout(&self) -> &VertexLayout {
        &self.layout
    }

    pub fn push(&mut self, vertex: V) {
        self.vertices.push(vertex);
        self.dirty = true;
    }

    pub fn append(&mut self, vertices: &[V]) {
        self.vertices.extend_from_slice(vertices);
        self.dirty = true;
    }

    pub fn set_vertices(&mut self, vertices: Vec<V>) {
        self.vertices = vertices;
        self.dirty = true;
    }

    pub fn len(&self) -> usize {
        self.vertices.len()
    }

    pub fn clear(&mut self) {
        self.vertices.clear();
        self.dirty = true;
    }

    pub fn resize(&mut self, device: &RenderDevice) -> bool {
        let buffer_size = self.buffer.as_ref().map_or(0, |buffer| buffer.size());
        let size = (self.vertices.len() * std::mem::size_of::<V>()) as u64;
        if size > 0 && (self.dirty || buffer_size != size) {
            let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: None,
                contents: bytemuck::cast_slice(&self.vertices),
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
        if !self.resize(device) && self.flags.is_write() {
            self.buffer.as_ref().map(|buffer| {
                let bytes = &self.vertices[0..self.vertices.len()];
                queue.write_buffer(buffer, 0, bytemuck::cast_slice(bytes));
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
    pub fn create(indices: Indices, flags: BufferFlags) -> Self {
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

    pub fn set_indices(&mut self, indices: Indices) {
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

    pub fn resize(&mut self, device: &RenderDevice) -> bool {
        let buffer_size = self.buffer.as_ref().map_or(0, |buffer| buffer.size());
        let size = match &self.indices {
            Indices::U16(v) => (v.len() * std::mem::size_of::<u16>()) as u64,
            Indices::U32(v) => (v.len() * std::mem::size_of::<u32>()) as u64,
        };

        if size > 0 && (self.dirty || buffer_size != size) {
            let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: None,
                contents: match &self.indices {
                    Indices::U16(v) => bytemuck::cast_slice(v),
                    Indices::U32(v) => bytemuck::cast_slice(v),
                },
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
        if !self.resize(device) && self.flags.is_write() {
            self.buffer.as_ref().map(|buffer| {
                queue.write_buffer(buffer, 0, &self.indices.bytes());
            });
        }
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

    pub fn resize(&mut self, device: &RenderDevice) -> bool {
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
        if (!self.resize(device)) && self.flags.is_write() {
            self.buffer.as_ref().map(|buffer| {
                queue.write_buffer(buffer, 0, bytemuck::bytes_of(&self.value));
            });
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
    flags: BufferFlags,
    dirty: bool,
}

impl<T: ShaderType + bytemuck::Pod> UniformBufferArray<T> {
    pub fn create(flags: BufferFlags) -> Self {
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

    pub fn resize(&mut self, device: &RenderDevice) -> bool {
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
        if (!self.resize(device) || self.dirty) && self.flags.is_write() {
            self.buffer.as_ref().map(|buffer| {
                queue.write_buffer(buffer, 0, bytemuck::cast_slice(&self.values));
            });
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

    pub fn resize(&mut self, device: &RenderDevice) -> bool {
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
        if (!self.resize(device) || self.dirty) && self.flags.is_write() {
            self.buffer.as_ref().map(|buffer| {
                queue.write_buffer(buffer, 0, bytemuck::bytes_of(&self.value));
            });
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
    flags: BufferFlags,
    dirty: bool,
}

impl<T: ShaderType + bytemuck::Pod> StorageBufferArray<T> {
    pub fn create(flags: BufferFlags) -> Self {
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

    pub fn resize(&mut self, device: &RenderDevice) -> bool {
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
        if (!self.resize(device) || self.dirty) && self.flags.is_write() {
            self.buffer.as_ref().map(|buffer| {
                queue.write_buffer(buffer, 0, bytemuck::cast_slice(&self.values));
            });
        }
    }
}
