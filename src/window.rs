use std::error::Error;
use std::num::NonZeroU32;

use glam::vec2;
use glutin::surface::GlSurface;
use loki_draw::drawer::Drawer;
use loki_draw::OpenglDrawer;
use winit::event::{ElementState, Event, KeyEvent, WindowEvent};
use winit::event_loop::ControlFlow;
use winit::keyboard::{KeyCode, PhysicalKey};

use crate::opengl::{create_opengl_window, OpenglCtx};
use crate::Scene;

pub fn spawn_window(scene: &mut impl Scene) -> Result<(), Box<dyn Error>> {
	let (width, height) = (500, 800);

	let OpenglCtx {
		gl_ctx,
		gl_surface,
		gl_display,
		events,
		window,
	} = create_opengl_window(width, height)?;

	let mut drawer = OpenglDrawer::new(width, height, 1.);
	let mut viewport = vec2(width as f32, height as f32);

	// Event loop
	events.run(move |event, elwt| {
		// They need to be present
		let _gl_display = &gl_display;
		let _window = &window;

		elwt.set_control_flow(ControlFlow::Wait);

		match event {
			Event::WindowEvent { ref event, .. } => match event {
				WindowEvent::RedrawRequested => {
					scene.update();
					scene.draw(viewport, &mut drawer);

					gl_surface.swap_buffers(&gl_ctx).unwrap();
					window.request_redraw();
				}
				WindowEvent::Resized(physical_size) => {
					// Handle window resizing
					viewport = vec2(physical_size.width as f32, physical_size.height as f32);
					drawer.resize(viewport, 1.);

					gl_surface.resize(
						&gl_ctx,
						NonZeroU32::new(physical_size.width).unwrap(),
						NonZeroU32::new(physical_size.height).unwrap(),
					);
					window.request_redraw();
				}
				WindowEvent::CloseRequested => elwt.exit(),
				WindowEvent::KeyboardInput {
					event:
						KeyEvent {
							physical_key: PhysicalKey::Code(KeyCode::Escape),
							state: ElementState::Pressed,
							..
						},
					..
				} => elwt.exit(),
				_ => (),
			},
			Event::AboutToWait => {
				window.request_redraw();
			}
			_ => (),
		}
	})?;

	Ok(())
}
