use serde::{Deserialize, Serialize};

use crate::key::serialize_key;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
	pub speed: u32,

	#[serde(default = "default::yes")]
	pub transparent_window: bool,

	#[serde(default = "default::config::window_width")]
	pub window_width: u32,
	#[serde(default = "default::config::window_height")]
	pub window_height: u32,

	#[serde(default = "default::yes")]
	pub display_keys: bool,
	#[serde(default = "default::yes")]
	pub display_counters: bool,

	#[serde(default = "default::config::key_spacing")]
	pub key_spacing: u32,
	#[serde(default = "default::config::default_key_width")]
	pub default_key_width: u32,
	#[serde(default = "default::config::key_height")]
	pub key_height: u32,

	pub columns: Vec<ColumnProps>,
}

pub fn default_config() -> Config {
	Config {
		speed: 300,
		transparent_window: default::yes(),
		window_width: default::config::window_width(),
		window_height: default::config::window_height(),
		display_keys: default::yes(),
		display_counters: default::yes(),
		key_spacing: default::config::key_spacing(),
		default_key_width: default::config::default_key_width(),
		key_height: default::config::key_height(),
		columns: vec![
			ColumnProps::new(rdev::Key::KeyD),
			ColumnProps::new(rdev::Key::KeyF),
			ColumnProps::new(rdev::Key::KeyJ),
			ColumnProps::new(rdev::Key::KeyK),
		],
	}
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnProps {
	pub key: String,
	#[serde(default = "default::column::width")]
	pub width: u32,
	#[serde(default = "default::column::color")]
	pub color: u32,
	#[serde(default = "default::column::hover_color")]
	pub hover_color: u32,
	#[serde(default = "default::column::border_color")]
	pub border_color: u32,
	#[serde(default = "default::column::alpha")]
	pub alpha: f32,
}

impl ColumnProps {
	fn new(key: rdev::Key) -> ColumnProps {
		ColumnProps {
			key: serialize_key(key),
			width: default::column::width(),
			color: default::column::color(),
			hover_color: default::column::hover_color(),
			border_color: default::column::border_color(),
			alpha: default::column::alpha(),
		}
	}
}

mod default {
	pub fn yes() -> bool {
		true
	}

	pub mod config {
		pub fn window_width() -> u32 {
			500
		}

		pub fn window_height() -> u32 {
			800
		}

		pub fn key_spacing() -> u32 {
			10
		}

		pub fn default_key_width() -> u32 {
			100
		}

		pub fn key_height() -> u32 {
			100
		}
	}

	pub mod column {
		pub fn width() -> u32 {
			100
		}

		pub fn color() -> u32 {
			0x63ffec
		}

		pub fn hover_color() -> u32 {
			0x555555
		}

		pub fn border_color() -> u32 {
			0xeeeeee
		}

		pub fn alpha() -> f32 {
			0.5
		}
	}
}
