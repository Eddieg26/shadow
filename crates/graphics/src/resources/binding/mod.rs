use crate::core::RenderDevice;

pub type ShaderBinding = wgpu::BindGroupLayoutEntry;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShaderBindings {
    bindings: Vec<ShaderBinding>,
}

impl ShaderBindings {
    pub fn new() -> Self {
        Self {
            bindings: Vec::new(),
        }
    }

    pub fn add(&mut self, binding: ShaderBinding) -> &mut Self {
        self.bindings.push(binding);
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
        self.bindings.push(ShaderBinding {
            binding,
            visibility,
            ty: wgpu::BindingType::Buffer {
                ty,
                has_dynamic_offset,
                min_binding_size,
            },
            count: None,
        });
        self
    }

    pub fn add_texture(
        &mut self,
        binding: u32,
        visibility: wgpu::ShaderStages,
        sample_type: wgpu::TextureSampleType,
        view_dimension: wgpu::TextureViewDimension,
        multisampled: bool,
    ) -> &mut Self {
        self.bindings.push(ShaderBinding {
            binding,
            visibility,
            ty: wgpu::BindingType::Texture {
                sample_type,
                view_dimension,
                multisampled,
            },
            count: None,
        });
        self
    }

    pub fn add_sampler(
        &mut self,
        binding: u32,
        visibility: wgpu::ShaderStages,
        ty: wgpu::SamplerBindingType,
    ) -> &mut Self {
        self.bindings.push(ShaderBinding {
            binding,
            visibility,
            ty: wgpu::BindingType::Sampler(ty),
            count: None,
        });
        self
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

pub struct BindGroupLayout(wgpu::BindGroupLayout);
impl From<wgpu::BindGroupLayout> for BindGroupLayout {
    fn from(layout: wgpu::BindGroupLayout) -> Self {
        Self(layout)
    }
}

impl std::ops::Deref for BindGroupLayout {
    type Target = wgpu::BindGroupLayout;
    fn deref(&self) -> &Self::Target {
        &self.0
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

impl From<wgpu::BindGroup> for BindGroup {
    fn from(binding: wgpu::BindGroup) -> Self {
        Self(binding)
    }
}
