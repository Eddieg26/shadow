use crate::{
    bounds::BoundingBox,
    partition::{Entity3D, Partition},
};
use glam::Vec3;

pub struct OctreeNode<N: Entity3D> {
    children: Vec<OctreeNode<N>>,
    objects: Vec<N>,
    bounds: BoundingBox,
    depth: usize,
    max_objects: usize,
    max_depth: usize,
}

impl<N: Entity3D> OctreeNode<N> {
    pub fn new(bounds: BoundingBox, max_objects: usize, depth: usize, max_depth: usize) -> Self {
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
                .items()
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

        let mut children = Vec::with_capacity(8);

        children.push(OctreeNode::new(
            BoundingBox::new(min, mid),
            self.max_objects,
            self.depth + 1,
            self.max_depth,
        ));
        children.push(OctreeNode::new(
            BoundingBox::new(
                Vec3::new(mid.x, min.y, min.z),
                Vec3::new(max.x, mid.y, mid.z),
            ),
            self.max_objects,
            self.depth + 1,
            self.max_depth,
        ));
        children.push(OctreeNode::new(
            BoundingBox::new(
                Vec3::new(mid.x, min.y, mid.z),
                Vec3::new(max.x, mid.y, max.z),
            ),
            self.max_objects,
            self.depth + 1,
            self.max_depth,
        ));
        children.push(OctreeNode::new(
            BoundingBox::new(
                Vec3::new(min.x, min.y, mid.z),
                Vec3::new(mid.x, mid.y, max.z),
            ),
            self.max_objects,
            self.depth + 1,
            self.max_depth,
        ));
        children.push(OctreeNode::new(
            BoundingBox::new(
                Vec3::new(min.x, mid.y, min.z),
                Vec3::new(mid.x, max.y, mid.z),
            ),
            self.max_objects,
            self.depth + 1,
            self.max_depth,
        ));
        children.push(OctreeNode::new(
            BoundingBox::new(
                Vec3::new(mid.x, mid.y, min.z),
                Vec3::new(max.x, max.y, mid.z),
            ),
            self.max_objects,
            self.depth + 1,
            self.max_depth,
        ));
        children.push(OctreeNode::new(
            BoundingBox::new(mid, max),
            self.max_objects,
            self.depth + 1,
            self.max_depth,
        ));

        let mut objects = vec![];
        std::mem::swap(&mut self.objects, &mut objects);

        for object in objects {
            let index = children
                .items()
                .position(|child| child.bounds.contains(object.bounds()));

            match index {
                Some(index) => children[index].insert(object),
                None => self.objects.push(object),
            };
        }

        self.children = children;
    }

    pub fn contains(&self, bounds: &BoundingBox) -> bool {
        self.bounds.contains(bounds)
    }

    pub fn iter(&self) -> impl Iterator<Item = &N> {
        self.objects.items().chain(
            self.children
                .items()
                .flat_map(|child| child.objects.items()),
        )
    }

    pub fn drain(&mut self) -> Vec<N> {
        let mut objects = vec![];

        for child in &mut self.children {
            objects.extend(child.drain());
        }

        objects.append(&mut self.objects);

        objects
    }

    pub fn query(&self, bounds: &BoundingBox, filter: impl Fn(&N) -> bool) -> Vec<&N> {
        let mut result = vec![];

        if bounds.intersects(&self.bounds) {
            for object in &self.objects {
                if filter(object) {
                    result.push(object);
                }
            }

            for child in &self.children {
                result.extend(child.query(bounds, &filter));
            }
        }

        result
    }
}

pub struct Octree<N: Entity3D> {
    root: OctreeNode<N>,
    max_objects: usize,
    max_depth: usize,
}

impl<N: Entity3D> Octree<N> {
    pub fn new(bounds: BoundingBox, max_objects: usize, max_depth: usize) -> Self {
        Self {
            root: OctreeNode::new(bounds, max_objects, 0, max_depth),
            max_objects,
            max_depth,
        }
    }
}

pub struct OctreeQuery<N: Entity3D> {
    bounds: BoundingBox,
    filter: Box<dyn Fn(&N) -> bool>,
}

impl<N: Entity3D> OctreeQuery<N> {
    pub fn new(bounds: BoundingBox, filter: impl Fn(&N) -> bool + 'static) -> Self {
        Self {
            bounds,
            filter: Box::new(filter),
        }
    }
}

impl<N: Entity3D> Partition for Octree<N> {
    type Item = N;
    type Query = OctreeQuery<N>;

    fn insert(&mut self, item: Self::Item) {
        self.root.insert(item);
    }

    fn items(&self) -> impl Iterator<Item = &Self::Item> {
        self.root.iter()
    }

    fn query(&self, query: &Self::Query) -> Vec<&Self::Item> {
        self.root.query(&query.bounds, &query.filter)
    }

    fn drain(&mut self) -> Vec<Self::Item> {
        self.root.drain()
    }

    fn clear(&mut self) {
        self.root = OctreeNode::new(self.root.bounds, self.max_objects, 0, self.max_depth);
    }
}

impl<N: Entity3D> Default for Octree<N> {
    fn default() -> Self {
        Self::new(BoundingBox::new(Vec3::ZERO, Vec3::ZERO), 8, 8)
    }
}
