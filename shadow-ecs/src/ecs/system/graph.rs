use std::{
    collections::{HashMap, HashSet},
    marker::PhantomData,
};

pub trait GraphNode: 'static {
    fn is_dependency(&self, other: &Self) -> bool;
}

type NodeId = usize;

pub struct Graph<N: GraphNode> {
    built: bool,
    nodes: Vec<N>,
    hierarchy: Vec<Vec<NodeId>>,
    dependencies: HashMap<NodeId, HashSet<NodeId>>,
}

impl<Node: GraphNode> Graph<Node> {
    pub fn new() -> Self {
        Self {
            built: false,
            nodes: vec![],
            hierarchy: vec![],
            dependencies: HashMap::new(),
        }
    }

    pub fn is_built(&self) -> bool {
        self.built
    }

    pub fn get(&self, id: NodeId) -> Option<&Node> {
        self.nodes.get(id)
    }

    pub fn insert(&mut self, node: Node) -> NodeId {
        let id = self.nodes.len();
        self.built = false;
        self.nodes.push(node);
        id
    }

    pub fn add_depenency(&mut self, id: NodeId, dependency: NodeId) {
        self.built = false;
        self.dependencies.entry(id).or_default().insert(dependency);
    }

    pub fn iter(&self) -> GraphIter<Node> {
        GraphIter::new(&self.nodes, &self.hierarchy)
    }

    pub fn build(&mut self) {
        if self.built || self.nodes.len() == 0 {
            self.built = true;
            return;
        }

        if self.nodes.len() == 1 {
            self.hierarchy = vec![vec![0]];
            self.built = true;
            return;
        }

        let mut dependencies = self.dependencies.clone();
        let mut ids = HashSet::new();
        for (id, node) in self.nodes[0..(self.nodes.len() / 2)].iter().enumerate() {
            ids.insert(id);
            for (other_id, other_node) in self.nodes[id..].iter().enumerate() {
                if id != other_id {
                    ids.insert(other_id);
                    if node.is_dependency(other_node) {
                        let dependencies = dependencies.entry(other_id).or_insert(HashSet::new());
                        dependencies.insert(id);
                    };
                }
            }
        }

        let mut hierarchy = vec![];
        while !ids.is_empty() {
            let mut group = vec![];
            for id in ids.iter() {
                let count = dependencies.get(id).map_or(0, |d| d.len());
                if count == 0 {
                    dependencies.remove(id);
                    group.push(*id);
                }
            }

            for id in &group {
                ids.remove(&id);
                for sets in dependencies.values_mut() {
                    sets.remove(&id);
                }
            }

            if group.is_empty() {
                panic!("Circular dependency");
            }

            group.sort();

            hierarchy.push(group);
        }

        self.built = true;
        self.hierarchy = hierarchy;
    }
}

impl<Node: GraphNode> std::fmt::Display for Graph<Node> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Nodes: ")?;
        for group in self.hierarchy.iter() {
            write!(f, "{:?} ", group)?;
        }

        writeln!(f)?;

        writeln!(f, "Dependencies")?;
        for (id, deps) in self.dependencies.iter() {
            writeln!(f, "{} -> {:?}", id, deps)?;
        }
        Ok(())
    }
}

pub struct GraphIter<'a, Node: GraphNode> {
    index: usize,
    nodes: &'a Vec<Node>,
    hierarchy: &'a [Vec<NodeId>],
    _marker: PhantomData<Node>,
}

impl<'a, Node: GraphNode> GraphIter<'a, Node> {
    fn new(nodes: &'a Vec<Node>, hierarchy: &'a [Vec<NodeId>]) -> Self {
        Self {
            index: 0,
            nodes,
            hierarchy,
            _marker: PhantomData,
        }
    }
}

impl<'a, Node: GraphNode> Iterator for GraphIter<'a, Node> {
    type Item = Vec<&'a Node>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(group) = self.hierarchy.get(self.index) {
            self.index += 1;
            let group = group.iter().map(|i| &self.nodes[*i]).collect::<Vec<_>>();
            Some(group)
        } else {
            None
        }
    }
}
