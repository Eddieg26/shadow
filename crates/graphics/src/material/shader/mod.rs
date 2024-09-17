use crate::core::color::Color;
use std::hash::Hash;

pub mod fragment;
pub mod nodes;
pub mod snippets;
pub mod vertex;

pub const CAMERA_GROUP: u32 = 0;
pub const OBJECT_GROUP: u32 = 1;
pub const MATERIAL_GROUP: u32 = 2;
pub const CAMERA_BINDING: u32 = 0;
pub const OBJECT_BINDING: u32 = 0;

pub const FRAGMENT_INPUT_STRUCT: &str = "FragmentInput";
pub const VERTEX_OUTPUT_STRUCT: &str = "VertexOutput";
pub const VERTEX_INPUT_STRUCT: &str = "VertexInput";
pub const SURFACE_STRUCT: &str = "Surface";
pub const CAMERA_STRUCT: &str = "Camera";
pub const OBJECT_STRUCT: &str = "Object";
pub const MATERIAL_STRUCT: &str = "Material";
pub const FRAGMENT_INPUT: &str = "input";
pub const SURFACE: &str = "surface";
pub const CAMERA: &str = "camera";
pub const MATERIAL: &str = "material";
pub const OBJECT: &str = "object";
pub const VERTEX_INPUT: &str = "vertex_input";
pub const VERTEX_OUTPUT: &str = "vertex_output";

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
            VertexInput::TexCoord0 => "texcoord0",
            VertexInput::TexCoord1 => "texcoord1",
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
            VertexOutput::TexCoord0 => "texcoord0",
            VertexOutput::TexCoord1 => "texcoord1",
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
    pub fn new(code: String) -> Self {
        Self {
            code,
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
    pub fn gen() -> Self {
        let mut hasher = crc32fast::Hasher::new();
        let id = ulid::Ulid::new();
        id.hash(&mut hasher);
        NodeId(hasher.finalize())
    }
}

impl std::ops::Deref for NodeId {
    type Target = u32;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub trait ShaderNode: downcast_rs::Downcast + Send + Sync + 'static {
    fn id(&self) -> NodeId;
    fn execute(&self, inputs: &[Option<&ShaderInput>]) -> Option<NodeOutput>;
    fn inputs(&self) -> &[ShaderProperty];
    fn outputs(&self) -> &[ShaderProperty];
}
downcast_rs::impl_downcast!(ShaderNode);
