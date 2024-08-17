use crate::bounds::{BoundingBox, BoundingRect};
use glam::{Mat3, Mat4};
use shadow_ecs::core::Entity;

pub trait Partition: Sized + 'static {
    type Item;
    type Query;

    fn insert(&mut self, item: Self::Item);
    fn iter(&self) -> impl Iterator<Item = &Self::Item>;
    fn query(&self, query: &Self::Query) -> Vec<&Self::Item>;
    fn clear(&mut self);
}

pub trait Entity3D: 'static {
    fn entity(&self) -> Entity;
    fn transform(&self) -> Mat4;
    fn bounds(&self) -> &BoundingBox;
}

pub trait Entity2D: 'static {
    fn entity(&self) -> Entity;
    fn transform(&self) -> Mat3;
    fn bounds(&self) -> &BoundingRect;
}
