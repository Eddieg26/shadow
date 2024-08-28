use glam::{Vec2, Vec3};

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
        point.x >= self.min.x
            && point.y >= self.min.y
            && point.z >= self.min.z
            && point.x <= self.max.x
            && point.y <= self.max.y
            && point.z <= self.max.z
    }

    pub fn contains(&self, other: &BoundingBox) -> bool {
        self.contains_point(other.min) && self.contains_point(other.max)
    }

    pub fn intersects(&self, other: &BoundingBox) -> bool {
        self.min.x <= other.max.x
            && self.max.x >= other.min.x
            && self.min.y <= other.max.y
            && self.max.y >= other.min.y
            && self.min.z <= other.max.z
            && self.max.z >= other.min.z
    }

    pub fn size(&self) -> Vec3 {
        self.max - self.min
    }
}

#[derive(Clone, Copy, Debug)]
pub struct BoundingRect {
    pub min: Vec2,
    pub max: Vec2,
}

impl BoundingRect {
    pub fn new(min: Vec2, max: Vec2) -> Self {
        Self { min, max }
    }

    pub fn mid(&self) -> Vec2 {
        (self.min + self.max) / 2.0
    }

    pub fn contains_point(&self, point: Vec2) -> bool {
        point.x >= self.min.x
            && point.y >= self.min.y
            && point.x <= self.max.x
            && point.y <= self.max.y
    }

    pub fn contains(&self, other: &BoundingRect) -> bool {
        self.contains_point(other.min) && self.contains_point(other.max)
    }

    pub fn intersects(&self, other: &BoundingRect) -> bool {
        self.min.x <= other.max.x
            && self.max.x >= other.min.x
            && self.min.y <= other.max.y
            && self.max.y >= other.min.y
    }

    pub fn size(&self) -> Vec2 {
        self.max - self.min
    }
}

#[derive(Clone, Copy, Debug)]
pub struct BoundingSphere {
    pub center: Vec3,
    pub radius: f32,
}

impl BoundingSphere {
    pub fn new(center: Vec3, radius: f32) -> Self {
        Self { center, radius }
    }

    pub fn contains_point(&self, point: Vec3) -> bool {
        (point - self.center).length() <= self.radius
    }

    pub fn contains(&self, other: &BoundingSphere) -> bool {
        (self.center - other.center).length() + other.radius <= self.radius
    }

    pub fn intersects(&self, other: &BoundingSphere) -> bool {
        (self.center - other.center).length() <= self.radius + other.radius
    }
}

#[derive(Clone, Copy, Debug)]
pub struct BoundingCircle {
    pub center: Vec2,
    pub radius: f32,
}

impl BoundingCircle {
    pub fn new(center: Vec2, radius: f32) -> Self {
        Self { center, radius }
    }

    pub fn contains_point(&self, point: Vec2) -> bool {
        (point - self.center).length() <= self.radius
    }

    pub fn contains(&self, other: &BoundingCircle) -> bool {
        (self.center - other.center).length() + other.radius <= self.radius
    }

    pub fn intersects(&self, other: &BoundingCircle) -> bool {
        (self.center - other.center).length() <= self.radius + other.radius
    }
}
