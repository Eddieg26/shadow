use shadow_ecs::core::Resource;

pub struct RenderInstance(wgpu::Instance);

impl RenderInstance {
    pub fn create() -> Self {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });

        Self(instance)
    }
}

impl std::ops::Deref for RenderInstance {
    type Target = wgpu::Instance;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub struct RenderDevice(wgpu::Device);

impl RenderDevice {
    pub async fn create(
        adapter: &wgpu::Adapter,
    ) -> Result<(RenderDevice, RenderQueue), wgpu::RequestDeviceError> {
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default(), None)
            .await?;

        Ok((RenderDevice(device), RenderQueue(queue)))
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

impl std::ops::Deref for RenderQueue {
    type Target = wgpu::Queue;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Resource for RenderQueue {}
