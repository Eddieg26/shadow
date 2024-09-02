use crate::{
    components::{ClearFlag, RenderFrame},
    core::Color,
    renderer::{
        draw::{Draw, DrawCalls},
        graph::{context::RenderContext, RenderGraphNode},
    },
    resources::ResourceId,
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
    pub stencil_store_op: Operations<u32>,
}

pub struct RenderPass {
    colors: Vec<ColorAttachment>,
    depth: Option<DepthAttachment>,
    subpasses: Vec<Subpass>,
}

impl RenderPass {
    pub fn new() -> Self {
        Self {
            subpasses: Vec::new(),
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
        stencil_store_op: Operations<u32>,
    ) -> Self {
        self.depth = Some(DepthAttachment {
            attachment,
            depth_store_op,
            stencil_store_op,
        });

        self
    }

    pub fn with_subpass(mut self, subpass: Subpass) -> Self {
        self.subpasses.push(subpass);
        self
    }

    pub fn add_subpass(&mut self, subpass: Subpass) {
        self.subpasses.push(subpass);
    }

    pub fn with_render_group<R: RenderGroup>(mut self, subpass: usize, group: R) -> Self {
        self.subpasses[subpass].add_group(group);
        self
    }

    pub fn add_render_group<R: RenderGroup>(&mut self, subpass: usize, group: R) -> &mut Self {
        self.subpasses[subpass].add_group(group);
        self
    }

    fn begin<'a>(
        &self,
        frame: &RenderFrame,
        ctx: &RenderContext,
        encoder: &'a mut wgpu::CommandEncoder,
    ) -> Option<RenderCommands<'a>> {
        let target = match frame.camera.target {
            Some(id) => ctx.render_target(id)?,
            None => ctx.render_target(ctx.surface_id())?,
        };

        let mut color_attachments = vec![];
        for color in self.colors.iter() {
            let view = match color.attachment {
                Attachment::Surface => target.color()?,
                Attachment::Texture(id) => ctx.texture(id)?,
            };

            let resolve_target = match color.resolve_target {
                Some(attachment) => match attachment {
                    Attachment::Surface => Some(target.color()?),
                    Attachment::Texture(id) => Some(ctx.texture(id)?),
                },
                None => None,
            };

            let load = match frame.camera.clear {
                Some(ClearFlag::Color(color)) => wgpu::LoadOp::Clear(color.into()),
                Some(ClearFlag::Skybox) => wgpu::LoadOp::Load,
                None => match color.clear {
                    Some(color) => wgpu::LoadOp::Clear(color.into()),
                    None => wgpu::LoadOp::Load,
                },
            };

            let attachement = wgpu::RenderPassColorAttachment {
                view,
                resolve_target,
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
                    Attachment::Surface => target.depth()?,
                    Attachment::Texture(id) => ctx.texture(id)?,
                },
                depth_ops: Some(wgpu::Operations {
                    load: match attachment.depth_store_op.load {
                        LoadOp::Clear(value) => wgpu::LoadOp::Clear(value),
                        LoadOp::Load => wgpu::LoadOp::Load,
                    },
                    store: attachment.depth_store_op.store.into(),
                }),
                stencil_ops: Some(wgpu::Operations {
                    load: match attachment.stencil_store_op.load {
                        LoadOp::Clear(value) => wgpu::LoadOp::Clear(value),
                        LoadOp::Load => wgpu::LoadOp::Load,
                    },
                    store: attachment.stencil_store_op.store.into(),
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

impl RenderGraphNode for RenderPass {
    fn execute(&self, ctx: &RenderContext) {
        let mut encoder = ctx.encoder();
        if let Some(mut commands) = self.begin(ctx.frame(), ctx, &mut encoder) {
            for pass in &self.subpasses {
                pass.run(ctx, &mut commands);
            }
        }

        ctx.submit(encoder.finish());
    }
}

pub struct RenderCommands<'a> {
    pass: wgpu::RenderPass<'a>,
}

impl<'a> RenderCommands<'a> {
    fn new(pass: wgpu::RenderPass<'a>) -> Self {
        Self { pass }
    }
}

pub trait RenderGroup: 'static {
    type Draw: Draw;
    fn render(
        &self,
        frame: &RenderFrame,
        draws: &DrawCalls<Self::Draw>,
        commands: &mut RenderCommands,
    );
}

pub struct ErasedRenderGroup {
    render: Box<dyn Fn(&RenderContext, &mut RenderCommands)>,
}

impl ErasedRenderGroup {
    pub fn new<R: RenderGroup>(group: R) -> Self {
        Self {
            render: Box::new(move |ctx, commands| {
                if let Some(draws) = ctx.try_resource::<DrawCalls<R::Draw>>() {
                    group.render(ctx.frame(), draws, commands);
                }
            }),
        }
    }

    pub fn render(&self, ctx: &RenderContext, commands: &mut RenderCommands) {
        (self.render)(ctx, commands);
    }
}

pub struct Subpass {
    groups: Vec<ErasedRenderGroup>,
}

impl Subpass {
    pub fn new() -> Self {
        Self { groups: Vec::new() }
    }

    pub fn with_group<R: RenderGroup>(mut self, group: R) -> Self {
        self.groups.push(ErasedRenderGroup::new(group));
        self
    }

    pub fn add_group<R: RenderGroup>(&mut self, group: R) {
        self.groups.push(ErasedRenderGroup::new(group));
    }

    pub fn run(&self, ctx: &RenderContext, commands: &mut RenderCommands) {
        for group in &self.groups {
            group.render(ctx, commands);
        }
    }
}
