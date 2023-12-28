use serde::{Deserialize, Serialize};

use crate::key::serialize_key;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
	pub speed: u32,

	#[serde(default = "default::config::key_spacing")]
	pub key_spacing: u32,
	#[serde(default = "default::config::default_key_width")]
	pub default_key_width: u32,
	#[serde(default = "default::config::key_height")]
	pub key_height: u32,

	pub columns: Vec<Column>,
}

pub fn default_config() -> Config {
	Config {
		speed: 300,
		key_spacing: default::config::key_spacing(),
		default_key_width: default::config::default_key_width(),
		key_height: default::config::key_height(),
		columns: vec![
			Column::new(rdev::Key::KeyD),
			Column::new(rdev::Key::KeyF),
			Column::new(rdev::Key::KeyJ),
			Column::new(rdev::Key::KeyK),
		],
	}
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Column {
	pub key: String,
	#[serde(default = "default::column::width")]
	pub width: u32,
	#[serde(default = "default::column::color")]
	pub color: u32,
	#[serde(default = "default::column::border_color")]
	pub border_color: u32,
	#[serde(default = "default::column::alpha")]
	pub alpha: f32,
}

impl Column {
	fn new(key: rdev::Key) -> Column {
		Column {
			key: serialize_key(key),
			width: default::column::width(),
			color: default::column::color(),
			border_color: default::column::border_color(),
			alpha: default::column::alpha(),
		}
	}
}

mod default {
	pub mod config {
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

		pub fn border_color() -> u32 {
			0xeeeeee
		}

		pub fn alpha() -> f32 {
			0.5
		}
	}
}
