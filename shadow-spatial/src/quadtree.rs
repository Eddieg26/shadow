use glam::Vec2;

use crate::{
    bounds::BoundingRect,
    partition::{Entity2D, Partition},
};

pub struct QuadTreeNode<N: Entity2D> {
    children: Vec<QuadTreeNode<N>>,
    objects: Vec<N>,
    bounds: BoundingRect,
    depth: usize,
    max_objects: usize,
    max_depth: usize,
}

impl<N: Entity2D> QuadTreeNode<N> {
    pub fn new(bounds: BoundingRect, max_objects: usize, depth: usize, max_depth: usize) -> Self {
        Self {
            children: vec![],
            objects: vec![],
            bounds,
            max_objects,
            depth,
            max_depth,
        }
    }

    pub fn insert(&mut self, object: N) {
        if self.children.is_empty() {
            self.objects.push(object);
            if self.objects.len() > self.max_objects && self.depth < self.max_depth {
                self.split();
            }
        } else {
            let index = self
                .children
                .iter()
                .position(|child| child.contains(object.bounds()));

            match index {
                Some(index) => self.children[index].insert(object),
                None => self.objects.push(object),
            };
        }
    }

    pub fn split(&mut self) {
        let min = self.bounds.min;
        let max = self.bounds.max;
        let mid = self.bounds.mid();

        let mut children = Vec::with_capacity(4);

        children.push(QuadTreeNode::new(
            BoundingRect::new(min, mid),
            self.max_objects,
            self.depth + 1,
            self.max_depth,
        ));
        children.push(QuadTreeNode::new(
            BoundingRect::new(Vec2::new(mid.x(), min.y()), Vec2::new(max.x(), mid.y())),
            self.max_objects,
            self.depth + 1,
            self.max_depth,
        ));
        children.push(QuadTreeNode::new(
            BoundingRect::new(mid, max),
            self.max_objects,
            self.depth + 1,
            self.max_depth,
        ));
        children.push(QuadTreeNode::new(
            BoundingRect::new(Vec2::new(min.x(), mid.y()), Vec2::new(mid.x(), max.y())),
            self.max_objects,
            self.depth + 1,
            self.max_depth,
        ));

        let mut objects = vec![];
        std::mem::swap(&mut self.objects, &mut objects);

        for object in objects {
            let index = children
                .iter()
                .position(|child| child.bounds.contains(object.bounds()));

            match index {
                Some(index) => children[index].insert(object),
                None => self.objects.push(object),
            };
        }

        self.children = children;
    }

    pub fn contains(&self, bounds: &BoundingRect) -> bool {
        self.bounds.contains(bounds)
    }

    pub fn iter(&self) -> impl Iterator<Item = &N> {
        self.objects
            .iter()
            .chain(self.children.iter().flat_map(|child| child.objects.iter()))
    }

    pub fn query(&self, bounds: &BoundingRect, filter: impl Fn(&N) -> bool) -> Vec<&N> {
        let mut results = vec![];

        if self.bounds.intersects(bounds) {
            for object in &self.objects {
                if filter(object) {
                    results.push(object);
                }
            }

            for child in &self.children {
                results.extend(child.query(bounds, &filter));
            }
        }

        results
    }
}

pub struct QuadTree<N: Entity2D> {
    root: QuadTreeNode<N>,
    max_objects: usize,
    max_depth: usize,
}

impl<N: Entity2D> QuadTree<N> {
    pub fn new(bounds: BoundingRect, max_objects: usize, max_depth: usize) -> Self {
        Self {
            root: QuadTreeNode::new(bounds, max_objects, 0, max_depth),
            max_objects,
            max_depth,
        }
    }
}

pub struct QuadTreeQuery<N: Entity2D> {
    bounds: BoundingRect,
    filter: Box<dyn Fn(&N) -> bool>,
}

impl<N: Entity2D> QuadTreeQuery<N> {
    pub fn new(bounds: BoundingRect, filter: impl Fn(&N) -> bool + 'static) -> Self {
        Self {
            bounds,
            filter: Box::new(filter),
        }
    }
}

impl<N: Entity2D> Partition for QuadTree<N> {
    type Item = N;
    type Query = QuadTreeQuery<N>;

    fn insert(&mut self, item: Self::Item) {
        self.root.insert(item);
    }

    fn iter(&self) -> impl Iterator<Item = &Self::Item> {
        self.root.iter()
    }

    fn query(&self, query: &Self::Query) -> Vec<&Self::Item> {
        self.root.query(&query.bounds, &query.filter)
    }

    fn clear(&mut self) {
        self.root = QuadTreeNode::new(self.root.bounds, self.max_objects, 0, self.max_depth);
    }
}
