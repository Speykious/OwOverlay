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

pub fn center_from(pos: Vec2, size: f32, anchor: Anchor) -> Vec2 {
	Vec2 {
		x: pos.x - size * (1. - anchor.x),
		y: pos.y - size * anchor.y,
	}
}

pub fn square(center: Vec2, size: f32) -> Rect {
	Rect {
		x: center.x,
		y: center.y,
		w: size,
		h: size,
	}
}
