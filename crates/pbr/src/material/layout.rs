use super::{Material, MaterialType};
use ecs::core::Resource;
use graphics::{
    camera::CameraData,
    core::RenderDevice,
    resources::{
        binding::BindGroup,
        buffer::{BufferFlags, UniformBuffer, UniformBufferArray},
        RenderAsset,
    },
};
use std::sync::Arc;

pub struct MaterialLayout(wgpu::BindGroupLayout);
impl MaterialLayout {
    pub fn create<M: Material>(device: &RenderDevice) -> Self {
        Self(M::bind_group_layout(device))
    }
}

impl std::ops::Deref for MaterialLayout {
    type Target = wgpu::BindGroupLayout;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl RenderAsset for MaterialLayout {
    type Id = MaterialType;
}

pub struct GlobalBinding {
    binding: BindGroup<Arc<wgpu::BindGroupLayout>>,
    camera: UniformBufferArray<CameraData>,
}

impl GlobalBinding {
    pub fn create(device: &RenderDevice) -> Self {
        use crate::material::shader::constants::*;

        let camera = wgpu::BindGroupLayoutEntry {
            binding: CAMERA_BINDING,
            visibility: wgpu::ShaderStages::all(),
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: true,
                min_binding_size: None,
            },
            count: None,
        };

        let layout = Arc::new(
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Global Binding"),
                entries: &[camera],
            }),
        );

        let mut camera = UniformBufferArray::<CameraData>::new(BufferFlags::COPY_DST);
        camera.push(CameraData::default());
        camera.create(device);

        let binding = BindGroup::create(
            device,
            &layout.clone(),
            &[wgpu::BindGroupEntry {
                binding: CAMERA_BINDING,
                resource: camera.binding().unwrap(),
            }],
            layout,
        );

        Self { binding, camera }
    }

    pub fn binding(&self) -> &BindGroup<Arc<wgpu::BindGroupLayout>> {
        &self.binding
    }

    pub fn layout(&self) -> &wgpu::BindGroupLayout {
        self.binding.data()
    }

    pub fn camera(&self) -> &UniformBufferArray<CameraData> {
        &self.camera
    }

    pub fn camera_mut(&mut self) -> &mut UniformBufferArray<CameraData> {
        &mut self.camera
    }
}

impl Resource for GlobalBinding {}

#[derive(Debug, Default, Clone, Copy, bytemuck::Zeroable, bytemuck::Pod)]
#[repr(C)]
pub struct ObjectModel {
    pub world: glam::Mat4,
}

impl From<glam::Mat4> for ObjectModel {
    fn from(world: glam::Mat4) -> Self {
        Self { world }
    }
}

pub struct ObjectBinding {
    binding: BindGroup<Arc<wgpu::BindGroupLayout>>,
    object: UniformBuffer<ObjectModel>,
}

impl ObjectBinding {
    pub fn create(device: &RenderDevice) -> Self {
        use crate::material::shader::constants::*;

        let layout = Arc::new(
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Object Binding"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: OBJECT_BINDING,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            }),
        );

        let mut object =
            UniformBuffer::<ObjectModel>::new(ObjectModel::default(), BufferFlags::COPY_DST);
        object.create(device);

        let binding = BindGroup::create(
            device,
            &layout.clone(),
            &[wgpu::BindGroupEntry {
                binding: 0,
                resource: object.binding().unwrap(),
            }],
            layout,
        );

        Self { binding, object }
    }

    pub fn binding(&self) -> &BindGroup<Arc<wgpu::BindGroupLayout>> {
        &self.binding
    }

    pub fn layout(&self) -> &wgpu::BindGroupLayout {
        self.binding.data()
    }

    pub fn object(&self) -> &UniformBuffer<ObjectModel> {
        &self.object
    }

    pub fn object_mut(&mut self) -> &mut UniformBuffer<ObjectModel> {
        &mut self.object
    }
}

impl Resource for ObjectBinding {}
