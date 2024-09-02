use asset::{
    importer::{AssetImporter, ImportContext},
    io::AssetReader,
    Asset, AssetId, Settings,
};
use glam::{Vec2, Vec3};

use crate::{
    core::VertexAttributeValues,
    resources::{
        mesh::{Mesh, MeshTopology},
        ReadWrite,
    },
};

#[derive(serde::Serialize, serde::Deserialize)]
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

    pub fn with_meshes(mut self, meshes: Vec<AssetId>) -> Self {
        self.meshes = meshes;
        self
    }

    pub fn meshes(&self) -> &[AssetId] {
        &self.meshes
    }
}

impl Asset for Model {}

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct MeshLoadSettings {
    read_write: ReadWrite,
}

impl Settings for MeshLoadSettings {}

pub struct ObjLoader;

impl AssetImporter for ObjLoader {
    type Asset = Model;
    type Settings = MeshLoadSettings;
    type Error = tobj::LoadError;

    fn import(
        ctx: &mut ImportContext<Self::Settings>,
        reader: &mut dyn AssetReader,
    ) -> Result<Self::Asset, Self::Error> {
        let mut obj_reader = reader
            .buf_reader()
            .map_err(|_| tobj::LoadError::OpenFileFailed)?;

        let (models, _) =
            tobj::load_obj_buf(&mut obj_reader, &tobj::LoadOptions::default(), |path| {
                let reader = ctx.config().reader(path);
                let mut buf = reader
                    .buf_reader()
                    .map_err(|_| tobj::LoadError::OpenFileFailed)?;
                tobj::load_mtl_buf(&mut buf)
            })?;

        let mut meshes = vec![];
        for model in models {
            println!("Name: {}", model.name);
            let positions = model
                .mesh
                .positions
                .chunks(3)
                .map(|chunk| Vec3::new(chunk[0], chunk[1], chunk[2]))
                .collect::<Vec<_>>();

            let normals = model
                .mesh
                .normal_indices
                .chunks(3)
                .map(|chunk| {
                    let n0 = model.mesh.normals[chunk[0] as usize];
                    let n1 = model.mesh.normals[chunk[1] as usize];
                    let n2 = model.mesh.normals[chunk[2] as usize];
                    Vec3::new(n0, n1, n2)
                })
                .collect::<Vec<_>>();

            let uvs = model
                .mesh
                .normal_indices
                .chunks(3)
                .map(|chunk| {
                    let u = model.mesh.normals[chunk[0] as usize];
                    let v = model.mesh.normals[chunk[1] as usize];
                    Vec2::new(u, v)
                })
                .collect::<Vec<_>>();

            let mut mesh = Mesh::new(MeshTopology::TriangleList, ctx.settings().read_write);
            if !positions.is_empty() {
                mesh.add_attribute(VertexAttributeValues::Position(positions));
            }

            if !normals.is_empty() {
                mesh.add_attribute(VertexAttributeValues::Normal(normals));
            }

            if !uvs.is_empty() {
                mesh.add_attribute(VertexAttributeValues::TexCoord0(uvs));
            }

            meshes.push(ctx.add_sub_asset(&model.name, mesh));
        }

        Ok(Model::new().with_meshes(meshes))
    }

    fn extensions() -> &'static [&'static str] {
        &["obj"]
    }
}
