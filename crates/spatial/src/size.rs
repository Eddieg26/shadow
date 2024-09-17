#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Size {
    pub width: u32,
    pub height: u32,
}

impl Size {
    pub const ZERO: Self = Self::new(0, 0);

    pub const fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }
}
