use super::Mesh;
use asset::{Asset, AssetId};
use std::sync::Arc;

pub struct Model {
    meshes: Vec<Arc<Mesh>>,
    materials: Vec<AssetId>,
}

impl Model {
    pub fn new() -> Self {
        Self {
            meshes: Vec::new(),
            materials: Vec::new(),
        }
    }

    pub fn meshes(&self) -> &[Arc<Mesh>] {
        &self.meshes
    }

    pub fn materials(&self) -> &[AssetId] {
        &self.materials
    }

    pub fn add_mesh(&mut self, mesh: Mesh) {
        self.meshes.push(Arc::new(mesh));
    }

    pub fn add_material(&mut self, material: AssetId) {
        self.materials.push(material);
    }
}

impl Asset for Model {}
