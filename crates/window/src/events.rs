use ecs::world::{event::Event, World};
use std::path::PathBuf;
use winit::{
    dpi::{PhysicalPosition, PhysicalSize},
    event::{
        AxisId, DeviceId, ElementState, KeyEvent, Modifiers, MouseButton, MouseScrollDelta,
        TouchPhase,
    },
    window::WindowId,
};

pub struct CloseRequested {
    window: WindowId,
}

impl CloseRequested {
    pub fn new(window: WindowId) -> Self {
        CloseRequested { window }
    }

    pub fn window(&self) -> WindowId {
        self.window
    }
}

impl Event for CloseRequested {
    type Output = Self;

    fn invoke(self, _: &mut World) -> Option<Self::Output> {
        Some(self)
    }
}

pub struct AxisMotion {
    pub device_id: DeviceId,
    pub axis: AxisId,
    pub value: f64,
}

impl AxisMotion {
    pub fn new(device_id: DeviceId, axis: AxisId, value: f64) -> Self {
        AxisMotion {
            device_id,
            axis,
            value,
        }
    }
}

impl Event for AxisMotion {
    type Output = Self;

    fn invoke(self, _: &mut World) -> Option<Self::Output> {
        Some(self)
    }
}

#[derive(Clone, Copy)]
pub struct Resized {
    pub width: u32,
    pub height: u32,
}

impl Resized {
    pub fn new(width: u32, height: u32) -> Self {
        Resized { width, height }
    }
}

impl From<PhysicalSize<u32>> for Resized {
    fn from(size: PhysicalSize<u32>) -> Self {
        Resized {
            width: size.width,
            height: size.height,
        }
    }
}

impl Event for Resized {
    type Output = Self;

    fn invoke(self, _: &mut World) -> Option<Self::Output> {
        Some(self)
    }
}

pub struct Moved {
    pub x: i32,
    pub y: i32,
}

impl From<PhysicalPosition<i32>> for Moved {
    fn from(position: PhysicalPosition<i32>) -> Self {
        Moved {
            x: position.x,
            y: position.y,
        }
    }
}

impl Moved {
    pub fn new(x: i32, y: i32) -> Self {
        Moved { x, y }
    }
}

impl Event for Moved {
    type Output = Self;

    fn invoke(self, _: &mut World) -> Option<Self::Output> {
        Some(self)
    }
}

pub struct Destroyed {
    id: WindowId,
}

impl Destroyed {
    pub fn new(id: WindowId) -> Self {
        Destroyed { id }
    }

    pub fn id(&self) -> WindowId {
        self.id
    }
}

impl Event for Destroyed {
    type Output = Self;

    fn invoke(self, _: &mut World) -> Option<Self::Output> {
        Some(self)
    }
}

pub struct DroppedFile {
    pub path: PathBuf,
}

impl DroppedFile {
    pub fn new(path: PathBuf) -> Self {
        DroppedFile { path }
    }
}

impl Event for DroppedFile {
    type Output = Self;

    fn invoke(self, _: &mut World) -> Option<Self::Output> {
        Some(self)
    }
}

pub struct HoveredFile {
    pub path: PathBuf,
}

impl HoveredFile {
    pub fn new(path: PathBuf) -> Self {
        HoveredFile { path }
    }
}

impl Event for HoveredFile {
    type Output = Self;

    fn invoke(self, _: &mut World) -> Option<Self::Output> {
        Some(self)
    }
}

pub struct HoveredFileCancelled;

impl Event for HoveredFileCancelled {
    type Output = Self;

    fn invoke(self, _: &mut World) -> Option<Self::Output> {
        Some(self)
    }
}

pub struct Focused(bool);

impl Focused {
    pub fn new(focused: bool) -> Self {
        Focused(focused)
    }
}

impl std::ops::Deref for Focused {
    type Target = bool;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Event for Focused {
    type Output = Self;

    fn invoke(self, _: &mut World) -> Option<Self::Output> {
        Some(self)
    }
}

#[derive(Debug, Clone)]
pub struct KeyboardInput {
    pub device_id: DeviceId,
    pub event: KeyEvent,
    pub is_synthetic: bool,
}

impl KeyboardInput {
    pub fn new(device_id: DeviceId, event: KeyEvent, is_synthetic: bool) -> Self {
        KeyboardInput {
            device_id,
            event,
            is_synthetic,
        }
    }
}

impl Event for KeyboardInput {
    type Output = Self;

    fn invoke(self, _: &mut World) -> Option<Self::Output> {
        Some(self)
    }
}

pub struct ModifiersChanged(Modifiers);

impl ModifiersChanged {
    pub fn new(modifiers: Modifiers) -> Self {
        ModifiersChanged(modifiers)
    }
}

impl std::ops::Deref for ModifiersChanged {
    type Target = Modifiers;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Event for ModifiersChanged {
    type Output = Self;

    fn invoke(self, _: &mut World) -> Option<Self::Output> {
        Some(self)
    }
}

pub struct CursorMoved {
    pub device_id: DeviceId,
    pub x: f64,
    pub y: f64,
}

impl CursorMoved {
    pub fn new(device_id: DeviceId, x: f64, y: f64) -> Self {
        CursorMoved { device_id, x, y }
    }
}

impl Event for CursorMoved {
    type Output = Self;

    fn invoke(self, _: &mut World) -> Option<Self::Output> {
        Some(self)
    }
}

pub struct CursorEntered {
    pub device_id: DeviceId,
}

impl CursorEntered {
    pub fn new(device_id: DeviceId) -> Self {
        CursorEntered { device_id }
    }
}

impl Event for CursorEntered {
    type Output = Self;

    fn invoke(self, _: &mut World) -> Option<Self::Output> {
        Some(self)
    }
}

pub struct CursorLeft {
    pub device_id: DeviceId,
}

impl CursorLeft {
    pub fn new(device_id: DeviceId) -> Self {
        CursorLeft { device_id }
    }
}

impl Event for CursorLeft {
    type Output = Self;

    fn invoke(self, _: &mut World) -> Option<Self::Output> {
        Some(self)
    }
}

pub struct MouseWheel {
    pub device_id: DeviceId,
    pub delta: MouseScrollDelta,
    pub phase: TouchPhase,
}

impl MouseWheel {
    pub fn new(device_id: DeviceId, delta: MouseScrollDelta, phase: TouchPhase) -> Self {
        MouseWheel {
            device_id,
            delta,
            phase,
        }
    }
}

impl Event for MouseWheel {
    type Output = Self;

    fn invoke(self, _: &mut World) -> Option<Self::Output> {
        Some(self)
    }
}

pub struct MouseInput {
    pub device_id: DeviceId,
    pub state: ElementState,
    pub button: MouseButton,
}

impl MouseInput {
    pub fn new(device_id: DeviceId, state: ElementState, button: MouseButton) -> Self {
        MouseInput {
            device_id,
            state,
            button,
        }
    }
}

impl Event for MouseInput {
    type Output = Self;

    fn invoke(self, _: &mut World) -> Option<Self::Output> {
        Some(self)
    }
}

pub struct PinchGesture {
    pub device_id: DeviceId,
    pub delta: f64,
    pub phase: TouchPhase,
}

impl PinchGesture {
    pub fn new(device_id: DeviceId, delta: f64, phase: TouchPhase) -> Self {
        PinchGesture {
            device_id,
            delta,
            phase,
        }
    }
}

impl Event for PinchGesture {
    type Output = Self;

    fn invoke(self, _: &mut World) -> Option<Self::Output> {
        Some(self)
    }
}

pub struct PanGesture {
    pub device_id: DeviceId,
    pub delta_x: f32,
    pub delta_y: f32,
    pub phase: TouchPhase,
}

impl PanGesture {
    pub fn new(device_id: DeviceId, delta_x: f32, delta_y: f32, phase: TouchPhase) -> Self {
        PanGesture {
            device_id,
            delta_x,
            delta_y,
            phase,
        }
    }
}

impl Event for PanGesture {
    type Output = Self;

    fn invoke(self, _: &mut World) -> Option<Self::Output> {
        Some(self)
    }
}

pub struct DoubleTapGesture {
    pub device_id: DeviceId,
}

impl DoubleTapGesture {
    pub fn new(device_id: DeviceId) -> Self {
        DoubleTapGesture { device_id }
    }
}

impl Event for DoubleTapGesture {
    type Output = Self;

    fn invoke(self, _: &mut World) -> Option<Self::Output> {
        Some(self)
    }
}

pub struct RotationGesture {
    pub device_id: DeviceId,
    pub delta: f32,
    pub phase: TouchPhase,
}

impl RotationGesture {
    pub fn new(device_id: DeviceId, delta: f32, phase: TouchPhase) -> Self {
        RotationGesture {
            device_id,
            delta,
            phase,
        }
    }
}

impl Event for RotationGesture {
    type Output = Self;

    fn invoke(self, _: &mut World) -> Option<Self::Output> {
        Some(self)
    }
}

pub struct TouchpadPressure {
    pub device_id: DeviceId,
    pub pressure: f32,
    pub stage: i64,
}

impl TouchpadPressure {
    pub fn new(device_id: DeviceId, pressure: f32, stage: i64) -> Self {
        TouchpadPressure {
            device_id,
            pressure,
            stage,
        }
    }
}

impl Event for TouchpadPressure {
    type Output = Self;

    fn invoke(self, _: &mut World) -> Option<Self::Output> {
        Some(self)
    }
}

pub struct Touch(winit::event::Touch);

impl Touch {
    pub fn new(touch: winit::event::Touch) -> Self {
        Touch(touch)
    }
}

impl std::ops::Deref for Touch {
    type Target = winit::event::Touch;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Event for Touch {
    type Output = Self;

    fn invoke(self, _: &mut World) -> Option<Self::Output> {
        Some(self)
    }
}

pub struct ScaleFactorChanged {
    pub scale_factor: f64,
}

impl ScaleFactorChanged {
    pub fn new(scale_factor: f64) -> Self {
        ScaleFactorChanged { scale_factor }
    }
}

impl Event for ScaleFactorChanged {
    type Output = Self;

    fn invoke(self, _: &mut World) -> Option<Self::Output> {
        Some(self)
    }
}

pub struct Occluded(bool);

impl Occluded {
    pub fn new(occluded: bool) -> Self {
        Occluded(occluded)
    }
}

impl std::ops::Deref for Occluded {
    type Target = bool;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Event for Occluded {
    type Output = Self;

    fn invoke(self, _: &mut World) -> Option<Self::Output> {
        Some(self)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct WindowCreated {
    id: WindowId,
}

impl WindowCreated {
    pub fn new(id: WindowId) -> Self {
        WindowCreated { id }
    }

    pub fn id(&self) -> WindowId {
        self.id
    }
}

impl Event for WindowCreated {
    type Output = Self;

    fn invoke(self, _: &mut World) -> Option<Self::Output> {
        Some(self)
    }
}
