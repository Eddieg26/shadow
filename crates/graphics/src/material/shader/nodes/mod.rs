use super::{
    snippets::{convert_input, declare_value, surface_field},
    NodeId, NodeOutput, ShaderInput, ShaderNode, ShaderProperty, ShaderValue, SurfaceAttribute,
};
use glam::Vec2;

pub struct Texture2DSampler {
    id: NodeId,
    name: String,
}

impl Texture2DSampler {
    pub const TEXTURE: usize = 0;
    pub const UV: usize = 1;
    pub const SAMPLER: usize = 2;

    pub const RGBA: usize = 0;
    pub const R: usize = 1;
    pub const G: usize = 2;
    pub const B: usize = 3;
    pub const A: usize = 4;

    pub fn new(name: impl ToString) -> Self {
        Self {
            id: NodeId::gen(),
            name: name.to_string(),
        }
    }
}

impl Default for Texture2DSampler {
    fn default() -> Self {
        let id = NodeId::gen();
        Self {
            id,
            name: format!("texture2D_sampler_{}", *id),
        }
    }
}

impl ShaderNode for Texture2DSampler {
    fn id(&self) -> NodeId {
        self.id
    }

    fn execute(&self, inputs: &[Option<&ShaderInput>]) -> Option<NodeOutput> {
        let texture = inputs[Self::TEXTURE]?;
        let uv = match inputs[Self::UV].and_then(|i| convert_input(i, ShaderProperty::Vec2)) {
            Some(uv) => uv,
            None => declare_value(&ShaderValue::Vec2(Vec2::ZERO)),
        };
        let sampler = inputs[Self::SAMPLER]?;
        println!("texture: {:?}", texture);

        let rgba = ShaderInput::sub(&self.name, "rgba", ShaderProperty::Color);
        let r = ShaderInput::sub(&self.name, "r", ShaderProperty::Float);
        let g = ShaderInput::sub(&self.name, "g", ShaderProperty::Float);
        let b = ShaderInput::sub(&self.name, "b", ShaderProperty::Float);
        let a = ShaderInput::sub(&self.name, "a", ShaderProperty::Float);

        let code = format!(
            r#"
                let {rgba} = textureSample({texture}, {sampler}, {uv});
                let {r} = {rgba}.r;
                let {g} = {rgba}.g;
                let {b} = {rgba}.b;
                let {a} = {rgba}.a;
            "#,
            rgba = rgba.name,
            r = r.name,
            g = g.name,
            b = b.name,
            a = a.name,
            texture = texture.name,
            sampler = sampler.name,
            uv = uv,
        );

        let output = NodeOutput::new(code)
            .with_output(rgba)
            .with_output(r)
            .with_output(g)
            .with_output(b)
            .with_output(a);

        Some(output)
    }

    fn inputs(&self) -> &[ShaderProperty] {
        &[
            ShaderProperty::Texture2D,
            ShaderProperty::Vec2,
            ShaderProperty::Sampler,
        ]
    }

    fn outputs(&self) -> &[ShaderProperty] {
        &[
            ShaderProperty::Color,
            ShaderProperty::Float,
            ShaderProperty::Float,
            ShaderProperty::Float,
            ShaderProperty::Float,
        ]
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SurfaceNode {
    id: NodeId,
    attribute: SurfaceAttribute,
    name: String,
}

impl SurfaceNode {
    pub fn new(attribute: SurfaceAttribute) -> Self {
        Self {
            id: NodeId::gen(),
            attribute,
            name: surface_field(attribute),
        }
    }
}

impl ShaderNode for SurfaceNode {
    fn id(&self) -> NodeId {
        self.id
    }

    fn execute(&self, inputs: &[Option<&ShaderInput>]) -> Option<NodeOutput> {
        let input = match inputs.first() {
            Some(Some(input)) => convert_input(&input, self.attribute.property())
                .unwrap_or(declare_value(&self.attribute.value())),
            _ => declare_value(&self.attribute.value()),
        };

        let code = format!("{} = {};", self.name, input);
        let output = NodeOutput::new(code);
        Some(output)
    }

    fn inputs(&self) -> &[ShaderProperty] {
        &[ShaderProperty::Dynamic]
    }

    fn outputs(&self) -> &[ShaderProperty] {
        &[ShaderProperty::Dynamic]
    }
}
