use super::{
    constants::{CAMERA_BINDING, CAMERA_GROUP, OBJECT_BINDING, OBJECT_GROUP, VERTEX_OUTPUT},
    NodeDependency, NodeId, ShaderNode, VertexInput, VertexOutput,
};
use crate::material::shader::{
    snippets::{self},
    ShaderOutput,
};
use graphics::resources::{mesh::MeshLayout, shader::ShaderSource};
use std::collections::{HashMap, HashSet};

pub struct MeshShader {
    inputs: Vec<VertexInput>,
    nodes: Vec<Box<dyn ShaderNode>>,
    edges: Vec<Edge>,
}

impl MeshShader {
    pub fn new() -> Self {
        Self {
            inputs: Vec::new(),
            nodes: Vec::new(),
            edges: Vec::new(),
        }
    }

    pub fn inputs(&self) -> &[VertexInput] {
        &self.inputs
    }

    pub fn nodes(&self) -> &[Box<dyn ShaderNode>] {
        &self.nodes
    }

    pub fn edges(&self) -> &[Edge] {
        &self.edges
    }

    pub fn add_input(&mut self, input: VertexInput) {
        self.inputs.push(input);
    }

    pub fn add_node(&mut self, node: impl ShaderNode) -> NodeId {
        let node = Box::new(node);
        let id = node.id();
        self.nodes.push(node);
        id
    }

    pub fn add_edge(&mut self, edge: impl Into<Edge>) {
        self.edges.push(edge.into());
    }

    pub fn remove_input(&mut self, input: VertexInput) -> Option<VertexInput> {
        let index = self.inputs.iter().position(|&i| i == input)?;
        self.remove_input_edges(input);
        Some(self.inputs.remove(index))
    }

    pub fn remove_node(&mut self, id: NodeId) -> Option<Box<dyn ShaderNode>> {
        let index = self.node_index(id)?;
        self.remove_node_edges(id);
        Some(self.nodes.remove(index))
    }

    pub fn remove_edge(&mut self, from: &EdgeSlot, to: &EdgeSlot) {
        self.edges
            .retain(|edge| edge.from() != from || edge.to() != to);
    }

    pub fn remove_node_edges(&mut self, id: NodeId) {
        self.edges
            .retain(|edge| edge.from().id() != Some(id) && edge.to().id() != Some(id));
    }

    pub fn remove_input_edges(&mut self, input: VertexInput) {
        self.edges.retain(|edge| {
            edge.from().input_kind() != Some(input) && edge.to().input_kind() != Some(input)
        });
    }

    pub fn remove_output_edges(&mut self, output: VertexOutput) {
        self.edges.retain(|edge| {
            edge.from().output_kind() != Some(output) && edge.to().output_kind() != Some(output)
        });
    }

    pub fn node_index(&self, id: NodeId) -> Option<usize> {
        self.nodes.iter().position(|node| node.id() == id)
    }

    pub fn layout(&self) -> MeshLayout {
        let mut layout = vec![];
        for input in self.inputs() {
            layout.push(input.attribute());
        }

        MeshLayout::from(layout)
    }

    pub fn generate(&self) -> ShaderSource {
        let mut outputs: HashMap<NodeId, Vec<ShaderOutput>> = HashMap::new();
        let mut attributes = vec![];
        for input in self.inputs() {
            attributes.push(input.attribute());
            let output = snippets::vertex_input(*input);
            let id = input.id();
            outputs.entry(id).or_insert(vec![]).push(output);
        }

        let mut definitions = String::new();
        definitions += &snippets::define_camera(CAMERA_GROUP, CAMERA_BINDING);
        definitions += &snippets::define_object(OBJECT_GROUP, OBJECT_BINDING);
        definitions += &snippets::define_vertex_input(&self.inputs);
        definitions += &snippets::define_vertex_output();

        let (node_inputs, vertex_outputs) = self.get_order();

        let mut body = String::new();
        for (index, inputs) in node_inputs {
            let node = self.nodes[index].as_ref();
            let inputs = inputs
                .iter()
                .map(|input| match input {
                    Some(input) => outputs
                        .get(&input.node)
                        .and_then(|outputs| outputs.get(input.slot)),
                    None => None,
                })
                .collect::<Vec<_>>();

            let mut output = match node.execute(&inputs) {
                Some(output) => output,
                None => continue,
            };

            body += &output.code;

            let mut node_outputs = vec![];
            for output in output.outputs.drain(..) {
                node_outputs.push(output);
            }

            outputs.insert(node.id(), node_outputs);
        }

        for (attribute, input) in vertex_outputs {
            let output = match outputs
                .get(&input.node)
                .and_then(|outputs| outputs.get(input.slot))
            {
                Some(output) => output,
                None => continue,
            };

            let value = match snippets::convert_input(output, attribute.property()) {
                Some(v) => v,
                None => continue,
            };

            let code = format!("{}.{} = {};", VERTEX_OUTPUT, attribute.name(), value);
            body += &code;
        }

        let vertex_body = snippets::define_vertex_body(body);
        let source = format!("{}{}", definitions, vertex_body);

        ShaderSource::Wgsl(source.into())
    }

    fn get_order(
        &self,
    ) -> (
        Vec<(usize, Box<[Option<NodeDependency>]>)>,
        HashMap<&VertexOutput, NodeDependency>,
    ) {
        let mut dependencies = self
            .nodes
            .iter()
            .map(|node| (node.id(), HashMap::new()))
            .collect::<HashMap<_, _>>();
        let mut outputs: HashMap<&VertexOutput, NodeDependency> = HashMap::new();

        for edge in self.edges() {
            let (from_id, from_slot) = match edge.from() {
                EdgeSlot::Node { id, slot } => (*id, *slot),
                EdgeSlot::Input { input } => (input.id(), 0),
                _ => continue,
            };

            let (to_id, to_slot) = match edge.to() {
                EdgeSlot::Node { id, slot } => (*id, *slot),
                EdgeSlot::Output { output } => {
                    outputs.insert(output, NodeDependency::new(from_id, from_slot));
                    continue;
                }
                _ => continue,
            };

            dependencies
                .entry(to_id)
                .or_insert(HashMap::new())
                .insert(to_slot, NodeDependency::new(from_id, from_slot));
        }

        self.purge_unsused_nodes(&mut dependencies, outputs.values());

        let mut order = vec![];
        while !dependencies.is_empty() {
            let mut next = vec![];
            for (node, inputs) in dependencies.iter() {
                if inputs
                    .values()
                    .all(|input| !dependencies.contains_key(&input.node))
                {
                    next.push(*node);
                }
            }

            for node in next {
                let inputs = match dependencies.remove(&node) {
                    Some(inputs) => inputs,
                    None => continue,
                };

                let index = self.node_index(node).unwrap();
                let node = &self.nodes[index];
                let mut slots = vec![None; node.inputs().len()];
                for (slot, input) in inputs {
                    slots[slot] = Some(input);
                }

                order.push((index, slots.into_boxed_slice()));
            }
        }

        (order, outputs)
    }

    fn purge_unsused_nodes<'a>(
        &self,
        dependencies: &mut HashMap<NodeId, HashMap<usize, NodeDependency>>,
        outputs: impl Iterator<Item = &'a NodeDependency>,
    ) {
        let mut marked = HashSet::new();
        for output in outputs {
            marked.extend(self.mark_used_nodes(&output.node, dependencies));
        }

        dependencies.retain(|id, _| marked.contains(id));
    }

    fn mark_used_nodes(
        &self,
        id: &NodeId,
        dependencies: &HashMap<NodeId, HashMap<usize, NodeDependency>>,
    ) -> HashSet<NodeId> {
        let mut marked = HashSet::new();
        marked.insert(*id);
        match dependencies.get(id) {
            Some(inputs) => {
                for input in inputs.values() {
                    marked.extend(self.mark_used_nodes(&input.node, dependencies));
                }
            }
            None => (),
        }

        marked
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum EdgeSlot {
    Node { id: NodeId, slot: usize },
    Input { input: VertexInput },
    Output { output: VertexOutput },
}

impl EdgeSlot {
    pub fn node(id: NodeId, slot: usize) -> Self {
        Self::Node { id, slot }
    }

    pub fn input(input: VertexInput) -> Self {
        Self::Input { input }
    }

    pub fn output(output: VertexOutput) -> Self {
        Self::Output { output }
    }

    pub fn id(&self) -> Option<NodeId> {
        match self {
            Self::Node { id, .. } => Some(*id),
            _ => None,
        }
    }

    pub fn slot(&self) -> Option<usize> {
        match self {
            Self::Node { slot, .. } => Some(*slot),
            _ => None,
        }
    }

    pub fn input_kind(&self) -> Option<VertexInput> {
        match self {
            Self::Input { input } => Some(*input),
            _ => None,
        }
    }

    pub fn output_kind(&self) -> Option<VertexOutput> {
        match self {
            Self::Output { output } => Some(*output),
            _ => None,
        }
    }
}

impl From<(NodeId, usize)> for EdgeSlot {
    fn from(value: (NodeId, usize)) -> Self {
        Self::node(value.0, value.1)
    }
}

impl From<VertexInput> for EdgeSlot {
    fn from(value: VertexInput) -> Self {
        Self::input(value)
    }
}

impl From<VertexOutput> for EdgeSlot {
    fn from(value: VertexOutput) -> Self {
        Self::output(value)
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
