use crate::core::Color;
use glam::{UVec2, Vec2};
use shadow_asset::asset::AssetId;
use shadow_ecs::core::Component;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ClearFlag {
    Skybox,
    Color(Color),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Viewport {
    pub position: UVec2,
    pub size: Vec2,
    pub depth: i32,
}

impl Default for Viewport {
    fn default() -> Self {
        Self {
            position: UVec2::ZERO,
            size: Vec2::new(1.0, 1.0),
            depth: 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Projection {
    Orthographic {
        left: f32,
        right: f32,
        bottom: f32,
        top: f32,
        near: f32,
        far: f32,
    },
    Perspective {
        fov: f32,
        aspect: f32,
        near: f32,
        far: f32,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct Camera {
    pub clear: Option<ClearFlag>,
    pub viewport: Viewport,
    pub projection: Projection,
    pub target: Option<AssetId>,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            clear: None,
            viewport: Viewport::default(),
            projection: Projection::Perspective {
                fov: 27.0,
                aspect: 1.0,
                near: 0.3,
                far: 1000.0,
            },
            target: None,
        }
    }
}

impl Component for Camera {}

#[derive(Debug, Clone, PartialEq)]
pub struct RenderFrame {
    pub camera: Camera,
    pub world: glam::Mat4,
    pub view: glam::Mat4,
    pub projection: glam::Mat4,
}

impl RenderFrame {
    pub fn new(camera: Camera, world: glam::Mat4) -> Self {
        let view = glam::Mat4::look_at_rh(
            glam::Vec3::new(0.0, 0.0, 0.0),
            glam::Vec3::new(0.0, 0.0, -1.0),
            glam::Vec3::new(0.0, 1.0, 0.0),
        );
        let projection = match camera.projection {
            Projection::Orthographic {
                left,
                right,
                bottom,
                top,
                near,
                far,
            } => glam::Mat4::orthographic_rh(left, right, bottom, top, near, far),
            Projection::Perspective {
                fov,
                aspect,
                near,
                far,
            } => glam::Mat4::perspective_rh(fov.to_radians(), aspect, near, far),
        };
        Self {
            camera,
            world,
            view,
            projection,
        }
    }
}
