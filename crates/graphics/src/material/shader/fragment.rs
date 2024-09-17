use super::{
    snippets::{self},
    NodeId, ShaderInput, ShaderNode, ShaderOutput, ShaderProperty, SurfaceAttribute,
    CAMERA_BINDING, CAMERA_GROUP, FRAGMENT_INPUT_STRUCT, MATERIAL_GROUP, SURFACE,
};
use crate::{
    material::{BlendMode, Material, ShaderModel},
    resources::shader::ShaderSource,
};
use std::collections::{BTreeMap, HashMap, HashSet};

pub struct MaterialShader {
    mode: BlendMode,
    model: ShaderModel,
    inputs: Vec<ShaderInput>,
    nodes: Vec<Box<dyn ShaderNode>>,
    edges: Vec<Edge>,
}

impl MaterialShader {
    pub fn new<M: Material>() -> Self {
        Self {
            mode: M::mode(),
            model: M::model(),
            inputs: vec![],
            nodes: vec![],
            edges: vec![],
        }
    }

    pub fn mode(&self) -> BlendMode {
        self.mode
    }

    pub fn model(&self) -> ShaderModel {
        self.model
    }

    pub fn inputs(&self) -> &[ShaderInput] {
        &self.inputs
    }

    pub fn nodes(&self) -> &[Box<dyn ShaderNode>] {
        &self.nodes
    }

    pub fn edges(&self) -> &[Edge] {
        &self.edges
    }

    pub fn get_node(&self, id: NodeId) -> Option<&dyn ShaderNode> {
        self.nodes
            .iter()
            .find(|node| node.id() == id)
            .map(|node| node.as_ref())
    }

    pub fn add_input(&mut self, name: &str, property: ShaderProperty) -> &mut Self {
        self.inputs.push(ShaderInput::new(name, property));
        self
    }

    pub fn add_node(&mut self, node: impl ShaderNode) -> NodeId {
        let id = node.id();
        match self.node_index(id) {
            Some(index) => self.nodes[index] = Box::new(node),
            None => self.nodes.push(Box::new(node)),
        };

        id
    }

    pub fn add_edge(&mut self, edge: impl Into<Edge>) -> &mut Self {
        let edge = edge.into();
        match self.edges.iter().position(|e| e == &edge) {
            Some(index) => self.edges[index] = edge,
            None => self.edges.push(edge),
        }

        self
    }

    pub fn remove_input(&mut self, name: &str) -> Option<ShaderInput> {
        let index = self.inputs.iter().position(|input| &input.name == name)?;
        self.remove_input_edges(name);
        Some(self.inputs.remove(index))
    }

    pub fn remove_node(&mut self, id: NodeId) -> Option<Box<dyn ShaderNode>> {
        let index = self.nodes.iter().position(|node| node.id() == id)?;
        self.remove_node_edges(self.nodes[index].id());
        Some(self.nodes.remove(index))
    }

    pub fn remove_edge(&mut self, from: &EdgeSlot, to: &EdgeSlot) -> Option<Edge> {
        let index = self
            .edges
            .iter()
            .position(|edge| edge.from() == from && edge.to() == to)?;
        Some(self.edges.remove(index))
    }

    fn remove_node_edges(&mut self, id: NodeId) {
        let mut index = 0;
        while index < self.edges.len() {
            if self.edges[index].from().id() == Some(id) || self.edges[index].to().id() == Some(id)
            {
                self.edges.remove(index);
            } else {
                index += 1;
            }
        }
    }

    fn remove_input_edges(&mut self, name: &str) -> Vec<Edge> {
        let mut edges = vec![];
        let mut index = 0;
        while index < self.edges.len() {
            if self.edges[index].from().name() == Some(name)
                || self.edges[index].to().name() == Some(name)
            {
                edges.push(self.edges.remove(index));
            } else {
                index += 1;
            }
        }

        edges
    }

    pub fn node_index(&self, id: NodeId) -> Option<usize> {
        self.nodes.iter().position(|node| node.id() == id)
    }

    pub fn generate(&self) -> ShaderSource {
        let mut outputs: HashMap<EdgeSlot, ShaderOutput> = HashMap::new();
        for input in self.inputs() {
            let output = snippets::material_input(input);
            outputs.insert(EdgeSlot::input(&input.name), output);
        }

        let mut definitions = String::new();
        definitions += &snippets::define_camera(CAMERA_GROUP, CAMERA_BINDING);
        definitions += &snippets::define_material(MATERIAL_GROUP, self.inputs());
        definitions += &snippets::define_surface(self.model);
        definitions += &snippets::define_fragment_input(FRAGMENT_INPUT_STRUCT);

        let mut body = String::new();
        body += &snippets::declare_surface(self.model);

        let (node_inputs, surface_inputs) = self.get_order();

        for (index, inputs) in node_inputs {
            let node = self.nodes[index].as_ref();
            let inputs = inputs
                .iter()
                .map(|input| match input {
                    Some(input) => outputs.get(*input),
                    None => None,
                })
                .collect::<Vec<_>>();

            let mut output = match node.execute(&inputs) {
                Some(output) => output,
                None => continue,
            };

            body += &output.code;

            for (slot, output) in output.outputs.drain(..).enumerate() {
                outputs.insert(EdgeSlot::node(node.id(), slot), output);
            }
        }

        for (attribute, input) in surface_inputs {
            let output = match outputs.get(input) {
                Some(output) => output,
                None => continue,
            };

            let value = match snippets::convert_input(output, attribute.property()) {
                Some(v) => v,
                None => continue,
            };

            let code = format!("{}.{} = {}", SURFACE, attribute.name(), value);
            body += &code;
        }

        let fragment_body = snippets::define_fragment_body(body, self.mode);
        let source = format!("{}{}", definitions, fragment_body);

        ShaderSource::Wgsl(source.into())
    }

    fn get_order(
        &self,
    ) -> (
        Vec<(usize, Box<[Option<&EdgeSlot>]>)>,
        Vec<(&SurfaceAttribute, &EdgeSlot)>,
    ) {
        let mut dependencies = HashMap::new();
        let mut outputs = HashSet::new();

        for edge in self.edges() {
            let to = match edge.to() {
                EdgeSlot::Node { .. } => edge.to(),
                EdgeSlot::Output { .. } => {
                    outputs.insert(edge.to());
                    edge.to()
                }
                _ => continue,
            };

            let from = match edge.from() {
                EdgeSlot::Node { .. } => edge.from(),
                EdgeSlot::Input { .. } => edge.from(),
                _ => continue,
            };

            dependencies.entry(to).or_insert(from);
        }

        self.purge_unsused_nodes(&mut dependencies, &outputs);

        let mut inputs = BTreeMap::new();
        let mut outputs: Vec<(&SurfaceAttribute, &EdgeSlot)> = vec![];
        while !dependencies.is_empty() {
            let mut next = vec![];
            for (slot, input) in dependencies.iter() {
                if !dependencies.contains_key(input) {
                    next.push(*slot);
                }
            }

            for to in next {
                let from = match dependencies.remove(to) {
                    Some(from) => from,
                    None => continue,
                };

                match to {
                    EdgeSlot::Node { id, slot } => {
                        if let Some(index) = self.node_index(*id) {
                            let node = &self.nodes[index];
                            let inputs = inputs
                                .entry(index)
                                .or_insert(vec![None; node.inputs().len()]);
                            inputs[*slot] = Some(from);
                        }
                    }
                    EdgeSlot::Output { output } => {
                        outputs.push((output, from));
                    }
                    _ => continue,
                }
            }
        }

        let inputs = inputs
            .into_iter()
            .map(|(index, inputs)| (index, inputs.into()))
            .collect();

        (inputs, outputs)
    }

    fn purge_unsused_nodes(
        &self,
        dependencies: &mut HashMap<&EdgeSlot, &EdgeSlot>,
        outputs: &HashSet<&EdgeSlot>,
    ) {
        let mut marked = HashSet::new();
        for output in outputs {
            marked.extend(self.mark_used_nodes(output, dependencies));
        }

        dependencies.retain(|slot, _| match slot.id() {
            Some(id) => marked.contains(&id),
            None => true,
        });
    }

    fn mark_used_nodes<'a, 'b>(
        &'a self,
        slot: &'a EdgeSlot,
        dependencies: &'b HashMap<&EdgeSlot, &'a EdgeSlot>,
    ) -> HashSet<NodeId> {
        let mut marked = HashSet::new();
        if let Some(id) = slot.id() {
            marked.insert(id);
        }
        match dependencies.get(slot) {
            Some(slot) => {
                marked.extend(self.mark_used_nodes(slot, dependencies));
            }
            None => (),
        }

        marked
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum EdgeSlot {
    Node { id: NodeId, slot: usize },
    Input { name: String },
    Output { output: SurfaceAttribute },
}

impl EdgeSlot {
    pub fn node(id: NodeId, slot: usize) -> Self {
        Self::Node { id, slot }
    }

    pub fn input(name: &str) -> Self {
        Self::Input {
            name: name.to_string(),
        }
    }

    pub fn output(output: SurfaceAttribute) -> Self {
        Self::Output { output }
    }

    pub fn id(&self) -> Option<NodeId> {
        match self {
            Self::Node { id, .. } => Some(*id),
            _ => None,
        }
    }

    pub fn name(&self) -> Option<&str> {
        match self {
            Self::Input { name } => Some(name),
            _ => None,
        }
    }

    pub fn slot(&self) -> usize {
        match self {
            Self::Node { slot, .. } => *slot,
            _ => 0,
        }
    }

    pub fn attribute(&self) -> Option<SurfaceAttribute> {
        match self {
            Self::Output { output } => Some(*output),
            _ => None,
        }
    }
}

impl From<(NodeId, usize)> for EdgeSlot {
    fn from((id, slot): (NodeId, usize)) -> Self {
        Self::node(id, slot)
    }
}

impl From<&str> for EdgeSlot {
    fn from(name: &str) -> Self {
        Self::input(name)
    }
}

impl From<SurfaceAttribute> for EdgeSlot {
    fn from(output: SurfaceAttribute) -> Self {
        Self::output(output)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Edge {
    from: EdgeSlot,
    to: EdgeSlot,
}

impl Edge {
    pub fn new(from: EdgeSlot, to: EdgeSlot) -> Self {
        Self { from, to }
    }

    pub fn from(&self) -> &EdgeSlot {
        &self.from
    }

    pub fn to(&self) -> &EdgeSlot {
        &self.to
    }
}

impl<A: Into<EdgeSlot>, B: Into<EdgeSlot>> From<(A, B)> for Edge {
    fn from((from, to): (A, B)) -> Self {
        Self::new(from.into(), to.into())
    }
}
