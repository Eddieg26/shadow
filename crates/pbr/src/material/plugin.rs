use super::{
    extractor::MaterialExtractor,
    layout::{GlobalBinding, MaterialLayout, ObjectBinding},
    pass::{DrawMesh, ForwardPass},
    pipeline::{MaterialPipeline, MaterialPipelineRegistry, MaterialPipelines},
    Material, MaterialInstance, MaterialTypeTracker,
};
use ecs::world::event::{AddResource, Events};
use game::{Game, Plugin, Plugins};
use graphics::{
    camera::RenderCamera,
    core::{RenderDevice, RenderQueue},
    plugin::{GraphicsExt, GraphicsPlugin, PreRender, RenderApp, SurfaceCreated},
    renderer::graph::RenderGraphBuilder,
    resources::RenderAssets,
};

pub struct MaterialPlugin;

impl Plugin for MaterialPlugin {
    fn dependencies(&self) -> game::Plugins {
        let mut deps = Plugins::new();
        deps.add_plugin(GraphicsPlugin);
        deps
    }

    fn start(&self, game: &mut game::Game) {
        game.register_event::<AddResource<GlobalBinding>>();
        game.register_event::<AddResource<ObjectBinding>>();
        game.add_draw_calls::<DrawMesh>();
        game.resource_mut::<RenderGraphBuilder>()
            .add_node(ForwardPass::new());

        let app = game.sub_app_mut::<RenderApp>().unwrap();
        app.init_resource::<RenderAssets<MaterialLayout>>();
        app.init_resource::<RenderAssets<MaterialInstance>>();
        app.init_resource::<RenderAssets<MaterialPipelines>>();
        app.init_resource::<MaterialPipelineRegistry>();
        app.init_resource::<MaterialTypeTracker>();
        app.add_system(PreRender, update_camera_buffer);
        app.observe::<SurfaceCreated, _>(create_resources);
    }
}

fn create_resources(
    _: &[()],
    device: &RenderDevice,
    piplines: &mut RenderAssets<MaterialPipelines>,
    registry: &MaterialPipelineRegistry,
    events: &Events,
) {
    events.add(AddResource::new(GlobalBinding::create(device)));
    events.add(AddResource::new(ObjectBinding::create(device)));

    for (key, meta) in registry.iter() {
        piplines.add(*key, MaterialPipelines::create(device, meta));
    }
}

fn update_camera_buffer(
    device: &RenderDevice,
    queue: &RenderQueue,
    cameras: &RenderAssets<RenderCamera>,
    global: &mut GlobalBinding,
) {
    let len = global.camera_mut().len().min(cameras.len());
    for index in 0..len {
        global.camera_mut().set(index, cameras[index].data);
    }

    for index in len..cameras.len() {
        global.camera_mut().push(cameras[index].data);
    }

    global.camera_mut().commit(device, queue);
}

pub trait MaterialExt: 'static {
    fn register_material<M: Material>(&mut self) -> &mut Self;
    fn register_material_pipeline<M: MaterialPipeline>(&mut self) -> &mut Self;
}

impl MaterialExt for Game {
    fn register_material<M: Material>(&mut self) -> &mut Self {
        self.add_render_asset_extractor::<MaterialExtractor<M>>()
    }

    fn register_material_pipeline<M: MaterialPipeline>(&mut self) -> &mut Self {
        let app = self.sub_app_mut::<RenderApp>().unwrap();
        app.resource_mut::<MaterialPipelineRegistry>()
            .register::<M>();
        self
    }
}
