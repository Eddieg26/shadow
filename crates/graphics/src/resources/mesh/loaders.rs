use super::{MeshAttribute, MeshAttributeKind};
use crate::resources::{
    buffer::Indices,
    mesh::{Mesh, MeshTopology, SubMesh},
    ReadWrite,
};
use asset::{
    importer::{AssetImporter, ImportContext},
    io::AssetReader,
    Settings,
};
use glam::{Vec2, Vec3};

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct MeshLoadSettings {
    read_write: ReadWrite,
}

impl Settings for MeshLoadSettings {}
pub struct ObjImporter;

impl AssetImporter for ObjImporter {
    type Asset = Mesh;
    type Settings = MeshLoadSettings;
    type Error = tobj::LoadError;

    fn import(
        ctx: &mut ImportContext<Self::Settings>,
        reader: &mut dyn AssetReader,
    ) -> Result<Self::Asset, Self::Error> {
        let mut obj_reader = reader
            .buf_reader()
            .map_err(|_| tobj::LoadError::OpenFileFailed)?;

        let (models, _) = tobj::load_obj_buf(
            &mut obj_reader,
            &tobj::LoadOptions {
                triangulate: true,
                ..Default::default()
            },
            |path| {
                let reader = ctx.config().reader(path);
                let mut buf = reader
                    .buf_reader()
                    .map_err(|_| tobj::LoadError::OpenFileFailed)?;
                tobj::load_mtl_buf(&mut buf)
            },
        )?;

        let mut mesh =
            Mesh::new(MeshTopology::TriangleList).with_read_write(ctx.settings().read_write);

        for model in models {
            let start_vertex = mesh.vertex_count() as u32;
            let start_index = mesh.index_count() as u32;

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
                .texcoord_indices
                .chunks(3)
                .map(|chunk| {
                    let u = model.mesh.texcoords[chunk[0] as usize];
                    let v = model.mesh.texcoords[chunk[1] as usize];
                    Vec2::new(u, v)
                })
                .collect::<Vec<_>>();

            let indices = model.mesh.indices.iter().map(|i| *i).collect::<Vec<_>>();

            if !positions.is_empty() {
                match mesh.attribute_mut(MeshAttributeKind::Position) {
                    Some(attribute) => attribute.extend(&mut MeshAttribute::Position(positions)),
                    None => mesh.add_attribute(MeshAttribute::Position(positions)),
                }
            }

            if !normals.is_empty() {
                match mesh.attribute_mut(MeshAttributeKind::Normal) {
                    Some(attribute) => attribute.extend(&mut MeshAttribute::Normal(normals)),
                    None => mesh.add_attribute(MeshAttribute::Normal(normals)),
                }
            }

            if !uvs.is_empty() {
                match mesh.attribute_mut(MeshAttributeKind::TexCoord0) {
                    Some(attribute) => attribute.extend(&mut MeshAttribute::TexCoord0(uvs)),
                    None => mesh.add_attribute(MeshAttribute::TexCoord0(uvs)),
                }
            }

            if !indices.is_empty() {
                mesh.add_indices(Indices::U32(indices));
            }

            let vertex_count = mesh.vertex_count() as u32 - start_vertex;
            let index_count = mesh.index_count() as u32 - start_index;

            let sub_mesh = SubMesh::new(start_vertex, vertex_count, start_index, index_count);
            ctx.add_sub_asset(&model.name, sub_mesh);
            mesh.add_sub_mesh(sub_mesh);
        }

        Ok(mesh)
    }

    fn extensions() -> &'static [&'static str] {
        &["obj"]
    }
}
