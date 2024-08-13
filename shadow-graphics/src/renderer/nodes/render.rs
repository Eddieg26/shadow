use crate::{
    renderer::graph::{Render, RenderGraphContext, RenderGraphNode, RenderGraphNodeBuilder},
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
    subpasses: Vec<SubpassBuilder>,
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

    pub fn with_subpass(mut self, subpass: SubpassBuilder) -> Self {
        self.subpasses.push(subpass);

        self
    }

    pub fn add_group<G: RenderGroupBuilder>(&mut self, subpass: usize, group: G) -> &mut Self {
        self.subpasses[subpass].add_group(group);
        self
    }
}

impl RenderGraphNodeBuilder for RenderPassBuilder {
    fn build(&self, world: &shadow_ecs::world::World) -> Box<dyn RenderGraphNode> {
        Box::new(RenderPass::new(
            self.colors.clone(),
            self.depth.clone(),
            self.subpasses
                .iter()
                .map(|subpass| subpass.build(world))
                .collect(),
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

    fn begin_render_pass<'a>(
        &self,
        render: &Render,
        ctx: &RenderGraphContext,
        encoder: &'a mut wgpu::CommandEncoder,
    ) -> wgpu::RenderPass<'a> {
        encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Render Pass"),
            color_attachments: &self
                .colors
                .iter()
                .map(|color| {
                    Some(wgpu::RenderPassColorAttachment {
                        view: match color.attachment {
                            Attachment::Surface => ctx.target(),
                            Attachment::Texture(id) => {
                                ctx.resources().texture(id).expect("Texture not found")
                            }
                        },
                        ops: wgpu::Operations {
                            store: color.store_op,
                            load: match render.clear_color {
                                Some(color) => wgpu::LoadOp::Clear(color),
                                None => wgpu::LoadOp::Load,
                            },
                        },
                        resolve_target: match color.resolve_target {
                            Some(ref attachment) => Some(match attachment {
                                Attachment::Surface => ctx.target(),
                                Attachment::Texture(id) => {
                                    ctx.resources().texture(*id).expect("Texture not found")
                                }
                            }),
                            None => None,
                        },
                    })
                })
                .collect::<Vec<_>>(),
            depth_stencil_attachment: match self.depth {
                Some(ref depth) => Some(wgpu::RenderPassDepthStencilAttachment {
                    view: match depth.attachment {
                        Attachment::Surface => ctx.target(),
                        Attachment::Texture(id) => {
                            ctx.resources().texture(id).expect("Texture not found")
                        }
                    },
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(depth.clear_depth.unwrap_or(1.0)),
                        store: depth.depth_store_op,
                    }),
                    stencil_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(depth.clear_stencil.unwrap_or(0)),
                        store: depth.stencil_store_op,
                    }),
                }),
                None => None,
            },
            ..Default::default()
        })
    }
}

impl RenderGraphNode for RenderPass {
    fn execute(&self, ctx: &RenderGraphContext) {
        for pass in &self.subpasses {
            pass.render();
        }
    }
}

pub trait RenderGroup: Send + Sync + 'static {
    fn render(&self);
}

pub trait RenderGroupBuilder: Send + Sync + 'static {
    fn build(&self, world: &shadow_ecs::world::World) -> Box<dyn RenderGroup>;
}

pub struct SubpassBuilder {
    groups: Vec<Box<dyn RenderGroupBuilder>>,
}

impl SubpassBuilder {
    pub fn new() -> Self {
        Self { groups: Vec::new() }
    }

    pub fn add_group<G: RenderGroupBuilder>(&mut self, group: G) -> &mut Self {
        self.groups.push(Box::new(group));

        self
    }

    pub fn with_group(mut self, group: impl RenderGroupBuilder) -> Self {
        self.groups.push(Box::new(group));

        self
    }

    pub fn build(&self, world: &shadow_ecs::world::World) -> Subpass {
        let groups = self.groups.iter().map(|group| group.build(world)).collect();

        Subpass { groups }
    }
}

pub struct Subpass {
    groups: Vec<Box<dyn RenderGroup>>,
}

impl Subpass {
    pub fn new() -> Self {
        Self { groups: Vec::new() }
    }

    pub fn render(&self) {
        for group in &self.groups {
            group.render();
        }
    }
}
