use shadow_asset::asset::Asset;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum ShaderModel {
    Lit = 0,
    Unlit = 1,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum BlendMode {
    Opaque = 0,
    Translucent = 1,
}

pub trait Material: Asset + Clone + Sized {
    fn shader_model(&self) -> ShaderModel;
    fn blend_mode(&self) -> BlendMode;
}
