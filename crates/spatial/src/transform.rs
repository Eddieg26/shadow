use ecs::{
    core::Component,
    task::{max_thread_count, ScopedTaskPool},
    world::{
        components::{Children, Parent},
        query::{FilterQuery, Not, Query, With},
        World,
    },
};
use glam::{Mat4, Quat, Vec3};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Space {
    Local,
    World,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Axis {
    X,
    Y,
    Z,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Transform {
    pub position: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
    pub local_to_world: Mat4,
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            position: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
            local_to_world: Mat4::IDENTITY,
        }
    }
}

impl Transform {
    pub fn new(position: Vec3, rotation: Quat, scale: Vec3) -> Self {
        let local_to_world = Mat4::from_scale_rotation_translation(scale, rotation, position);
        Self {
            position,
            rotation,
            scale,
            local_to_world,
        }
    }

    pub fn zero() -> Self {
        Self::default()
    }

    pub fn with_position(mut self, position: Vec3) -> Self {
        self.position = position;
        self
    }

    pub fn with_rotation(mut self, rotation: Quat) -> Self {
        self.rotation = rotation;
        self
    }

    pub fn with_scale(mut self, scale: Vec3) -> Self {
        self.scale = scale;
        self
    }

    pub fn from_mat4(mat: Mat4) -> Self {
        let (scale, rotation, position) = mat.to_scale_rotation_translation();
        Self {
            position,
            rotation,
            scale,
            local_to_world: mat,
        }
    }

    pub fn foward(&self) -> Vec3 {
        self.rotation * Vec3::Z
    }

    pub fn right(&self) -> Vec3 {
        self.rotation * Vec3::X
    }

    pub fn up(&self) -> Vec3 {
        self.rotation * Vec3::Y
    }

    pub fn down(&self) -> Vec3 {
        -self.up()
    }

    pub fn left(&self) -> Vec3 {
        -self.right()
    }

    pub fn back(&self) -> Vec3 {
        -self.foward()
    }

    pub fn matrix(&self, world: Option<Mat4>) -> Mat4 {
        let mut mat =
            Mat4::from_scale_rotation_translation(self.scale, self.rotation, self.position);

        if let Some(world) = world {
            mat = world * mat;
        }
        mat
    }

    pub fn translate(&mut self, translation: Vec3) {
        self.position += translation;
    }

    pub fn rotate(&mut self, rotation: Quat) {
        self.rotation = self.rotation * rotation;
    }

    pub fn rotate_around(&mut self, axis: Axis, angle: f32) {
        let rotation = match axis {
            Axis::X => Quat::from_axis_angle(Vec3::X, angle),
            Axis::Y => Quat::from_axis_angle(Vec3::Y, angle),
            Axis::Z => Quat::from_axis_angle(Vec3::Z, angle),
        };
        self.rotation = self.rotation * rotation;
    }

    pub fn scale(&mut self, scale: Vec3) {
        self.scale *= scale;
    }

    pub fn look_at(&mut self, target: Vec3, up: Vec3) {
        let f = (target - self.position).normalize();
        let r = up.cross(f).normalize();
        let u = f.cross(r).normalize();
        self.rotation = Quat::from_mat4(&Mat4::from_cols(
            r.extend(0.0),
            u.extend(0.0),
            f.extend(0.0),
            Vec3::ZERO.extend(1.0),
        ));
    }

    pub fn update(&mut self, parent: Option<&Transform>) {
        self.local_to_world = self.matrix(parent.map(|p| p.local_to_world));
    }
}

impl Component for Transform {}

pub fn update_transforms(
    query: Query<(&mut Transform, Option<&Children>), Not<Parent>>,
    world: &World,
) {
    let mut scoped = ScopedTaskPool::new(max_thread_count());
    for (transform, children) in query {
        transform.update(None);

        if let Some(children) = children {
            scoped.spawn(move || {
                update_child_transforms(children, &transform, world);
            });
        }
    }
}

pub fn update_child_transforms(children: &Children, parent: &Transform, world: &World) {
    for (transform, children) in
        FilterQuery::<(&mut Transform, Option<&Children>), With<Parent>>::new(world, *&children)
    {
        transform.update(Some(parent));

        if let Some(children) = children {
            update_child_transforms(children, &transform, world);
        }
    }
}
