use glam::Vec2;

use super::{
    attribute::{ShaderAttribute, ShaderValue},
    snippets::Snippets,
    ShaderInput, ShaderOutput,
};

#[derive(Clone, Debug, PartialEq)]
pub struct ShaderNodeOutput {
    code: String,
    outputs: Vec<ShaderOutput>,
}

impl ShaderNodeOutput {
    pub fn new(code: impl ToString, outputs: Vec<ShaderOutput>) -> Self {
        Self {
            code: code.to_string(),
            outputs,
        }
    }

    pub fn empty() -> Self {
        Self {
            code: String::new(),
            outputs: Vec::new(),
        }
    }

    pub fn code(&self) -> &str {
        &self.code
    }

    pub fn outputs(&self) -> &[ShaderOutput] {
        &self.outputs
    }

    pub fn output(&self, index: usize) -> ShaderInput {
        self.outputs[index].clone()
    }

    pub fn output_by_name(&self, name: &str) -> Option<ShaderInput> {
        self.outputs.iter().find(|o| o.name() == name).cloned()
    }
}

pub trait ShaderNode: downcast_rs::Downcast + 'static {
    fn execute(&self, inputs: &[Option<ShaderInput>]) -> Option<ShaderNodeOutput>;
    fn inputs(&self) -> &[ShaderAttribute];
    fn outputs(&self) -> &[ShaderAttribute];
}

downcast_rs::impl_downcast!(ShaderNode);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum NodeId {
    Id(usize),
    Input,
    Output,
}

impl NodeId {
    pub fn id(&self) -> Option<usize> {
        match self {
            NodeId::Id(id) => Some(*id),
            NodeId::Input | NodeId::Output => None,
        }
    }

    pub fn node(&self) -> bool {
        matches!(self, NodeId::Id(_))
    }

    pub fn input(&self) -> bool {
        matches!(self, NodeId::Input)
    }

    pub fn output(&self) -> bool {
        matches!(self, NodeId::Output)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Slot {
    id: NodeId,
    slot: usize,
}

impl Slot {
    pub fn new(id: NodeId, slot: usize) -> Self {
        Self { id, slot }
    }

    pub fn id(&self) -> NodeId {
        self.id
    }

    pub fn slot(&self) -> usize {
        self.slot
    }
}

impl From<(NodeId, usize)> for Slot {
    fn from((id, slot): (NodeId, usize)) -> Self {
        Slot::new(id, slot)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum SlotType {
    Node { id: NodeId, slot: usize },
    Input { name: String },
    Output { name: String },
}

impl SlotType {
    pub fn node(id: NodeId, slot: usize) -> Self {
        SlotType::Node { id, slot }
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

    pub fn id(&self) -> NodeId {
        match self {
            SlotType::Node { id, .. } => *id,
            SlotType::Input { .. } => NodeId::Input,
            SlotType::Output { .. } => NodeId::Output,
        }
    }

    pub fn slot(&self) -> Option<usize> {
        match self {
            SlotType::Node { slot, .. } => Some(*slot),
            SlotType::Input { .. } => None,
            SlotType::Output { .. } => None,
        }
    }

    pub fn name(&self) -> Option<&str> {
        match self {
            SlotType::Node { .. } => None,
            SlotType::Input { name } => Some(name),
            SlotType::Output { name } => Some(name),
        }
    }
}

impl From<(NodeId, usize)> for SlotType {
    fn from((id, slot): (NodeId, usize)) -> Self {
        SlotType::Node { id, slot }
    }
}

impl From<(&str, ())> for SlotType {
    fn from((name, ()): (&str, ())) -> Self {
        SlotType::Output {
            name: name.to_string(),
        }
    }
}

impl From<&str> for SlotType {
    fn from(name: &str) -> Self {
        SlotType::Input {
            name: name.to_string(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShaderEdge {
    from: SlotType,
    to: SlotType,
}

impl ShaderEdge {
    pub fn new(from: impl Into<SlotType>, to: impl Into<SlotType>) -> Self {
        Self {
            from: from.into(),
            to: to.into(),
        }
    }

    pub fn from(&self) -> &SlotType {
        &self.from
    }

    pub fn to(&self) -> &SlotType {
        &self.to
    }
}

impl<A: Into<SlotType>, B: Into<SlotType>> From<(A, B)> for ShaderEdge {
    fn from((from, to): (A, B)) -> Self {
        let mut from: SlotType = from.into();
        let mut to: SlotType = to.into();

        from = match from {
            SlotType::Node { id, slot } => SlotType::Node { id, slot },
            SlotType::Input { name } => SlotType::Input { name },
            SlotType::Output { name } => SlotType::Input { name },
        };

        to = match to {
            SlotType::Node { id, slot } => SlotType::Node { id, slot },
            SlotType::Input { name } => SlotType::Input { name },
            SlotType::Output { name } => SlotType::Input { name },
        };

        ShaderEdge::new(from, to)
    }
}

pub struct ShaderInputNode;

impl ShaderInputNode {
    pub fn execute(inputs: &[ShaderInput]) -> Option<ShaderNodeOutput> {
        let mut fields = vec![];
        let mut bindings = vec![];
        let mut outputs = vec![];

        for input in inputs {
            match input.attribute().is_buffer_value() {
                true => {
                    let name = format!("material.{}", input.name());
                    outputs.push(input.with_name(name));
                    fields.push(input);
                }
                false => {
                    outputs.push(input.clone());
                    bindings.push(input);
                }
            }
        }

        let _struct = match fields.is_empty() {
            true => String::new(),
            false => Snippets::define_struct("Material", &fields)?,
        };

        let _bindings = match bindings.is_empty() {
            true => String::new(),
            false => Snippets::define_bindings(0, 0, &bindings)?,
        };

        let buffer = match fields.is_empty() {
            true => String::new(),
            false => Snippets::define_buffer("material", "Material", 0, bindings.len())?,
        };

        let code = format!("{}\n{}\n{}", _struct, _bindings, buffer,);
        Some(ShaderNodeOutput::new(code, outputs))
    }
}

pub struct SampleTexture2D;

impl SampleTexture2D {
    pub const TEXTURE: usize = 0;
    pub const SAMPLER: usize = 1;
    pub const UV: usize = 2;

    pub const RGBA: usize = 0;
    pub const R: usize = 1;
    pub const G: usize = 2;
    pub const B: usize = 3;
    pub const A: usize = 4;
}

impl ShaderNode for SampleTexture2D {
    fn execute(&self, inputs: &[Option<ShaderInput>]) -> Option<ShaderNodeOutput> {
        let name = format!("sample_texture2d_{}", ulid::Ulid::new().to_string());
        let rgba = ShaderOutput::field(&name, 0, ShaderAttribute::Color);
        let r = ShaderOutput::field(&name, 1, ShaderAttribute::Float);
        let g = ShaderOutput::field(&name, 2, ShaderAttribute::Float);
        let b = ShaderOutput::field(&name, 3, ShaderAttribute::Float);
        let a = ShaderOutput::field(name, 4, ShaderAttribute::Float);

        let texture = inputs[Self::TEXTURE].as_ref()?;
        let sampler = inputs[Self::SAMPLER].as_ref()?;

        let uv = match inputs[Self::UV].as_ref() {
            Some(uv) => Snippets::convert(uv.name(), uv.attribute(), ShaderAttribute::Vec2)?,
            None => Snippets::shader_value(&ShaderValue::Vec2(Vec2::new(0.0, 0.0)))?,
        };

        let code = format!(
            r#"
            {rgba} = textureSample({tex}, {sam}, {uv});
            {r} = {rgba_name}.r;
            {g} = {rgba_name}.g;
            {b} = {rgba_name}.b;
            {a} = {rgba_name}.a;
            "#,
            rgba = Snippets::define_variable(rgba.name(), rgba.attribute())?,
            rgba_name = rgba.name(),
            tex = texture.name(),
            sam = sampler.name(),
            uv = uv,
            r = Snippets::define_variable(r.name(), ShaderAttribute::Float)?,
            g = Snippets::define_variable(g.name(), ShaderAttribute::Float)?,
            b = Snippets::define_variable(b.name(), ShaderAttribute::Float)?,
            a = Snippets::define_variable(a.name(), ShaderAttribute::Float)?,
        );

        Some(ShaderNodeOutput::new(code, vec![rgba, r, g, b, a]))
    }

    fn inputs(&self) -> &[ShaderAttribute] {
        &[
            ShaderAttribute::Texture2D,
            ShaderAttribute::Vec2,
            ShaderAttribute::Sampler,
        ]
    }

    fn outputs(&self) -> &[ShaderAttribute] {
        &[
            ShaderAttribute::Color,
            ShaderAttribute::Float,
            ShaderAttribute::Float,
            ShaderAttribute::Float,
            ShaderAttribute::Float,
        ]
    }
}

pub struct Or;

impl Or {
    pub const A: usize = 0;
    pub const B: usize = 1;
    pub const OUT: usize = 0;
}

impl ShaderNode for Or {
    fn execute(&self, inputs: &[Option<ShaderInput>]) -> Option<ShaderNodeOutput> {
        let name = format!("or_{}", ulid::Ulid::new().to_string());
        let out = ShaderOutput::field(name, 0, ShaderAttribute::Bool);
        let left = inputs[0].as_ref()?;
        let right = inputs[1].as_ref()?;

        let code = format!(
            "{out} = {a} || {b};",
            out = Snippets::define_variable(out.name(), out.attribute())?,
            a = Snippets::convert(left.name(), left.attribute(), ShaderAttribute::Bool)?,
            b = Snippets::convert(right.name(), right.attribute(), ShaderAttribute::Bool)?,
        );

        let output = ShaderNodeOutput::new(code, vec![out]);
        Some(output)
    }

    fn inputs(&self) -> &[ShaderAttribute] {
        &[ShaderAttribute::Bool, ShaderAttribute::Bool]
    }

    fn outputs(&self) -> &[ShaderAttribute] {
        &[ShaderAttribute::Bool]
    }
}

pub struct And;

impl And {
    pub const A: usize = 0;
    pub const B: usize = 1;
    pub const OUT: usize = 0;
}

impl ShaderNode for And {
    fn execute(&self, inputs: &[Option<ShaderInput>]) -> Option<ShaderNodeOutput> {
        let name = format!("and{}", ulid::Ulid::new().to_string());
        let and = ShaderOutput::field(name, 0, ShaderAttribute::Bool);
        let left = inputs[0].as_ref()?;
        let right = inputs[1].as_ref()?;

        let code = format!(
            "{and} = {a} && {b};",
            and = Snippets::define_variable(and.name(), and.attribute())?,
            a = Snippets::convert(left.name(), left.attribute(), ShaderAttribute::Bool)?,
            b = Snippets::convert(right.name(), right.attribute(), ShaderAttribute::Bool)?,
        );

        let output = ShaderNodeOutput::new(code, vec![and]);
        Some(output)
    }

    fn inputs(&self) -> &[ShaderAttribute] {
        &[ShaderAttribute::Bool, ShaderAttribute::Bool]
    }

    fn outputs(&self) -> &[ShaderAttribute] {
        &[ShaderAttribute::Bool]
    }
}

pub struct Branch;

impl Branch {
    pub const CONDITION: usize = 0;
    pub const TRUE: usize = 1;
    pub const FALSE: usize = 2;
    pub const OUT: usize = 0;
}

impl ShaderNode for Branch {
    fn execute(&self, inputs: &[Option<ShaderInput>]) -> Option<ShaderNodeOutput> {
        let name = format!("branch_{}", ulid::Ulid::new().to_string());
        let out = ShaderOutput::field(name, 0, inputs[1].as_ref()?.attribute());
        let cond = inputs[0].as_ref()?;
        let left = inputs[1].as_ref()?;
        let right = inputs[2].as_ref()?;

        let code = format!(
            "{out} = {cond} ? {left} : {right};",
            out = Snippets::define_variable(out.name(), out.attribute())?,
            cond = Snippets::convert(cond.name(), cond.attribute(), ShaderAttribute::Bool)?,
            left = left.name(),
            right = Snippets::convert(right.name(), right.attribute(), out.attribute())?,
        );

        let output = ShaderNodeOutput::new(code, vec![out]);
        Some(output)
    }

    fn inputs(&self) -> &[ShaderAttribute] {
        &[
            ShaderAttribute::Bool,
            ShaderAttribute::Dynamic,
            ShaderAttribute::Dynamic,
        ]
    }

    fn outputs(&self) -> &[ShaderAttribute] {
        &[ShaderAttribute::Dynamic]
    }
}
