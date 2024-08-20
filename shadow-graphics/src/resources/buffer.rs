use crate::core::Color;
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
        let value = u8::deserialize(deserializer)?;

        Self::from_bits(value).ok_or(serde::de::Error::custom(
            "Failed to convert bits to BufferFlags",
        ))
    }
}

pub struct RenderBufferDesc<'a, T: bytemuck::Pod> {
    pub kind: BufferKind,
    pub flags: BufferFlags,
    pub data: &'a [T],
}

impl<'a, V: Vertex> RenderBufferDesc<'a, V> {
    pub fn vertex_buffer(flags: BufferFlags, vertices: &'a [V]) -> Self {
        Self {
            kind: BufferKind::Vertex,
            flags,
            data: vertices,
        }
    }
}

impl<'a, V: VertexIndex> RenderBufferDesc<'a, V> {
    pub fn index_buffer(flags: BufferFlags, data: &'a [V]) -> Self {
        Self {
            kind: BufferKind::Index,
            flags,
            data: data,
        }
    }
}

impl<'a, T: bytemuck::Pod> RenderBufferDesc<'a, T> {
    pub fn uniform_buffer(flags: BufferFlags, data: &'a [T]) -> Self {
        Self {
            kind: BufferKind::Uniform,
            flags,
            data,
        }
    }

    pub fn storage_buffer(flags: BufferFlags, data: &'a [T]) -> Self {
        Self {
            kind: BufferKind::Storage,
            flags,
            data,
        }
    }
}

pub struct RenderBuffer {
    buffer: wgpu::Buffer,
    count: NonZeroUsize,
}

impl RenderBuffer {
    pub fn create_vertex_buffer<T: bytemuck::Pod>(
        device: &wgpu::Device,
        desc: RenderBufferDesc<T>,
    ) -> Self {
        todo!()
    }
}
