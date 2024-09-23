use super::{
    layout::{GlobalBinding, ObjectBinding, ObjectModel},
    pipeline::{DepthWrite, MaterialPipeline, MaterialPipelineKey, MaterialPipelines},
    shader::{
        constants::{CAMERA_GROUP, MATERIAL_GROUP, OBJECT_GROUP},
        vertex::MeshShader,
    },
    BlendMode, MaterialInstance, ShaderModel,
};
use asset::AssetId;
use ecs::core::{LocalResource, Resource};
use graphics::{
    core::{RenderDevice, RenderQueue},
    renderer::{
        draw::{Draw, DrawCalls},
        graph::{
            context::RenderContext,
            node::{NodeInfo, RenderGraphNode},
            pass::{Attachment, LoadOp, Operations, RenderCommands, RenderPass, StoreOp},
        },
    },
    resources::{mesh::MeshBuffers, RenderAssets},
};
use spatial::{bounds::BoundingBox, partition::Partition};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ForwardSubPass {
    Opaque = 0,
    Transparent = 1,
}

pub const FORWARD_NODE: &str = "forward";

pub struct ForwardPass {
    name: String,
    pass: RenderPass,
    sub_passes: [SubPass; 2],
}

impl ForwardPass {
    pub fn new() -> Self {
        let pass = RenderPass::new()
            .with_color(Attachment::Surface, None, StoreOp::Store, None)
            .with_depth(
                Attachment::Surface,
                Operations {
                    load: LoadOp::Load,
                    store: StoreOp::Store,
                },
                None,
            );

        Self {
            name: FORWARD_NODE.to_string(),
            pass,
            sub_passes: [SubPass::new(), SubPass::new()],
        }
    }

    pub fn add_render_group<G: RenderGroup>(&mut self, pass: ForwardSubPass, group: G) {
        self.sub_passes[pass as usize].add_group(group);
    }
}

impl RenderGraphNode for ForwardPass {
    fn name(&self) -> &str {
        &self.name
    }

    fn info(&self) -> NodeInfo {
        self.pass.info()
    }

    fn execute(&mut self, ctx: &RenderContext) {
        let mut encoder = ctx.encoder();
        if let Some(mut commands) = self.pass.begin(ctx, &mut encoder) {
            let ctx = RenderGroupContext::new(ctx);
            for subpass in &self.sub_passes {
                for group in &subpass.groups {
                    group.render(&ctx, &mut commands)
                }
            }
        }

        ctx.submit(encoder.finish());
    }
}

pub struct SubPass {
    groups: Vec<Box<dyn RenderGroup>>,
}

impl SubPass {
    #[inline]
    pub fn new() -> Self {
        Self { groups: Vec::new() }
    }

    pub fn add_group<G: RenderGroup>(&mut self, group: G) {
        self.groups.push(Box::new(group));
    }
}

pub struct RenderGroupContext<'a> {
    ctx: &'a RenderContext<'a>,
}

impl<'a> RenderGroupContext<'a> {
    #[inline]
    fn new(ctx: &'a RenderContext) -> Self {
        Self { ctx }
    }

    #[inline]
    pub fn frame_index(&self) -> usize {
        self.ctx.frame_index()
    }

    #[inline]
    pub fn frame_count(&self) -> usize {
        self.ctx.frame_count()
    }

    #[inline]
    pub fn device(&self) -> &RenderDevice {
        self.ctx.device()
    }

    #[inline]
    pub fn queue(&self) -> &RenderQueue {
        self.ctx.queue()
    }

    #[inline]
    pub fn resource<R: Resource>(&self) -> &R {
        self.ctx.resource::<R>()
    }

    #[inline]
    pub fn resource_mut<R: Resource>(&self) -> &mut R {
        self.ctx.resource_mut::<R>()
    }

    #[inline]
    pub fn local_resource<R: LocalResource>(&self) -> &R {
        self.ctx.local_resource::<R>()
    }

    #[inline]
    pub fn local_resource_mut<R: LocalResource>(&self) -> &mut R {
        self.ctx.local_resource_mut::<R>()
    }

    #[inline]
    pub fn try_resource<R: Resource>(&self) -> Option<&R> {
        self.ctx.try_resource::<R>()
    }

    #[inline]
    pub fn try_resource_mut<R: Resource>(&self) -> Option<&mut R> {
        self.ctx.try_resource_mut::<R>()
    }

    #[inline]
    pub fn try_local_resource<R: LocalResource>(&self) -> Option<&R> {
        self.ctx.try_local_resource::<R>()
    }

    #[inline]
    pub fn try_local_resource_mut<R: LocalResource>(&self) -> Option<&mut R> {
        self.ctx.try_local_resource_mut::<R>()
    }
}

pub trait RenderGroup: Send + Sync + 'static {
    fn render<'a>(&self, ctx: &'a RenderGroupContext<'a>, commands: &mut RenderCommands<'a>);
}

pub struct DrawMesh {
    pub mesh: AssetId,
    pub material: MaterialInstance,
    pub transform: glam::Mat4,
    pub bounds: BoundingBox,
}

impl DrawMesh {
    pub fn new(
        mesh: AssetId,
        material: MaterialInstance,
        transform: glam::Mat4,
        bounds: BoundingBox,
    ) -> Self {
        Self {
            mesh,
            material,
            transform,
            bounds,
        }
    }

    pub fn model(&self) -> ShaderModel {
        self.material.model
    }

    pub fn mode(&self) -> BlendMode {
        self.material.mode
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DrawMeshQuery {
    pub blend_mode: Option<BlendMode>,
    pub model: Option<ShaderModel>,
}

impl DrawMeshQuery {
    pub fn new() -> Self {
        Self {
            blend_mode: None,
            model: None,
        }
    }

    pub fn with_blend_mode(mut self, blend_mode: BlendMode) -> Self {
        self.blend_mode = Some(blend_mode);
        self
    }

    pub fn with_model(mut self, model: ShaderModel) -> Self {
        self.model = Some(model);
        self
    }
}

pub struct DrawMeshPartition {
    items: [Vec<DrawMesh>; 4],
}

impl DrawMeshPartition {
    pub fn new() -> Self {
        Self {
            items: [Vec::new(), Vec::new(), Vec::new(), Vec::new()],
        }
    }

    pub fn item_index(mode: BlendMode, model: ShaderModel) -> usize {
        match (mode, model) {
            (BlendMode::Opaque, ShaderModel::Unlit) => 0,
            (BlendMode::Opaque, ShaderModel::Lit) => 1,
            (BlendMode::Transparent, ShaderModel::Unlit) => 2,
            (BlendMode::Transparent, ShaderModel::Lit) => 3,
        }
    }
}

impl Partition for DrawMeshPartition {
    type Item = DrawMesh;
    type Query = DrawMeshQuery;

    fn insert(&mut self, item: Self::Item) {
        let index = Self::item_index(item.mode(), item.model());
        self.items[index].push(item);
    }

    fn items(&self) -> impl Iterator<Item = &Self::Item> {
        self.items.iter().flat_map(|items| items.iter())
    }

    fn query(&self, query: &Self::Query) -> Vec<&Self::Item> {
        match (query.blend_mode, query.model) {
            (Some(blend_mode), Some(model)) => {
                let index = Self::item_index(blend_mode, model);
                self.items[index].iter().collect()
            }
            (Some(blend_mode), None) => match blend_mode {
                BlendMode::Opaque => self.items[0].iter().chain(self.items[1].iter()).collect(),
                BlendMode::Transparent => {
                    self.items[2].iter().chain(self.items[3].iter()).collect()
                }
            },
            (None, Some(model)) => match model {
                ShaderModel::Unlit => self.items[0].iter().chain(self.items[2].iter()).collect(),
                ShaderModel::Lit => self.items[1].iter().chain(self.items[3].iter()).collect(),
            },
            (None, None) => return Vec::new(),
        }
    }

    fn drain(&mut self) -> Vec<Self::Item> {
        self.items
            .iter_mut()
            .flat_map(|items| items.drain(..))
            .collect()
    }

    fn clear(&mut self) {
        for items in &mut self.items {
            items.clear();
        }
    }
}

pub struct Opaque3D;
impl MaterialPipeline for Opaque3D {
    fn depth_write() -> DepthWrite {
        DepthWrite::On
    }

    fn primitive() -> wgpu::PrimitiveState {
        wgpu::PrimitiveState::default()
    }

    fn shader() -> MeshShader {
        mesh_shader::create()
    }
}

pub struct Transparent3D;
impl MaterialPipeline for Transparent3D {
    fn depth_write() -> DepthWrite {
        DepthWrite::On
    }

    fn primitive() -> wgpu::PrimitiveState {
        wgpu::PrimitiveState::default()
    }

    fn shader() -> super::shader::vertex::MeshShader {
        mesh_shader::create()
    }
}

impl Draw for DrawMesh {
    type Partition = DrawMeshPartition;
}

pub struct RenderMeshGroup<M: MaterialPipeline> {
    key: MaterialPipelineKey,
    query: DrawMeshQuery,
    _marker: std::marker::PhantomData<M>,
}

impl<M: MaterialPipeline> RenderMeshGroup<M> {
    pub fn new(model: ShaderModel, mode: BlendMode) -> Self {
        Self {
            key: MaterialPipelineKey::of::<M>(),
            query: DrawMeshQuery::new().with_model(model).with_blend_mode(mode),
            _marker: std::marker::PhantomData,
        }
    }
}

impl<M: MaterialPipeline> RenderGroup for RenderMeshGroup<M> {
    fn render<'a>(&self, ctx: &'a RenderGroupContext<'a>, commands: &mut RenderCommands<'a>) {
        let calls = ctx.resource::<DrawCalls<DrawMesh>>();
        let filtered = calls.query(&self.query);
        let global = ctx.resource::<GlobalBinding>();
        let object = ctx.resource_mut::<ObjectBinding>();
        let mesh_buffers = ctx.resource::<RenderAssets<MeshBuffers>>();
        let material_pipelines = match ctx
            .resource::<RenderAssets<MaterialPipelines>>()
            .get(&self.key)
        {
            Some(pipelines) => pipelines,
            None => return,
        };
        for mesh in filtered {
            let pipeline = match material_pipelines.pipeline(mesh.material.ty) {
                Some(pipeline) => pipeline,
                None => continue,
            };

            let buffers = match mesh_buffers.get(&mesh.mesh) {
                Some(buffers) => buffers,
                None => continue,
            };

            for (slot, attribute) in material_pipelines.layout().iter().enumerate() {
                let index = match buffers.attribute_index(*attribute) {
                    Some(index) => index,
                    None => continue,
                };

                let vertex_buffer = buffers.get_vertex_buffer(index).unwrap();
                if let Some(buffer) = vertex_buffer.buffer() {
                    commands.set_vertex_buffer(buffer.global_id(), slot as u32, buffer.slice(..))
                }
            }

            object
                .object_mut()
                .update(ObjectModel::from(mesh.transform));
            object.object_mut().commit(ctx.device(), ctx.queue());

            commands.set_pipeline(pipeline);
            commands.set_bind_group(CAMERA_GROUP, global.binding(), &[ctx.frame_index() as u32]);
            commands.set_bind_group(OBJECT_GROUP, object.binding(), &[]);
            commands.set_bind_group(MATERIAL_GROUP, &mesh.material.binding, &[]);

            if let Some(index) = buffers.index_buffer() {
                if let Some(buffer) = index.buffer() {
                    let format = index.indices().format();
                    commands.set_index_buffer(buffer.global_id(), buffer.slice(..), format);
                    commands.draw_indexed(0..index.indices().len() as u32, 0, 0..1);
                }
            } else {
                commands.draw(0..buffers.vertex_count() as u32, 0..1);
            }
        }
    }
}

pub mod mesh_shader {
    use crate::{
        material::shader::{
            nodes::{CameraNode, ConvertNode, MultiplyNode, ObjectModelNode},
            vertex::MeshShader,
            ShaderProperty, VertexInput, VertexOutput,
        },
        shader::{nodes::ShaderValueNode, ShaderValue},
    };

    // pub fn create() -> MeshShader {
    //     let mut shader = MeshShader::new();
    //     shader.add_input(VertexInput::Position);
    //     shader.add_input(VertexInput::TexCoord0);
    //     let camera = shader.add_node(CameraNode::new());
    //     let object = shader.add_node(ObjectModelNode::new());
    //     let convert = shader.add_node(ConvertNode::new(ShaderProperty::Vec4));
    //     let mult0 = shader.add_node(MultiplyNode::new());
    //     shader.add_edge((VertexInput::Position, (convert, ConvertNode::INPUT)));
    //     shader.add_edge((
    //         (object, ObjectModelNode::WORLD),
    //         (mult0, MultiplyNode::LEFT),
    //     ));
    //     shader.add_edge(((convert, ConvertNode::OUTPUT), (mult0, MultiplyNode::RIGHT)));
    //     let mult1 = shader.add_node(MultiplyNode::new());
    //     shader.add_edge((
    //         (camera, CameraNode::PROJECTION),
    //         (mult1, MultiplyNode::LEFT),
    //     ));
    //     shader.add_edge(((camera, CameraNode::VIEW), (mult1, MultiplyNode::RIGHT)));
    //     let mult2 = shader.add_node(MultiplyNode::new());
    //     shader.add_edge(((mult1, MultiplyNode::OUTPUT), (mult2, MultiplyNode::LEFT)));
    //     shader.add_edge(((mult0, MultiplyNode::OUTPUT), (mult2, MultiplyNode::RIGHT)));
    //     shader.add_edge((
    //         (mult0, MultiplyNode::OUTPUT),
    //         VertexOutput::Position { clip: false },
    //     ));
    //     shader.add_edge((
    //         (mult2, MultiplyNode::OUTPUT),
    //         VertexOutput::Position { clip: true },
    //     ));
    //     shader.add_edge((VertexInput::TexCoord0, VertexOutput::TexCoord0));

    //     shader
    // }

    pub fn create() -> MeshShader {
        let mut shader = MeshShader::new();
        shader.add_input(VertexInput::Position);
        shader.add_input(VertexInput::TexCoord0);
        let object = shader.add_node(ObjectModelNode::new());
        let convert = shader.add_node(ConvertNode::new(ShaderProperty::Vec4));
        let mult0 = shader.add_node(MultiplyNode::new());
        shader.add_edge((VertexInput::Position, (convert, ConvertNode::INPUT)));
        shader.add_edge((
            (object, ObjectModelNode::WORLD),
            (mult0, MultiplyNode::LEFT),
        ));
        shader.add_edge(((convert, ConvertNode::OUTPUT), (mult0, MultiplyNode::RIGHT)));
        shader.add_edge((
            (mult0, MultiplyNode::OUTPUT),
            VertexOutput::Position { clip: false },
        ));
        shader.add_edge((
            (mult0, MultiplyNode::OUTPUT),
            VertexOutput::Position { clip: true },
        ));
        shader.add_edge((VertexInput::TexCoord0, VertexOutput::TexCoord0));

        shader
    }
}
