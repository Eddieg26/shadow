use super::{
    layout::{GlobalBinding, MaterialLayout, ObjectBinding},
    pipeline::MaterialPipelines,
    Material, MaterialInstance, MaterialType, MaterialTypeTracker,
};
use ecs::system::{
    unlifetime::{Read, Write},
    StaticSystemArg,
};
use graphics::{
    core::RenderDevice,
    renderer::surface::RenderSurface,
    resources::{
        shader::Shader, AssetUsage, ExtractedResource, RenderAssetExtractor, RenderAssets,
    },
};
use std::ops::DerefMut;

pub struct MaterialExtractor<M: Material>(std::marker::PhantomData<M>);

impl<M: Material> RenderAssetExtractor for MaterialExtractor<M> {
    type Source = M;
    type Target = MaterialInstance;
    type Arg = StaticSystemArg<
        'static,
        (
            Read<RenderDevice>,
            Read<RenderSurface>,
            Write<RenderAssets<MaterialLayout>>,
            Write<RenderAssets<MaterialPipelines>>,
            Read<RenderAssets<Shader>>,
            Read<GlobalBinding>,
            Read<ObjectBinding>,
            Write<MaterialTypeTracker>,
            M::Arg,
        ),
    >;

    fn extract(
        id: &asset::AssetId,
        source: &mut Self::Source,
        arg: &mut ecs::system::ArgItem<Self::Arg>,
        assets: &mut RenderAssets<Self::Target>,
    ) -> Option<AssetUsage> {
        let (device, surface, layouts, pipelines, shaders, global, object, tracker, arg) =
            (*arg).deref_mut();

        let ty = MaterialType::of::<M>();
        if tracker.track(ty) {
            layouts.add(ty, MaterialLayout::create::<M>(device));

            let shader = Shader::create(device, &M::shader().generate());
            for (_, pipeline) in pipelines.iter_mut() {
                pipeline.add::<M>(device, surface, layouts, shaders, &shader, global, object);
            }
        }

        let layout = layouts.get(&ty)?;
        let instance = MaterialInstance {
            ty,
            model: M::model(),
            mode: M::mode(),
            binding: source.bind_group(device, &layout, arg)?,
        };

        assets.add(*id, instance);

        Some(AssetUsage::Discard)
    }

    fn remove(
        id: &asset::AssetId,
        assets: &mut RenderAssets<Self::Target>,
        arg: &mut ecs::system::ArgItem<Self::Arg>,
    ) {
        let (_, _, _, pipelines, _, _, _, tracker, _) = (*arg).deref_mut();

        let ty = MaterialType::of::<M>();
        if tracker.untrack(ty) {
            for (_, pipeline) in pipelines.iter_mut() {
                pipeline.remove(ty);
            }
        }

        assets.remove(id);
    }

    fn extracted_resource() -> Option<ExtractedResource> {
        Some(ExtractedResource::Pipeline)
    }
}
