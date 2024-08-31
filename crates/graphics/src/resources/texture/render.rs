use super::{FilterMode, Texture, TextureDimension, TextureFormat, WrapMode};
use asset::{loader::AssetLoader, Asset, Settings};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RenderTexture {
    format: TextureFormat,
    width: u32,
    height: u32,
    filter_mode: FilterMode,
    wrap_mode: WrapMode,
    depth_format: Option<TextureFormat>,
    mipmaps: bool,

    #[serde(skip)]
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

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RenderTextureSettings {
    pub width: u32,
    pub height: u32,
    pub format: TextureFormat,
    pub mipmaps: bool,
    pub filter_mode: FilterMode,
    pub wrap_mode: WrapMode,
}

impl Default for RenderTextureSettings {
    fn default() -> Self {
        Self {
            width: 1024,
            height: 1024,
            format: TextureFormat::Rgba8Unorm,
            mipmaps: false,
            filter_mode: FilterMode::Nearest,
            wrap_mode: WrapMode::ClampToEdge,
        }
    }
}

impl Settings for RenderTextureSettings {}

#[derive(Debug, Clone)]
pub struct RenderTextureLoadError;

impl std::fmt::Display for RenderTextureLoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "failed to load render texture")
    }
}

impl std::error::Error for RenderTextureLoadError {}

impl AssetLoader for RenderTexture {
    type Asset = RenderTexture;
    type Settings = RenderTextureSettings;
    type Error = RenderTextureLoadError;

    fn load(
        ctx: &mut asset::loader::LoadContext<Self::Settings>,
        _: &mut dyn asset::io::AssetReader,
    ) -> Result<Self::Asset, Self::Error> {
        let settings = ctx.settings();
        let asset = RenderTexture::new(
            settings.width,
            settings.height,
            settings.format,
            settings.mipmaps,
        )
        .with_filter_mode(settings.filter_mode)
        .with_wrap_mode(settings.wrap_mode);

        Ok(asset)
    }

    fn extensions() -> &'static [&'static str] {
        &["rt"]
    }
}
