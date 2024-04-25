use std::ops::{Deref, DerefMut};

use glam::Vec2;
use loki_draw::rect::Rect;

#[derive(Debug, Clone, Copy, Default)]
pub struct Anchor(Vec2);

impl Deref for Anchor {
	type Target = Vec2;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl DerefMut for Anchor {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.0
	}
}

#[allow(unused)]
impl Anchor {
	pub const TL: Anchor = Anchor(Vec2::new(0.0, 0.0));
	pub const TC: Anchor = Anchor(Vec2::new(0.5, 0.0));
	pub const TR: Anchor = Anchor(Vec2::new(1.0, 0.0));
	pub const CL: Anchor = Anchor(Vec2::new(0.0, 0.5));
	pub const CC: Anchor = Anchor(Vec2::new(0.5, 0.5));
	pub const CR: Anchor = Anchor(Vec2::new(1.0, 0.5));
	pub const BL: Anchor = Anchor(Vec2::new(0.0, 1.0));
	pub const BC: Anchor = Anchor(Vec2::new(0.5, 1.0));
	pub const BR: Anchor = Anchor(Vec2::new(1.0, 1.0));
}

#[derive(Debug, Clone, Default)]
pub struct OwoRect {
	pub pos: Vec2,
	pub size: Vec2,
	pub origin: Anchor,
}

impl OwoRect {
	pub fn top_left(&self) -> Vec2 {
		self.pos - self.size * Vec2::new(self.origin.x, self.origin.y)
	}

	pub fn center(&self) -> Vec2 {
		self.top_left() + self.size / 2.
	}

	pub fn anchor(&self, anchor: Anchor) -> Vec2 {
		self.top_left() + self.size * Vec2::new(anchor.x, anchor.y)
	}

	pub fn to_rect(&self) -> Rect {
		let tl = self.top_left();

		Rect {
			x: tl.x,
			y: tl.y,
			w: self.size.x,
			h: self.size.y,
		}
	}
}
