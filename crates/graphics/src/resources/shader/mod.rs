pub mod attribute;
pub mod material;
pub mod nodes;
pub mod snippets;

use attribute::ShaderAttribute;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum ShaderModel {
    Unlit = 0,
    Lit = 1,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum BlendMode {
    Opaque = 0,
    Translucent = 1,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ShaderInput {
    name: String,
    attribute: ShaderAttribute,
}

impl ShaderInput {
    pub fn new(name: impl ToString, attribute: ShaderAttribute) -> Self {
        Self {
            name: name.to_string(),
            attribute,
        }
    }

    pub fn field(name: impl ToString, index: usize, attribute: ShaderAttribute) -> Self {
        Self::new(format!("{}_{}", name.to_string(), index), attribute)
    }

    pub fn new_checked(name: impl ToString, attribute: ShaderAttribute) -> Option<Self> {
        match attribute {
            ShaderAttribute::Dynamic => None,
            _ => Some(Self {
                name: name.to_string(),
                attribute,
            }),
        }
    }

    pub fn field_checked(
        name: impl ToString,
        index: usize,
        attribute: ShaderAttribute,
    ) -> Option<Self> {
        Self::new_checked(format!("{}.{}", name.to_string(), index), attribute)
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn attribute(&self) -> ShaderAttribute {
        self.attribute
    }

    pub fn with_name(&self, name: impl ToString) -> Self {
        let mut input = self.clone();
        input.name = name.to_string();
        input
    }
}

pub type ShaderOutput = ShaderInput;
