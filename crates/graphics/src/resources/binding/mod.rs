use crate::core::RenderDevice;
use std::num::NonZeroU32;

pub type ShaderBinding = wgpu::BindGroupLayoutEntry;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShaderBindGroup {
    group: u32,
    bindings: Vec<ShaderBinding>,
}

impl ShaderBindGroup {
    pub fn new(group: u32) -> Self {
        Self {
            group,
            bindings: Vec::new(),
        }
    }

    pub fn group(&self) -> u32 {
        self.group
    }

    pub fn add(&mut self, binding: ShaderBinding) -> &mut Self {
        self.bindings.push(binding);
        self
    }

    pub fn add_binding(
        &mut self,
        binding: u32,
        visibility: wgpu::ShaderStages,
        ty: wgpu::BindingType,
        count: Option<u32>,
    ) -> &mut Self {
        self.bindings.push(ShaderBinding {
            binding,
            visibility,
            ty,
            count: count.and_then(|c| NonZeroU32::new(c)),
        });
        self
    }

    pub fn add_buffer(
        &mut self,
        binding: u32,
        visibility: wgpu::ShaderStages,
        ty: wgpu::BufferBindingType,
        has_dynamic_offset: bool,
        min_binding_size: Option<wgpu::BufferSize>,
    ) -> &mut Self {
        self.add_binding(
            binding,
            visibility,
            wgpu::BindingType::Buffer {
                has_dynamic_offset,
                min_binding_size,
                ty,
            },
            None,
        )
    }

    pub fn add_texture(
        &mut self,
        binding: u32,
        visibility: wgpu::ShaderStages,
        sample_type: wgpu::TextureSampleType,
        view_dimension: wgpu::TextureViewDimension,
        multisampled: bool,
    ) -> &mut Self {
        self.add_binding(
            binding,
            visibility,
            wgpu::BindingType::Texture {
                sample_type,
                view_dimension,
                multisampled,
            },
            None,
        )
    }

    pub fn add_sampler(
        &mut self,
        binding: u32,
        visibility: wgpu::ShaderStages,
        ty: wgpu::SamplerBindingType,
    ) -> &mut Self {
        self.add_binding(binding, visibility, wgpu::BindingType::Sampler(ty), None)
    }

    pub fn bindings(&self) -> &[ShaderBinding] {
        &self.bindings
    }

    pub fn create_layout(&self, device: &RenderDevice) -> wgpu::BindGroupLayout {
        let entries = self.bindings();
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &entries,
            label: None,
        })
    }
}

pub struct BindGroup(wgpu::BindGroup);

impl BindGroup {
    pub fn create(
        device: &RenderDevice,
        layout: &wgpu::BindGroupLayout,
        entries: &[wgpu::BindGroupEntry],
    ) -> Self {
        Self(device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout,
            entries,
            label: None,
        }))
    }
}

impl std::ops::Deref for BindGroup {
    type Target = wgpu::BindGroup;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
