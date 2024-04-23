use glam::{vec2, Vec2};
use loki_draw::drawer::Drawer;
use loki_draw::OpenglDrawer;
use winit::event::WindowEvent;
use winit::event_loop::EventLoopWindowTarget;

use crate::app_frame::App;
use crate::Scene;

pub struct OwOverlayApp<S: Scene> {
	pub drawer: Option<OpenglDrawer>,
	pub viewport: Vec2,
	pub scene: S,
}

impl<S: Scene> OwOverlayApp<S> {
	pub fn new(width: u32, height: u32, scene: S) -> Self {
		Self {
			drawer: None,
			viewport: vec2(width as f32, height as f32),
			scene,
		}
	}
}

impl<S: Scene> App for OwOverlayApp<S> {
	fn resume_window(&mut self) {
		self.drawer = Some(OpenglDrawer::new(self.viewport.x as u32, self.viewport.y as u32, 1.));
	}

	fn resize(&mut self, width: i32, height: i32) {
		self.viewport = vec2(width as f32, height as f32);

		if let Some(drawer) = &mut self.drawer {
			drawer.resize(self.viewport, 1.);
		}
	}

	fn draw(&mut self) {
		self.scene.update();

		if let Some(drawer) = &mut self.drawer {
			self.scene.draw(self.viewport, drawer);
		}
	}

	fn handle_window_event(&self, event: WindowEvent, elwt: &EventLoopWindowTarget<()>) {
		if event == WindowEvent::CloseRequested {
			elwt.exit();
		}
	}
}
