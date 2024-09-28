use ecs::core::Component;
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

pub struct Transform {
    pub position: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            position: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
        }
    }
}

impl Transform {
    pub fn new(position: Vec3, rotation: Quat, scale: Vec3) -> Self {
        Self {
            position,
            rotation,
            scale,
        }
    }

    pub fn from_mat4(mat: Mat4) -> Self {
        let (scale, rotation, position) = mat.to_scale_rotation_translation();
        Self {
            position,
            rotation,
            scale,
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

    pub fn translate(&mut self, translation: Vec3, space: Space) -> &mut Self {
        match space {
            Space::Local => {
                self.position += self.rotation * translation;
            }
            Space::World => {
                self.position += translation;
            }
        }

        self
    }

    pub fn rotate(&mut self, rotation: Quat) -> &mut Self {
        self.rotation = self.rotation * rotation;
        self
    }

    pub fn rotate_around(&mut self, axis: Axis, angle: f32) -> &mut Self {
        let rotation = match axis {
            Axis::X => Quat::from_axis_angle(Vec3::X, angle),
            Axis::Y => Quat::from_axis_angle(Vec3::Y, angle),
            Axis::Z => Quat::from_axis_angle(Vec3::Z, angle),
        };
        self.rotation = self.rotation * rotation;
        self
    }

    pub fn scale(&mut self, scale: Vec3) -> &mut Self {
        self.scale *= scale;
        self
    }

    pub fn look_at(&mut self, target: Vec3, up: Vec3) -> &mut Self {
        let f = (target - self.position).normalize();
        let r = up.cross(f).normalize();
        let u = f.cross(r).normalize();
        self.rotation = Quat::from_mat4(&Mat4::from_cols(
            r.extend(0.0),
            u.extend(0.0),
            f.extend(0.0),
            Vec3::ZERO.extend(1.0),
        ));

        self
    }
}

impl Component for Transform {}

pub struct LocalToWorld(pub Mat4);

impl Component for LocalToWorld {}

impl Default for LocalToWorld {
    fn default() -> Self {
        Self(Mat4::IDENTITY)
    }
}

impl LocalToWorld {
    pub fn update(&mut self, transform: &Transform, parent: Option<&LocalToWorld>) {
        self.0 = transform.matrix(parent.map(|p| p.0));
    }
}
