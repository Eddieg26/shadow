use asset::{
    database::events::AssetLoaded, embed_asset, importer::AssetError, plugin::AssetExt,
    AssetHandle, AssetId, Assets,
};
use ecs::{
    core::{Component, Resource},
    system::{
        unlifetime::{Read, StaticQuery, Write},
        ArgItem, StaticSystemArg,
    },
    world::{
        event::Spawn,
        query::{Query, With},
    },
};
use game::{
    game::Game,
    plugin::{Plugin, Plugins},
    time::Time,
    Extract, Last, Main, Update,
};
use glam::{EulerRot, Mat3, Mat4, Quat, UVec2, Vec2, Vec3};
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
        binding::BindGroup,
        buffer::{BatchIndex, BatchedUniformBuffer, BufferFlags, Indices, UniformBuffer},
        mesh::{Mesh, MeshAttribute, MeshAttributeKind, MeshBuffers, MeshTopology},
        pipeline::{
            FragmentState, RenderPipeline, RenderPipelineDesc, VertexBufferLayout, VertexState,
        },
        shader::Shader,
        texture::{GpuTexture, Texture, Texture2d},
        RenderAssets, RenderResourceExtractor,
    },
};
use spatial::{Axis, Transform};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    u32,
};
use window::{
    events::{CursorMoved, KeyboardInput, MouseInput, WindowCreated},
    keyboard::{KeyCode, PhysicalKey},
};

const TEST_ID: AssetId = AssetId::raw(100);
const CUBE_ID: AssetId = AssetId::raw(200);
const SHADER_ID: AssetId = AssetId::raw(300);
const MATERIAL_ID: AssetId = AssetId::raw(400);
const SKYBOX_SHADER_ID: AssetId = AssetId::raw(500);
const SKYBOX_LEFT: AssetId = AssetId::raw(600);
const SKYBOX_RIGHT: AssetId = AssetId::raw(700);
const SKYBOX_TOP: AssetId = AssetId::raw(800);
const SKYBOX_BOTTOM: AssetId = AssetId::raw(900);
const SKYBOX_FRONT: AssetId = AssetId::raw(1000);
const SKYBOX_BACK: AssetId = AssetId::raw(1100);
const GENGAR: AssetId = AssetId::raw(1200);
const PLANE: AssetId = AssetId::raw(1300);

const MESH_TEXTURE: AssetId = GENGAR;
const CAMERA_SPEED: f32 = 5.0;
const MOUSE_SPEED: f32 = 0.75;

fn main() {
    Game::new().add_plugin(BasicPlugin).run();
}

#[derive(Debug, Clone, Copy)]
pub struct MeshRenderer {
    pub mesh: AssetId,
    pub texture: AssetId,
}

impl MeshRenderer {
    pub fn new(mesh: AssetId, texture: AssetId) -> Self {
        Self { mesh, texture }
    }
}

impl Component for MeshRenderer {}

#[derive(Debug, Default, Clone, Copy)]
pub struct MouseTracker {
    pub x: f32,
    pub y: f32,
}

impl Component for MouseTracker {}

pub struct InputState {
    keys: HashMap<window::keyboard::KeyCode, window::event::ElementState>,
    mouse_state: HashMap<window::event::MouseButton, window::event::ElementState>,
    cursor_position: Vec2,
}

impl InputState {
    pub fn new() -> Self {
        Self {
            keys: HashMap::new(),
            mouse_state: HashMap::new(),
            cursor_position: Vec2::ZERO,
        }
    }

    pub fn update(&mut self, event: &window::events::KeyboardInput) {
        match event.event.physical_key {
            PhysicalKey::Code(key_code) => {
                self.keys.insert(key_code, event.event.state);
            }
            _ => {}
        }
    }

    pub fn update_mouse(&mut self, event: &window::events::MouseInput) {
        self.mouse_state.insert(event.button, event.state);
    }

    pub fn is_pressed(&self, key: window::keyboard::KeyCode) -> bool {
        self.keys.get(&key) == Some(&window::event::ElementState::Pressed)
    }

    pub fn is_released(&self, key: window::keyboard::KeyCode) -> bool {
        self.keys.get(&key) == Some(&window::event::ElementState::Released)
    }

    pub fn is_down(&self, key: window::keyboard::KeyCode) -> bool {
        self.is_pressed(key) || self.is_released(key)
    }

    pub fn is_up(&self, key: window::keyboard::KeyCode) -> bool {
        !self.is_down(key)
    }

    pub fn cursor_position(&self) -> Vec2 {
        self.cursor_position
    }

    pub fn mouse_button_state(
        &self,
        button: window::event::MouseButton,
    ) -> Option<&window::event::ElementState> {
        self.mouse_state.get(&button)
    }

    pub fn clear(&mut self) {
        self.keys.clear();
        self.mouse_state.clear();
    }

    pub fn clear_released(&mut self) {
        self.keys
            .retain(|_, state| *state == window::event::ElementState::Pressed);

        self.mouse_state
            .retain(|_, state| *state == window::event::ElementState::Pressed);
    }
}

impl Resource for InputState {}

pub fn create_cube() -> Mesh {
    let mut mesh = Mesh::new(MeshTopology::TriangleList);

    mesh.add_attribute(MeshAttribute::Position(vec![
        // Front
        Vec3::new(1.0, -1.0, -1.0),
        Vec3::new(-1.0, -1.0, -1.0),
        Vec3::new(1.0, 1.0, -1.0),
        Vec3::new(-1.0, -1.0, -1.0),
        Vec3::new(-1.0, 1.0, -1.0),
        Vec3::new(1.0, 1.0, -1.0),
        // Back
        Vec3::new(1.0, 1.0, 1.0),
        Vec3::new(-1.0, 1.0, 1.0),
        Vec3::new(-1.0, -1.0, 1.0),
        Vec3::new(1.0, 1.0, 1.0),
        Vec3::new(-1.0, -1.0, 1.0),
        Vec3::new(1.0, -1.0, 1.0),
        // Top
        Vec3::new(1.0, 1.0, -1.0),
        Vec3::new(-1.0, 1.0, -1.0),
        Vec3::new(1.0, 1.0, 1.0),
        Vec3::new(-1.0, 1.0, -1.0),
        Vec3::new(-1.0, 1.0, 1.0),
        Vec3::new(1.0, 1.0, 1.0),
        // Bottom
        Vec3::new(1.0, -1.0, 1.0),
        Vec3::new(-1.0, -1.0, 1.0),
        Vec3::new(-1.0, -1.0, -1.0),
        Vec3::new(1.0, -1.0, 1.0),
        Vec3::new(-1.0, -1.0, -1.0),
        Vec3::new(1.0, -1.0, -1.0),
        // Right
        Vec3::new(1.0, 1.0, 1.0),
        Vec3::new(1.0, -1.0, 1.0),
        Vec3::new(1.0, -1.0, -1.0),
        Vec3::new(1.0, 1.0, 1.0),
        Vec3::new(1.0, -1.0, -1.0),
        Vec3::new(1.0, 1.0, -1.0),
        // Left
        Vec3::new(-1.0, -1.0, -1.0),
        Vec3::new(-1.0, -1.0, 1.0),
        Vec3::new(-1.0, 1.0, 1.0),
        Vec3::new(-1.0, 1.0, -1.0),
        Vec3::new(-1.0, -1.0, -1.0),
        Vec3::new(-1.0, 1.0, 1.0),
    ]));

    mesh.add_attribute(MeshAttribute::TexCoord0(vec![
        // Front
        Vec2::new(1.0, 1.0),
        Vec2::new(0.0, 1.0),
        Vec2::new(1.0, 0.0),
        Vec2::new(0.0, 1.0),
        Vec2::new(0.0, 0.0),
        Vec2::new(1.0, 0.0),
        // Back
        Vec2::new(1.0, 0.0),
        Vec2::new(0.0, 0.0),
        Vec2::new(0.0, 1.0),
        Vec2::new(1.0, 0.0),
        Vec2::new(0.0, 1.0),
        Vec2::new(1.0, 1.0),
        // Top
        Vec2::new(1.0, 1.0),
        Vec2::new(0.0, 1.0),
        Vec2::new(1.0, 0.0),
        Vec2::new(0.0, 1.0),
        Vec2::new(0.0, 0.0),
        Vec2::new(1.0, 0.0),
        // Bottom
        Vec2::new(1.0, 0.0),
        Vec2::new(0.0, 0.0),
        Vec2::new(0.0, 1.0),
        Vec2::new(1.0, 0.0),
        Vec2::new(0.0, 1.0),
        Vec2::new(1.0, 1.0),
        // Right
        Vec2::new(0.0, 0.0),
        Vec2::new(0.0, 1.0),
        Vec2::new(1.0, 1.0),
        Vec2::new(0.0, 0.0),
        Vec2::new(1.0, 1.0),
        Vec2::new(1.0, 0.0),
        // Left
        Vec2::new(0.0, 1.0),
        Vec2::new(1.0, 1.0),
        Vec2::new(1.0, 0.0),
        Vec2::new(0.0, 0.0),
        Vec2::new(0.0, 1.0),
        Vec2::new(1.0, 0.0),
    ]));

    mesh
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
        // embed_asset!(game, CUBE_ID, "cube.obj");
        embed_asset!(game, SHADER_ID, "shader.wgsl");
        embed_asset!(game, SKYBOX_SHADER_ID, "skybox.wgsl");
        embed_asset!(game, SKYBOX_BACK, "skybox/back.jpg");
        embed_asset!(game, SKYBOX_BOTTOM, "skybox/bottom.jpg");
        embed_asset!(game, SKYBOX_FRONT, "skybox/front.jpg");
        embed_asset!(game, SKYBOX_LEFT, "skybox/left.jpg");
        embed_asset!(game, SKYBOX_RIGHT, "skybox/right.jpg");
        embed_asset!(game, SKYBOX_TOP, "skybox/top.jpg");
        embed_asset!(game, GENGAR, "gengar.png");

        let mut plane = Mesh::new(MeshTopology::TriangleList);
        plane.add_attribute(MeshAttribute::Position(vec![
            Vec3::new(-1.0, -1.0, 0.0),
            Vec3::new(1.0, -1.0, 0.0),
            Vec3::new(1.0, 1.0, 0.0),
            Vec3::new(-1.0, 1.0, 0.0),
        ]));

        plane.add_attribute(MeshAttribute::TexCoord0(vec![
            Vec2::new(0.0, 1.0),
            Vec2::new(1.0, 1.0),
            Vec2::new(1.0, 0.0),
            Vec2::new(0.0, 0.0),
        ]));

        plane.set_indices(Indices::U32(vec![0, 1, 2, 0, 2, 3]));

        game.add_resource(InputState::new());
        game.register::<MeshRenderer>();
        game.register::<MouseTracker>();
        game.events().add(AssetLoaded::add(PLANE, plane));
        game.events().add(AssetLoaded::add(CUBE_ID, create_cube()));
        // game.events().add(
        //     Spawn::new()
        //         .with(
        //             Transform::default()
        //                 .with_scale((0.5, 0.5, 0.5).into())
        //                 .with_position(Vec3::new(1.0, 0.0, 0.0)),
        //         )
        //         .with(MeshRenderer::new(PLANE, MESH_TEXTURE)),
        // );
        game.events().add(
            Spawn::new()
                .with(
                    Transform::default()
                        .with_scale((0.5, 0.5, 0.5).into())
                        .with_position(Vec3::new(0.0, 1.0, 0.0)),
                )
                .with(MeshRenderer::new(CUBE_ID, MESH_TEXTURE)),
        );
        game.events().add(
            Spawn::new()
                .with(Camera::default().with_clear(ClearFlag::Skybox))
                .with(Transform::zero().with_position(Vec3::new(0.0, 0.0, 10.0)))
                .with(MouseTracker::default()),
        );
        game.add_draw_call_extractor::<DrawMeshExtractor>();
        game.add_render_resource_extractor::<CameraBinding>();
        game.add_render_resource_extractor::<ObjectBindings>();
        game.add_render_resource_extractor::<TextureBinding>();
        game.add_render_resource_extractor::<BasicRenderPipeline>();
        game.add_render_resource_extractor::<SkyboxPipeline>();
        game.add_render_node(BasicRenderNode::new());
        game.observe::<AssetError, _>(|errors: &[AssetError]| {
            for error in errors {
                println!("{}", error);
            }
        });
        game.add_system(Last, |inputs: &mut InputState| {
            inputs.clear_released();
        });
        game.observe::<KeyboardInput, _>(|events: &[KeyboardInput], inputs: &mut InputState| {
            for event in events {
                inputs.update(event);
            }
        });
        game.observe::<CursorMoved, _>(|events: &[CursorMoved], inputs: &mut InputState| {
            let event = events.last().unwrap();
            inputs.cursor_position = Vec2::new(event.x as f32, event.y as f32);
        });
        game.observe::<MouseInput, _>(|events: &[MouseInput], inputs: &mut InputState| {
            for event in events {
                inputs.update_mouse(event);
            }
        });

        game.add_system(
            Update,
            |transforms: Query<&mut Transform, With<Camera>>, time: &Time, inputs: &InputState| {
                for transform in transforms {
                    let mut translation = Vec3::ZERO;
                    if inputs.is_pressed(KeyCode::KeyW) {
                        translation -= Vec3::Z;
                    }

                    if inputs.is_pressed(KeyCode::KeyA) {
                        translation -= Vec3::X;
                    }

                    if inputs.is_pressed(KeyCode::KeyS) {
                        translation += Vec3::Z;
                    }

                    if inputs.is_pressed(KeyCode::KeyD) {
                        translation += Vec3::X;
                    }

                    transform.translate_world(translation * CAMERA_SPEED * time.delta_secs());
                }
            },
        );

        game.add_system(
            Update,
            |cameras: Query<(&mut Transform, &mut MouseTracker), With<Camera>>,
             inputs: &InputState| {
                for (transform, tracker) in cameras {
                    if inputs.mouse_button_state(window::event::MouseButton::Left)
                        == Some(&window::event::ElementState::Pressed)
                    {
                        let Vec2 { x, y } = inputs.cursor_position;
                        let delta = Vec2::new(x - tracker.x, y - tracker.y);

                        tracker.x = x;
                        tracker.y = y;

                        let dx = delta.x.clamp(-1.0, 1.0);
                        let dy = delta.y.clamp(-1.0, 1.0);

                        let yaw = Quat::from_rotation_y((-dx * MOUSE_SPEED).to_radians());
                        let pitch = Quat::from_rotation_x((-dy * MOUSE_SPEED).to_radians());

                        let rotation = yaw * pitch;
                        transform.rotate(rotation);
                    }
                }
            },
        );

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
        // game.add_system(
        //     Update,
        //     |transforms: Query<&mut Transform, With<MeshRenderer>>, time: &Time| {
        //         for transform in transforms {
        //             transform.rotate_around(Axis::Y, (45.0 * time.delta_secs()).to_radians());
        //         }
        //     },
        // );
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
            label: Some("CameraBindGroupLayout"),
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

    fn extract(device: ArgItem<Self::Arg>) -> Option<Self::Target> {
        Some(Self::new(&device))
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

    fn extract(device: ArgItem<Self::Arg>) -> Option<Self::Target> {
        Some(Self::new(&device))
    }
}

pub struct TextureBinding {
    pub layout: wgpu::BindGroupLayout,
    pub binding: BindGroup,
}

impl TextureBinding {
    pub fn new(device: &RenderDevice, textures: &RenderAssets<GpuTexture>) -> Option<Self> {
        let texture = textures.get(&MESH_TEXTURE)?;

        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let binding = BindGroup::create(
            device,
            &layout,
            &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(texture.view()),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(texture.sampler()),
                },
            ],
            (),
        );

        Some(Self { layout, binding })
    }
}

impl Resource for TextureBinding {}

impl RenderResourceExtractor for TextureBinding {
    type Event = WindowCreated;
    type Target = TextureBinding;
    type Arg = StaticSystemArg<'static, (Read<RenderDevice>, Read<RenderAssets<GpuTexture>>)>;

    fn extract(device: ArgItem<Self::Arg>) -> Option<Self::Target> {
        let (device, textures) = device.into_inner();
        Self::new(device, textures)
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
        texture: &TextureBinding,
        shaders: &RenderAssets<Shader>,
    ) -> Option<Self> {
        let layout: &[&wgpu::BindGroupLayout] = &[&camera.layout, &object.layout, &texture.layout];

        let desc = RenderPipelineDesc {
            label: Some("BasicRenderPipeline"),
            layout: Some(layout),
            vertex: VertexState {
                shader: AssetHandle::<Shader>::Id(SHADER_ID),
                entry: "vs_main".into(),
                buffers: vec![
                    VertexBufferLayout {
                        array_stride: std::mem::size_of::<Vec3>() as u64,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: vec![wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Float32x3,
                            offset: 0,
                            shader_location: 0,
                        }],
                    },
                    VertexBufferLayout {
                        array_stride: std::mem::size_of::<Vec2>() as u64,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: vec![wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Float32x2,
                            offset: 0,
                            shader_location: 1,
                        }],
                    },
                ],
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
            primitive: wgpu::PrimitiveState {
                cull_mode: Some(wgpu::Face::Back),
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
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
            Read<TextureBinding>,
            Read<RenderAssets<Shader>>,
        ),
    >;

    fn extract(arg: ArgItem<Self::Arg>) -> Option<Self::Target> {
        let (device, surface, camera, object, texture, shaders) = arg.into_inner();
        Some(Self::new(
            device, surface, camera, object, texture, shaders,
        )?)
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

    fn render_skybox(&self, ctx: &RenderContext, pass: &mut wgpu::RenderPass) {
        let pipeline = ctx.resource_mut::<SkyboxPipeline>();
        let cube = match ctx.resource::<RenderAssets<MeshBuffers>>().get(&CUBE_ID) {
            Some(cube) => cube,
            None => return,
        };
        let cube = match cube.vertex_buffer(MeshAttributeKind::Position) {
            Some(cube) => cube,
            None => return,
        };

        let view = Mat4::from_mat3(Mat3::from_mat4(ctx.camera().data.view));
        let data = CameraData {
            view,
            ..ctx.camera().data
        };
        pipeline.camera.update(ctx.queue(), data);
        pass.set_pipeline(pipeline.pipeline());
        pass.set_bind_group(0, &pipeline.binding, &[]);
        pass.set_vertex_buffer(0, cube.slice(..));
        pass.draw(0..cube.len() as u32, 0..1);
    }
}

impl RenderGraphNode for BasicRenderNode {
    fn name(&self) -> &str {
        "BasicRenderNode"
    }

    fn info(&self) -> graphics::renderer::graph::node::NodeInfo {
        self.pass.info()
    }

    fn execute(&mut self, ctx: &RenderContext) {
        let mut encoder = ctx.encoder();
        if let Some(mut pass) = self.pass.begin(ctx, &mut encoder) {
            let queue = ctx.queue();
            let device = ctx.device();
            let draws = ctx.resource::<DrawCalls<DrawMesh>>();
            let meshes = ctx.resource::<RenderAssets<MeshBuffers>>();
            let camera = ctx.resource_mut::<CameraBinding>();
            let object = ctx.resource_mut::<ObjectBindings>();
            let texture = ctx.resource::<TextureBinding>();
            let pipeline = ctx.resource::<BasicRenderPipeline>();

            object.commit(device, queue);
            camera.buffer.update(queue, ctx.camera().data);
            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, &camera.binding, &[]);
            pass.set_bind_group(2, &texture.binding, &[]);

            for call in draws.iter() {
                let mesh = match meshes.get(&call.mesh) {
                    Some(mesh) => mesh,
                    None => continue,
                };

                let position_buffer = match mesh.vertex_buffer(MeshAttributeKind::Position) {
                    Some(buffer) => buffer,
                    None => continue,
                };

                let uv_buffer = match mesh.vertex_buffer(MeshAttributeKind::TexCoord0) {
                    Some(buffer) => buffer,
                    None => continue,
                };

                let index = call.batch_index.index();
                let offset = call.batch_index.offset();
                pass.set_bind_group(1, &object.binding(index), &[offset]);
                pass.set_vertex_buffer(0, position_buffer.slice(..));
                pass.set_vertex_buffer(1, uv_buffer.slice(..));

                match mesh.index_buffer() {
                    Some(buffer) => {
                        pass.set_index_buffer(buffer.slice(..), buffer.format());
                        pass.draw_indexed(0..buffer.len() as u32, 0, 0..1);
                    }
                    None => pass.draw(0..position_buffer.len() as u32, 0..1),
                }
            }

            self.render_skybox(ctx, &mut pass);
        }

        ctx.submit(encoder);
    }
}

pub struct SkyboxPipeline {
    pipeline: RenderPipeline,
    binding: BindGroup,
    sampler: wgpu::Sampler,
    camera: UniformBuffer<CameraData>,
}

impl SkyboxPipeline {
    pub fn new(
        device: &RenderDevice,
        queue: &RenderQueue,
        surface: &RenderSurface,
        shaders: &RenderAssets<Shader>,
        textures: &Assets<Texture2d>,
    ) -> Option<Self> {
        let shader = shaders.get(&SKYBOX_SHADER_ID)?;
        let textures = vec![
            textures.get(&SKYBOX_RIGHT)?,
            textures.get(&SKYBOX_LEFT)?,
            textures.get(&SKYBOX_TOP)?,
            textures.get(&SKYBOX_BOTTOM)?,
            textures.get(&SKYBOX_FRONT)?,
            textures.get(&SKYBOX_BACK)?,
        ];

        let (width, height) = textures.iter().fold((u32::MAX, u32::MAX), |(w, h), t| {
            (w.min(t.width()), h.min(t.height()))
        });

        let format = textures[0].format().into();
        let size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 6,
        };

        let texture_cube = device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[format],
        });

        for (layer, texture) in textures.iter().enumerate() {
            queue.write_texture(
                wgpu::ImageCopyTexture {
                    texture: &texture_cube,
                    mip_level: 0,
                    origin: wgpu::Origin3d {
                        x: 0,
                        y: 0,
                        z: layer as u32,
                    },
                    aspect: wgpu::TextureAspect::All,
                },
                texture.pixels(),
                wgpu::ImageDataLayout {
                    bytes_per_row: format
                        .block_copy_size(Some(texture.format().aspect()))
                        .map(|s| s * size.width),
                    ..Default::default()
                },
                wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
            );
        }

        let texture_view = texture_cube.create_view(&wgpu::TextureViewDescriptor {
            dimension: Some(wgpu::TextureViewDimension::Cube),
            aspect: wgpu::TextureAspect::All,
            ..Default::default()
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor::default());

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::Cube,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let camera = UniformBuffer::new(device, CameraData::default(), BufferFlags::COPY_DST);

        let binding = BindGroup::create(
            device,
            &bind_group_layout,
            &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: camera.binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
            (),
        );

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Skybox Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_sky",
                compilation_options: Default::default(),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<Vec3>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &[wgpu::VertexAttribute {
                        format: wgpu::VertexFormat::Float32x3,
                        offset: 0,
                        shader_location: 0,
                    }],
                }],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_sky",
                compilation_options: Default::default(),
                targets: &[Some(surface.format().into())],
            }),
            primitive: wgpu::PrimitiveState {
                front_face: wgpu::FrontFace::Cw,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: surface.depth_format(),
                depth_write_enabled: false,
                depth_compare: wgpu::CompareFunction::LessEqual,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        Some(Self {
            pipeline: RenderPipeline::from(pipeline),
            binding,
            sampler,
            camera,
        })
    }

    pub fn binding(&self) -> &BindGroup {
        &self.binding
    }

    pub fn pipeline(&self) -> &RenderPipeline {
        &self.pipeline
    }

    pub fn sampler(&self) -> &wgpu::Sampler {
        &self.sampler
    }

    pub fn camera(&self) -> &UniformBuffer<CameraData> {
        &self.camera
    }

    pub fn camera_mut(&mut self) -> &mut UniformBuffer<CameraData> {
        &mut self.camera
    }
}

impl Resource for SkyboxPipeline {}

impl RenderResourceExtractor for SkyboxPipeline {
    type Event = WindowCreated;
    type Target = SkyboxPipeline;
    type Arg = StaticSystemArg<
        'static,
        (
            Read<RenderDevice>,
            Read<RenderQueue>,
            Read<RenderSurface>,
            Read<RenderAssets<Shader>>,
            Main<'static, Read<Assets<Texture2d>>>,
        ),
    >;

    fn extract(arg: ArgItem<Self::Arg>) -> Option<Self::Target> {
        let (device, queue, surface, shader, textures) = arg.into_inner();

        Self::new(&device, &queue, &surface, &shader, &textures)
    }

    fn extracted_resource() -> Option<graphics::resources::ExtractedResource> {
        Some(graphics::resources::ExtractedResource::Pipeline)
    }
}
