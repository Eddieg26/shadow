pub struct WindowBuilder {
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

impl WindowBuilder {
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
