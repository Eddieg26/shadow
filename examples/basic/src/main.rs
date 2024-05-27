use shadow_game::game::{
    plugin::Plugin,
    schedule::{Execute, Main, PostUpdate, Start, Update},
    Game,
};

pub struct TestPluginA;

impl Plugin for TestPluginA {
    fn start(&mut self, ctx: &mut shadow_game::game::plugin::PluginContext) {
        println!("TestPluginA::start");
    }

    fn run(&mut self, ctx: &mut shadow_game::game::plugin::PluginContext) {
        println!("TestPluginA::run");
    }

    fn finish(&mut self, ctx: &mut shadow_game::game::plugin::PluginContext) {
        println!("TestPluginA::finish");
    }
}

pub struct TestPluginB;

impl Plugin for TestPluginB {
    fn dependencies(&self) -> shadow_game::game::plugin::Plugins {
        let mut plugins = shadow_game::game::plugin::Plugins::new();
        plugins.add_plugin(TestPluginA);
        plugins
    }

    fn start(&mut self, ctx: &mut shadow_game::game::plugin::PluginContext) {
        println!("TestPluginB::start");
    }

    fn run(&mut self, ctx: &mut shadow_game::game::plugin::PluginContext) {
        println!("TestPluginB::run");
    }

    fn finish(&mut self, ctx: &mut shadow_game::game::plugin::PluginContext) {
        println!("TestPluginB::finish");
    }
}

fn main() {
    let mut game = Game::new();
    game.add_plugin(TestPluginB);

    game.add_system(Start, || println!("Hello, World!"));
    game.add_system(Update, || println!("Update, World!"));
    game.add_system(PostUpdate, || println!("Goodbye, World!"));

    game.run();
}
