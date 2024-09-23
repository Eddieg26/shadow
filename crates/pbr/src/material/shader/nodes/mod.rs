use super::{
    constants::{CAMERA, OBJECT},
    snippets::{convert_input, declare_value, surface_field},
    NodeId, NodeOutput, ShaderInput, ShaderNode, ShaderOutput, ShaderProperty, ShaderValue,
    SurfaceAttribute,
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

pub struct CameraNode {
    id: NodeId,
}

impl CameraNode {
    pub const VIEW: usize = 0;
    pub const PROJECTION: usize = 1;
    pub const POSITION: usize = 2;

    pub fn new() -> Self {
        Self { id: NodeId::gen() }
    }
}

impl ShaderNode for CameraNode {
    fn id(&self) -> NodeId {
        self.id
    }

    fn execute(&self, _: &[Option<&ShaderInput>]) -> Option<NodeOutput> {
        let view = ShaderOutput::child(CAMERA, "view", ShaderProperty::Mat4);
        let projection = ShaderOutput::child(CAMERA, "projection", ShaderProperty::Mat4);
        let position = ShaderOutput::child(CAMERA, "world", ShaderProperty::Vec3);

        let output = NodeOutput::new("")
            .with_output(view)
            .with_output(projection)
            .with_output(position);

        Some(output)
    }

    fn inputs(&self) -> &[ShaderProperty] {
        &[]
    }

    fn outputs(&self) -> &[ShaderProperty] {
        &[
            ShaderProperty::Mat4,
            ShaderProperty::Mat4,
            ShaderProperty::Vec3,
        ]
    }
}

pub struct ObjectModelNode {
    id: NodeId,
}

impl ObjectModelNode {
    pub const WORLD: usize = 0;

    pub fn new() -> Self {
        Self { id: NodeId::gen() }
    }
}

impl ShaderNode for ObjectModelNode {
    fn id(&self) -> NodeId {
        self.id
    }

    fn execute(&self, _: &[Option<&ShaderInput>]) -> Option<NodeOutput> {
        let world = ShaderOutput::child(OBJECT, "world", ShaderProperty::Mat4);
        let output = NodeOutput::new("").with_output(world);
        Some(output)
    }

    fn inputs(&self) -> &[ShaderProperty] {
        &[]
    }

    fn outputs(&self) -> &[ShaderProperty] {
        &[ShaderProperty::Mat4]
    }
}

pub struct AddNode {
    id: NodeId,
    name: String,
}

impl AddNode {
    pub fn new() -> Self {
        let id = NodeId::gen();
        Self {
            id,
            name: format!("add_{:?}", *id),
        }
    }
}

impl ShaderNode for AddNode {
    fn id(&self) -> NodeId {
        self.id
    }

    fn execute(&self, inputs: &[Option<&ShaderInput>]) -> Option<NodeOutput> {
        let left = inputs[0]?;
        let right = inputs[1]?;

        let code = format!(
            "let {} = {} + {};",
            self.name,
            left.name,
            convert_input(right, left.property)?
        );

        let output =
            NodeOutput::new(code).with_output(ShaderOutput::new(&self.name, left.property));
        Some(output)
    }

    fn inputs(&self) -> &[ShaderProperty] {
        &[ShaderProperty::Dynamic, ShaderProperty::Dynamic]
    }

    fn outputs(&self) -> &[ShaderProperty] {
        &[ShaderProperty::Dynamic]
    }
}

pub struct SubtractNode {
    id: NodeId,
    name: String,
}

impl SubtractNode {
    pub fn new() -> Self {
        let id = NodeId::gen();
        Self {
            id,
            name: format!("subtract_{:?}", *id),
        }
    }
}

impl ShaderNode for SubtractNode {
    fn id(&self) -> NodeId {
        self.id
    }

    fn execute(&self, inputs: &[Option<&ShaderInput>]) -> Option<NodeOutput> {
        let left = inputs[0]?;
        let right = inputs[1]?;

        let code = format!(
            "let {} = {} - {};",
            self.name,
            left.name,
            convert_input(right, left.property)?
        );

        let output =
            NodeOutput::new(code).with_output(ShaderOutput::new(&self.name, left.property));
        Some(output)
    }

    fn inputs(&self) -> &[ShaderProperty] {
        &[ShaderProperty::Dynamic, ShaderProperty::Dynamic]
    }

    fn outputs(&self) -> &[ShaderProperty] {
        &[ShaderProperty::Dynamic]
    }
}

pub struct MultiplyNode {
    id: NodeId,
    name: String,
}

impl MultiplyNode {
    pub const LEFT: usize = 0;
    pub const RIGHT: usize = 1;

    pub const OUTPUT: usize = 0;

    pub fn new() -> Self {
        let id = NodeId::gen();
        Self {
            id,
            name: format!("multiply_{:?}", *id),
        }
    }

    fn output_property(left: ShaderProperty, right: ShaderProperty) -> Option<ShaderProperty> {
        if left == right {
            return Some(left);
        }
        match (left, right) {
            (ShaderProperty::Vec2, ShaderProperty::Float) => Some(ShaderProperty::Vec2),
            (ShaderProperty::Vec3, ShaderProperty::Float) => Some(ShaderProperty::Vec3),
            (ShaderProperty::Vec4, ShaderProperty::Float) => Some(ShaderProperty::Vec4),
            (ShaderProperty::Mat2, ShaderProperty::Float) => Some(ShaderProperty::Mat2),
            (ShaderProperty::Mat3, ShaderProperty::Float) => Some(ShaderProperty::Mat3),
            (ShaderProperty::Mat4, ShaderProperty::Float) => Some(ShaderProperty::Mat4),
            (ShaderProperty::Mat2, ShaderProperty::Vec2) => Some(ShaderProperty::Vec2),
            (ShaderProperty::Mat3, ShaderProperty::Vec3) => Some(ShaderProperty::Vec3),
            (ShaderProperty::Mat4, ShaderProperty::Vec4) => Some(ShaderProperty::Vec4),
            (ShaderProperty::Mat4, ShaderProperty::Color) => Some(ShaderProperty::Color),
            (ShaderProperty::Float, ShaderProperty::Vec2) => Some(ShaderProperty::Vec2),
            (ShaderProperty::Float, ShaderProperty::Vec3) => Some(ShaderProperty::Vec3),
            (ShaderProperty::Float, ShaderProperty::Vec4) => Some(ShaderProperty::Vec4),
            (ShaderProperty::Float, ShaderProperty::Color) => Some(ShaderProperty::Color),
            (ShaderProperty::Float, ShaderProperty::Mat4) => Some(ShaderProperty::Mat4),
            (ShaderProperty::Float, ShaderProperty::Mat2) => Some(ShaderProperty::Mat2),
            (ShaderProperty::Float, ShaderProperty::Mat3) => Some(ShaderProperty::Mat3),
            (ShaderProperty::Vec4, ShaderProperty::Color) => Some(ShaderProperty::Vec4),
            (ShaderProperty::Color, ShaderProperty::Vec4) => Some(ShaderProperty::Color),
            _ => None,
        }
    }
}

impl ShaderNode for MultiplyNode {
    fn id(&self) -> NodeId {
        self.id
    }

    fn execute(&self, inputs: &[Option<&ShaderInput>]) -> Option<NodeOutput> {
        let left = inputs[0]?;
        let right = inputs[1]?;

        let code = format!("let {} = {} * {};", self.name, left.name, right.name);

        let output = NodeOutput::new(code).with_output(ShaderOutput::new(
            &self.name,
            Self::output_property(left.property, right.property)?,
        ));
        Some(output)
    }

    fn inputs(&self) -> &[ShaderProperty] {
        &[ShaderProperty::Dynamic, ShaderProperty::Dynamic]
    }

    fn outputs(&self) -> &[ShaderProperty] {
        &[ShaderProperty::Dynamic]
    }
}

pub struct DivideNode {
    id: NodeId,
    name: String,
}

impl DivideNode {
    pub fn new() -> Self {
        let id = NodeId::gen();

        Self {
            id,
            name: format!("divide_{:?}", *id),
        }
    }
}

impl ShaderNode for DivideNode {
    fn id(&self) -> NodeId {
        self.id
    }

    fn execute(&self, inputs: &[Option<&ShaderInput>]) -> Option<NodeOutput> {
        let left = inputs[0]?;
        let right = inputs[1]?;

        let code = format!(
            "let {} = {} / {};",
            self.name,
            left.name,
            convert_input(right, left.property)?
        );

        let output =
            NodeOutput::new(code).with_output(ShaderOutput::new(&self.name, left.property));
        Some(output)
    }

    fn inputs(&self) -> &[ShaderProperty] {
        &[ShaderProperty::Dynamic, ShaderProperty::Dynamic]
    }

    fn outputs(&self) -> &[ShaderProperty] {
        &[ShaderProperty::Dynamic]
    }
}

pub struct DotNode {
    id: NodeId,
    name: String,
}

impl DotNode {
    pub fn new() -> Self {
        let id = NodeId::gen();

        Self {
            id,
            name: format!("dot_{:?}", *id),
        }
    }
}

impl ShaderNode for DotNode {
    fn id(&self) -> NodeId {
        self.id
    }

    fn execute(&self, inputs: &[Option<&ShaderInput>]) -> Option<NodeOutput> {
        let left = inputs[0]?;
        let right = inputs[1]?;

        if !left.property.is_vector() || !right.property.is_vector() {
            return None;
        }

        let code = format!(
            "let {} = dot({}, {});",
            self.name,
            left.name,
            convert_input(right, left.property)?
        );

        let output =
            NodeOutput::new(code).with_output(ShaderOutput::new(&self.name, ShaderProperty::Float));
        Some(output)
    }

    fn inputs(&self) -> &[ShaderProperty] {
        &[ShaderProperty::Dynamic, ShaderProperty::Dynamic]
    }

    fn outputs(&self) -> &[ShaderProperty] {
        &[ShaderProperty::Float]
    }
}

pub struct CrossNode {
    id: NodeId,
    name: String,
}

impl CrossNode {
    pub fn new() -> Self {
        let id = NodeId::gen();

        Self {
            id,
            name: format!("cross_{:?}", *id),
        }
    }
}

impl ShaderNode for CrossNode {
    fn id(&self) -> NodeId {
        self.id
    }

    fn execute(&self, inputs: &[Option<&ShaderInput>]) -> Option<NodeOutput> {
        let left = inputs[0]?;
        let right = inputs[1]?;

        if !left.property.is_vector() || !right.property.is_vector() {
            return None;
        }

        let code = format!(
            "let {} = cross({}, {});",
            self.name,
            left.name,
            convert_input(right, left.property)?
        );

        let output =
            NodeOutput::new(code).with_output(ShaderOutput::new(&self.name, ShaderProperty::Vec3));
        Some(output)
    }

    fn inputs(&self) -> &[ShaderProperty] {
        &[ShaderProperty::Dynamic, ShaderProperty::Dynamic]
    }

    fn outputs(&self) -> &[ShaderProperty] {
        &[ShaderProperty::Vec3]
    }
}

pub struct ConvertNode {
    id: NodeId,
    name: String,
    property: ShaderProperty,
}

impl ConvertNode {
    pub const INPUT: usize = 0;
    pub const OUTPUT: usize = 0;

    pub fn new(property: ShaderProperty) -> Self {
        let id = NodeId::gen();

        Self {
            id,
            name: format!("convert_{}", *id),
            property,
        }
    }
}

impl ShaderNode for ConvertNode {
    fn id(&self) -> NodeId {
        self.id
    }

    fn execute(&self, inputs: &[Option<&ShaderInput>]) -> Option<NodeOutput> {
        let input = inputs[0]?;
        let code = format!(
            "let {} = {};",
            self.name,
            convert_input(input, self.property)?
        );

        let output =
            NodeOutput::new(code).with_output(ShaderOutput::new(&self.name, self.property));
        Some(output)
    }

    fn inputs(&self) -> &[ShaderProperty] {
        &[ShaderProperty::Dynamic]
    }

    fn outputs(&self) -> &[ShaderProperty] {
        &[ShaderProperty::Dynamic]
    }
}

pub struct ShaderValueNode {
    id: NodeId,
    name: String,
    value: ShaderValue,
}

impl ShaderValueNode {
    pub const OUTPUT: usize = 0;
    
    pub fn new(value: impl Into<ShaderValue>) -> Self {
        let id = NodeId::gen();
        Self {
            id,
            name: format!("value_{}", *id),
            value: value.into(),
        }
    }
}

impl ShaderNode for ShaderValueNode {
    fn id(&self) -> NodeId {
        self.id
    }

    fn execute(&self, _: &[Option<&ShaderInput>]) -> Option<NodeOutput> {
        let code = format!("let {} = {};", self.name, declare_value(&self.value));
        let output =
            NodeOutput::new(code).with_output(ShaderOutput::new(&self.name, self.value.property()));
        Some(output)
    }

    fn inputs(&self) -> &[ShaderProperty] {
        &[]
    }

    fn outputs(&self) -> &[ShaderProperty] {
        &[ShaderProperty::Dynamic]
    }
}
