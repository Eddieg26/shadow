use shadow_ecs::core::{DenseMap, Resource};
use winit::{event_loop::ActiveEventLoop, window::WindowId};

pub struct WindowConfig {
    width: u32,
    height: u32,
    title: String,
    resizable: bool,
    visible: bool,
    blur: bool,
    transparent: bool,
    maximized: bool,
    decorations: bool,
}

impl WindowConfig {
    pub fn new(title: impl ToString) -> Self {
        Self {
            width: 800,
            height: 600,
            title: title.to_string(),
            resizable: true,
            visible: true,
            blur: false,
            transparent: false,
            maximized: false,
            decorations: true,
        }
    }

    pub fn with_title(mut self, title: &str) -> Self {
        self.title = title.to_string();
        self
    }

    pub fn with_size(mut self, width: u32, height: u32) -> Self {
        self.width = width;
        self.height = height;
        self
    }

    pub fn with_resizable(mut self, resizable: bool) -> Self {
        self.resizable = resizable;
        self
    }

    pub fn with_visible(mut self, visible: bool) -> Self {
        self.visible = visible;
        self
    }

    pub fn with_blur(mut self, blur: bool) -> Self {
        self.blur = blur;
        self
    }

    pub fn with_transparent(mut self, transparent: bool) -> Self {
        self.transparent = transparent;
        self
    }

    pub fn with_maximized(mut self, maximized: bool) -> Self {
        self.maximized = maximized;
        self
    }

    pub fn with_decorations(mut self, decorations: bool) -> Self {
        self.decorations = decorations;
        self
    }
}

pub struct Window {
    inner: winit::window::Window,
}

impl Window {
    pub fn new(config: WindowConfig, event_loop: &ActiveEventLoop) -> Self {
        let attributes = winit::window::Window::default_attributes()
            .with_title(config.title)
            .with_inner_size(winit::dpi::PhysicalSize::new(config.width, config.height))
            .with_resizable(config.resizable)
            .with_visible(config.visible)
            .with_transparent(config.transparent)
            .with_maximized(config.maximized)
            .with_decorations(config.decorations);

        let inner = event_loop.create_window(attributes).unwrap();

        Self { inner }
    }
}

impl std::ops::Deref for Window {
    type Target = winit::window::Window;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl std::ops::DerefMut for Window {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

#[derive(Default)]
pub struct Windows {
    windows: DenseMap<WindowId, Window>,
    configs: Vec<WindowConfig>,
}

impl Windows {
    pub fn new() -> Self {
        Self {
            windows: DenseMap::new(),
            configs: Vec::new(),
        }
    }

    pub fn get(&self, id: &WindowId) -> Option<&Window> {
        self.windows.get(id)
    }

    pub fn add_config(&mut self, config: WindowConfig) {
        self.configs.push(config);
    }

    pub fn create_windows(&mut self, event_loop: &ActiveEventLoop) -> Vec<WindowId> {
        let mut ids = Vec::new();
        for config in self.configs.drain(..) {
            let window = Window::new(config, event_loop);
            ids.push(window.id());
            self.windows.insert(window.id(), window);
        }

        ids
    }

    pub fn remove_window(&mut self, id: &WindowId) -> Option<Window> {
        self.windows.remove(id)
    }

    pub fn len(&self) -> usize {
        self.windows.len()
    }

    pub fn is_empty(&self) -> bool {
        self.windows.is_empty()
    }
}

impl Resource for Windows {}
