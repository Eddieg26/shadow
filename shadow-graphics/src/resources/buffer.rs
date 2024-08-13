use super::GpuResourceId;
use std::collections::HashMap;

pub struct BufferInfo {
    pub size: u64,
    pub usage: wgpu::BufferUsages,
    pub mapped_at_creation: bool,
}

impl BufferInfo {
    pub fn new(size: u64, usage: wgpu::BufferUsages) -> Self {
        Self {
            size,
            usage,
            mapped_at_creation: false,
        }
    }

    pub fn with_mapped_at_creation(mut self, mapped_at_creation: bool) -> Self {
        self.mapped_at_creation = mapped_at_creation;
        self
    }
}

pub struct VertexBuffers {
    buffers: HashMap<GpuResourceId, wgpu::Buffer>,
}

impl VertexBuffers {
    pub fn new() -> Self {
        Self {
            buffers: HashMap::new(),
        }
    }

    pub fn create(&mut self, device: &wgpu::Device, id: GpuResourceId, info: BufferInfo) {
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: info.size,
            usage: info.usage | wgpu::BufferUsages::VERTEX,
            mapped_at_creation: info.mapped_at_creation,
        });

        self.buffers.insert(id, buffer);
    }

    pub fn get(&self, id: &GpuResourceId) -> Option<&wgpu::Buffer> {
        self.buffers.get(id)
    }

    pub fn remove(&mut self, id: &GpuResourceId) -> Option<wgpu::Buffer> {
        self.buffers.remove(id)
    }

    pub fn contains(&self, id: &GpuResourceId) -> bool {
        self.buffers.contains_key(id)
    }

    pub fn clear(&mut self) {
        self.buffers.clear();
    }
}

pub struct IndexBuffers {
    buffers: HashMap<GpuResourceId, wgpu::Buffer>,
}

impl IndexBuffers {
    pub fn new() -> Self {
        Self {
            buffers: HashMap::new(),
        }
    }

    pub fn create(&mut self, device: &wgpu::Device, id: GpuResourceId, info: BufferInfo) {
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: info.size,
            usage: info.usage | wgpu::BufferUsages::INDEX,
            mapped_at_creation: info.mapped_at_creation,
        });

        self.buffers.insert(id, buffer);
    }

    pub fn get(&self, id: &GpuResourceId) -> Option<&wgpu::Buffer> {
        self.buffers.get(id)
    }

    pub fn remove(&mut self, id: &GpuResourceId) -> Option<wgpu::Buffer> {
        self.buffers.remove(id)
    }

    pub fn contains(&self, id: &GpuResourceId) -> bool {
        self.buffers.contains_key(id)
    }

    pub fn clear(&mut self) {
        self.buffers.clear();
    }
}

pub struct UniformBuffers {
    buffers: HashMap<GpuResourceId, wgpu::Buffer>,
}

impl UniformBuffers {
    pub fn new() -> Self {
        Self {
            buffers: HashMap::new(),
        }
    }

    pub fn create(&mut self, device: &wgpu::Device, id: GpuResourceId, info: BufferInfo) {
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: info.size,
            usage: info.usage | wgpu::BufferUsages::UNIFORM,
            mapped_at_creation: info.mapped_at_creation,
        });

        self.buffers.insert(id, buffer);
    }

    pub fn get(&self, id: &GpuResourceId) -> Option<&wgpu::Buffer> {
        self.buffers.get(id)
    }

    pub fn remove(&mut self, id: &GpuResourceId) -> Option<wgpu::Buffer> {
        self.buffers.remove(id)
    }

    pub fn contains(&self, id: &GpuResourceId) -> bool {
        self.buffers.contains_key(id)
    }

    pub fn clear(&mut self) {
        self.buffers.clear();
    }
}

pub struct StorageBuffers {
    buffers: HashMap<GpuResourceId, wgpu::Buffer>,
}

impl StorageBuffers {
    pub fn new() -> Self {
        Self {
            buffers: HashMap::new(),
        }
    }

    pub fn create(&mut self, device: &wgpu::Device, id: GpuResourceId, info: BufferInfo) {
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: info.size,
            usage: info.usage | wgpu::BufferUsages::STORAGE,
            mapped_at_creation: info.mapped_at_creation,
        });

        self.buffers.insert(id, buffer);
    }

    pub fn get(&self, id: &GpuResourceId) -> Option<&wgpu::Buffer> {
        self.buffers.get(id)
    }

    pub fn remove(&mut self, id: &GpuResourceId) -> Option<wgpu::Buffer> {
        self.buffers.remove(id)
    }

    pub fn contains(&self, id: &GpuResourceId) -> bool {
        self.buffers.contains_key(id)
    }

    pub fn clear(&mut self) {
        self.buffers.clear();
    }
}
