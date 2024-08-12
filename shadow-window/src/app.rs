use crate::{
    events::{
        AxisMotion, CloseRequested, CursorEntered, CursorLeft, CursorMoved, Destroyed,
        DoubleTapGesture, DroppedFile, Focused, HoveredFile, HoveredFileCancelled, KeyboardInput,
        ModifiersChanged, MouseInput, MouseWheel, Moved, Occluded, PanGesture, PinchGesture,
        Resized, RotationGesture, ScaleFactorChanged, Touch, TouchpadPressure, WindowCreated,
    },
    window::Windows,
};
use shadow_ecs::world::event::Event;
use shadow_game::game::Game;
use winit::{
    application::ApplicationHandler,
    error::EventLoopError,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::WindowId,
};

pub struct App<'a> {
    game: &'a mut Game,
}

impl<'a> App<'a> {
    pub fn new(game: &'a mut Game) -> Self {
        Self { game }
    }

    pub fn start(&mut self) {
        self.game.start();
    }

    pub fn update(&mut self) {
        self.game.update();
    }

    pub fn shutdown(&mut self) {
        self.game.shutdown();
    }

    fn run_event<E: Event>(&mut self, event: E) {
        self.game.events().add(event);
        self.game.flush_events::<E>();
    }

    fn run(&mut self, event_loop: EventLoop<()>) {
        event_loop.set_control_flow(ControlFlow::Poll);

        self.start();

        if let Err(e) = event_loop.run_app(self) {
            self.run_event(AppRunError::new(e));
        }

        self.shutdown();
    }

    pub fn runner(game: &mut Game) {
        match EventLoop::new() {
            Ok(event_loop) => {
                let mut app = App::new(game);
                app.run(event_loop)
            }
            Err(_) => {}
        }
    }
}

impl ApplicationHandler for App<'_> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        {
            let windows = self.game.resource_mut::<Windows>();
            for id in windows.create_windows(event_loop) {
                self.game.events().add(WindowCreated::new(id));
            }
        }

        self.game.flush_events::<WindowCreated>();
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, window: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                if let Some(window) = self.game.resource_mut::<Windows>().remove_window(&window) {
                    self.run_event(CloseRequested::new(window.id()));
                }

                if self.game.resource::<Windows>().is_empty() {
                    event_loop.exit();
                }
            }
            WindowEvent::RedrawRequested => self.update(),
            WindowEvent::AxisMotion {
                device_id,
                axis,
                value,
            } => self.run_event(AxisMotion::new(device_id, axis, value)),
            WindowEvent::Resized(size) => self.run_event(Resized::from(size)),
            WindowEvent::Moved(position) => self.run_event(Moved::from(position)),
            WindowEvent::Destroyed => self.run_event(Destroyed::new(window)),
            WindowEvent::DroppedFile(path) => self.run_event(DroppedFile::new(path)),
            WindowEvent::HoveredFile(path) => self.run_event(HoveredFile::new(path)),
            WindowEvent::HoveredFileCancelled => self.run_event(HoveredFileCancelled),
            WindowEvent::Focused(focused) => self.run_event(Focused::new(focused)),
            WindowEvent::KeyboardInput {
                device_id,
                event,
                is_synthetic,
            } => self.run_event(KeyboardInput::new(device_id, event, is_synthetic)),
            WindowEvent::ModifiersChanged(modifiers) => {
                self.run_event(ModifiersChanged::new(modifiers))
            }
            WindowEvent::CursorMoved {
                device_id,
                position,
            } => self.run_event(CursorMoved::new(device_id, position.x, position.y)),
            WindowEvent::CursorEntered { device_id } => {
                self.run_event(CursorEntered::new(device_id))
            }
            WindowEvent::CursorLeft { device_id } => self.run_event(CursorLeft::new(device_id)),
            WindowEvent::MouseWheel {
                device_id,
                delta,
                phase,
            } => self.run_event(MouseWheel::new(device_id, delta, phase)),
            WindowEvent::MouseInput {
                device_id,
                state,
                button,
            } => self.run_event(MouseInput::new(device_id, state, button)),
            WindowEvent::PinchGesture {
                device_id,
                delta,
                phase,
            } => self.run_event(PinchGesture::new(device_id, delta, phase)),
            WindowEvent::PanGesture {
                device_id,
                delta,
                phase,
            } => self.run_event(PanGesture::new(device_id, delta.x, delta.y, phase)),
            WindowEvent::DoubleTapGesture { device_id } => {
                self.run_event(DoubleTapGesture::new(device_id))
            }
            WindowEvent::RotationGesture {
                device_id,
                delta,
                phase,
            } => self.run_event(RotationGesture::new(device_id, delta, phase)),
            WindowEvent::TouchpadPressure {
                device_id,
                pressure,
                stage,
            } => self.run_event(TouchpadPressure::new(device_id, pressure, stage)),
            WindowEvent::Touch(touch) => self.run_event(Touch::new(touch)),
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                self.run_event(ScaleFactorChanged::new(scale_factor))
            }
            WindowEvent::Occluded(occluded) => self.run_event(Occluded::new(occluded)),
            _ => {}
        }
    }
}

pub struct AppRunError(EventLoopError);

impl AppRunError {
    fn new(error: EventLoopError) -> Self {
        Self(error)
    }

    pub fn error(&self) -> &EventLoopError {
        &self.0
    }
}

impl std::ops::Deref for AppRunError {
    type Target = EventLoopError;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Event for AppRunError {
    type Output = Self;

    fn invoke(self, _: &mut shadow_ecs::world::World) -> Option<Self::Output> {
        Some(self)
    }
}
