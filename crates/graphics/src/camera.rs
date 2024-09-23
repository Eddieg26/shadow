use crate::{
    core::Color,
    resources::buffer::{BufferFlags, UniformBuffer},
};
use asset::AssetId;
use ecs::core::{Component, Resource};
use glam::{UVec2, Vec2, Vec3};

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
            target: None,
            depth: 0,
        }
    }
}

impl Component for Camera {}

#[derive(Debug, Clone, Copy, Default, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
pub struct CameraData {
    pub view: [f32; 16],
    pub projection: [f32; 16],
    pub world: [f32; 3],
    pub _padding: f32,
}

impl CameraData {
    pub fn new(view: glam::Mat4, projection: glam::Mat4, world: glam::Vec3) -> Self {
        Self {
            view: view.to_cols_array(),
            projection: projection.to_cols_array(),
            world: world.to_array(),
            _padding: 0.0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct RenderFrame {
    pub camera: Camera,
    pub buffer: CameraData,
}

impl RenderFrame {
    pub fn new(camera: Camera, world: glam::Mat4) -> Self {
        let (_, rotation, translation) = world.to_scale_rotation_translation();
        let view =
            glam::Mat4::from_scale_rotation_translation(Vec3::ONE, rotation, translation).inverse();

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

        let buffer = CameraData::new(view, projection, translation);

        Self { camera, buffer }
    }
}

impl Default for RenderFrame {
    fn default() -> Self {
        Self {
            camera: Camera::default(),
            buffer: CameraData::default(),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct RenderFrames {
    frames: Vec<RenderFrame>,
}

impl RenderFrames {
    pub fn new() -> Self {
        Self { frames: Vec::new() }
    }

    pub fn add(&mut self, frame: RenderFrame) {
        self.frames.push(frame);
    }

    pub fn clear(&mut self) {
        self.frames.clear();
    }

    pub fn extract(&mut self, other: &mut Self) {
        self.frames = std::mem::take(&mut other.frames);
        self.frames
            .sort_by(|a, b| a.camera.depth.cmp(&b.camera.depth));
    }

    pub fn frames(&self) -> &[RenderFrame] {
        &self.frames
    }

    pub fn len(&self) -> usize {
        self.frames.len()
    }

    pub fn drain(&mut self) -> std::vec::Drain<RenderFrame> {
        self.frames.drain(..)
    }
}

impl std::ops::Index<usize> for RenderFrames {
    type Output = RenderFrame;

    fn index(&self, index: usize) -> &Self::Output {
        &self.frames[index]
    }
}

impl IntoIterator for RenderFrames {
    type Item = RenderFrame;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.frames.into_iter()
    }
}

impl<'a> IntoIterator for &'a RenderFrames {
    type Item = &'a RenderFrame;
    type IntoIter = std::slice::Iter<'a, RenderFrame>;

    fn into_iter(self) -> Self::IntoIter {
        self.frames.iter()
    }
}

impl Resource for RenderFrames {}

pub type CameraBuffer = UniformBuffer<CameraData>;
impl Resource for CameraBuffer {}
impl Default for CameraBuffer {
    fn default() -> Self {
        CameraBuffer::new(CameraData::default(), BufferFlags::COPY_DST)
    }
}
