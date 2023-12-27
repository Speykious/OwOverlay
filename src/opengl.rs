use std::env;
use std::error::Error;
use std::ffi::CString;
use std::num::NonZeroU32;

use glutin::context::{
    ContextApi, ContextAttributesBuilder, NotCurrentGlContext, PossiblyCurrentContext, Version,
};
use glutin::display::{Display, GetGlDisplay};
use glutin::prelude::{GlConfig, GlDisplay};
use glutin::surface::{Surface, SurfaceAttributesBuilder, WindowSurface};
use glutin_winit::ApiPreference;
use raw_window_handle::HasRawWindowHandle;
use winit::event_loop::EventLoop;
use winit::window::Window;

pub struct OpenglCtx {
    pub gl_ctx: PossiblyCurrentContext,
    pub gl_surface: Surface<WindowSurface>,
    pub gl_display: Display,
    pub window: Window,
    pub events: EventLoop<()>,
}

pub fn create_opengl_window(width: u32, height: u32) -> Result<OpenglCtx, Box<dyn Error>> {
    if cfg!(target_os = "linux") {
        // disables vsync sometimes on x11
        if env::var("vblank_mode").is_err() {
            env::set_var("vblank_mode", "0");
        }
    }

    let events = winit::event_loop::EventLoop::new()?;

    let window_builder = winit::window::WindowBuilder::new()
        .with_transparent(true)
        .with_resizable(true)
        .with_inner_size(winit::dpi::PhysicalSize::new(width, height))
        .with_title("OwOverlay");

    let (window, gl_config) = glutin_winit::DisplayBuilder::new()
        .with_preference(ApiPreference::FallbackEgl)
        .with_window_builder(Some(window_builder))
        .build(&events, <_>::default(), |configs| {
            configs
                .filter(|c| c.srgb_capable())
                .max_by_key(|c| c.num_samples())
                .unwrap()
        })?;

    let window = window.unwrap(); // set in display builder
    let raw_window_handle = window.raw_window_handle();
    let gl_display = gl_config.display();

    let context_attributes = ContextAttributesBuilder::new()
        .with_context_api(ContextApi::OpenGl(Some(Version::new(3, 1))))
        .with_profile(glutin::context::GlProfile::Core)
        .build(Some(raw_window_handle));

    let dimensions = window.inner_size();

    let (gl_surface, gl_ctx) = {
        let attrs = SurfaceAttributesBuilder::<glutin::surface::WindowSurface>::new().build(
            raw_window_handle,
            NonZeroU32::new(dimensions.width).unwrap(),
            NonZeroU32::new(dimensions.height).unwrap(),
        );

        let surface = unsafe { gl_display.create_window_surface(&gl_config, &attrs)? };
        let context = unsafe { gl_display.create_context(&gl_config, &context_attributes)? }
            .make_current(&surface)?;
        (surface, context)
    };

    // Load the OpenGL function pointers
    gl::load_with(|symbol| gl_display.get_proc_address(&CString::new(symbol).unwrap()) as *const _);

    Ok(OpenglCtx {
        gl_ctx,
        gl_surface,
        gl_display,
        window,
        events,
    })
}
