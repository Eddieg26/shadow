use super::BlendMode;

pub struct MaterialShader {
    mode: BlendMode,
    inputs: Vec<ShaderInput>,
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

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum ShaderOutput {
    Color,
    Normal,
    Specular,
    Metallic,
    Roughness,
    Emissive,
    Opacity,
}

impl ShaderOutput {
    pub fn value(&self) -> ShaderValue {
        match self {
            Self::Color => ShaderValue::Color,
            Self::Normal => ShaderValue::Vec3F,
            Self::Specular => ShaderValue::Float,
            Self::Metallic => ShaderValue::Float,
            Self::Roughness => ShaderValue::Float,
            Self::Emissive => ShaderValue::Vec3F,
            Self::Opacity => ShaderValue::Float,
        }
    }
}

pub struct ShaderInput {
    name: String,
    value: ShaderValue,
}

pub struct NodeValue {
    name: String,
    value: ShaderValue,
}

impl NodeValue {
    pub fn input(name: &str, value: ShaderValue) -> Self {
        Self {
            name: name.to_string(),
            value,
        }
    }

    pub fn output(node: &str, index: usize, value: ShaderValue) -> Self {
        Self {
            name: format!("{}_{}", node, index),
            value,
        }
    }

    pub fn name(node: &str, index: usize) -> String {
        format!("{}_{}", node, index)
    }
}

pub type NodeInput = NodeValue;
pub type NodeOutput = NodeValue;

pub struct SampleTexture2D {
    name: String,
}

impl ShaderNode for SampleTexture2D {
    fn name(&self) -> &str {
        &self.name
    }

    fn execute(&self, inputs: &[NodeValue]) -> String {
        format!(
            r#"
            vec4 {color} = texture({input0}, {input1});
            float {r} = {color}.r;
            float {g} = {color}.g;
            float {b} = {color}.b;
            float {a} = {color}.a;
            "#,
            input0 = inputs[0].name,
            input1 = inputs[1].name,
            color = self.output(0),
            r = self.output(1),
            g = self.output(2),
            b = self.output(3),
            a = self.output(4),
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
            ShaderValue::Vec4F,
            ShaderValue::Float,
            ShaderValue::Float,
            ShaderValue::Float,
            ShaderValue::Float,
        ]
    }
}

pub struct ShaderTexture2D {
    name: String,
}

impl ShaderNode for ShaderTexture2D {
    fn name(&self) -> &str {
        &self.name
    }

    fn execute(&self, _: &[NodeValue]) -> String {
        format!(
            r#"
                @group(0) @binding(1)
                var {name}_texture: texture_2d<f32>;
            "#,
            name = &self.name
        )
    }

    fn inputs(&self) -> &[ShaderValue] {
        &[]
    }

    fn outputs(&self) -> &[ShaderValue] {
        &[ShaderValue::Texture2D]
    }
}

pub struct ShaderOutputNode {
    outputs: Vec<ShaderOutput>,
    inputs: Vec<ShaderValue>,
}

impl ShaderOutputNode {
    pub fn new() -> Self {
        Self {
            outputs: Vec::new(),
            inputs: Vec::new(),
        }
    }

    pub fn add(&mut self, output: ShaderOutput) {
        if !self.outputs.contains(&output) {
            self.outputs.push(output);
            self.inputs.push(output.value());
        }
    }

    pub fn remove(&mut self, output: ShaderOutput) {
        if let Some(index) = self.outputs.iter().position(|o| *o == output) {
            self.outputs.remove(index);
            self.inputs.remove(index);
        }
    }

    pub fn remove_at(&mut self, index: usize) {
        if index < self.outputs.len() {
            self.outputs.remove(index);
            self.inputs.remove(index);
        }
    }

    pub fn outputs(&self) -> &[ShaderOutput] {
        &self.outputs
    }
}

impl ShaderNode for ShaderOutputNode {
    fn name(&self) -> &str {
        "output"
    }

    fn execute(&self, _: &[NodeValue]) -> String {
        let code = String::new();
        code
    }

    fn inputs(&self) -> &[ShaderValue] {
        &self.inputs
    }

    fn outputs(&self) -> &[ShaderValue] {
        &[]
    }
}

pub trait ShaderNode: 'static {
    fn name(&self) -> &str;
    fn execute(&self, inputs: &[NodeValue]) -> String;
    fn inputs(&self) -> &[ShaderValue];
    fn outputs(&self) -> &[ShaderValue];
    fn output(&self, index: usize) -> String {
        NodeValue::name(self.name(), index)
    }
}
