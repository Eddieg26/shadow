use crate::{
    app::{App, AppRunError},
    events::{
        AxisMotion, CloseRequested, CursorEntered, CursorLeft, CursorMoved, Destroyed,
        DoubleTapGesture, DroppedFile, Focused, HoveredFile, HoveredFileCancelled, KeyboardInput,
        ModifiersChanged, MouseInput, MouseWheel, Moved, Occluded, PanGesture, PinchGesture,
        Resized, RotationGesture, ScaleFactorChanged, TouchpadPressure, WindowCreated,
    },
    window::WindowConfig,
};
use game::{plugin::Plugin, Game};

pub struct WindowPlugin;

impl Plugin for WindowPlugin {
    fn run(&mut self, game: &mut Game) {
        game.register_event::<AppRunError>()
            .register_event::<WindowCreated>()
            .register_event::<CloseRequested>()
            .register_event::<AxisMotion>()
            .register_event::<Resized>()
            .register_event::<Moved>()
            .register_event::<Destroyed>()
            .register_event::<DroppedFile>()
            .register_event::<HoveredFile>()
            .register_event::<HoveredFileCancelled>()
            .register_event::<Focused>()
            .register_event::<KeyboardInput>()
            .register_event::<ModifiersChanged>()
            .register_event::<CursorMoved>()
            .register_event::<CursorEntered>()
            .register_event::<CursorLeft>()
            .register_event::<MouseWheel>()
            .register_event::<MouseInput>()
            .register_event::<PinchGesture>()
            .register_event::<PanGesture>()
            .register_event::<DoubleTapGesture>()
            .register_event::<RotationGesture>()
            .register_event::<TouchpadPressure>()
            .register_event::<TouchpadPressure>()
            .register_event::<ScaleFactorChanged>()
            .register_event::<Occluded>()
            .set_runner(App::runner);
    }

    fn finish(&mut self, game: &mut Game) {
        if let None = game.try_resource::<WindowConfig>() {
            game.add_resource(WindowConfig::new("Window"));
        }
    }
}