use super::BlendMode;
use nodes::{
    attribute::ShaderAttribute, NodeId, SampleTexture2D, ShaderEdge, ShaderInput, ShaderInputNode,
    ShaderNode, ShaderOutput, Slot, SlotType,
};
use shadow_ecs::core::DenseMap;
use slab::Slab;
use std::collections::{HashMap, HashSet};

pub mod nodes;

pub struct MaterialShader {
    mode: BlendMode,
    inputs: Vec<ShaderInput>,
    outputs: Vec<ShaderOutput>,
    nodes: Slab<Box<dyn ShaderNode>>,
    edges: Vec<ShaderEdge>,
}

impl MaterialShader {
    pub fn new() -> Self {
        Self {
            mode: BlendMode::Opaque,
            inputs: Vec::new(),
            outputs: Vec::new(),
            nodes: Slab::new(),
            edges: Vec::new(),
        }
    }

    pub fn blend_mode(&self) -> BlendMode {
        self.mode
    }

    pub fn outputs(&self) -> &[ShaderOutput] {
        &self.outputs
    }

    pub fn inputs(&self) -> &[ShaderInput] {
        &self.inputs
    }

    pub fn node<T: ShaderNode>(&self, id: NodeId) -> Option<&T> {
        let id = id.id()?;
        self.nodes.get(id).and_then(|node| node.downcast_ref())
    }

    pub fn node_mut<T: ShaderNode>(&mut self, id: NodeId) -> Option<&mut T> {
        let id = id.id()?;
        self.nodes.get_mut(id).and_then(|node| node.downcast_mut())
    }

    pub fn node_dyn(&self, id: NodeId) -> Option<&dyn ShaderNode> {
        let id = id.id()?;
        self.nodes.get(id).map(|node| node.as_ref())
    }

    pub fn node_mut_dyn(&mut self, id: NodeId) -> Option<&mut dyn ShaderNode> {
        let id = id.id()?;
        self.nodes.get_mut(id).map(|node| node.as_mut())
    }

    pub fn add_input(&mut self, name: &str, attribute: ShaderAttribute) {
        if let Some(input) = ShaderInput::new_checked(name, attribute) {
            self.inputs.push(input);
        }
    }

    pub fn remove_input(&mut self, name: &str) -> Option<ShaderAttribute> {
        let index = self.inputs.iter().position(|input| input.name() == name)?;
        self.remove_edges(SlotType::input(name));
        Some(self.inputs.remove(index).attribute())
    }

    pub fn add_output(&mut self, name: &str, attribute: ShaderAttribute) {
        if let Some(output) = ShaderOutput::new_checked(name, attribute) {
            self.outputs.push(output);
        }
    }

    pub fn remove_output(&mut self, name: &str) -> Option<ShaderAttribute> {
        let index = self.outputs.iter().position(|o| o.name() == name)?;
        self.remove_edges(SlotType::output(name));
        Some(self.outputs.remove(index).attribute())
    }

    pub fn add_node<T: ShaderNode>(&mut self, node: T) -> NodeId {
        let id = self.nodes.insert(Box::new(node));
        NodeId::Id(id)
    }

    pub fn remove_node(&mut self, node: NodeId) -> Option<Box<dyn ShaderNode>> {
        let id = node.id()?;
        let removed = match self.nodes.contains(id) {
            true => self.nodes.remove(id),
            false => return None,
        };

        self.edges
            .retain(|e| e.from().id() != node && e.to().id() != node);
        Some(removed)
    }

    pub fn add_edge(&mut self, edge: impl Into<ShaderEdge>) {
        self.edges.push(edge.into());
    }

    pub fn remove_edge(&mut self, edge: &ShaderEdge) {
        self.edges.retain(|e| e != edge);
    }

    pub fn remove_edges(&mut self, slot: SlotType) {
        match &slot {
            SlotType::Node { id, .. } => self
                .edges
                .retain(|e| e.from().id() != *id && e.to().id() != *id),
            SlotType::Input { name } | SlotType::Output { name } => self
                .edges
                .retain(|e| e.from().name() != Some(name) && e.to().name() != Some(name)),
        }
    }

    fn input_slot(&self, name: &str) -> Option<Slot> {
        let index = self.inputs.iter().position(|input| input.name() == name)?;
        Some(Slot::new(NodeId::Input, index))
    }

    fn output_slot(&self, name: &str) -> Option<Slot> {
        let index = self
            .outputs
            .iter()
            .position(|output| output.name() == name)?;
        Some(Slot::new(NodeId::Output, index))
    }

    pub fn build(self) -> Option<String> {
        let mut dependencies: HashMap<NodeId, Vec<NodeId>> = HashMap::new();
        let mut connections = HashMap::new();
        for edge in &self.edges {
            let from = match edge.from() {
                SlotType::Node { id, slot } => Slot::new(*id, *slot),
                SlotType::Input { name } => self.input_slot(&name)?,
                SlotType::Output { .. } => continue,
            };

            let to = match edge.to() {
                SlotType::Node { id, slot } => Slot::new(*id, *slot),
                SlotType::Output { name } => self.output_slot(name)?,
                SlotType::Input { .. } => continue,
            };

            dependencies.entry(to.id()).or_default().push(from.id());
            connections.insert(to, from);
        }

        let mut sorted = vec![];
        let mut visited = HashSet::<NodeId>::new();
        while visited.len() < self.nodes.len() {
            let mut added = vec![];
            for (id, _) in self.nodes.iter() {
                let node = NodeId::Id(id);
                match dependencies.get(&node) {
                    Some(deps) => {
                        if deps.iter().all(|dep| !dep.node() || !visited.contains(dep)) {
                            added.push(id)
                        }
                    }
                    None => added.push(id),
                };
            }

            if added.is_empty() {
                break;
            }

            visited.extend(added.iter().map(|id| NodeId::Id(*id)));
            sorted.extend(added);
        }

        let mut outputs = DenseMap::new();
        outputs.insert(NodeId::Input, ShaderInputNode::execute(&self.inputs)?);

        for id in sorted {
            let node = self.nodes.get(id)?;
            let mut inputs = vec![];
            for index in 0..node.inputs().len() {
                let to = Slot::new(NodeId::Id(id), index);
                let from = connections.get(&to);
                match from {
                    Some(from) => match outputs.get(&from.id()) {
                        Some(output) => inputs.push(Some(output.output(from.slot()))),
                        None => inputs.push(None),
                    },
                    None => inputs.push(None),
                }
            }

            let output = node.execute(&inputs)?;
            outputs.insert(NodeId::Id(id), output);
        }

        let mut code = String::new();
        for output in outputs.values() {
            code = format!("{}\n{}", code, output.code());
        }

        //TODO: Run Internal Nodes
        //TODO: Run Output Node

        Some(code)
    }
}

fn ex() {
    let mut shader = MaterialShader::new();

    let sampler = shader.add_node(SampleTexture2D);
    shader.add_input("main_texture", ShaderAttribute::Texture2D);
    shader.add_output("color", ShaderAttribute::Color);
    shader.add_edge(ShaderEdge::new(
        SlotType::input("main_texture"),
        SlotType::node(sampler, SampleTexture2D::TEXTURE),
    ));
    shader.add_edge(ShaderEdge::new(
        SlotType::node(sampler, SampleTexture2D::RGBA),
        SlotType::output("color"),
    ));
}
