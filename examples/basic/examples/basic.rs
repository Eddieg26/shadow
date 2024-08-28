use std::hash::{Hash, Hasher};

use game::{
    game::Game,
    phases::{Init, Update},
    plugin::{Plugin, Plugins},
};
use graphics::{
    core::Color,
    plugin::GraphicsPlugin,
    renderer::{
        draw::{DrawCalls, Render, RenderCalls},
        graph::RenderGraphBuilder,
        nodes::render::{Attachment, RenderCommands, RenderPass, StoreOp, Subpass},
    },
    resources::material::shader::{
        nodes::{attribute::ShaderAttribute, SampleTexture2D, ShaderInput, ShaderNode},
        MaterialShader,
    },
};
use window::{events::WindowCreated, plugin::WindowPlugin};

pub struct ClearScreen {
    color: Color,
}

impl ClearScreen {
    pub fn new(color: Color) -> Self {
        Self { color }
    }
}

impl Render for ClearScreen {
    fn texture(&self) -> Option<graphics::resources::ResourceId> {
        None
    }

    fn clear_color(&self) -> Option<Color> {
        Some(self.color)
    }
}

pub struct BasicPlugin;

impl Plugin for BasicPlugin {
    fn dependencies(&self) -> Plugins {
        let mut plugins = Plugins::new();
        plugins.add_plugin(GraphicsPlugin);
        plugins
    }

    fn start(&self, game: &mut Game) {
        let builder = game.resource_mut::<RenderGraphBuilder>();
        let pass = RenderPass::new().with_color(Attachment::Surface, None, StoreOp::Store, None);

        builder.add_node("basic", pass);
    }

    fn run(&mut self, game: &mut Game) {
        game.add_system(Update, |renders: &mut RenderCalls| {
            renders.add(ClearScreen::new(Color::blue()))
        });
    }

    fn finish(&mut self, _: &mut Game) {}
}

fn main() {
    // Game::new()
    //     .add_plugin(BasicPlugin)
    //     .add_system(Init, || println!("Init"))
    //     .run();

    // let mut shader = MaterialShader::new();

    // let sampler = shader.add_node(SampleTexture2D);
    // shader.add_input("main_texture", ShaderAttribute::Texture2D);
    // shader.add_output("color", ShaderAttribute::Color);
    // shader.add_edge(("main_texture", (sampler, SampleTexture2D::TEXTURE)));
    // shader.add_edge(((sampler, SampleTexture2D::RGBA), ("color", ())));

    // let output = shader.build();

    // std::fs::write("node_output.wgsl", output.unwrap()).unwrap();

    let mut shader = MaterialShader::new();
    let sampler = shader.add_node(SampleTexture2D);
    shader.add_input("uv", ShaderAttribute::Vec2);
    shader.add_input("main_texture", ShaderAttribute::Texture2D);
    shader.add_input("main_sampler", ShaderAttribute::Sampler);
    shader.add_output("color", ShaderAttribute::Color);
    shader.add_edge(("main_texture", (sampler, SampleTexture2D::TEXTURE)));
    shader.add_edge(("main_sampler", (sampler, SampleTexture2D::SAMPLER)));
    shader.add_edge(("uv", (sampler, SampleTexture2D::UV)));
    shader.add_edge(((sampler, SampleTexture2D::RGBA), ("color", ())));

    let output = shader.build();

    std::fs::write("node_output.wgsl", output.unwrap()).unwrap();
}
