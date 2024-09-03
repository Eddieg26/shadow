use super::resources::{RenderGraphResources, RenderTarget};
use crate::{
    camera::RenderFrame,
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
    surface_id: ResourceId,
    frame: &'a RenderFrame,
    frame_index: usize,
    total_frames: usize,
    device: &'a RenderDevice,
    queue: &'a RenderQueue,
    resources: &'a RenderGraphResources,
    world: &'a World,
    actions: Arc<Mutex<Vec<RenderNodeAction>>>,
}

impl<'a> RenderContext<'a> {
    pub fn new(
        surface_id: ResourceId,
        frame: &'a RenderFrame,
        frame_index: usize,
        total_frames: usize,
        device: &'a RenderDevice,
        queue: &'a RenderQueue,
        resources: &'a RenderGraphResources,
        world: &'a World,
    ) -> Self {
        Self {
            surface_id,
            frame,
            frame_index,
            total_frames,
            device,
            queue,
            resources,
            world,
            actions: Arc::default(),
        }
    }

    pub fn surface_id(&self) -> ResourceId {
        self.surface_id
    }

    pub fn frame(&self) -> &RenderFrame {
        self.frame
    }

    pub fn frame_index(&self) -> usize {
        self.frame_index
    }

    pub fn total_frames(&self) -> usize {
        self.total_frames
    }

    pub fn device(&self) -> &RenderDevice {
        self.device
    }

    pub fn queue(&self) -> &RenderQueue {
        self.queue
    }

    pub fn render_target(&self, id: impl Into<ResourceId>) -> Option<&RenderTarget> {
        self.resources.target(id.into())
    }

    pub fn texture(&self, texture: ResourceId) -> Option<&wgpu::TextureView> {
        self.resources
            .target(self.surface_id)
            .and_then(|t| t.texture(texture))
            .or_else(|| self.resources.texture(texture))
    }

    pub fn buffer(&self, id: ResourceId) -> Option<&wgpu::Buffer> {
        self.resources.buffer(id)
    }

    pub fn resource<R: Resource>(&self) -> &R {
        self.world.resource::<R>()
    }

    pub fn try_resource<R: Resource>(&self) -> Option<&R> {
        self.world.try_resource::<R>()
    }

    pub fn local_resource<R: LocalResource>(&self) -> &R {
        self.world.local_resource::<R>()
    }

    pub fn try_local_resource<R: LocalResource>(&self) -> Option<&R> {
        self.world.try_local_resource::<R>()
    }

    pub fn encoder(&self) -> wgpu::CommandEncoder {
        self.device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default())
    }

    pub fn submit(&self, buffer: wgpu::CommandBuffer) {
        self.actions
            .lock()
            .unwrap()
            .push(RenderNodeAction::Submit(buffer));
    }

    pub(crate) fn append_actions(&self, actions: impl IntoIterator<Item = RenderNodeAction>) {
        self.actions.lock().unwrap().extend(actions);
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
            surface_id: self.surface_id,
            frame: self.frame,
            frame_index: self.frame_index,
            total_frames: self.total_frames,
            device: self.device,
            queue: self.queue,
            resources: self.resources,
            world: self.world,
            actions: Arc::default(),
        }
    }
}
