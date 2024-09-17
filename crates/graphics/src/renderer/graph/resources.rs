use crate::{
    core::RenderDevice,
    renderer::surface::RenderSurface,
    resources::{RenderAsset, ResourceId},
};
use spatial::size::Size;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy)]
pub struct BufferDesc {
    pub size: wgpu::BufferAddress,
    pub usage: wgpu::BufferUsages,
}

pub struct RenderGraphBuffer(wgpu::Buffer);
impl RenderGraphBuffer {
    pub fn create(device: &RenderDevice, desc: &BufferDesc) -> Self {
        Self(device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: desc.size,
            usage: desc.usage,
            mapped_at_creation: false,
        }))
    }
}

impl std::ops::Deref for RenderGraphBuffer {
    type Target = wgpu::Buffer;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<wgpu::Buffer> for RenderGraphBuffer {
    fn from(buffer: wgpu::Buffer) -> Self {
        Self(buffer)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct TextureDesc {
    pub format: wgpu::TextureFormat,
    pub usages: wgpu::TextureUsages,
}

pub struct RenderGraphTexture(wgpu::TextureView);
impl RenderGraphTexture {
    pub fn create(device: &RenderDevice, desc: &TextureDesc, width: u32, height: u32) -> Self {
        Self(
            device
                .create_texture(&wgpu::TextureDescriptor {
                    label: None,
                    size: wgpu::Extent3d {
                        width,
                        height,
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: desc.format,
                    usage: desc.usages,
                    view_formats: &[desc.format],
                })
                .create_view(&wgpu::TextureViewDescriptor::default()),
        )
    }
}

impl std::ops::Deref for RenderGraphTexture {
    type Target = wgpu::TextureView;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<wgpu::TextureView> for RenderGraphTexture {
    fn from(view: wgpu::TextureView) -> Self {
        Self(view)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct RenderTargetDesc {
    pub width: u32,
    pub height: u32,
    pub format: wgpu::TextureFormat,
    pub depth: wgpu::TextureFormat,
}

impl RenderTargetDesc {
    pub fn new(
        width: u32,
        height: u32,
        format: wgpu::TextureFormat,
        depth: wgpu::TextureFormat,
    ) -> Self {
        Self {
            width,
            height,
            format,
            depth,
        }
    }
}

pub struct RenderTarget {
    size: Size,
    color: Option<RenderGraphTexture>,
    format: wgpu::TextureFormat,
    depth: Option<RenderGraphTexture>,
    depth_format: wgpu::TextureFormat,
}

impl RenderTarget {
    pub fn create(device: &RenderDevice, desc: RenderTargetDesc) -> Self {
        let color = RenderGraphTexture(
            device
                .create_texture(&wgpu::TextureDescriptor {
                    label: None,
                    size: wgpu::Extent3d {
                        width: desc.width,
                        height: desc.height,
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: desc.format,
                    usage: wgpu::TextureUsages::all(),
                    view_formats: &[desc.format],
                })
                .create_view(&wgpu::TextureViewDescriptor::default()),
        );

        let depth = RenderGraphTexture(
            device
                .create_texture(&wgpu::TextureDescriptor {
                    label: None,
                    size: wgpu::Extent3d {
                        width: desc.width,
                        height: desc.height,
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: desc.depth,
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                        | wgpu::TextureUsages::TEXTURE_BINDING
                        | wgpu::TextureUsages::COPY_DST
                        | wgpu::TextureUsages::COPY_SRC,
                    view_formats: &[desc.depth],
                })
                .create_view(&wgpu::TextureViewDescriptor::default()),
        );

        Self {
            size: Size::new(desc.width, desc.height),
            color: Some(color),
            format: desc.format,
            depth: Some(depth),
            depth_format: wgpu::TextureFormat::Depth32Float,
        }
    }

    pub fn from_surface(device: &RenderDevice, surface: &RenderSurface) -> Self {
        let width = surface.width();
        let height = surface.height();
        let format = surface.format();
        let depth_format = surface.depth_format();

        let depth = RenderGraphTexture(
            device
                .create_texture(&wgpu::TextureDescriptor {
                    label: None,
                    size: wgpu::Extent3d {
                        width,
                        height,
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: depth_format,
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                        | wgpu::TextureUsages::TEXTURE_BINDING
                        | wgpu::TextureUsages::COPY_DST
                        | wgpu::TextureUsages::COPY_SRC,
                    view_formats: &[depth_format],
                })
                .create_view(&wgpu::TextureViewDescriptor::default()),
        );

        Self {
            size: Size::new(width, height),
            color: None,
            format,
            depth: Some(depth),
            depth_format: wgpu::TextureFormat::Depth32Float,
        }
    }

    pub fn empty(
        size: Size,
        format: wgpu::TextureFormat,
        depth_format: wgpu::TextureFormat,
    ) -> Self {
        Self {
            size,
            color: None,
            format,
            depth: None,
            depth_format,
        }
    }

    pub fn size(&self) -> Size {
        self.size
    }

    pub fn color(&self) -> Option<&RenderGraphTexture> {
        self.color.as_ref()
    }

    pub fn format(&self) -> wgpu::TextureFormat {
        self.format
    }

    pub fn depth(&self) -> Option<&RenderGraphTexture> {
        self.depth.as_ref()
    }

    pub fn depth_format(&self) -> wgpu::TextureFormat {
        self.depth_format
    }

    pub fn set_color(&mut self, color: Option<RenderGraphTexture>) {
        self.color = color;
    }

    pub fn resize(&mut self, device: &RenderDevice, width: u32, height: u32) {
        self.size = Size::new(width, height);

        if self.color.is_some() {
            self.color = Some(RenderGraphTexture(
                device
                    .create_texture(&wgpu::TextureDescriptor {
                        label: None,
                        size: wgpu::Extent3d {
                            width,
                            height,
                            depth_or_array_layers: 1,
                        },
                        mip_level_count: 1,
                        sample_count: 1,
                        dimension: wgpu::TextureDimension::D2,
                        format: self.format,
                        usage: wgpu::TextureUsages::all(),
                        view_formats: &[self.format],
                    })
                    .create_view(&wgpu::TextureViewDescriptor::default()),
            ));
        }

        if self.depth.is_some() {
            self.depth = Some(RenderGraphTexture(
                device
                    .create_texture(&wgpu::TextureDescriptor {
                        label: None,
                        size: wgpu::Extent3d {
                            width,
                            height,
                            depth_or_array_layers: 1,
                        },
                        mip_level_count: 1,
                        sample_count: 1,
                        dimension: wgpu::TextureDimension::D2,
                        format: self.depth_format,
                        usage: wgpu::TextureUsages::all(),
                        view_formats: &[self.depth_format],
                    })
                    .create_view(&wgpu::TextureViewDescriptor::default()),
            ));
        }
    }
}

impl RenderAsset for RenderTarget {
    type Id = ResourceId;
}

#[derive(Debug, Clone)]
pub enum ResourceDesc {
    Buffer(BufferDesc),
    Texture(TextureDesc),
}

impl From<BufferDesc> for ResourceDesc {
    fn from(desc: BufferDesc) -> Self {
        Self::Buffer(desc)
    }
}

impl From<TextureDesc> for ResourceDesc {
    fn from(desc: TextureDesc) -> Self {
        Self::Texture(desc)
    }
}

pub struct RenderGraphResources {
    size: Size,
    prev_size: Size,
    texture_descs: HashMap<ResourceId, TextureDesc>,
    buffer_descs: HashMap<ResourceId, BufferDesc>,
    textures: HashMap<ResourceId, RenderGraphTexture>,
    buffers: HashMap<ResourceId, RenderGraphBuffer>,
}

impl RenderGraphResources {
    pub fn new() -> Self {
        Self {
            size: Size::new(0, 0),
            prev_size: Size::new(0, 0),
            texture_descs: HashMap::new(),
            buffer_descs: HashMap::new(),
            textures: HashMap::new(),
            buffers: HashMap::new(),
        }
    }

    pub fn size(&self) -> Size {
        self.size
    }

    pub fn prev_size(&self) -> Size {
        self.prev_size
    }

    pub fn texture(&self, id: ResourceId) -> Option<&RenderGraphTexture> {
        self.textures.get(&id)
    }

    pub fn buffer(&self, id: ResourceId) -> Option<&RenderGraphBuffer> {
        self.buffers.get(&id)
    }

    pub fn add_texture(&mut self, id: ResourceId, desc: TextureDesc) {
        self.texture_descs.insert(id, desc);
    }

    pub fn add_buffer(&mut self, id: ResourceId, desc: BufferDesc) {
        self.buffer_descs.insert(id, desc);
    }

    pub fn import_texture(&mut self, id: ResourceId, texture: RenderGraphTexture) {
        self.textures.insert(id, texture);
    }

    pub fn import_buffer(&mut self, id: ResourceId, buffer: RenderGraphBuffer) {
        self.buffers.insert(id, buffer);
    }

    pub fn set_size(&mut self, size: Size) {
        self.prev_size = self.size;

        let width = self.size.width.max(size.width);
        let height = self.size.height.max(size.height);
        self.size = Size::new(width, height);
    }

    pub fn set_prev_size(&mut self) {
        self.prev_size = self.size;
    }
}

impl Default for RenderGraphResources {
    fn default() -> Self {
        Self::new()
    }
}
