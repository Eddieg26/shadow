use crate::{
    camera::ClearFlag,
    core::Color,
    renderer::{
        graph::{context::RenderContext, node::NodeInfo},
        surface::RenderSurface,
    },
    resources::ResourceId,
};
use std::{
    collections::HashMap,
    hash::Hash,
    ops::{Deref, Range},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Attachment {
    Surface,
    Texture(ResourceId),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum StoreOp {
    Store,
    Clear,
}

impl Into<wgpu::StoreOp> for StoreOp {
    fn into(self) -> wgpu::StoreOp {
        match self {
            StoreOp::Store => wgpu::StoreOp::Store,
            StoreOp::Clear => wgpu::StoreOp::Discard,
        }
    }
}

pub enum LoadOp<T> {
    Clear(T),
    Load,
}

impl<T> Into<wgpu::LoadOp<T>> for LoadOp<T> {
    fn into(self) -> wgpu::LoadOp<T> {
        match self {
            LoadOp::Clear(value) => wgpu::LoadOp::Clear(value),
            LoadOp::Load => wgpu::LoadOp::Load,
        }
    }
}

pub struct Operations<T> {
    pub load: LoadOp<T>,
    pub store: StoreOp,
}

impl<T> Into<wgpu::Operations<T>> for Operations<T> {
    fn into(self) -> wgpu::Operations<T> {
        wgpu::Operations {
            load: self.load.into(),
            store: self.store.into(),
        }
    }
}

pub struct ColorAttachment {
    pub attachment: Attachment,
    pub resolve_target: Option<Attachment>,
    pub store_op: StoreOp,
    pub clear: Option<Color>,
}

pub struct DepthAttachment {
    pub attachment: Attachment,
    pub depth_store_op: Operations<f32>,
    pub stencil_store_op: Option<Operations<u32>>,
}

pub struct RenderPass {
    colors: Vec<ColorAttachment>,
    depth: Option<DepthAttachment>,
}

impl RenderPass {
    pub fn new() -> Self {
        Self {
            colors: Vec::new(),
            depth: None,
        }
    }

    pub fn with_color(
        mut self,
        attachment: Attachment,
        resolve_target: Option<Attachment>,
        store_op: StoreOp,
        clear: Option<Color>,
    ) -> Self {
        self.colors.push(ColorAttachment {
            attachment,
            resolve_target,
            store_op,
            clear,
        });

        self
    }

    pub fn with_depth(
        mut self,
        attachment: Attachment,
        depth_store_op: Operations<f32>,
        stencil_store_op: Option<Operations<u32>>,
    ) -> Self {
        self.depth = Some(DepthAttachment {
            attachment,
            depth_store_op,
            stencil_store_op,
        });

        self
    }

    pub fn info(&self) -> NodeInfo {
        let mut info = NodeInfo::new();
        for color in &self.colors {
            match color.attachment {
                Attachment::Surface => info.write(RenderSurface::id_static()),
                Attachment::Texture(id) => info.write(id),
            }
        }

        info
    }

    pub fn begin<'a>(
        &self,
        ctx: &RenderContext,
        encoder: &'a mut wgpu::CommandEncoder,
    ) -> Option<RenderCommands<'a>> {
        let camera = ctx.camera();
        let mut color_attachments = vec![];
        for color in self.colors.iter() {
            let view = match color.attachment {
                Attachment::Surface => ctx.render_target().color()?,
                Attachment::Texture(id) => ctx.texture(id)?,
            };

            let resolve_target = match color.resolve_target {
                Some(attachment) => match attachment {
                    Attachment::Surface => Some(ctx.render_target().color()?),
                    Attachment::Texture(id) => Some(ctx.texture(id)?),
                },
                None => None,
            };

            let load = match camera.clear {
                Some(ClearFlag::Color(color)) => wgpu::LoadOp::Clear(color.into()),
                Some(ClearFlag::Skybox) => wgpu::LoadOp::Load,
                None => match color.clear {
                    Some(color) => wgpu::LoadOp::Clear(color.into()),
                    None => wgpu::LoadOp::Load,
                },
            };

            let attachement = wgpu::RenderPassColorAttachment {
                view,
                resolve_target: resolve_target.map(|t| t.deref()),
                ops: wgpu::Operations {
                    load,
                    store: color.store_op.into(),
                },
            };

            color_attachments.push(Some(attachement));
        }

        let depth_stencil_attachment = match &self.depth {
            Some(attachment) => Some(wgpu::RenderPassDepthStencilAttachment {
                view: match attachment.attachment {
                    Attachment::Surface => ctx.render_target().depth()?,
                    Attachment::Texture(id) => ctx.texture(id)?,
                },
                depth_ops: Some(wgpu::Operations {
                    load: match attachment.depth_store_op.load {
                        LoadOp::Clear(value) => wgpu::LoadOp::Clear(value),
                        LoadOp::Load => wgpu::LoadOp::Load,
                    },
                    store: attachment.depth_store_op.store.into(),
                }),
                stencil_ops: attachment
                    .stencil_store_op
                    .as_ref()
                    .map(|op| wgpu::Operations {
                        load: match op.load {
                            LoadOp::Clear(value) => wgpu::LoadOp::Clear(value),
                            LoadOp::Load => wgpu::LoadOp::Load,
                        },
                        store: op.store.into(),
                    }),
            }),
            None => None,
        };

        let pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            color_attachments: &color_attachments,
            depth_stencil_attachment,
            ..Default::default()
        });

        Some(RenderCommands::new(pass))
    }
}

pub struct RenderCommands<'a> {
    pass: wgpu::RenderPass<'a>,
    binding: Option<u32>,
    render_pipeline: Option<wgpu::Id<wgpu::RenderPipeline>>,
    vertex_buffers: HashMap<u32, wgpu::Id<wgpu::Buffer>>,
    index_buffer: Option<wgpu::Id<wgpu::Buffer>>,
}

impl<'a> RenderCommands<'a> {
    fn new(pass: wgpu::RenderPass<'a>) -> Self {
        Self {
            pass,
            binding: None,
            render_pipeline: None,
            vertex_buffers: HashMap::new(),
            index_buffer: None,
        }
    }

    pub fn set_bind_group(
        &mut self,
        index: u32,
        binding: &wgpu::BindGroup,
        offsets: &[wgpu::DynamicOffset],
    ) {
        let hash = Some(Self::hash((index, binding.global_id(), offsets)));
        if hash != self.binding {
            self.pass.set_bind_group(index, binding, offsets);
            self.binding = hash;
        }
    }

    pub fn set_pipeline(&mut self, pipeline: &wgpu::RenderPipeline) {
        let id = Some(pipeline.global_id());
        if id != self.render_pipeline {
            self.pass.set_pipeline(pipeline);
            self.render_pipeline = id;
        }
    }

    pub fn set_index_buffer(
        &mut self,
        id: wgpu::Id<wgpu::Buffer>,
        buffer_slice: wgpu::BufferSlice<'a>,
        format: wgpu::IndexFormat,
    ) {
        let id = Some(id);
        if id != self.index_buffer {
            self.pass.set_index_buffer(buffer_slice, format);
            self.index_buffer = id;
        }
    }

    pub fn set_vertex_buffer(
        &mut self,
        id: wgpu::Id<wgpu::Buffer>,
        location: u32,
        buffer_slice: wgpu::BufferSlice<'a>,
    ) {
        if self
            .vertex_buffers
            .get(&location)
            .map(|b| b != &id)
            .unwrap_or(true)
        {}
        {
            self.pass.set_vertex_buffer(location, buffer_slice);
            self.vertex_buffers.insert(location, id);
        }
    }

    pub fn draw(&mut self, vertices: Range<u32>, instances: Range<u32>) {
        self.pass.draw(vertices, instances);
    }

    pub fn draw_indirect(
        &mut self,
        indirect_buffer: &'a wgpu::Buffer,
        indirect_offset: wgpu::BufferAddress,
    ) {
        self.pass.draw_indirect(indirect_buffer, indirect_offset);
    }

    pub fn draw_indexed(&mut self, indices: Range<u32>, base_vertex: i32, instances: Range<u32>) {
        self.pass.draw_indexed(indices, base_vertex, instances);
    }

    pub fn draw_indexed_indirect(
        &mut self,
        indirect_buffer: &'a wgpu::Buffer,
        indirect_offset: wgpu::BufferAddress,
    ) {
        self.pass
            .draw_indexed_indirect(indirect_buffer, indirect_offset);
    }

    fn hash(value: impl Hash) -> u32 {
        let mut hasher = crc32fast::Hasher::new();
        value.hash(&mut hasher);
        hasher.finalize()
    }
}
