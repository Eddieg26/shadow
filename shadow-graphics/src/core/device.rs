use shadow_ecs::core::Resource;

pub struct RenderDevice(wgpu::Device);

impl RenderDevice {
    pub fn new(device: wgpu::Device) -> Self {
        Self(device)
    }
}

impl std::ops::Deref for RenderDevice {
    type Target = wgpu::Device;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Resource for RenderDevice {}

pub struct RenderQueue(wgpu::Queue);

impl RenderQueue {
    pub fn new(queue: wgpu::Queue) -> Self {
        Self(queue)
    }
}

impl std::ops::Deref for RenderQueue {
    type Target = wgpu::Queue;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Resource for RenderQueue {}
