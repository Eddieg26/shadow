use std::{
    borrow::Cow,
    hash::{Hash, Hasher},
    ops::Deref,
    path::PathBuf,
    u32,
};

use asset::{
    database::{events::AssetLoaded, AssetConfig, AssetDatabase},
    embed_asset,
    importer::{AssetImporter, ImportContext, StringError},
    io::local::{LocalAsset, LocalFileSystem},
    plugin::AssetExt,
    Asset, AssetActions, AssetId, AssetSettings, Assets, DefaultSettings,
};
use ecs::{
    core::Resource,
    system::schedule::{Phase, Root},
};
use game::{
    game::Game,
    phases::{Init, Update},
    plugin::{Plugin, Plugins},
    Execute, First, GameInstance, PostInit,
};
use glam::{Vec2, Vec3};
use graphics::{
    camera::{ClearFlag, RenderFrame, RenderFrames},
    core::{Color, RenderInstance},
    plugin::{GraphicsExt, GraphicsPlugin, RenderApp},
    renderer::{
        draw::{Draw, DrawCalls},
        graph::{
            context::RenderContext,
            pass::render::{Attachment, RenderCommands, RenderPass, StoreOp},
            RenderGraphBuilder,
        },
    },
    resources::{
        buffer::{BufferFlags, Indices, UniformBuffer},
        mesh::{
            loaders::{MeshLoadSettings, ObjImporter},
            Mesh, MeshAttribute, MeshTopology,
        },
        shader::ShaderSource,
        texture::{Texture, Texture2d, Texture2dSettings},
        ReadWrite, RenderAsset, RenderAssetExtractor, RenderAssets,
    },
};
use pbr::{
    pass::{DrawMesh, ForwardPass, ForwardSubPass, Opaque3D, RenderMeshGroup},
    pipeline::MaterialPipeline,
    plugin::{MaterialExt, MaterialPlugin},
    shader::{
        nodes::{CameraNode, ConvertNode, MultiplyNode, ObjectModelNode},
        vertex::MeshShader,
        ShaderProperty, VertexInput, VertexOutput,
    },
    unlit::UnlitMaterial,
    Material, MaterialInstance,
};
use spatial::bounds::BoundingBox;
use wgpu::core::instance;
use window::{events::WindowCreated, plugin::WindowPlugin};

const TEST_ID: AssetId = AssetId::raw(100);
const CUBE_ID: AssetId = AssetId::raw(200);
const SHADER_ID: AssetId = AssetId::raw(300);
const MATERIAL_ID: AssetId = AssetId::raw(400);
const TRIANGLE_ID: AssetId = AssetId::raw(500);

#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
pub struct ModelData([f32; 16]);

impl From<glam::Mat4> for ModelData {
    fn from(value: glam::Mat4) -> Self {
        Self(value.to_cols_array())
    }
}

pub struct BasicPlugin;

impl Plugin for BasicPlugin {
    fn dependencies(&self) -> Plugins {
        let mut plugins = Plugins::new();
        plugins.add_plugin(MaterialPlugin);
        plugins
    }

    fn start(&self, game: &mut Game) {
        let builder = game.resource_mut::<RenderGraphBuilder>();
        builder
            .node_mut::<ForwardPass>("forward")
            .unwrap()
            .add_render_group(
                ForwardSubPass::Opaque,
                RenderMeshGroup::<Opaque3D>::new(pbr::ShaderModel::Unlit, pbr::BlendMode::Opaque),
            );

        game.register_material_pipeline::<Opaque3D>();
        game.register_material::<UnlitMaterial>();
        game.events().add(AssetLoaded::add(
            MATERIAL_ID,
            UnlitMaterial {
                color: Color::blue(),
            },
        ));
    }

    fn run(&mut self, game: &mut Game) {
        embed_asset!(game, TEST_ID, "test.txt");
        embed_asset!(game, CUBE_ID, "cube.obj");
        embed_asset!(game, SHADER_ID, "shader.wgsl");

        let mut triange = Mesh::new(MeshTopology::TriangleList, ReadWrite::Disabled);
        triange.add_attribute(MeshAttribute::Position(vec![
            Vec3::new(-1.0, -1.0, 0.0),
            Vec3::new(1.0, -1.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        ]));

        triange.add_attribute(MeshAttribute::TexCoord0(vec![
            Vec2::new(0.0, 0.0),
            Vec2::new(1.0, 0.0),
            Vec2::new(0.5, 1.0),
        ]));

        triange.set_indices(Indices::U32(vec![0, 1, 2]));

        game.events().add(AssetLoaded::add(TRIANGLE_ID, triange));

        game.add_system(
            Update,
            |renders: &mut RenderFrames,
             draw: &mut DrawCalls<DrawMesh>,
             materials: &RenderAssets<MaterialInstance>| {
                let mut frame = RenderFrame::default();
                frame.camera.clear = Some(ClearFlag::Color(Color::green()));
                renders.add(frame);

                let material = match materials.get(&MATERIAL_ID) {
                    Some(material) => material,
                    None => return,
                };

                draw.add(DrawMesh::new(
                    CUBE_ID,
                    material.clone(),
                    glam::Mat4::from_translation(Vec3::new(0.0, 0.0, 10.0)),
                    BoundingBox::ZERO,
                ));
            },
        );
    }

    fn finish(&mut self, _: &mut Game) {}
}

fn main() {
    Game::new().add_plugin(BasicPlugin).run();

    // let shader = Opaque3D::shader().generate();
    // let source = match shader {
    //     ShaderSource::Wgsl(source) => source,
    //     _ => panic!("Expected WGSL shader"),
    // };

    // std::fs::write("mesh_shader.wgsl", source.deref()).unwrap();
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
