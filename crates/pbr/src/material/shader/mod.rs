use std::hash::Hash;

use graphics::{core::Color, resources::mesh::MeshAttributeKind};

pub mod constants;
pub mod fragment;
pub mod nodes;
pub mod snippets;
pub mod vertex;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShaderProperty {
    Float,
    UInt,
    SInt,
    Bool,
    Color,
    Vec2,
    Vec3,
    Vec4,
    Mat2,
    Mat3,
    Mat4,
    Texture2D,
    Texture2DArray,
    Texture3D,
    Texture3DArray,
    TextureCube,
    Sampler,
    Dynamic,
}

impl ShaderProperty {
    pub fn is_primitive(&self) -> bool {
        match self {
            ShaderProperty::Texture2D
            | ShaderProperty::Texture2DArray
            | ShaderProperty::Texture3D
            | ShaderProperty::Texture3DArray
            | ShaderProperty::TextureCube
            | ShaderProperty::Sampler
            | ShaderProperty::Dynamic => false,
            _ => true,
        }
    }

    pub fn is_scalar(&self) -> bool {
        match self {
            ShaderProperty::Float | ShaderProperty::UInt | ShaderProperty::SInt => true,
            _ => false,
        }
    }

    pub fn is_vector(&self) -> bool {
        match self {
            ShaderProperty::Vec2
            | ShaderProperty::Vec3
            | ShaderProperty::Vec4
            | ShaderProperty::Mat2
            | ShaderProperty::Mat3
            | ShaderProperty::Mat4 => true,
            _ => false,
        }
    }

    pub fn is_matrix(&self) -> bool {
        match self {
            ShaderProperty::Mat2 | ShaderProperty::Mat3 | ShaderProperty::Mat4 => true,
            _ => false,
        }
    }

    pub fn resource(&self) -> Option<ShaderResource> {
        match self {
            ShaderProperty::Texture2D => Some(ShaderResource::Texture2D),
            ShaderProperty::Texture2DArray => Some(ShaderResource::Texture2DArray),
            ShaderProperty::Texture3D => Some(ShaderResource::Texture3D),
            ShaderProperty::Texture3DArray => Some(ShaderResource::Texture3DArray),
            ShaderProperty::TextureCube => Some(ShaderResource::TextureCube),
            ShaderProperty::Sampler => Some(ShaderResource::Sampler),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub enum ShaderValue {
    Float(f32),
    UInt(u32),
    SInt(i32),
    Bool(bool),
    Color(Color),
    Vec2(glam::Vec2),
    Vec3(glam::Vec3),
    Vec4(glam::Vec4),
    Mat2(glam::Mat2),
    Mat3(glam::Mat3),
    Mat4(glam::Mat4),
}

impl ShaderValue {
    pub fn property(&self) -> ShaderProperty {
        match self {
            ShaderValue::Float(_) => ShaderProperty::Float,
            ShaderValue::UInt(_) => ShaderProperty::UInt,
            ShaderValue::SInt(_) => ShaderProperty::SInt,
            ShaderValue::Bool(_) => ShaderProperty::Bool,
            ShaderValue::Color(_) => ShaderProperty::Color,
            ShaderValue::Vec2(_) => ShaderProperty::Vec2,
            ShaderValue::Vec3(_) => ShaderProperty::Vec3,
            ShaderValue::Vec4(_) => ShaderProperty::Vec4,
            ShaderValue::Mat2(_) => ShaderProperty::Mat2,
            ShaderValue::Mat3(_) => ShaderProperty::Mat3,
            ShaderValue::Mat4(_) => ShaderProperty::Mat4,
        }
    }
}

impl From<f32> for ShaderValue {
    fn from(value: f32) -> Self {
        ShaderValue::Float(value)
    }
}

impl From<u32> for ShaderValue {
    fn from(value: u32) -> Self {
        ShaderValue::UInt(value)
    }
}

impl From<i32> for ShaderValue {
    fn from(value: i32) -> Self {
        ShaderValue::SInt(value)
    }
}

impl From<bool> for ShaderValue {
    fn from(value: bool) -> Self {
        ShaderValue::Bool(value)
    }
}

impl From<Color> for ShaderValue {
    fn from(value: Color) -> Self {
        ShaderValue::Color(value)
    }
}

impl From<glam::Vec2> for ShaderValue {
    fn from(value: glam::Vec2) -> Self {
        ShaderValue::Vec2(value)
    }
}

impl From<glam::Vec3> for ShaderValue {
    fn from(value: glam::Vec3) -> Self {
        ShaderValue::Vec3(value)
    }
}

impl From<glam::Vec4> for ShaderValue {
    fn from(value: glam::Vec4) -> Self {
        ShaderValue::Vec4(value)
    }
}

impl From<glam::Mat2> for ShaderValue {
    fn from(value: glam::Mat2) -> Self {
        ShaderValue::Mat2(value)
    }
}

impl From<glam::Mat3> for ShaderValue {
    fn from(value: glam::Mat3) -> Self {
        ShaderValue::Mat3(value)
    }
}

impl From<glam::Mat4> for ShaderValue {
    fn from(value: glam::Mat4) -> Self {
        ShaderValue::Mat4(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShaderResource {
    Texture2D,
    Texture2DArray,
    Texture3D,
    Texture3DArray,
    TextureCube,
    Sampler,
    Uniform { ty: String },
    Storage { ty: String, read_write: bool },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuiltinValue {
    VertexIndex,
    InstanceIndex,
    Position,
    FrontFacing,
    FragDepth,
    SampleIndex,
    SampleMask,
    LocalInvocationId,
    LocalInvocationIndex,
    GlobalInvocationId,
    WorkGroupId,
    NumWorkGroups,
}

impl BuiltinValue {
    pub fn to_str(&self) -> &str {
        match self {
            BuiltinValue::VertexIndex => "vertex_index",
            BuiltinValue::InstanceIndex => "instance_index",
            BuiltinValue::Position => "position",
            BuiltinValue::FrontFacing => "front_facing",
            BuiltinValue::FragDepth => "frag_depth",
            BuiltinValue::SampleIndex => "sample_index",
            BuiltinValue::SampleMask => "sample_mask",
            BuiltinValue::LocalInvocationId => "location_invocation_id",
            BuiltinValue::LocalInvocationIndex => "location_invocation_index",
            BuiltinValue::GlobalInvocationId => "global_invocation_id",
            BuiltinValue::WorkGroupId => "workgroup_id",
            BuiltinValue::NumWorkGroups => "num_workgroups",
        }
    }
}

impl ToString for BuiltinValue {
    fn to_string(&self) -> String {
        self.to_str().to_string()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShaderAttribute {
    Align(u32),
    Binding(u32),
    BlendSrc(bool),
    Builtin(BuiltinValue),
    Group(u32),
    Id(u32),
    Location(u32),
    Size(u32),
    WorkGroupSize {
        x: u32,
        y: Option<u32>,
        z: Option<u32>,
    },
}

impl ToString for ShaderAttribute {
    fn to_string(&self) -> String {
        match self {
            ShaderAttribute::Align(v) => format!("@align({}) ", v),
            ShaderAttribute::Binding(v) => format!("@binding({})", v),
            ShaderAttribute::BlendSrc(v) => format!("@blend_src({})", v),
            ShaderAttribute::Builtin(v) => format!("@builtin({})", v.to_str()),
            ShaderAttribute::Group(v) => format!("@group({})", v),
            ShaderAttribute::Id(v) => format!("@id({})", v),
            ShaderAttribute::Location(v) => format!("@location({})", v),
            ShaderAttribute::Size(v) => format!("@size({})", v),
            ShaderAttribute::WorkGroupSize { x, y, z } => {
                let y = y.unwrap_or(1);
                let z = z.unwrap_or(1);
                format!("@work_group_size({}, {}, {}", x, y, z)
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SurfaceAttribute {
    Color,
    Normal,
    Specular,
    Metallic,
    Smoothness,
    Emission,
    Opacity,
}

impl SurfaceAttribute {
    pub fn name(&self) -> &str {
        match self {
            SurfaceAttribute::Color => "color",
            SurfaceAttribute::Normal => "normal",
            SurfaceAttribute::Specular => "specular",
            SurfaceAttribute::Metallic => "metallic",
            SurfaceAttribute::Smoothness => "smoothness",
            SurfaceAttribute::Emission => "emission",
            SurfaceAttribute::Opacity => "opacity",
        }
    }

    pub fn property(&self) -> ShaderProperty {
        match self {
            SurfaceAttribute::Color => ShaderProperty::Color,
            SurfaceAttribute::Normal => ShaderProperty::Vec3,
            SurfaceAttribute::Specular => ShaderProperty::Float,
            SurfaceAttribute::Metallic => ShaderProperty::Float,
            SurfaceAttribute::Smoothness => ShaderProperty::Float,
            SurfaceAttribute::Emission => ShaderProperty::Vec3,
            SurfaceAttribute::Opacity => ShaderProperty::Float,
        }
    }

    pub fn value(&self) -> ShaderValue {
        match self {
            SurfaceAttribute::Color => ShaderValue::Color(Color::white()),
            SurfaceAttribute::Normal => ShaderValue::Vec3(glam::Vec3::ONE),
            SurfaceAttribute::Specular => ShaderValue::Float(0.0),
            SurfaceAttribute::Metallic => ShaderValue::Float(0.0),
            SurfaceAttribute::Smoothness => ShaderValue::Float(0.5),
            SurfaceAttribute::Emission => ShaderValue::Vec3(glam::Vec3::ZERO),
            SurfaceAttribute::Opacity => ShaderValue::Float(1.0),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VertexInput {
    Position,
    Normal,
    Tangent,
    TexCoord0,
    TexCoord1,
    Color,
}

impl VertexInput {
    pub fn name(&self) -> &str {
        match self {
            VertexInput::Position => "position",
            VertexInput::Normal => "normal",
            VertexInput::Tangent => "tangent",
            VertexInput::TexCoord0 => "uv_0",
            VertexInput::TexCoord1 => "uv_1",
            VertexInput::Color => "color",
        }
    }

    pub fn property(&self) -> ShaderProperty {
        match self {
            VertexInput::Position => ShaderProperty::Vec3,
            VertexInput::Normal => ShaderProperty::Vec3,
            VertexInput::Tangent => ShaderProperty::Vec4,
            VertexInput::TexCoord0 => ShaderProperty::Vec2,
            VertexInput::TexCoord1 => ShaderProperty::Vec2,
            VertexInput::Color => ShaderProperty::Color,
        }
    }

    pub fn attribute(&self) -> MeshAttributeKind {
        match self {
            VertexInput::Position => MeshAttributeKind::Position,
            VertexInput::Normal => MeshAttributeKind::Normal,
            VertexInput::Tangent => MeshAttributeKind::Tangent,
            VertexInput::TexCoord0 => MeshAttributeKind::TexCoord0,
            VertexInput::TexCoord1 => MeshAttributeKind::TexCoord1,
            VertexInput::Color => MeshAttributeKind::Color,
        }
    }

    pub fn id(&self) -> NodeId {
        NodeId::from(self)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VertexOutput {
    Position { clip: bool },
    Normal,
    Tangent,
    TexCoord0,
    TexCoord1,
    Color,
}

impl VertexOutput {
    pub fn name(&self) -> &str {
        match self {
            VertexOutput::Position { clip } => match clip {
                true => "clip_position",
                false => "position",
            },
            VertexOutput::Normal => "normal",
            VertexOutput::Tangent => "tangent",
            VertexOutput::TexCoord0 => "uv_0",
            VertexOutput::TexCoord1 => "uv_1",
            VertexOutput::Color => "color",
        }
    }

    pub fn property(&self) -> ShaderProperty {
        match self {
            VertexOutput::Position { clip } => match clip {
                true => ShaderProperty::Vec4,
                false => ShaderProperty::Vec3,
            },
            VertexOutput::Normal => ShaderProperty::Vec3,
            VertexOutput::Tangent => ShaderProperty::Vec4,
            VertexOutput::TexCoord0 => ShaderProperty::Vec2,
            VertexOutput::TexCoord1 => ShaderProperty::Vec2,
            VertexOutput::Color => ShaderProperty::Color,
        }
    }

    pub fn id(&self) -> NodeId {
        NodeId::from(self)
    }
}

#[derive(Debug, Clone)]
pub struct ShaderInput {
    pub name: String,
    pub property: ShaderProperty,
}

impl ShaderInput {
    #[inline]
    pub fn new(name: impl ToString, property: ShaderProperty) -> Self {
        Self {
            name: name.to_string(),
            property,
        }
    }

    pub fn sub(base: &str, name: &str, property: ShaderProperty) -> Self {
        Self {
            name: format!("{}_{}", base, name),
            property,
        }
    }

    pub fn child(base: &str, name: &str, property: ShaderProperty) -> Self {
        Self {
            name: format!("{}.{}", base, name),
            property,
        }
    }
}

pub type ShaderOutput = ShaderInput;

#[derive(Debug, Clone)]
pub struct ShaderField {
    pub name: String,
    pub property: ShaderProperty,
    pub attribute: Option<ShaderAttribute>,
}

impl ShaderField {
    pub fn new(name: impl ToString, property: ShaderProperty) -> Self {
        Self {
            name: name.to_string(),
            property,
            attribute: None,
        }
    }

    pub fn with_attribute(mut self, attribute: ShaderAttribute) -> Self {
        self.attribute = Some(attribute);
        self
    }
}

pub struct NodeOutput {
    pub code: String,
    pub outputs: Vec<ShaderOutput>,
}

impl NodeOutput {
    pub fn new(code: impl ToString) -> Self {
        Self {
            code: code.to_string(),
            outputs: Vec::new(),
        }
    }

    pub fn code(&self) -> &str {
        &self.code
    }

    pub fn outputs(&self) -> &[ShaderInput] {
        &self.outputs
    }

    pub fn with_output(mut self, output: ShaderOutput) -> Self {
        self.outputs.push(output);
        self
    }

    pub fn add_output(&mut self, output: ShaderOutput) -> &mut Self {
        self.outputs.push(output);
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeId(u32);

impl NodeId {
    pub fn new(id: u32) -> Self {
        NodeId(id)
    }

    pub fn gen() -> Self {
        let mut hasher = crc32fast::Hasher::new();
        let id = ulid::Ulid::new();
        id.hash(&mut hasher);
        NodeId(hasher.finalize())
    }
}

impl From<u32> for NodeId {
    fn from(id: u32) -> Self {
        NodeId(id)
    }
}

impl<H: Hash> From<&H> for NodeId {
    fn from(hash: &H) -> Self {
        let mut hasher = crc32fast::Hasher::new();
        hash.hash(&mut hasher);
        NodeId(hasher.finalize())
    }
}

impl std::ops::Deref for NodeId {
    type Target = u32;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeDependency {
    pub node: NodeId,
    pub slot: usize,
}

impl NodeDependency {
    pub fn new(node: NodeId, slot: usize) -> Self {
        Self { node, slot }
    }
}

pub trait ShaderNode: downcast_rs::Downcast + Send + Sync + 'static {
    fn id(&self) -> NodeId;
    fn execute(&self, inputs: &[Option<&ShaderInput>]) -> Option<NodeOutput>;
    fn inputs(&self) -> &[ShaderProperty];
    fn outputs(&self) -> &[ShaderProperty];
}
downcast_rs::impl_downcast!(ShaderNode);
