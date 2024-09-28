use asset::{database::events::AssetLoaded, embed_asset, plugin::AssetExt, AssetHandle, AssetId};
use ecs::{
    core::{
        internal::blob::{Blob, BlobCell},
        Entity, Resource,
    },
    system::{
        unlifetime::{Read, StaticQuery},
        ArgItem, StaticSystemArg,
    },
    world::{components::Children, event::Spawn, query::Query},
};
use game::{
    game::Game,
    plugin::{Plugin, Plugins},
    time::Time,
    Extract, Main, Update,
};
use glam::{Vec2, Vec3};
use graphics::{
    camera::{Camera, CameraData, RenderCamera},
    core::{Color, RenderDevice, RenderInstance},
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
        binding::BindGroup,
        buffer::{BufferFlags, Indices, UniformBuffer},
        mesh::{Mesh, MeshAttribute, MeshAttributeKind, MeshBuffers, MeshTopology},
        pipeline::{
            FragmentState, RenderPipeline, RenderPipelineDesc, VertexBufferLayout, VertexState,
        },
        shader::Shader,
        RenderAssets, RenderResourceExtractor,
    },
};
use spatial::{Axis, Transform};
use std::{collections::HashMap, sync::Arc, u32};
use window::events::WindowCreated;

const TEST_ID: AssetId = AssetId::raw(100);
const CUBE_ID: AssetId = AssetId::raw(200);
const SHADER_ID: AssetId = AssetId::raw(300);
const MATERIAL_ID: AssetId = AssetId::raw(400);
const TRIANGLE_ID: AssetId = AssetId::raw(500);

fn main() {
    // Game::new().add_plugin(BasicPlugin).run();

    // let shader = Opaque3D::shader().generate();
    // let source = match shader {
    //     ShaderSource::Wgsl(source) => source,
    //     _ => panic!("Expected WGSL shader"),
    // };

    // std::fs::write("mesh_shader.wgsl", source.deref()).unwrap();
}

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

        let mut triangle = Mesh::new(MeshTopology::TriangleList);
        triangle.add_attribute(MeshAttribute::Position(vec![
            Vec3::new(-1.0, -1.0, 0.0),
            Vec3::new(1.0, -1.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        ]));

        triangle.add_attribute(MeshAttribute::TexCoord0(vec![
            Vec2::new(0.0, 0.0),
            Vec2::new(1.0, 0.0),
            Vec2::new(0.5, 1.0),
        ]));

        triangle.set_indices(Indices::U32(vec![0, 1, 2]));

        game.events().add(AssetLoaded::add(TRIANGLE_ID, triangle));
        game.events().add(Spawn::new().with(Transform::default()));
        game.add_draw_call_extractor::<DrawMeshExtractor>();
        game.add_render_resource_extractor::<CameraBinding>();
        game.add_render_resource_extractor::<ObjectBinding>();
        game.add_render_resource_extractor::<BasicRenderPipeline>();
        game.add_render_node(BasicRenderNode::new());

        let app = game.sub_app_mut::<RenderApp>().unwrap();
        app.add_system(Extract, |cameras: &mut RenderAssets<RenderCamera>| {
            let world = glam::Mat4::from_translation(Vec3::new(0.0, 0.0, 10.0));
            cameras.add(0, RenderCamera::new(&Camera::default(), world));
        });
        game.add_system(Update, |transforms: Query<&mut Transform>, time: &Time| {
            for transform in transforms {
                transform.rotate_around(Axis::Y, (45.0 * time.delta_secs()).to_radians());
            }
        });
    }
}

pub async fn create_device() -> (wgpu::Device, wgpu::Queue) {
    let instance = RenderInstance::create();
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptionsBase::default())
        .await
        .unwrap();
    adapter
        .request_device(&wgpu::DeviceDescriptor::default(), None)
        .await
        .unwrap()
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
    pub binding: BindGroup<Arc<wgpu::BindGroupLayout>>,
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
        let layout = Arc::new(layout);

        let binding = BindGroup::create(
            device,
            &layout.clone(),
            &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera.buffer().as_entire_binding(),
            }],
            layout,
        );

        Self {
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

pub struct DrawMesh {
    pub mesh: AssetId,
    pub world: glam::Mat4,
}

impl Draw for DrawMesh {}

pub struct DrawMeshExtractor;

impl DrawCallExtractor for DrawMeshExtractor {
    type Draw = DrawMesh;
    type Arg = StaticSystemArg<'static, Main<'static, StaticQuery<Read<Transform>>>>;

    fn extract(draw: &mut DrawCalls<Self::Draw>, arg: ArgItem<Self::Arg>) {
        for transform in arg.into_inner().into_inner() {
            let world = glam::Mat4::from_scale_rotation_translation(
                transform.scale,
                transform.rotation,
                transform.position,
            );

            draw.add(DrawMesh {
                mesh: CUBE_ID,
                world,
            });
        }
    }
}

pub struct ObjectBinding {
    pub binding: BindGroup<Arc<wgpu::BindGroupLayout>>,
    pub buffer: UniformBuffer<ModelData>,
}

impl ObjectBinding {
    pub fn new(device: &RenderDevice) -> Self {
        let model = UniformBuffer::new(
            device,
            ModelData(glam::Mat4::IDENTITY),
            BufferFlags::COPY_DST,
        );
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let layout = Arc::new(layout);

        let binding = BindGroup::create(
            device,
            &layout.clone(),
            &[wgpu::BindGroupEntry {
                binding: 0,
                resource: model.buffer().as_entire_binding(),
            }],
            layout,
        );

        Self {
            binding,
            buffer: model,
        }
    }
}

impl Resource for ObjectBinding {}

impl RenderResourceExtractor for ObjectBinding {
    type Event = WindowCreated;
    type Target = ObjectBinding;
    type Arg = StaticSystemArg<'static, Read<RenderDevice>>;

    fn extract(device: &ArgItem<Self::Arg>) -> Option<Self::Target> {
        Some(Self::new(device))
    }
}

pub struct BasicRenderPipeline(RenderPipeline);

impl BasicRenderPipeline {
    pub fn new(
        device: &RenderDevice,
        surface: &RenderSurface,
        camera: &CameraBinding,
        object: &ObjectBinding,
        shaders: &RenderAssets<Shader>,
    ) -> Option<Self> {
        let layout: &[&wgpu::BindGroupLayout] = &[camera.binding.data(), object.binding.data()];

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
            Read<ObjectBinding>,
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
            let draws = ctx.resource::<DrawCalls<DrawMesh>>();
            let meshes = ctx.resource::<RenderAssets<MeshBuffers>>();
            let camera = ctx.resource_mut::<CameraBinding>();
            let object = ctx.resource_mut::<ObjectBinding>();
            let pipeline = ctx.resource::<BasicRenderPipeline>();

            commands.set_pipeline(pipeline);
            commands.set_bind_group(0, &camera.binding, &[]);
            commands.set_bind_group(1, &object.binding, &[]);
            camera.buffer.update(queue, ctx.camera().data);

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

                object.buffer.update(queue, call.world.into());
                commands.set_vertex_buffer(vertex_buffer.id(), 0, vertex_buffer.slice(..));
                commands.set_index_buffer(
                    index_buffer.id(),
                    index_buffer.slice(..),
                    index_buffer.format(),
                );
                commands.draw_indexed(0..index_buffer.len() as u32, 0, 0..1);
            }
        }

        ctx.submit(encoder);
    }
}
