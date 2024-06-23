use shadow_asset::{
    asset::{Asset, BasicSettings},
    bytes::ToBytes,
    loader::{AssetLoader, AssetPipeline, AssetSaver, BasicProcessor, LoadContextType},
};
use shadow_game::plugin::{Plugin, PluginContext, Plugins};

pub struct TestPluginA;

impl Plugin for TestPluginA {
    fn start(&mut self, ctx: &mut PluginContext) {
        println!("TestPluginA::start");
    }

    fn run(&mut self, ctx: &mut PluginContext) {
        println!("TestPluginA::run");
    }

    fn finish(&mut self, ctx: &mut PluginContext) {
        println!("TestPluginA::finish");
    }
}

pub struct TestPluginB;

impl Plugin for TestPluginB {
    fn dependencies(&self) -> Plugins {
        let mut plugins = Plugins::new();
        plugins.add_plugin(TestPluginA);
        plugins
    }

    fn start(&mut self, ctx: &mut PluginContext) {
        println!("TestPluginB::start");
    }

    fn run(&mut self, ctx: &mut PluginContext) {
        println!("TestPluginB::run");
    }

    fn finish(&mut self, ctx: &mut PluginContext) {
        println!("TestPluginB::finish");
    }
}

pub struct PlainText {
    pub text: String,
}

impl PlainText {
    pub fn new(text: &str) -> Self {
        Self {
            text: text.to_string(),
        }
    }
}

impl ToBytes for PlainText {
    fn to_bytes(&self) -> Vec<u8> {
        self.text.as_bytes().to_vec()
    }

    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        std::str::from_utf8(bytes).ok().map(|text| Self::new(text))
    }
}

impl Asset for PlainText {}

impl AssetLoader for PlainText {
    type Asset = PlainText;
    type Settings = BasicSettings;

    fn load(
        ctx: &mut shadow_asset::loader::LoadContext<Self::Settings>,
    ) -> Result<PlainText, String> {
        match ctx.ty() {
            LoadContextType::Processed { bytes } => {
                let text = std::str::from_utf8(bytes).map_err(|e| e.to_string())?;
                Ok(PlainText {
                    text: text.to_string(),
                })
            }
            LoadContextType::UnProcessed { bytes, .. } => {
                let text = std::str::from_utf8(bytes).map_err(|e| e.to_string())?;
                Ok(PlainText {
                    text: text.to_string(),
                })
            }
        }
    }

    fn extensions() -> &'static [&'static str] {
        &["txt"]
    }
}

impl AssetSaver for PlainText {
    type Asset = PlainText;
    type Settings = BasicSettings;

    fn save(asset: &PlainText) -> &[u8] {
        asset.text.as_bytes()
    }
}

impl AssetPipeline for PlainText {
    type Asset = Self;
    type Settings = BasicSettings;
    type Loader = Self;
    type Saver = Self;
    type Processor = BasicProcessor<Self>;
    type PostProcessor = BasicProcessor<Self>;
}
