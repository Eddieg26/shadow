use asset::{database::events::AssetLoaded, embed_asset, plugin::AssetExt, AssetHandle, AssetId};
use ecs::{
    archetype::table::EntityRow,
    core::{
        internal::blob::{Blob, BlobCell},
        Component, ComponentId, Entity, Resource,
    },
    system::{
        unlifetime::{Read, StaticQuery, Write},
        ArgItem, StaticSystemArg,
    },
    world::{
        event::Spawn,
        query::{Query, With},
        World,
    },
};
use game::{
    game::Game,
    plugin::{Plugin, Plugins},
    time::Time,
    Extract, Main, MainWorld, Update,
};
use glam::{Vec2, Vec3};
use graphics::{
    camera::{Camera, CameraData, ClearFlag, RenderCamera},
    core::{Color, RenderDevice, RenderQueue},
    plugin::{GraphicsExt, GraphicsPlugin, RenderApp},
    renderer::{
        draw::{Draw, DrawCallExtractor, DrawCalls},
        graph::{
            context::RenderContext,
            node::RenderGraphNode,
            pass::{
                render::{Attachment, RenderPass, StoreOp},
                LoadOp, Operations,
            },
        },
        surface::RenderSurface,
    },
    resources::{
        binding::{BindGroup, CreateBindGroup},
        buffer::{
            BatchIndex, BatchedUniformBuffer, BufferFlags, Indices, UniformBuffer,
            UniformBufferArray,
        },
        mesh::{Mesh, MeshAttribute, MeshAttributeKind, MeshBuffers, MeshTopology},
        pipeline::{
            FragmentState, RenderPipeline, RenderPipelineDesc, VertexBufferLayout, VertexState,
        },
        shader::Shader,
        RenderAssets, RenderResourceExtractor,
    },
};
use spatial::{Axis, Transform};
use std::{sync::Arc, u32};
use window::events::WindowCreated;

const TEST_ID: AssetId = AssetId::raw(100);
const CUBE_ID: AssetId = AssetId::raw(200);
const SHADER_ID: AssetId = AssetId::raw(300);
const MATERIAL_ID: AssetId = AssetId::raw(400);

fn main() {
    Game::new().add_plugin(BasicPlugin).run();
}

#[derive(Debug, Clone, Copy)]
pub struct MeshRenderer {
    pub mesh: AssetId,
    pub material: AssetId,
}

impl MeshRenderer {
    pub fn new(mesh: AssetId, material: AssetId) -> Self {
        Self { mesh, material }
    }
}

impl Component for MeshRenderer {}

pub struct BasicPlugin;

impl Plugin for BasicPlugin {
    fn dependencies(&self) -> Plugins {
        let mut plugins = Plugins::new();
        plugins.add_plugin(GraphicsPlugin);
        plugins
    }

    fn run(&mut self, game: &mut Game) {
        embed_asset!(game, TEST_ID, "test.txt");
        embed_asset!(game, CUBE_ID, "cube.obj");
        embed_asset!(game, SHADER_ID, "shader.wgsl");

        game.register::<MeshRenderer>();
        game.events().add(
            Spawn::new()
                .with(
                    Transform::default()
                        .with_scale((0.5, 0.5, 0.5).into())
                        .with_position(Vec3::new(1.0, 0.0, 0.0)),
                )
                .with(MeshRenderer::new(CUBE_ID, MATERIAL_ID)),
        );
        game.events().add(
            Spawn::new()
                .with(
                    Transform::default()
                        .with_scale((0.5, 0.5, 0.5).into())
                        .with_position(Vec3::new(0.0, 1.0, 0.0)),
                )
                .with(MeshRenderer::new(CUBE_ID, MATERIAL_ID)),
        );
        game.events().add(
            Spawn::new()
                .with(Camera::default().with_clear(Color::blue()))
                .with(Transform::zero().with_position(Vec3::new(0.0, 0.0, 10.0))),
        );
        game.add_draw_call_extractor::<DrawMeshExtractor>();
        game.add_render_resource_extractor::<CameraBinding>();
        game.add_render_resource_extractor::<ObjectBindings>();
        game.add_render_resource_extractor::<BasicRenderPipeline>();
        game.add_render_node(BasicRenderNode::new());

        let app = game.sub_app_mut::<RenderApp>().unwrap();
        app.add_system(
            Extract,
            |query: Main<Query<(&Camera, &Transform)>>,
             cameras: &mut RenderAssets<RenderCamera>| {
                for (camera, transform) in query.into_inner() {
                    let world = transform.local_to_world;
                    cameras.add(camera.depth, RenderCamera::new(camera, world));
                }
            },
        );
        game.add_system(
            Update,
            |transforms: Query<&mut Transform, With<MeshRenderer>>, time: &Time| {
                for transform in transforms {
                    transform.rotate_around(Axis::Y, (45.0 * time.delta_secs()).to_radians());
                }
            },
        );
    }
}

pub struct BasicRenderNode {
    pass: RenderPass,
}

impl BasicRenderNode {
    pub fn new() -> Self {
        let pass = RenderPass::new()
            .with_color(
                Attachment::Surface,
                None,
                StoreOp::Store,
                Some(Color::blue()),
            )
            .with_depth(
                Attachment::Surface,
                Operations {
                    load: LoadOp::Clear(1.0),
                    store: StoreOp::Store,
                },
                None,
            );

        Self { pass }
    }
}

pub struct CameraBinding {
    pub layout: wgpu::BindGroupLayout,
    pub binding: BindGroup,
    pub buffer: UniformBuffer<CameraData>,
}

impl CameraBinding {
    pub fn new(device: &RenderDevice) -> Self {
        let camera = UniformBuffer::new(device, CameraData::default(), BufferFlags::COPY_DST);
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let binding = BindGroup::create(
            device,
            &layout,
            &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera.buffer().as_entire_binding(),
            }],
            (),
        );

        Self {
            layout,
            binding,
            buffer: camera,
        }
    }
}

impl Resource for CameraBinding {}

impl RenderResourceExtractor for CameraBinding {
    type Event = WindowCreated;
    type Target = CameraBinding;
    type Arg = StaticSystemArg<'static, Read<RenderDevice>>;

    fn extract(device: &ArgItem<Self::Arg>) -> Option<Self::Target> {
        Some(Self::new(device))
    }
}

#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
pub struct ModelData(glam::Mat4);

impl From<glam::Mat4> for ModelData {
    fn from(value: glam::Mat4) -> Self {
        Self(value)
    }
}

pub struct ObjectBindings {
    layout: wgpu::BindGroupLayout,
    buffers: BatchedUniformBuffer<ModelData>,
    bind_groups: Vec<BindGroup>,
}

impl ObjectBindings {
    pub fn new(device: &RenderDevice) -> Self {
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: true,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        Self {
            layout,
            buffers: BatchedUniformBuffer::new(device, BufferFlags::COPY_DST),
            bind_groups: Vec::new(),
        }
    }

    pub fn push(&mut self, model: ModelData) -> BatchIndex<ModelData> {
        self.buffers.push(model)
    }

    pub fn binding(&self, index: usize) -> &BindGroup {
        &self.bind_groups[index]
    }

    pub fn commit(&mut self, device: &RenderDevice, queue: &RenderQueue) {
        self.buffers.commit(device, queue);

        for index in self.bind_groups.len()..self.buffers.buffer_count() {
            let buffer = self.buffers.buffer(index);

            let binding = BindGroup::create(
                device,
                &self.layout,
                &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer,
                        offset: 0,
                        size: wgpu::BufferSize::new(std::mem::size_of::<ModelData>() as u64),
                    }),
                }],
                (),
            );

            self.bind_groups.push(binding);
        }
    }
}

impl Resource for ObjectBindings {}

impl RenderResourceExtractor for ObjectBindings {
    type Event = WindowCreated;
    type Target = ObjectBindings;
    type Arg = StaticSystemArg<'static, Read<RenderDevice>>;

    fn extract(device: &ArgItem<Self::Arg>) -> Option<Self::Target> {
        Some(Self::new(device))
    }
}

pub struct DrawMesh {
    pub mesh: AssetId,
    pub batch_index: BatchIndex<ModelData>,
}

impl Draw for DrawMesh {}

pub struct DrawMeshExtractor;

impl DrawCallExtractor for DrawMeshExtractor {
    type Draw = DrawMesh;
    type Arg = StaticSystemArg<
        'static,
        (
            Main<'static, StaticQuery<(Read<MeshRenderer>, Read<Transform>)>>,
            Write<ObjectBindings>,
        ),
    >;

    fn extract(draw: &mut DrawCalls<Self::Draw>, arg: ArgItem<Self::Arg>) {
        let (main, bindings) = arg.into_inner();
        for (renderer, transform) in main.into_inner() {
            let batch_index = bindings.push(transform.local_to_world.into());
            draw.add(DrawMesh {
                mesh: renderer.mesh,
                batch_index,
            });
        }
    }
}

pub struct BasicRenderPipeline(RenderPipeline);

impl BasicRenderPipeline {
    pub fn new(
        device: &RenderDevice,
        surface: &RenderSurface,
        camera: &CameraBinding,
        object: &ObjectBindings,
        shaders: &RenderAssets<Shader>,
    ) -> Option<Self> {
        let layout: &[&wgpu::BindGroupLayout] = &[&camera.layout, &object.layout];

        let desc = RenderPipelineDesc {
            label: Some("BasicRenderPipeline"),
            layout: Some(layout),
            vertex: VertexState {
                shader: AssetHandle::<Shader>::Id(SHADER_ID),
                entry: "vs_main".into(),
                buffers: vec![VertexBufferLayout {
                    array_stride: std::mem::size_of::<Vec3>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: vec![wgpu::VertexAttribute {
                        format: wgpu::VertexFormat::Float32x3,
                        offset: 0,
                        shader_location: 0,
                    }],
                }],
            },
            fragment: Some(FragmentState {
                shader: AssetHandle::<Shader>::Id(SHADER_ID),
                entry: "fs_main".into(),
                targets: vec![Some(wgpu::ColorTargetState {
                    format: surface.format(),
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_state: Some(wgpu::DepthStencilState {
                format: surface.depth_format(),
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: Default::default(),
        };

        let pipeline = RenderPipeline::create(device, desc, shaders)?;

        Some(Self(pipeline))
    }
}

impl RenderResourceExtractor for BasicRenderPipeline {
    type Event = WindowCreated;
    type Target = BasicRenderPipeline;
    type Arg = StaticSystemArg<
        'static,
        (
            Read<RenderDevice>,
            Read<RenderSurface>,
            Read<CameraBinding>,
            Read<ObjectBindings>,
            Read<RenderAssets<Shader>>,
        ),
    >;

    fn extract(arg: &ArgItem<Self::Arg>) -> Option<Self::Target> {
        let (device, surface, camera, object, shaders) = **arg;
        Some(Self::new(device, surface, camera, object, shaders)?)
    }

    fn extracted_resource() -> Option<graphics::resources::ExtractedResource> {
        Some(graphics::resources::ExtractedResource::Pipeline)
    }
}

impl std::ops::Deref for BasicRenderPipeline {
    type Target = RenderPipeline;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Resource for BasicRenderPipeline {}

impl RenderGraphNode for BasicRenderNode {
    fn name(&self) -> &str {
        "BasicRenderNode"
    }

    fn info(&self) -> graphics::renderer::graph::node::NodeInfo {
        self.pass.info()
    }

    fn execute(&mut self, ctx: &RenderContext) {
        let mut encoder = ctx.encoder();
        if let Some(mut commands) = self.pass.begin(ctx, &mut encoder) {
            let queue = ctx.queue();
            let device = ctx.device();
            let draws = ctx.resource::<DrawCalls<DrawMesh>>();
            let meshes = ctx.resource::<RenderAssets<MeshBuffers>>();
            let camera = ctx.resource_mut::<CameraBinding>();
            let object = ctx.resource_mut::<ObjectBindings>();
            let pipeline = ctx.resource::<BasicRenderPipeline>();

            object.commit(device, queue);
            camera.buffer.update(queue, ctx.camera().data);
            commands.set_pipeline(pipeline);
            commands.set_bind_group(0, &camera.binding, &[]);

            for call in draws.iter() {
                let mesh = match meshes.get(&call.mesh) {
                    Some(mesh) => mesh,
                    None => continue,
                };

                let vertex_buffer = match mesh.vertex_buffer(MeshAttributeKind::Position) {
                    Some(buffer) => buffer,
                    None => continue,
                };

                let index_buffer = match mesh.index_buffer() {
                    Some(buffer) => buffer,
                    None => continue,
                };

                let index = call.batch_index.index();
                let offset = call.batch_index.offset();
                commands.set_bind_group(1, &object.binding(index), &[offset]);
                commands.set_vertex_buffer(0, vertex_buffer.slice(..));
                commands.set_index_buffer(index_buffer.slice(..), index_buffer.format());
                commands.draw_indexed(0..index_buffer.len() as u32, 0, 0..1);
            }
        }

        ctx.submit(encoder);
    }
}
