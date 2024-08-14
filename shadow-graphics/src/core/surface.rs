use shadow_ecs::core::Resource;
use shadow_window::window::Window;
use wgpu::{
    rwh::{HasDisplayHandle, HasWindowHandle},
    SurfaceTargetUnsafe,
};

use crate::resources::GpuResourceId;

pub enum RenderSurfaceError {
    Create(wgpu::CreateSurfaceError),
    Adapter,
    DisplayHandle(String),
    WindowHandle(String),
}

impl From<wgpu::CreateSurfaceError> for RenderSurfaceError {
    fn from(error: wgpu::CreateSurfaceError) -> Self {
        Self::Create(error)
    }
}

pub struct RenderSurface {
    id: GpuResourceId,
    inner: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    adapter: wgpu::Adapter,
    format: wgpu::TextureFormat,
    depth_format: Option<wgpu::TextureFormat>,
}

impl RenderSurface {
    pub async fn create(
        instance: &wgpu::Instance,
        window: &Window,
    ) -> Result<Self, RenderSurfaceError> {
        let surface = unsafe {
            let display_handle = window
                .inner()
                .display_handle()
                .map_err(|e| RenderSurfaceError::DisplayHandle(e.to_string()))?;

            let window_handle = window
                .inner()
                .window_handle()
                .map_err(|e| RenderSurfaceError::WindowHandle(e.to_string()))?;

            let target = SurfaceTargetUnsafe::RawHandle {
                raw_display_handle: display_handle.into(),
                raw_window_handle: window_handle.into(),
            };
            instance
                .create_surface_unsafe(target)
                .map_err(|e| RenderSurfaceError::from(e))?
        };

        let size = window.size();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                ..Default::default()
            })
            .await
            .ok_or(RenderSurfaceError::Adapter)?;

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(surface_caps.formats[0]);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 3,
        };

        Ok(Self {
            id: GpuResourceId::gen(),
            inner: surface,
            config,
            adapter,
            format: surface_format,
            depth_format: None,
        })
    }

    pub fn id(&self) -> GpuResourceId {
        self.id
    }

    pub fn resize(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        self.config.width = width;
        self.config.height = height;
        self.inner.configure(device, &self.config);
    }

    pub fn configure(&mut self, device: &wgpu::Device) {
        self.inner.configure(device, &self.config);
    }

    pub fn inner(&self) -> &wgpu::Surface {
        &self.inner
    }

    pub fn config(&self) -> &wgpu::SurfaceConfiguration {
        &self.config
    }

    pub fn adapter(&self) -> &wgpu::Adapter {
        &self.adapter
    }

    pub fn width(&self) -> u32 {
        self.config.width
    }

    pub fn height(&self) -> u32 {
        self.config.height
    }

    pub fn format(&self) -> wgpu::TextureFormat {
        self.format
    }

    pub fn depth_format(&self) -> Option<wgpu::TextureFormat> {
        self.depth_format
    }

    pub fn surface_texture(&self) -> Result<wgpu::SurfaceTexture, wgpu::SurfaceError> {
        self.inner.get_current_texture()
    }
}

impl Resource for RenderSurface {}
