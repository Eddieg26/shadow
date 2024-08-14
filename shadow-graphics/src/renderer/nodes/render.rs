use std::any::TypeId;

use shadow_ecs::{core::Entity, world::World};

use crate::{
    renderer::graph::{
        Render, RenderGraphContext, RenderGraphNode, RenderGraphNodeBuilder, RenderTarget,
    },
    resources::GpuResourceId,
};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Attachment {
    Surface,
    Texture(GpuResourceId),
}

#[derive(Clone)]
pub struct ColorAttachment {
    pub attachment: Attachment,
    pub resolve_target: Option<Attachment>,
    pub store_op: wgpu::StoreOp,
}

#[derive(Clone)]
pub struct DepthAttachment {
    pub attachment: Attachment,
    pub depth_store_op: wgpu::StoreOp,
    pub stencil_store_op: wgpu::StoreOp,
    pub clear_depth: Option<f32>,
    pub clear_stencil: Option<u32>,
}

pub struct RenderPassBuilder {
    colors: Vec<ColorAttachment>,
    depth: Option<DepthAttachment>,
    subpasses: Vec<Subpass>,
}

impl RenderPassBuilder {
    pub fn new() -> Self {
        Self {
            colors: Vec::new(),
            depth: None,
            subpasses: Vec::new(),
        }
    }

    pub fn with_color(
        mut self,
        attachment: Attachment,
        resolve_target: Option<Attachment>,
        store_op: wgpu::StoreOp,
    ) -> Self {
        self.colors.push(ColorAttachment {
            attachment,
            resolve_target,
            store_op,
        });

        self
    }

    pub fn with_depth(
        mut self,
        attachment: Attachment,
        depth_store_op: wgpu::StoreOp,
        stencil_store_op: wgpu::StoreOp,
        clear_depth: Option<f32>,
        clear_stencil: Option<u32>,
    ) -> Self {
        self.depth = Some(DepthAttachment {
            attachment,
            depth_store_op,
            stencil_store_op,
            clear_depth,
            clear_stencil,
        });

        self
    }

    pub fn with_subpass(mut self, subpass: Subpass) -> Self {
        self.subpasses.push(subpass);

        self
    }

    pub fn add_draw_group<G: DrawGroup>(&mut self, subpass: usize, group: G) -> &mut Self {
        self.subpasses[subpass].add_draw_group(group);
        self
    }
}

impl RenderGraphNodeBuilder for RenderPassBuilder {
    fn build(&self, world: &shadow_ecs::world::World) -> Box<dyn RenderGraphNode> {
        Box::new(RenderPass::new(
            self.colors.clone(),
            self.depth.clone(),
            self.subpasses.into_iter().map(|subpass| subpass).collect(),
        ))
    }
}

pub struct RenderPass {
    colors: Vec<ColorAttachment>,
    depth: Option<DepthAttachment>,
    subpasses: Vec<Subpass>,
}

impl RenderPass {
    pub fn new(
        colors: Vec<ColorAttachment>,
        depth: Option<DepthAttachment>,
        subpasses: Vec<Subpass>,
    ) -> Self {
        Self {
            colors,
            depth,
            subpasses,
        }
    }

    fn get_color_attachment<'a>(
        color: &ColorAttachment,
        render: &Render,
        target: &'a RenderTarget,
    ) -> Option<wgpu::RenderPassColorAttachment<'a>> {
        let view = match color.attachment {
            Attachment::Surface => target.color_texture()?,
            Attachment::Texture(id) => target.texture(id)?,
        };

        let ops = wgpu::Operations {
            store: color.store_op,
            load: match render.clear_color {
                Some(color) => wgpu::LoadOp::Clear(color),
                None => wgpu::LoadOp::Load,
            },
        };

        let resolve_target = match color.resolve_target {
            Some(ref attachment) => Some(match attachment {
                Attachment::Surface => target.depth_texture()?,
                Attachment::Texture(id) => target.texture(*id)?,
            }),
            None => None,
        };

        Some(wgpu::RenderPassColorAttachment {
            view,
            resolve_target,
            ops,
        })
    }

    fn get_depth_stencil_attachment<'a>(
        depth: &'a DepthAttachment,
        target: &'a RenderTarget,
    ) -> Option<wgpu::RenderPassDepthStencilAttachment<'a>> {
        Some(wgpu::RenderPassDepthStencilAttachment {
            view: match depth.attachment {
                Attachment::Surface => target.depth_texture()?,
                Attachment::Texture(id) => target.texture(id)?,
            },
            depth_ops: Some(wgpu::Operations {
                load: wgpu::LoadOp::Clear(depth.clear_depth.unwrap_or(1.0)),
                store: depth.depth_store_op,
            }),
            stencil_ops: Some(wgpu::Operations {
                load: wgpu::LoadOp::Clear(depth.clear_stencil.unwrap_or(0)),
                store: depth.stencil_store_op,
            }),
        })
    }

    fn begin_render_pass<'a>(
        &self,
        ctx: &RenderGraphContext,
        render: &Render,
        encoder: &'a mut wgpu::CommandEncoder,
    ) -> Option<wgpu::RenderPass<'a>> {
        let target = match render.target {
            Some(id) => ctx.resources().render_target(id)?,
            None => ctx.target()?,
        };

        let pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            color_attachments: &self
                .colors
                .iter()
                .map(|color| Some(Self::get_color_attachment(color, render, target)?))
                .collect::<Vec<_>>(),
            depth_stencil_attachment: match &self.depth {
                Some(depth) => Some(Self::get_depth_stencil_attachment(depth, target)?),
                None => None,
            },
            ..Default::default()
        });

        Some(pass)
    }
}

impl RenderGraphNode for RenderPass {
    fn execute(&self, ctx: &RenderGraphContext) {
        let mut encoder = ctx.encoder();
        // if let Some(pass) = self.begin_render_pass(ctx, render, &mut encoder) {
        //     for sub in &self.subpasses {
        //         sub.render();
        //     }
        // }
    }
}

pub trait Renderer: 'static {
    fn entity(&self) -> Entity;
    fn clear_color(&self) -> Option<wgpu::Color>;
    fn target(&self) -> Option<GpuResourceId>;
}

pub trait Draw: 'static {}

pub trait DrawGroup: Send + Sync + 'static {
    type Renderer: Renderer;
    type Draw: Draw;

    fn draw(&self);
}

pub struct DrawGroupExecutor {
    execute: Box<dyn Fn() + Send + Sync + 'static>,
}

impl DrawGroupExecutor {
    pub fn new<D: DrawGroup>(group: D) -> Self {
        Self {
            execute: Box::new(move || {
                group.draw();
            }),
        }
    }

    pub fn draw(&self) {
        (self.execute)();
    }
}

pub struct Subpass {
    groups: Vec<DrawGroupExecutor>,
}

impl Subpass {
    pub fn new() -> Self {
        Self { groups: Vec::new() }
    }

    pub fn add_draw_group<D: DrawGroup>(&mut self, group: D) -> &mut Self {
        self.groups.push(DrawGroupExecutor::new(group));
        self
    }

    pub fn with_draw_group<D: DrawGroup>(mut self, group: D) -> Self {
        self.add_draw_group(group);
        self
    }

    pub fn render(&self) {
        for group in &self.groups {
            group.draw();
        }
    }
}
