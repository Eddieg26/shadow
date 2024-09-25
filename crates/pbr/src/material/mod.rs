use asset::{Asset, AssetId};
use ecs::core::Resource;
use graphics::resources::{
    binding::{BindGroup, CreateBindGroup},
    RenderAsset,
};
use shader::fragment::MaterialShader;
use std::{collections::HashMap, hash::Hash, ops::Deref};

pub mod extractor;
pub mod layout;
pub mod pass;
pub mod pipeline;
pub mod plugin;
pub mod shader;

pub trait Material: Asset + Clone + CreateBindGroup<Data = ()> + 'static {
    fn model() -> ShaderModel;
    fn mode() -> BlendMode;
    fn shader() -> MaterialShader<Self>;
}

#[derive(Debug, Clone, Copy)]
pub struct MaterialMeta {
    pub ty: MaterialType,
    pub model: ShaderModel,
    pub mode: BlendMode,
}

impl MaterialMeta {
    pub fn from<M: Material>() -> Self {
        Self {
            ty: MaterialType::of::<M>(),
            model: M::model(),
            mode: M::mode(),
        }
    }

    pub fn is_transparent(&self) -> bool {
        self.mode == BlendMode::Transparent
    }

    pub fn is_lit(&self) -> bool {
        self.model == ShaderModel::Lit
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ShaderModel {
    Unlit,
    Lit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BlendMode {
    Opaque,
    Transparent,
}

impl BlendMode {
    pub fn state(&self) -> wgpu::BlendState {
        match self {
            Self::Opaque => wgpu::BlendState::REPLACE,
            Self::Transparent => wgpu::BlendState::ALPHA_BLENDING,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MaterialType(u32);

impl MaterialType {
    pub fn of<M: Material>() -> Self {
        let mut hasher = crc32fast::Hasher::new();
        std::any::TypeId::of::<M>().hash(&mut hasher);
        Self(hasher.finalize())
    }

    pub fn raw(id: u32) -> Self {
        Self(id)
    }
}

impl From<AssetId> for MaterialType {
    fn from(id: AssetId) -> Self {
        Self((*(id.deref())) as u32)
    }
}

impl std::ops::Deref for MaterialType {
    type Target = u32;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug, Clone)]
pub struct MaterialInstance {
    pub ty: MaterialType,
    pub model: ShaderModel,
    pub mode: BlendMode,
    pub binding: BindGroup<()>,
}

impl RenderAsset for MaterialInstance {
    type Id = AssetId;
}

#[derive(Debug, Default)]
pub struct MaterialTypeTracker {
    loaded: HashMap<MaterialType, u32>,
}

impl MaterialTypeTracker {
    pub fn new() -> Self {
        Self {
            loaded: HashMap::new(),
        }
    }

    /// Returns true if the material type is newly tracked.
    pub fn track(&mut self, ty: MaterialType) -> bool {
        match self.loaded.entry(ty) {
            std::collections::hash_map::Entry::Occupied(v) => {
                *v.into_mut() += 1;
                false
            }
            std::collections::hash_map::Entry::Vacant(v) => {
                v.insert(1);
                true
            }
        }
    }

    pub fn untrack(&mut self, ty: MaterialType) -> bool {
        if let Some(count) = self.loaded.get_mut(&ty) {
            *count -= 1;
            if *count == 0 {
                self.loaded.remove(&ty);
            }
        }

        self.loaded.contains_key(&ty)
    }

    pub fn is_loaded(&self, ty: MaterialType) -> bool {
        self.loaded.contains_key(&ty)
    }
}

impl Resource for MaterialTypeTracker {}

pub mod unlit {
    use asset::Asset;
    use graphics::{
        core::{Color, RenderDevice},
        resources::binding::{BindGroup, CreateBindGroup},
    };
    use wgpu::util::DeviceExt;

    use crate::shader::{ShaderProperty, SurfaceAttribute};

    use super::{shader::fragment::MaterialShader, BlendMode, Material, ShaderModel};

    #[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
    pub struct UnlitMaterial {
        // #[uniform(0, name = "color")]
        pub color: Color,
    }

    impl Asset for UnlitMaterial {}

    impl Material for UnlitMaterial {
        fn model() -> ShaderModel {
            ShaderModel::Unlit
        }

        fn mode() -> BlendMode {
            BlendMode::Opaque
        }

        fn shader() -> MaterialShader<Self> {
            let mut shader = MaterialShader::<Self>::new();
            shader.add_input("color", ShaderProperty::Color);
            shader.add_edge(("color", SurfaceAttribute::Color));
            shader
        }
    }

    impl CreateBindGroup for UnlitMaterial {
        type Arg = ();
        type Data = ();

        fn bind_group(
            &self,
            device: &RenderDevice,
            layout: &wgpu::BindGroupLayout,
            _: &ecs::system::ArgItem<Self::Arg>,
        ) -> Option<BindGroup<Self::Data>> {
            let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: None,
                contents: bytemuck::bytes_of(&self.color),
                usage: wgpu::BufferUsages::UNIFORM,
            });

            let binding = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Self::label(),
                layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: buffer.as_entire_binding(),
                }],
            });

            Some(BindGroup::from(binding))
        }

        fn bind_group_layout(device: &RenderDevice) -> wgpu::BindGroupLayout {
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Self::label(),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::all(),
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            })
        }
    }
}
