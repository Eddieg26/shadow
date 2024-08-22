use super::BlendMode;
use shadow_ecs::core::DenseMap;

pub struct MaterialShader {
    mode: BlendMode,
    inputs: Vec<ShaderInput>,
    outputs: Vec<ShaderOutput>,
    nodes: DenseMap<String, Box<dyn ShaderNode>>,
    edges: Vec<ShaderEdge>,
}

impl MaterialShader {
    pub fn new() -> Self {
        Self {
            mode: BlendMode::Opaque,
            inputs: Vec::new(),
            outputs: Vec::new(),
            nodes: DenseMap::new(),
            edges: Vec::new(),
        }
    }

    pub fn blend_mode(&self) -> BlendMode {
        self.mode
    }

    pub fn outputs(&self) -> &[ShaderOutput] {
        &self.outputs
    }

    pub fn inputs(&self) -> &[ShaderInput] {
        &self.inputs
    }

    pub fn node<T: ShaderNode>(&self, name: &String) -> Option<&T> {
        self.nodes.get(name).and_then(|node| node.downcast_ref())
    }

    pub fn node_mut<T: ShaderNode>(&mut self, name: &String) -> Option<&mut T> {
        self.nodes
            .get_mut(name)
            .and_then(|node| node.downcast_mut())
    }

    pub fn node_dyn(&self, name: &String) -> Option<&dyn ShaderNode> {
        self.nodes.get(name).map(|node| node.as_ref())
    }

    pub fn node_mut_dyn(&mut self, name: &String) -> Option<&mut dyn ShaderNode> {
        self.nodes.get_mut(name).map(|node| node.as_mut())
    }

    pub fn add_input(&mut self, name: &str, value: ShaderValue) -> &mut Self {
        self.inputs.push(ShaderInput::new(name, value));
        self
    }

    pub fn add_output(&mut self, name: &str, value: ShaderValue) -> &mut Self {
        self.outputs.push(ShaderOutput::new(name, value));
        self
    }

    pub fn add_node<T: ShaderNode>(&mut self, node: T) -> &mut Self {
        self.nodes.insert(node.name().to_string(), Box::new(node));
        self
    }

    pub fn remove_node(&mut self, name: &String) -> Option<Box<dyn ShaderNode>> {
        let removed = self.nodes.remove(name)?;
        self.edges
            .retain(|e| e.from().name() != name && e.to().name() != name);
        Some(removed)
    }

    pub fn add_edge(&mut self, edge: ShaderEdge) -> &mut Self {
        self.edges.push(edge);
        self
    }

    pub fn remove_edge(&mut self, edge: &ShaderEdge) {
        self.edges.retain(|e| e != edge);
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum ShaderValue {
    Float,
    UInt,
    SInt,
    Vec2F,
    Vec3F,
    Vec4F,
    Vec2U,
    Vec3U,
    Vec4U,
    Vec2I,
    Vec3I,
    Vec4I,
    Mat2,
    Mat3,
    Mat4,
    Color,
    Bool,
    Texture2D,
    Texture2DArray,
    Texture3D,
    Texture3DArray,
    Cubemap,
    Sampler,
}

pub struct ShaderInput {
    name: String,
    value: ShaderValue,
}

impl ShaderInput {
    pub fn new(name: &str, value: ShaderValue) -> Self {
        Self {
            name: name.to_string(),
            value,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn value(&self) -> ShaderValue {
        self.value
    }
}

pub type ShaderOutput = ShaderInput;

pub struct SampleTexture2D {
    name: String,
}

impl SampleTexture2D {
    pub const TEXTURE: usize = 0;
    pub const UV: usize = 1;
    pub const SAMPLER: usize = 2;

    pub fn new(name: &str) -> Self {
        SampleTexture2D {
            name: name.to_string(),
        }
    }

    pub fn create() -> Self {
        SampleTexture2D {
            name: format!("sample_texture2d_{}", ulid::Ulid::new().to_string()),
        }
    }
}

impl ShaderNode for SampleTexture2D {
    fn name(&self) -> &str {
        &self.name
    }

    fn execute(&self, inputs: &[ShaderNodeInput]) -> String {
        format!(
            r#"
            vec4 {0} = texture({1}, {2});
            float {3} = {0}.r;
            float {4} = {0}.g;
            float {5} = {0}.b;
            float {6} = {0}.a;
            "#,
            self.output(0),
            inputs[0].name(),
            inputs[1].name(),
            self.output(1),
            self.output(2),
            self.output(3),
            self.output(4),
        )
    }

    fn inputs(&self) -> &[ShaderValue] {
        &[
            ShaderValue::Texture2D,
            ShaderValue::Vec2F,
            ShaderValue::Sampler,
        ]
    }

    fn outputs(&self) -> &[ShaderValue] {
        &[
            ShaderValue::Color,
            ShaderValue::Float,
            ShaderValue::Float,
            ShaderValue::Float,
            ShaderValue::Float,
        ]
    }
}

pub struct ShaderNodeInput {
    name: String,
    value: ShaderValue,
}

impl ShaderNodeInput {
    pub fn new(name: &str, value: ShaderValue) -> Self {
        Self {
            name: name.to_string(),
            value,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn value(&self) -> ShaderValue {
        self.value
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SlotType {
    Node { name: String, slot: usize },
    Input { name: String },
    Output { name: String },
}

impl SlotType {
    pub fn node(name: &str, slot: usize) -> Self {
        SlotType::Node {
            name: name.to_string(),
            slot,
        }
    }

    pub fn input(name: &str) -> Self {
        SlotType::Input {
            name: name.to_string(),
        }
    }

    pub fn output(name: &str) -> Self {
        SlotType::Output {
            name: name.to_string(),
        }
    }

    pub fn name(&self) -> &str {
        match self {
            SlotType::Node { name, .. } => name,
            SlotType::Input { name } => name,
            SlotType::Output { name } => name,
        }
    }

    pub fn slot(&self) -> Option<usize> {
        match self {
            SlotType::Node { slot, .. } => Some(*slot),
            SlotType::Input { .. } => None,
            SlotType::Output { .. } => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShaderEdge {
    from: SlotType,
    to: SlotType,
}

impl ShaderEdge {
    pub fn new(from: SlotType, to: SlotType) -> Self {
        Self { from, to }
    }

    pub fn from(&self) -> &SlotType {
        &self.from
    }

    pub fn to(&self) -> &SlotType {
        &self.to
    }
}

pub trait ShaderNode: downcast_rs::Downcast + 'static {
    fn name(&self) -> &str;
    fn execute(&self, inputs: &[ShaderNodeInput]) -> String;
    fn inputs(&self) -> &[ShaderValue];
    fn outputs(&self) -> &[ShaderValue];
    fn output(&self, index: usize) -> String {
        format!("{}_{}", self.name(), index)
    }
}

downcast_rs::impl_downcast!(ShaderNode);

fn ex() {
    let mut shader = MaterialShader::new();

    shader.add_input("main_texture", ShaderValue::Texture2D);
    shader.add_node(SampleTexture2D::new("main_sampler"));
    shader.add_output("color", ShaderValue::Color);
    shader.add_edge(ShaderEdge::new(
        SlotType::input("main_texture"),
        SlotType::node("main_sampler", SampleTexture2D::TEXTURE),
    ));
    shader.add_edge(ShaderEdge::new(
        SlotType::node("main_sampler", 0),
        SlotType::output("color"),
    ));
}
