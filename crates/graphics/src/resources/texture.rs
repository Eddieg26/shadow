use crate::core::{RenderDevice, RenderQueue, TextureFormat};
use asset::Asset;
use ecs::core::{DenseMap, Resource};

use super::{RenderResource, ResourceId};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum TextureDimension {
    D1,
    D2,
    D2Array,
    Cube,
    CubeArray,
    D3,
}

impl Into<wgpu::TextureDimension> for TextureDimension {
    fn into(self) -> wgpu::TextureDimension {
        match self {
            TextureDimension::D1 => wgpu::TextureDimension::D1,
            TextureDimension::D2 => wgpu::TextureDimension::D2,
            TextureDimension::D2Array => wgpu::TextureDimension::D2,
            TextureDimension::Cube => wgpu::TextureDimension::D3,
            TextureDimension::CubeArray => wgpu::TextureDimension::D3,
            TextureDimension::D3 => wgpu::TextureDimension::D3,
        }
    }
}

impl Into<wgpu::TextureViewDimension> for TextureDimension {
    fn into(self) -> wgpu::TextureViewDimension {
        match self {
            TextureDimension::D1 => wgpu::TextureViewDimension::D1,
            TextureDimension::D2 => wgpu::TextureViewDimension::D2,
            TextureDimension::D2Array => wgpu::TextureViewDimension::D2,
            TextureDimension::Cube => wgpu::TextureViewDimension::Cube,
            TextureDimension::CubeArray => wgpu::TextureViewDimension::Cube,
            TextureDimension::D3 => wgpu::TextureViewDimension::D3,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum FilterMode {
    Nearest,
    Linear,
}

impl Into<wgpu::FilterMode> for FilterMode {
    fn into(self) -> wgpu::FilterMode {
        match self {
            FilterMode::Nearest => wgpu::FilterMode::Nearest,
            FilterMode::Linear => wgpu::FilterMode::Linear,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum WrapMode {
    Repeat,
    ClampToEdge,
    ClampToBorder,
    MirrorRepeat,
}

impl Into<wgpu::AddressMode> for WrapMode {
    fn into(self) -> wgpu::AddressMode {
        match self {
            WrapMode::Repeat => wgpu::AddressMode::Repeat,
            WrapMode::ClampToEdge => wgpu::AddressMode::ClampToEdge,
            WrapMode::ClampToBorder => wgpu::AddressMode::ClampToBorder,
            WrapMode::MirrorRepeat => wgpu::AddressMode::MirrorRepeat,
        }
    }
}

pub trait Texture: Asset + 'static {
    fn width(&self) -> u32;
    fn height(&self) -> u32;
    fn depth(&self) -> u32;
    fn format(&self) -> TextureFormat;
    fn dimension(&self) -> TextureDimension;
    fn filter_mode(&self) -> FilterMode;
    fn wrap_mode(&self) -> WrapMode;
    fn mipmaps(&self) -> bool;
    fn usage(&self) -> wgpu::TextureUsages;
    fn pixels(&self) -> &[u8];
}

pub struct Texture2d {
    width: u32,
    height: u32,
    format: TextureFormat,
    filter_mode: FilterMode,
    wrap_mode: WrapMode,
    mipmaps: bool,
    pixels: Vec<u8>,
}

impl Texture for Texture2d {
    fn width(&self) -> u32 {
        self.width
    }

    fn height(&self) -> u32 {
        self.height
    }

    fn depth(&self) -> u32 {
        1
    }

    fn format(&self) -> TextureFormat {
        self.format
    }

    fn dimension(&self) -> TextureDimension {
        TextureDimension::D2
    }

    fn filter_mode(&self) -> FilterMode {
        self.filter_mode
    }

    fn wrap_mode(&self) -> WrapMode {
        self.wrap_mode
    }

    fn mipmaps(&self) -> bool {
        self.mipmaps
    }

    fn usage(&self) -> wgpu::TextureUsages {
        wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST
    }

    fn pixels(&self) -> &[u8] {
        &self.pixels
    }
}

impl Asset for Texture2d {}

pub struct RenderTexture {
    format: TextureFormat,
    width: u32,
    height: u32,
    filter_mode: FilterMode,
    wrap_mode: WrapMode,
    depth_format: Option<TextureFormat>,
    mipmaps: bool,
    pixels: Vec<u8>,
}

impl RenderTexture {
    pub fn new(width: u32, height: u32, format: TextureFormat, mipmaps: bool) -> Self {
        let size = format.size();
        let pixels = vec![0; (width * height * size) as usize];

        Self {
            format,
            width,
            height,
            filter_mode: FilterMode::Nearest,
            wrap_mode: WrapMode::ClampToEdge,
            depth_format: None,
            mipmaps,
            pixels,
        }
    }

    pub fn with_filter_mode(mut self, filter_mode: FilterMode) -> Self {
        self.filter_mode = filter_mode;
        self
    }

    pub fn with_wrap_mode(mut self, wrap_mode: WrapMode) -> Self {
        self.wrap_mode = wrap_mode;
        self
    }

    pub fn with_depth(mut self, format: TextureFormat) -> Self {
        self.depth_format = Some(format);
        self
    }

    pub fn depth_format(&self) -> Option<TextureFormat> {
        self.depth_format
    }
}

impl Texture for RenderTexture {
    fn width(&self) -> u32 {
        self.width
    }

    fn height(&self) -> u32 {
        self.height
    }

    fn depth(&self) -> u32 {
        1
    }

    fn format(&self) -> TextureFormat {
        self.format
    }

    fn dimension(&self) -> TextureDimension {
        TextureDimension::D2
    }

    fn filter_mode(&self) -> FilterMode {
        self.filter_mode
    }

    fn wrap_mode(&self) -> WrapMode {
        self.wrap_mode
    }

    fn mipmaps(&self) -> bool {
        self.mipmaps
    }

    fn usage(&self) -> wgpu::TextureUsages {
        wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC
    }

    fn pixels(&self) -> &[u8] {
        &self.pixels
    }
}

impl Asset for RenderTexture {}

pub struct GraphicsTexture {
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    sampler: wgpu::Sampler,
}

impl GraphicsTexture {
    pub fn create<T: Texture>(device: &RenderDevice, queue: &RenderQueue, texture: &T) -> Self {
        let size = wgpu::Extent3d {
            width: texture.width(),
            height: texture.height(),
            depth_or_array_layers: texture.depth(),
        };

        let mip_level_count = if texture.mipmaps() {
            let dimension = texture.dimension().into();
            size.max_mips(dimension)
        } else {
            1
        };

        let format = texture.format().into();

        let created = device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: wgpu::Extent3d {
                width: texture.width(),
                height: texture.height(),
                depth_or_array_layers: texture.depth(),
            },
            mip_level_count,
            sample_count: 1,
            dimension: texture.dimension().into(),
            format,
            usage: texture.usage(),
            view_formats: &[format],
        });

        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &created,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: texture.format().aspect(),
            },
            texture.pixels(),
            wgpu::ImageDataLayout {
                bytes_per_row: format
                    .block_copy_size(Some(texture.format().aspect()))
                    .map(|s| s * size.width),
                ..Default::default()
            },
            size,
        );

        let address_mode = texture.wrap_mode().into();
        let filter_mode = texture.filter_mode().into();

        let view = created.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: None,
            address_mode_u: address_mode,
            address_mode_v: address_mode,
            address_mode_w: address_mode,
            mag_filter: filter_mode,
            min_filter: filter_mode,
            mipmap_filter: filter_mode,
            lod_min_clamp: 0.0,
            lod_max_clamp: 100.0,
            compare: None,
            anisotropy_clamp: 1,
            border_color: match address_mode {
                wgpu::AddressMode::ClampToBorder => {
                    Some(wgpu::SamplerBorderColor::TransparentBlack)
                }
                _ => None,
            },
        });

        Self {
            texture: created,
            view,
            sampler,
        }
    }

    pub fn texture(&self) -> &wgpu::Texture {
        &self.texture
    }

    pub fn view(&self) -> &wgpu::TextureView {
        &self.view
    }

    pub fn sampler(&self) -> &wgpu::Sampler {
        &self.sampler
    }

    pub fn update<T: Texture>(&self, queue: &RenderQueue, texture: &T) {
        let pixels = texture.pixels();
        let aspect = texture.format().aspect();

        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &self.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect,
            },
            pixels,
            wgpu::ImageDataLayout {
                bytes_per_row: self
                    .texture
                    .format()
                    .block_copy_size(Some(aspect))
                    .map(|s| s * self.texture.size().width),
                ..Default::default()
            },
            self.texture.size(),
        );
    }
}

impl std::ops::Deref for GraphicsTexture {
    type Target = wgpu::TextureView;

    fn deref(&self) -> &Self::Target {
        &self.view
    }
}

pub struct GraphicsTextures {
    textures: DenseMap<ResourceId, GraphicsTexture>,
}

impl GraphicsTextures {
    pub fn new() -> Self {
        Self {
            textures: DenseMap::new(),
        }
    }

    pub fn create<T: Texture>(
        &mut self,
        device: &RenderDevice,
        queue: &RenderQueue,
        id: impl Into<ResourceId>,
        texture: &T,
    ) {
        let graphics_texture = GraphicsTexture::create(device, queue, texture);
        self.textures.insert(id.into(), graphics_texture);
    }

    pub fn get(&self, id: impl Into<ResourceId>) -> Option<&GraphicsTexture> {
        self.textures.get(&id.into())
    }

    pub fn get_mut(&mut self, id: impl Into<ResourceId>) -> Option<&mut GraphicsTexture> {
        self.textures.get_mut(&id.into())
    }

    pub fn remove(&mut self, id: impl Into<ResourceId>) -> Option<GraphicsTexture> {
        self.textures.remove(&id.into())
    }

    pub fn update<T: Texture>(&self, queue: &RenderQueue, id: ResourceId, texture: &T) {
        if let Some(graphics_texture) = self.textures.get(&id) {
            graphics_texture.update(queue, texture);
        }
    }
}

impl Resource for GraphicsTextures {}
impl RenderResource for GraphicsTextures {}
