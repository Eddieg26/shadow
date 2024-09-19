use super::{layout::MaterialLayout, shader::fragment::MaterialShader, BlendMode, Material, MaterialType, ShaderModel};
use crate::{core::RenderDevice, resources::RenderAssets};
use ecs::core::Resource;
use std::sync::{Arc, RwLock};

#[derive(Debug, Clone, Copy)]
pub struct MaterialMeta {
    pub ty: MaterialType,
    pub model: ShaderModel,
    pub mode: BlendMode,
    pub shader: fn() -> MaterialShader,
    layout: fn(&RenderDevice) -> MaterialLayout,
}

impl MaterialMeta {
    pub fn new<M: Material>() -> Self {
        Self {
            ty: MaterialType::of::<M>(),
            shader: M::shader,
            model: M::model(),
            mode: M::mode(),
            layout: |device| MaterialLayout::create::<M>(device),
        }
    }

    pub fn layout(&self, device: &RenderDevice) -> MaterialLayout {
        (self.layout)(device)
    }
}

#[derive(Debug, Clone, Default)]
pub struct MaterialRegistry {
    metas: Arc<RwLock<Vec<MaterialMeta>>>,
}

impl MaterialRegistry {
    pub fn new() -> Self {
        Self {
            metas: Arc::default(),
        }
    }

    pub fn register<M: Material>(&mut self) {
        let meta = MaterialMeta::new::<M>();
        let mut metas = self.metas.write().unwrap();
        metas.push(meta);
    }

    pub fn get(&self, ty: &MaterialType) -> Option<MaterialMeta> {
        let metas = self.metas.read().unwrap();
        metas.iter().find(|meta| &meta.ty == ty).copied()
    }

    pub fn metas(&self) -> std::sync::RwLockReadGuard<'_, Vec<MaterialMeta>> {
        self.metas.read().unwrap()
    }

    pub fn create_metas(
        _: &[()],
        device: &RenderDevice,
        registry: &Self,
        layouts: &mut RenderAssets<MaterialLayout>,
    ) {
        for meta in registry.metas.read().unwrap().iter() {
            layouts.add(meta.ty, (meta.layout)(device));
        }
    }
}

impl Resource for MaterialRegistry {}
