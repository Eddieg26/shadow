use asset::Asset;
use std::num::NonZeroU32;
use wgpu::ShaderStages;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ShaderKind {
    None,
    Vertex,
    Fragment,
    Compute,
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct ShaderBinding {
    binding: u32,
    ty: wgpu::BindingType,
    stages: ShaderStages,
    count: u32,
}

impl ShaderBinding {
    pub fn new(binding: u32, ty: wgpu::BindingType, stages: ShaderStages, count: u32) -> Self {
        Self {
            binding,
            ty,
            stages,
            count,
        }
    }

    pub fn binding(&self) -> u32 {
        self.binding
    }

    pub fn ty(&self) -> wgpu::BindingType {
        self.ty
    }

    pub fn stages(&self) -> ShaderStages {
        self.stages
    }

    pub fn count(&self) -> u32 {
        self.count
    }
}

impl Into<wgpu::BindGroupLayoutEntry> for &ShaderBinding {
    fn into(self) -> wgpu::BindGroupLayoutEntry {
        wgpu::BindGroupLayoutEntry {
            binding: self.binding,
            visibility: self.stages,
            ty: self.ty,
            count: NonZeroU32::new(self.count),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ShaderBindGroup {
    group: u32,
    bindings: Vec<ShaderBinding>,
}

impl ShaderBindGroup {
    pub fn new(group: u32, bindings: Vec<ShaderBinding>) -> Self {
        Self { group, bindings }
    }

    pub fn group(&self) -> u32 {
        self.group
    }

    pub fn bindings(&self) -> &[ShaderBinding] {
        &self.bindings
    }
}

pub trait Shader: Asset + 'static {
    fn source(&self) -> &str;
    fn source_mut(&mut self) -> &mut String;
    fn bindings(&self, device: &wgpu::Device) -> &[ShaderBindGroup];
    fn kind() -> ShaderKind;
}

pub struct VertexShader {
    source: String,
    bindings: Vec<ShaderBindGroup>,
}

pub struct GpuShader {
    entry: String,
    module: wgpu::ShaderModule,
    bindings: Vec<wgpu::BindGroupLayout>,
    kind: ShaderKind,
}

impl GpuShader {
    pub fn create<S: Shader>(device: &wgpu::Device, shader: &mut S) -> Option<Self> {
        let content = shader.source();
        match content.is_empty() {
            true => None,
            false => {
                let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                    label: None,
                    source: wgpu::ShaderSource::Wgsl(content.into()),
                });

                let mut bindings = Vec::new();
                for group in shader.bindings(device) {
                    let entries = group
                        .bindings()
                        .iter()
                        .map(|binding| binding.into())
                        .collect::<Vec<_>>();

                    let layout =
                        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                            label: None,
                            entries: &entries,
                        });

                    bindings.push(layout);
                }

                Some(Self {
                    entry: "main".to_string(),
                    module,
                    bindings,
                    kind: S::kind(),
                })
            }
        }
    }
}
