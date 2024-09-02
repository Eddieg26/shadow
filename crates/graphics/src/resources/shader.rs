use asset::{
    io::{AssetIoError, AssetReader, PathExt},
    importer::{AssetImporter, ImportContext},
    Asset, AssetId, DefaultSettings,
};
use ecs::core::{DenseMap, Resource};
use std::borrow::Cow;

use crate::core::RenderDevice;

use super::{RenderAsset, RenderAssetUsage, RenderResource};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum ShaderStage {
    Vertex,
    Fragment,
    Compute,
}

impl Into<naga::ShaderStage> for ShaderStage {
    fn into(self) -> naga::ShaderStage {
        match self {
            Self::Vertex => naga::ShaderStage::Vertex,
            Self::Fragment => naga::ShaderStage::Fragment,
            Self::Compute => naga::ShaderStage::Compute,
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum ShaderSource {
    Spirv(Cow<'static, [u32]>),
    Glsl {
        shader: Cow<'static, str>,
        stage: ShaderStage,
    },
    Wgsl(Cow<'static, str>),
}

#[derive(Debug)]
pub enum ShaderLoadError {
    Io(AssetIoError),
    Parse(String),
}

impl From<AssetIoError> for ShaderLoadError {
    fn from(err: AssetIoError) -> Self {
        Self::Io(err)
    }
}

impl std::fmt::Display for ShaderLoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(err) => write!(f, "IO error: {}", err),
            Self::Parse(err) => write!(f, "Parse error: {}", err),
        }
    }
}

impl std::error::Error for ShaderLoadError {}

impl Asset for ShaderSource {}

impl AssetImporter for ShaderSource {
    type Asset = Self;
    type Settings = DefaultSettings;
    type Error = ShaderLoadError;

    fn import(
        _: &mut ImportContext<Self::Settings>,
        reader: &mut dyn AssetReader,
    ) -> Result<Self::Asset, Self::Error> {
        let path = reader.path();
        let ext = path.ext();

        match ext {
            Some("spv") => {
                reader.read_to_end().map_err(ShaderLoadError::Io)?;
                let data = reader.flush().map_err(ShaderLoadError::Io)?;
                let data = data.iter().map(|b| *b as u32).collect();
                Ok(ShaderSource::Spirv(Cow::Owned(data)))
            }
            Some("wgsl") => {
                let data = reader.read_to_string().map_err(ShaderLoadError::Io)?;
                Ok(ShaderSource::Wgsl(Cow::Owned(data)))
            }
            Some("vert") => {
                let data = reader.read_to_string().map_err(ShaderLoadError::Io)?;
                Ok(ShaderSource::Glsl {
                    shader: Cow::Owned(data),
                    stage: ShaderStage::Vertex,
                })
            }
            Some("frag") => {
                let data = reader.read_to_string().map_err(ShaderLoadError::Io)?;
                Ok(ShaderSource::Glsl {
                    shader: Cow::Owned(data),
                    stage: ShaderStage::Fragment,
                })
            }
            Some("comp") => {
                let data = reader.read_to_string().map_err(ShaderLoadError::Io)?;
                Ok(ShaderSource::Glsl {
                    shader: Cow::Owned(data),
                    stage: ShaderStage::Compute,
                })
            }
            _ => Err(ShaderLoadError::Parse(format!(
                "Invalid extension: {:?}",
                ext
            ))),
        }
    }

    fn extensions() -> &'static [&'static str] {
        &["spv", "wgsl", "vert", "frag", "comp"]
    }
}

impl RenderAsset for ShaderSource {
    type Asset = ShaderSource;
    type Arg<'a> = (&'a RenderDevice, &'a mut Shaders);
    type Error = ShaderLoadError;

    fn extract<'a>(
        id: &AssetId,
        asset: &mut Self::Asset,
        arg: &mut Self::Arg<'a>,
    ) -> Result<RenderAssetUsage, Self::Error> {
        let shader = Shader::create(arg.0, &asset);
        arg.1.insert(*id, shader);
        Ok(RenderAssetUsage::Discard)
    }

    fn remove<'a>(id: AssetId, arg: &mut Self::Arg<'a>) {
        arg.1.remove(&id);
    }
}

pub struct Shader(wgpu::ShaderModule);

impl Shader {
    pub fn create(device: &wgpu::Device, source: &ShaderSource) -> Self {
        let module = match source {
            ShaderSource::Spirv(data) => {
                device.create_shader_module(wgpu::ShaderModuleDescriptor {
                    label: None,
                    source: wgpu::ShaderSource::SpirV(data.clone()),
                })
            }
            ShaderSource::Glsl { shader, stage } => {
                device.create_shader_module(wgpu::ShaderModuleDescriptor {
                    label: None,
                    source: wgpu::ShaderSource::Glsl {
                        shader: shader.clone(),
                        stage: (*stage).into(),
                        defines: Default::default(),
                    },
                })
            }
            ShaderSource::Wgsl(data) => device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: None,
                source: wgpu::ShaderSource::Wgsl(data.clone()),
            }),
        };

        Self(module)
    }
}

impl std::ops::Deref for Shader {
    type Target = wgpu::ShaderModule;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub struct Shaders {
    shaders: DenseMap<AssetId, Shader>,
}

impl Shaders {
    pub fn new() -> Self {
        Self {
            shaders: DenseMap::new(),
        }
    }

    pub fn insert(&mut self, id: impl Into<AssetId>, shader: Shader) {
        self.shaders.insert(id.into(), shader);
    }

    pub fn get(&self, id: &AssetId) -> Option<&Shader> {
        self.shaders.get(id)
    }

    pub fn remove(&mut self, id: &AssetId) -> Option<Shader> {
        self.shaders.remove(id)
    }

    pub fn clear(&mut self) {
        self.shaders.clear();
    }

    pub fn iter(&self) -> impl Iterator<Item = (&AssetId, &Shader)> {
        self.shaders.iter()
    }
}

impl Resource for Shaders {}
impl RenderResource for Shaders {}
