use std::collections::HashMap;

use crate::resources::ResourceId;

pub struct TextureDesc {
    pub format: wgpu::TextureFormat,
    pub usage: wgpu::TextureUsages,
}

pub struct BufferDesc {
    pub size: wgpu::BufferAddress,
    pub usage: wgpu::BufferUsages,
}

pub struct RenderTargetDesc {
    pub width: u32,
    pub height: u32,
    pub format: wgpu::TextureFormat,
    pub depth_format: Option<wgpu::TextureFormat>,
}

pub struct RenderTarget {
    width: u32,
    height: u32,
    color: Option<wgpu::TextureView>,
    depth: Option<wgpu::TextureView>,
    textures: HashMap<ResourceId, wgpu::TextureView>,
}

impl RenderTarget {
    pub fn create(device: &wgpu::Device, desc: RenderTargetDesc) -> Self {
        let size = wgpu::Extent3d {
            width: desc.width,
            height: desc.height,
            depth_or_array_layers: 1,
        };

        let color = device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size,
            dimension: wgpu::TextureDimension::D2,
            format: desc.format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::COPY_SRC
                | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[desc.format],
            mip_level_count: 1,
            sample_count: 1,
        });

        let depth = desc.depth_format.map(|format| {
            device.create_texture(&wgpu::TextureDescriptor {
                label: None,
                size,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::COPY_SRC
                    | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[format],
                mip_level_count: 1,
                sample_count: 1,
            })
        });

        Self {
            width: desc.width,
            height: desc.height,
            color: Some(color.create_view(&wgpu::TextureViewDescriptor::default())),
            depth: depth.map(|d| d.create_view(&wgpu::TextureViewDescriptor::default())),
            textures: HashMap::new(),
        }
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn color(&self) -> Option<&wgpu::TextureView> {
        self.color.as_ref()
    }

    pub fn depth(&self) -> Option<&wgpu::TextureView> {
        self.depth.as_ref()
    }

    pub fn texture(&self, id: ResourceId) -> Option<&wgpu::TextureView> {
        self.textures.get(&id)
    }

    pub fn add_texture(&mut self, id: ResourceId, texture: wgpu::TextureView) {
        self.textures.insert(id, texture);
    }

    pub(crate) fn set_color(&mut self, color: Option<wgpu::TextureView>) {
        self.color = color
    }
}

pub struct RenderGraphResources {
    targets: HashMap<ResourceId, RenderTarget>,
    buffers: HashMap<ResourceId, wgpu::Buffer>,
    textures: HashMap<ResourceId, wgpu::TextureView>,
    texture_descs: HashMap<ResourceId, TextureDesc>,
    buffer_descs: HashMap<ResourceId, BufferDesc>,
}

impl RenderGraphResources {
    pub fn new() -> Self {
        Self {
            targets: HashMap::new(),
            buffers: HashMap::new(),
            textures: HashMap::new(),
            texture_descs: HashMap::new(),
            buffer_descs: HashMap::new(),
        }
    }

    pub fn target(&self, id: ResourceId) -> Option<&RenderTarget> {
        self.targets.get(&id)
    }

    pub fn texture(&self, id: ResourceId) -> Option<&wgpu::TextureView> {
        self.textures.get(&id)
    }

    pub fn buffer(&self, id: ResourceId) -> Option<&wgpu::Buffer> {
        self.buffers.get(&id)
    }

    pub fn create_texture(&mut self, id: ResourceId, desc: TextureDesc) {
        self.texture_descs.insert(id, desc);
    }

    pub fn create_buffer(&mut self, id: ResourceId, desc: BufferDesc) {
        self.buffer_descs.insert(id, desc);
    }

    pub fn create_target(&mut self, device: &wgpu::Device, id: ResourceId, desc: RenderTargetDesc) {
        let size = wgpu::Extent3d {
            width: desc.width,
            height: desc.height,
            depth_or_array_layers: 1,
        };
        let mut target = RenderTarget::create(device, desc);
        for info in &self.texture_descs {
            let texture = device.create_texture(&wgpu::TextureDescriptor {
                label: None,
                size,
                dimension: wgpu::TextureDimension::D2,
                format: info.1.format,
                usage: info.1.usage,
                view_formats: &[info.1.format],
                mip_level_count: 1,
                sample_count: 1,
            });

            target.add_texture(
                *info.0,
                texture.create_view(&wgpu::TextureViewDescriptor::default()),
            );
        }

        self.targets.insert(id, target);
    }

    pub fn import_texture(&mut self, id: ResourceId, texture: wgpu::TextureView) {
        self.textures.insert(id, texture);
    }

    pub fn import_buffer(&mut self, id: ResourceId, buffer: wgpu::Buffer) {
        self.buffers.insert(id, buffer);
    }

    pub fn remove_render_target(&mut self, id: ResourceId) -> Option<RenderTarget> {
        self.targets.remove(&id)
    }

    pub(crate) fn set_target_color(&mut self, id: ResourceId, color: Option<wgpu::TextureView>) {
        if let Some(target) = self.targets.get_mut(&id) {
            target.set_color(color);
        }
    }
}
