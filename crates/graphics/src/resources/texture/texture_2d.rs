use super::{FilterMode, Texture, TextureDimension, TextureFormat, WrapMode};
use asset::{loader::AssetLoader, Asset, Settings};

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct Texture2d {
    width: u32,
    height: u32,
    format: TextureFormat,
    filter_mode: FilterMode,
    wrap_mode: WrapMode,
    mipmaps: bool,
    pixels: Vec<u8>,
}

impl Texture2d {
    pub fn new(
        width: u32,
        height: u32,
        format: TextureFormat,
        filter_mode: FilterMode,
        wrap_mode: WrapMode,
        mipmaps: bool,
        pixels: Vec<u8>,
    ) -> Self {
        Self {
            width,
            height,
            format,
            filter_mode,
            wrap_mode,
            mipmaps,
            pixels,
        }
    }
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

impl std::fmt::Display for Texture2d {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Texture2d {{ width: {}, height: {}, format: {:?}, filter_mode: {:?}, wrap_mode: {:?}, mipmaps: {} }}",
            self.width, self.height, self.format, self.filter_mode, self.wrap_mode, self.mipmaps
        )
    }
}

impl std::fmt::Debug for Texture2d {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self, f)
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Texture2dSettings {
    pub format: TextureFormat,
    pub filter_mode: FilterMode,
    pub wrap_mode: WrapMode,
    pub mipmaps: bool,
}

impl Default for Texture2dSettings {
    fn default() -> Self {
        Self {
            format: TextureFormat::Rgba8Unorm,
            filter_mode: FilterMode::Linear,
            wrap_mode: WrapMode::ClampToEdge,
            mipmaps: false,
        }
    }
}

impl Settings for Texture2dSettings {}

impl AssetLoader for Texture2d {
    type Asset = Self;
    type Settings = Texture2dSettings;
    type Error = image::ImageError;

    fn load(
        ctx: &mut asset::loader::LoadContext<Self::Settings>,
        reader: &mut dyn asset::io::AssetReader,
    ) -> Result<Self::Asset, Self::Error> {
        let image = image::ImageReader::open(reader.path())?.decode()?;
        let width = image.width();
        let height = image.height();

        let pixels = match ctx.settings().format {
            TextureFormat::R8Unorm
            | TextureFormat::R8Snorm
            | TextureFormat::R8Uint
            | TextureFormat::R8Sint => image.into_bytes(),
            TextureFormat::R16Uint | TextureFormat::R16Sint | TextureFormat::R16Float => image
                .into_rgba16()
                .into_raw()
                .iter()
                .map(|value| value.to_ne_bytes())
                .flatten()
                .collect(),
            TextureFormat::R32Uint
            | TextureFormat::R32Sint
            | TextureFormat::R32Float
            | TextureFormat::Rg16Uint
            | TextureFormat::Rg16Sint
            | TextureFormat::Rg16Float
            | TextureFormat::Rgba8Unorm
            | TextureFormat::Rgba8UnormSrgb
            | TextureFormat::Rgba8Snorm
            | TextureFormat::Rgba8Uint
            | TextureFormat::Rgba8Sint => image.into_rgba8().into_raw(),
            TextureFormat::Bgra8Unorm => {
                let mut bytes = image.into_rgba8().into_raw();
                for index in (0..bytes.len()).step_by(4) {
                    bytes.swap(index, index + 2);
                }
                bytes
            }
            TextureFormat::Bgra8UnormSrgb => {
                let mut bytes = image.into_rgba8().into_raw();
                for index in (0..bytes.len()).step_by(4) {
                    bytes.swap(index, index + 2);
                }
                bytes
            }
            TextureFormat::Rg32Uint => image
                .into_rgba8()
                .into_raw()
                .iter()
                .map(|value| {
                    let value: u32 = value.scale();
                    value.to_ne_bytes()
                })
                .flatten()
                .collect(),
            TextureFormat::Rg32Sint => image
                .into_rgba8()
                .into_raw()
                .iter()
                .map(|value| {
                    let value: i32 = value.scale();
                    value.to_ne_bytes()
                })
                .flatten()
                .collect(),
            TextureFormat::Rg32Float => image
                .into_rgba32f()
                .into_raw()
                .iter()
                .map(|value| value.to_ne_bytes())
                .flatten()
                .collect(),
            TextureFormat::Rgba16Uint | TextureFormat::Rgba16Sint | TextureFormat::Rgba16Float => {
                image
                    .into_rgba16()
                    .into_raw()
                    .iter()
                    .map(|value| value.to_ne_bytes())
                    .flatten()
                    .collect()
            }
            TextureFormat::Rgba32Uint | TextureFormat::Rgba32Sint | TextureFormat::Rgba32Float => {
                image
                    .into_rgba32f()
                    .into_raw()
                    .iter()
                    .map(|value| value.to_ne_bytes())
                    .flatten()
                    .collect()
            }
            _ => {
                return Err(image::ImageError::IoError(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("Unsupported texture format: {:?}", ctx.settings().format),
                )))
            }
        };

        Ok(Self {
            width,
            height,
            format: ctx.settings().format,
            filter_mode: ctx.settings().filter_mode,
            wrap_mode: ctx.settings().wrap_mode,
            mipmaps: ctx.settings().mipmaps,
            pixels,
        })
    }

    fn extensions() -> &'static [&'static str] {
        &["png", "jpeg", "jpg", "tiff", "bmp", "tga"]
    }
}

pub trait ScaleNumber<O: 'static> {
    fn scale(&self) -> O;
}

impl ScaleNumber<u16> for u8 {
    fn scale(&self) -> u16 {
        let value = *self as f32;
        let percentage = (value / 255.0) * 100.0;
        percentage as u16 * (u16::MAX / 100)
    }
}

impl ScaleNumber<u32> for u8 {
    fn scale(&self) -> u32 {
        let value = *self as f32;
        let percentage = (value / 255.0) * 100.0;
        percentage as u32 * (u32::MAX / 100)
    }
}

impl ScaleNumber<i32> for u8 {
    fn scale(&self) -> i32 {
        let value = *self as f32;
        let percentage = (value / 255.0) * 100.0;
        percentage as i32 * (i32::MAX / 100)
    }
}

impl ScaleNumber<f32> for u8 {
    fn scale(&self) -> f32 {
        let value = *self as f32;
        let percentage = (value / 255.0) * 100.0;
        percentage as f32 * (f32::MAX / 100.0)
    }
}

impl ScaleNumber<u32> for u16 {
    fn scale(&self) -> u32 {
        let value = *self as f32;
        let percentage = (value / 65535.0) * 100.0;
        percentage as u32 * (u32::MAX / 100)
    }
}
