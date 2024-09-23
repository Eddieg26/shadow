use crate::core::RenderDevice;
use ecs::system::{ArgItem, SystemArg};
use std::sync::Arc;
use wgpu::{BindGroupEntry, BindGroupLayout};

#[derive(Debug, Clone)]
pub struct BindGroup<D: Send + Sync + Clone + 'static = ()> {
    binding: Arc<wgpu::BindGroup>,
    data: D,
}

impl<D: Send + Sync + Clone + 'static> BindGroup<D> {
    pub fn create(
        device: &RenderDevice,
        layout: &wgpu::BindGroupLayout,
        entries: &[BindGroupEntry],
        data: D,
    ) -> Self {
        let binding = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout,
            entries,
        });

        Self {
            binding: Arc::new(binding),
            data,
        }
    }

    #[inline]
    pub fn inner(&self) -> &wgpu::BindGroup {
        &self.binding
    }

    pub fn data(&self) -> &D {
        &self.data
    }
}

impl From<wgpu::BindGroup> for BindGroup<()> {
    fn from(binding: wgpu::BindGroup) -> Self {
        Self {
            binding: Arc::new(binding),
            data: (),
        }
    }
}

impl<D: Send + Sync + Clone + 'static> std::ops::Deref for BindGroup<D> {
    type Target = wgpu::BindGroup;

    fn deref(&self) -> &Self::Target {
        &self.binding
    }
}

pub trait CreateBindGroup {
    type Arg: SystemArg + 'static;
    type Data: Send + Sync + Clone + 'static;

    fn label() -> Option<&'static str> {
        None
    }

    fn bind_group(
        &self,
        device: &RenderDevice,
        layout: &wgpu::BindGroupLayout,
        arg: &ArgItem<Self::Arg>,
    ) -> Option<BindGroup<Self::Data>>;
    fn bind_group_layout(device: &RenderDevice) -> BindGroupLayout;
}
