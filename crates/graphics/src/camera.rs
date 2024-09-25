use crate::{core::Color, resources::RenderAsset};
use asset::AssetId;
use ecs::core::Component;
use glam::{Vec2, Vec3};
use spatial::Size;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ClearFlag {
    Skybox,
    Color(Color),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Viewport {
    pub position: Size,
    pub size: Vec2,
    pub depth: i32,
}

impl Default for Viewport {
    fn default() -> Self {
        Self {
            position: Size::ZERO,
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
    pub depth: u32,
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
            // projection: Projection::Orthographic {
            //     left: -1.0,
            //     right: 1.0,
            //     bottom: -1.0,
            //     top: 1.0,
            //     near: 0.3,
            //     far: 1000.0,
            // },
            target: None,
            depth: 0,
        }
    }
}

impl Component for Camera {}

#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
pub struct CameraData {
    pub view: glam::Mat4,
    pub projection: glam::Mat4,
    pub world: glam::Vec3,
    pub _padding: f32,
}

impl CameraData {
    pub fn new(view: glam::Mat4, projection: glam::Mat4, world: glam::Vec3) -> Self {
        Self {
            view,
            projection,
            world,
            _padding: 0.0,
        }
    }
}

impl Default for CameraData {
    fn default() -> Self {
        Self {
            view: glam::Mat4::IDENTITY,
            projection: glam::Mat4::IDENTITY,
            world: Vec3::ZERO,
            _padding: 0.0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct RenderCamera {
    pub clear: Option<ClearFlag>,
    pub viewport: Viewport,
    pub projection: Projection,
    pub target: Option<AssetId>,
    pub depth: u32,
    pub data: CameraData,
}

impl RenderCamera {
    pub fn new(camera: &Camera, world: glam::Mat4) -> Self {
        let (_, rotation, translation) = world.to_scale_rotation_translation();
        let view =
            glam::Mat4::from_scale_rotation_translation(Vec3::ONE, rotation, translation).inverse();

        let projection = Self::calculate_projection(camera.projection);

        Self {
            clear: camera.clear,
            viewport: camera.viewport,
            projection: camera.projection,
            target: camera.target,
            depth: camera.depth,
            data: CameraData::new(view, projection, translation),
        }
    }

    fn calculate_projection(projection: Projection) -> glam::Mat4 {
        match projection {
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
        }
    }
}

impl Default for RenderCamera {
    fn default() -> Self {
        let camera = Camera::default();
        let projection = Self::calculate_projection(camera.projection);

        Self {
            clear: camera.clear,
            viewport: camera.viewport,
            projection: camera.projection,
            target: camera.target,
            depth: camera.depth,
            data: CameraData::new(glam::Mat4::IDENTITY, projection, Vec3::ZERO),
        }
    }
}

impl RenderAsset for RenderCamera {
    type Id = u32;
}
