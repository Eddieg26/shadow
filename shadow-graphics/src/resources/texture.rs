use shadow_asset::asset::Asset;
use wgpu::{FilterMode, TextureDimension, TextureFormat, TextureViewDimension};

pub type WrapMode = wgpu::AddressMode;

pub trait Texture: Asset + 'static {
    fn width(&self) -> u32;
    fn height(&self) -> u32;
    fn depth(&self) -> u32;
    fn format(&self) -> TextureFormat;
    fn dimension(&self) -> TextureDimension;
    fn view_dimension(&self) -> TextureViewDimension;
    fn filter_mode(&self) -> FilterMode;
    fn wrap_mode(&self) -> WrapMode;
    fn mipmaps(&self) -> bool;
    fn usage(&self) -> wgpu::TextureUsages;
    fn pixels(&self) -> &[u8];
    fn pixels_mut(&mut self) -> &mut Vec<u8>;
}

pub struct Texture2d {
    width: u32,
    height: u32,
    format: TextureFormat,
    filter_mode: FilterMode,
    wrap_mode: WrapMode,
    mipmaps: bool,
    usage: wgpu::TextureUsages,
    pixels: Vec<u8>,
}

impl Asset for Texture2d {}

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

    fn view_dimension(&self) -> TextureViewDimension {
        TextureViewDimension::D2
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
        self.usage
    }

    fn pixels(&self) -> &[u8] {
        &self.pixels
    }

    fn pixels_mut(&mut self) -> &mut Vec<u8> {
        &mut self.pixels
    }
}

pub struct GpuTexture {
    handle: wgpu::Texture,
    view: wgpu::TextureView,
    sampler: wgpu::Sampler,
}

impl GpuTexture {
    pub fn create<T: Texture>(device: &wgpu::Device, texture: &T) -> Self {
        let size = wgpu::Extent3d {
            width: texture.width(),
            height: texture.height(),
            depth_or_array_layers: texture.depth(),
        };

        let mip_level_count = if texture.mipmaps() {
            size.max_mips(texture.dimension())
        } else {
            0
        };

        let handle = device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size,
            mip_level_count,
            sample_count: 1,
            dimension: texture.dimension(),
            format: texture.format(),
            usage: texture.usage(),
            view_formats: &[texture.format()],
        });

        let view = handle.create_view(&wgpu::TextureViewDescriptor {
            label: None,
            format: Some(texture.format()),
            dimension: Some(texture.view_dimension()),
            aspect: wgpu::TextureAspect::All,
            base_mip_level: 0,
            mip_level_count: Some(mip_level_count),
            base_array_layer: 0,
            array_layer_count: None,
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: None,
            address_mode_u: texture.wrap_mode(),
            address_mode_v: texture.wrap_mode(),
            address_mode_w: texture.wrap_mode(),
            mag_filter: texture.filter_mode(),
            min_filter: texture.filter_mode(),
            mipmap_filter: texture.filter_mode(),
            ..Default::default()
        });

        Self {
            handle,
            view,
            sampler,
        }
    }

    pub fn handle(&self) -> &wgpu::Texture {
        &self.handle
    }

    pub fn view(&self) -> &wgpu::TextureView {
        &self.view
    }

    pub fn sampler(&self) -> &wgpu::Sampler {
        &self.sampler
    }
}
