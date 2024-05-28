use shadow_asset::{asset::Asset, bytes::AsBytes, loader::AssetLoader};
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

impl AsBytes for PlainText {
    fn as_bytes(&self) -> Vec<u8> {
        self.text.as_bytes().to_vec()
    }

    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        std::str::from_utf8(bytes).ok().map(|text| Self::new(text))
    }
}

impl Asset for PlainText {}

impl AssetLoader for PlainText {
    type Asset = PlainText;
    type Settings = ();

    fn load(
        ctx: &mut shadow_asset::loader::LoadContext<Self::Settings>,
    ) -> std::io::Result<Self::Asset> {
        let path = ctx.path();
        let text = std::fs::read_to_string(path)?;
        Ok(PlainText::new(&text))
    }

    fn extensions() -> &'static [&'static str] {
        &["txt"]
    }
}
