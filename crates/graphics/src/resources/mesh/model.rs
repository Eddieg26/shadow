use asset::AssetId;

pub struct Model {
    meshes: Vec<AssetId>,
}

impl Model {
    pub fn new() -> Self {
        Self { meshes: Vec::new() }
    }

    pub fn with_mesh(mut self, mesh: AssetId) -> Self {
        self.meshes.push(mesh);
        self
    }

    pub fn meshes(&self) -> &[AssetId] {
        &self.meshes
    }
}
