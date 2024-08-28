use asset::Asset;
use wgpu::{FilterMode, TextureFormat, TextureViewDimension};

pub type WrapMode = wgpu::AddressMode;

pub trait Texture: Asset + 'static {
    fn width(&self) -> u32;
    fn height(&self) -> u32;
    fn depth(&self) -> u32;
    fn format(&self) -> TextureFormat;
    fn dimension(&self) -> TextureViewDimension;
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

    fn dimension(&self) -> TextureViewDimension {
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
        wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST
    }

    fn pixels(&self) -> &[u8] {
        &self.pixels
    }

    fn pixels_mut(&mut self) -> &mut Vec<u8> {
        &mut self.pixels
    }
}

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
        let size = format.target_pixel_byte_cost().unwrap_or(4);
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

impl Asset for RenderTexture {}

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

    fn dimension(&self) -> TextureViewDimension {
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
        wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC
    }

    fn pixels(&self) -> &[u8] {
        &self.pixels
    }

    fn pixels_mut(&mut self) -> &mut Vec<u8> {
        &mut self.pixels
    }
}
