use super::{registry::MaterialMeta, shader::constants, Material, MaterialBinding, MaterialType};
use crate::{
    camera::CameraData,
    core::RenderDevice,
    resources::{RenderAsset, RenderAssets},
};
use ecs::core::Resource;
use std::num::NonZero;

pub struct MaterialLayout {
    layout: wgpu::BindGroupLayout,
}

impl MaterialLayout {
    pub fn create<M: Material>(device: &RenderDevice) -> Self {
        let mut entries = vec![];
        let offset = match M::layout()
            .iter()
            .any(|b| matches!(b, MaterialBinding::Buffer))
        {
            true => {
                entries.push(wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                });
                1
            }
            false => 0,
        };

        for binding in M::layout() {
            match binding {
                MaterialBinding::Texture(dimension) => entries.push(wgpu::BindGroupLayoutEntry {
                    binding: offset + entries.len() as u32,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: (*dimension).into(),
                        multisampled: false,
                    },
                    count: None,
                }),
                MaterialBinding::Sampler => todo!(),
                _ => {}
            }
        }

        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &entries,
        });

        Self { layout }
    }

    #[inline]
    pub fn inner(&self) -> &wgpu::BindGroupLayout {
        &self.layout
    }
}

impl std::ops::Deref for MaterialLayout {
    type Target = wgpu::BindGroupLayout;
    fn deref(&self) -> &Self::Target {
        &self.layout
    }
}

impl RenderAsset for MaterialLayout {
    type Id = MaterialType;
}

pub struct MaterialLayouts {
    layouts: RenderAssets<MaterialLayout>,
    global: Option<wgpu::BindGroupLayout>,
    model: Option<wgpu::BindGroupLayout>,
}

impl MaterialLayouts {
    pub fn new() -> Self {
        Self {
            layouts: RenderAssets::new(),
            global: None,
            model: None,
        }
    }

    pub fn global(&self) -> Option<&wgpu::BindGroupLayout> {
        self.global.as_ref()
    }

    pub fn model(&self) -> Option<&wgpu::BindGroupLayout> {
        self.model.as_ref()
    }

    pub fn layout(&self, ty: MaterialType) -> Option<&MaterialLayout> {
        self.layouts.get(&ty)
    }

    pub fn add_layout(&mut self, device: &RenderDevice, meta: &MaterialMeta) {
        if !self.layouts.contains(&meta.ty) {
            self.layouts.add(meta.ty, meta.layout(device));
        }
    }

    pub fn remove_layout(&mut self, meta: &MaterialMeta) {
        self.layouts.remove(&meta.ty);
    }

    pub fn create_layouts(&mut self, device: &RenderDevice) {
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: constants::CAMERA_BINDING,
                visibility: wgpu::ShaderStages::all(),
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: NonZero::<u64>::new(
                        std::mem::size_of::<CameraData>() as wgpu::BufferAddress
                    ),
                },
                count: None,
            }],
        });

        self.global = Some(layout);

        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: constants::OBJECT_BINDING,
                visibility: wgpu::ShaderStages::all(),
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: NonZero::<u64>::new(
                        std::mem::size_of::<glam::Mat4>() as wgpu::BufferAddress
                    ),
                },
                count: None,
            }],
        });

        self.global = Some(layout);
    }
}

impl Resource for MaterialLayouts {}
