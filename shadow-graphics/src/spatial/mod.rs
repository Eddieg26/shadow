use std::any::TypeId;

use glam::{Mat4, Vec3};
use shadow_ecs::core::{DenseMap, Entity, Resource};

use crate::resources::ResourceId;

#[derive(Clone, Copy, Debug)]
pub struct BoundingBox {
    pub min: Vec3,
    pub max: Vec3,
}

impl BoundingBox {
    pub fn new(min: Vec3, max: Vec3) -> Self {
        Self { min, max }
    }

    pub fn mid(&self) -> Vec3 {
        (self.min + self.max) / 2.0
    }

    pub fn contains_point(&self, point: Vec3) -> bool {
        point.x() >= self.min.x()
            && point.y() >= self.min.y()
            && point.z() >= self.min.z()
            && point.x() <= self.max.x()
            && point.y() <= self.max.y()
            && point.z() <= self.max.z()
    }

    pub fn contains(&self, other: &BoundingBox) -> bool {
        self.contains_point(other.min) && self.contains_point(other.max)
    }

    pub fn intersects(&self, other: &BoundingBox) -> bool {
        self.min.x() <= other.max.x()
            && self.max.x() >= other.min.x()
            && self.min.y() <= other.max.y()
            && self.max.y() >= other.min.y()
            && self.min.z() <= other.max.z()
            && self.max.z() >= other.min.z()
    }

    pub fn size(&self) -> Vec3 {
        self.max - self.min
    }
}

pub trait Object3D: 'static {
    fn entity(&self) -> Entity;
    fn transform(&self) -> Mat4;
    fn bounds(&self) -> &BoundingBox;
}

pub struct OctreeNode<N: Object3D> {
    children: Vec<OctreeNode<N>>,
    objects: Vec<N>,
    bounds: BoundingBox,
    depth: usize,
    max_objects: usize,
    max_depth: usize,
}

impl<N: Object3D> OctreeNode<N> {
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

        let mut children = Vec::with_capacity(8);

        children.push(OctreeNode::new(
            BoundingBox::new(min, mid),
            self.max_objects,
            self.depth + 1,
            self.max_depth,
        ));
        children.push(OctreeNode::new(
            BoundingBox::new(
                Vec3::new(mid.x(), min.y(), min.z()),
                Vec3::new(max.x(), mid.y(), mid.z()),
            ),
            self.max_objects,
            self.depth + 1,
            self.max_depth,
        ));
        children.push(OctreeNode::new(
            BoundingBox::new(
                Vec3::new(mid.x(), min.y(), mid.z()),
                Vec3::new(max.x(), mid.y(), max.z()),
            ),
            self.max_objects,
            self.depth + 1,
            self.max_depth,
        ));
        children.push(OctreeNode::new(
            BoundingBox::new(
                Vec3::new(min.x(), min.y(), mid.z()),
                Vec3::new(mid.x(), mid.y(), max.z()),
            ),
            self.max_objects,
            self.depth + 1,
            self.max_depth,
        ));
        children.push(OctreeNode::new(
            BoundingBox::new(
                Vec3::new(min.x(), mid.y(), min.z()),
                Vec3::new(mid.x(), max.y(), mid.z()),
            ),
            self.max_objects,
            self.depth + 1,
            self.max_depth,
        ));
        children.push(OctreeNode::new(
            BoundingBox::new(
                Vec3::new(mid.x(), mid.y(), min.z()),
                Vec3::new(max.x(), max.y(), mid.z()),
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

        self.children = children;
    }

    pub fn contains(&self, bounds: &BoundingBox) -> bool {
        self.bounds.contains(bounds)
    }

    pub fn iter(&self) -> impl Iterator<Item = &N> {
        self.objects
            .iter()
            .chain(self.children.iter().flat_map(|child| child.objects.iter()))
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

pub struct Octree<N: Object3D> {
    root: OctreeNode<N>,
    max_objects: usize,
    max_depth: usize,
}

impl<N: Object3D> Octree<N> {
    pub fn new(bounds: BoundingBox, max_objects: usize, max_depth: usize) -> Self {
        Self {
            root: OctreeNode::new(bounds, max_objects, 0, max_depth),
            max_objects,
            max_depth,
        }
    }
}

pub trait Partition: Sized + 'static {
    type Item;
    type Bounds;

    fn insert(&mut self, item: Self::Item);
    fn iter(&self) -> impl Iterator<Item = &Self::Item>;
    fn query(
        &self,
        bounds: &Self::Bounds,
        filter: impl Fn(&Self::Item) -> bool,
    ) -> Vec<&Self::Item>;
    fn clear(&mut self);
}

pub struct DensePartition<T: 'static> {
    items: Vec<T>,
}

impl<T: 'static> Partition for DensePartition<T> {
    type Item = T;
    type Bounds = ();

    fn insert(&mut self, item: Self::Item) {
        self.items.push(item);
    }

    fn iter(&self) -> impl Iterator<Item = &Self::Item> {
        self.items.iter()
    }

    fn query(&self, _: &Self::Bounds, filter: impl Fn(&Self::Item) -> bool) -> Vec<&Self::Item> {
        self.items.iter().filter(|item| filter(*item)).collect()
    }

    fn clear(&mut self) {
        self.items.clear();
    }
}

impl<N: Object3D> Partition for Octree<N> {
    type Item = N;

    type Bounds = BoundingBox;

    fn insert(&mut self, object: N) {
        self.root.insert(object);
    }

    fn iter(&self) -> impl Iterator<Item = &N> {
        self.root.iter()
    }

    fn query(&self, bounds: &BoundingBox, filter: impl Fn(&N) -> bool) -> Vec<&N> {
        self.root.query(bounds, filter)
    }

    fn clear(&mut self) {
        self.root = OctreeNode::new(self.root.bounds, self.max_objects, 0, self.max_depth);
    }
}

pub trait Draw: 'static {
    type Partition: Partition<Item = Self>;
}

pub struct DrawCalls<D: Draw> {
    calls: D::Partition,
}

impl<D: Draw> DrawCalls<D> {
    pub fn new(partition: D::Partition) -> Self {
        Self { calls: partition }
    }

    pub fn add(&mut self, draw: D) {
        self.calls.insert(draw);
    }

    pub fn query(
        &self,
        bounds: &<D::Partition as Partition>::Bounds,
        filter: impl Fn(&D) -> bool,
    ) -> Vec<&D> {
        self.calls.query(bounds, filter)
    }

    pub fn clear(&mut self) {
        self.calls.clear();
    }
}

impl<D: Draw> Resource for DrawCalls<D> {}

pub trait Render: downcast_rs::Downcast + 'static {
    fn texture(&self) -> Option<ResourceId>;
    fn clear_color(&self) -> Option<wgpu::Color>;
}

downcast_rs::impl_downcast!(Render);
