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
