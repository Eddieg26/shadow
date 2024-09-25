use super::resources::{RenderGraphBuffer, RenderGraphResources, RenderGraphTexture, RenderTarget};
use crate::{
    camera::RenderCamera,
    core::device::{RenderDevice, RenderQueue},
    resources::ResourceId,
};
use ecs::{
    core::{LocalResource, Resource},
    world::World,
};
use std::sync::{Arc, Mutex};

pub enum RenderNodeAction {
    Submit(wgpu::CommandBuffer),
    Flush,
}

pub struct RenderContext<'a> {
    world: &'a World,
    camera: &'a RenderCamera,
    camera_index: usize,
    camera_count: usize,
    target: &'a RenderTarget,
    device: &'a RenderDevice,
    queue: &'a RenderQueue,
    resources: &'a RenderGraphResources,
    actions: Arc<Mutex<Vec<RenderNodeAction>>>,
}

impl<'a> RenderContext<'a> {
    pub fn new(
        world: &'a World,
        camera: &'a RenderCamera,
        camera_index: usize,
        camera_count: usize,
        target: &'a RenderTarget,
        device: &'a RenderDevice,
        queue: &'a RenderQueue,
        resources: &'a RenderGraphResources,
    ) -> Self {
        Self {
            world,
            camera,
            camera_index,
            camera_count,
            target,
            device,
            queue,
            resources,
            actions: Arc::default(),
        }
    }

    pub fn camera(&self) -> &RenderCamera {
        self.camera
    }

    pub fn camera_index(&self) -> usize {
        self.camera_index
    }

    pub fn camera_count(&self) -> usize {
        self.camera_count
    }

    pub fn device(&self) -> &RenderDevice {
        self.device
    }

    pub fn queue(&self) -> &RenderQueue {
        self.queue
    }

    pub fn render_target(&self) -> &RenderTarget {
        self.target
    }

    pub fn texture(&self, id: ResourceId) -> Option<&RenderGraphTexture> {
        self.resources.texture(id)
    }

    pub fn buffer(&self, id: ResourceId) -> Option<&RenderGraphBuffer> {
        self.resources.buffer(id)
    }

    pub fn resource<R: Resource>(&self) -> &R {
        self.world.resource::<R>()
    }

    pub fn resource_mut<R: Resource>(&self) -> &mut R {
        self.world.resource_mut::<R>()
    }

    pub fn local_resource<R: LocalResource>(&self) -> &R {
        self.world.local_resource::<R>()
    }

    pub fn local_resource_mut<R: LocalResource>(&self) -> &mut R {
        self.world.local_resource_mut::<R>()
    }

    pub fn try_resource<R: Resource>(&self) -> Option<&R> {
        self.world.try_resource::<R>()
    }

    pub fn try_resource_mut<R: Resource>(&self) -> Option<&mut R> {
        self.world.try_resource_mut::<R>()
    }

    pub fn try_local_resource<R: LocalResource>(&self) -> Option<&R> {
        self.world.try_local_resource::<R>()
    }

    pub fn try_local_resource_mut<R: LocalResource>(&self) -> Option<&mut R> {
        self.world.try_local_resource_mut::<R>()
    }

    pub fn encoder(&self) -> wgpu::CommandEncoder {
        self.device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default())
    }

    pub fn submit(&self, encoder: wgpu::CommandEncoder) {
        let buffer = encoder.finish();
        self.actions
            .lock()
            .unwrap()
            .push(RenderNodeAction::Submit(buffer));
    }

    pub(crate) fn finish(self) -> Vec<RenderNodeAction> {
        match self.actions.try_lock() {
            Ok(mut actions) => std::mem::take(&mut *actions),
            Err(_) => Vec::new(),
        }
    }
}

impl<'a> Clone for RenderContext<'a> {
    fn clone(&self) -> Self {
        Self {
            world: self.world,
            camera: self.camera,
            camera_index: self.camera_index,
            camera_count: self.camera_count,
            target: self.target,
            device: self.device,
            queue: self.queue,
            resources: self.resources,
            actions: Arc::default(),
        }
    }
}
