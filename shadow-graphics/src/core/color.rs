use shadow_asset::bytes::IntoBytes;

#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Color {
    pub r: f64,
    pub g: f64,
    pub b: f64,
    pub a: f64,
}

impl Color {
    pub const fn new(r: f64, g: f64, b: f64, a: f64) -> Self {
        Self { r, g, b, a }
    }

    pub fn r(&self) -> f64 {
        self.r
    }

    pub fn g(&self) -> f64 {
        self.g
    }

    pub fn b(&self) -> f64 {
        self.b
    }

    pub fn a(&self) -> f64 {
        self.a
    }

    pub const fn red() -> Self {
        Self::new(1.0, 0.0, 0.0, 1.0)
    }

    pub const fn green() -> Self {
        Self::new(0.0, 1.0, 0.0, 1.0)
    }

    pub const fn blue() -> Self {
        Self::new(0.0, 0.0, 1.0, 1.0)
    }

    pub const fn white() -> Self {
        Self::new(1.0, 1.0, 1.0, 1.0)
    }

    pub const fn black() -> Self {
        Self::new(0.0, 0.0, 0.0, 1.0)
    }
}

impl Default for Color {
    fn default() -> Self {
        Self::white()
    }
}

impl Into<wgpu::Color> for Color {
    fn into(self) -> wgpu::Color {
        wgpu::Color {
            r: self.r,
            g: self.g,
            b: self.b,
            a: self.a,
        }
    }
}

impl From<wgpu::Color> for Color {
    fn from(color: wgpu::Color) -> Self {
        Self {
            r: color.r,
            g: color.g,
            b: color.b,
            a: color.a,
        }
    }
}

impl From<(f64, f64, f64, f64)> for Color {
    fn from((r, g, b, a): (f64, f64, f64, f64)) -> Self {
        Self::new(r, g, b, a)
    }
}

impl From<Color> for (f64, f64, f64, f64) {
    fn from(color: Color) -> (f64, f64, f64, f64) {
        (color.r, color.g, color.b, color.a)
    }
}

impl IntoBytes for Color {
    fn into_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&self.r.into_bytes());
        bytes.extend_from_slice(&self.g.into_bytes());
        bytes.extend_from_slice(&self.b.into_bytes());
        bytes.extend_from_slice(&self.a.into_bytes());

        bytes
    }

    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        let r = f64::from_bytes(&bytes[0..8])?;
        let g = f64::from_bytes(&bytes[8..16])?;
        let b = f64::from_bytes(&bytes[16..24])?;
        let a = f64::from_bytes(&bytes[24..32])?;

        Some(Self::new(r, g, b, a))
    }
}
