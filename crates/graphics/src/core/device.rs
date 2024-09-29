use ecs::core::Resource;
use std::sync::Arc;

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

#[derive(Clone)]
pub struct RenderDevice(Arc<wgpu::Device>);

impl RenderDevice {
    pub async fn create(
        adapter: &wgpu::Adapter,
    ) -> Result<(RenderDevice, RenderQueue), wgpu::RequestDeviceError> {
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default(), None)
            .await?;

        Ok((RenderDevice(Arc::new(device)), RenderQueue(Arc::new(queue))))
    }

    pub async fn dummy() -> (Self, RenderQueue) {
        let instance = RenderInstance::create();
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptionsBase::default())
            .await
            .unwrap();
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default(), None)
            .await
            .unwrap();

        (RenderDevice(Arc::new(device)), RenderQueue(Arc::new(queue)))
    }
}

impl std::ops::Deref for RenderDevice {
    type Target = wgpu::Device;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Resource for RenderDevice {}

#[derive(Clone)]
pub struct RenderQueue(Arc<wgpu::Queue>);

impl std::ops::Deref for RenderQueue {
    type Target = wgpu::Queue;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Resource for RenderQueue {}
