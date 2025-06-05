use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowProps {
	#[serde(default = "default::yes")]
	pub transparent: bool,
	#[serde(default = "default::yes")]
	pub resizable: bool,
	#[serde(default = "default::config::window::width")]
	pub width: u32,
	#[serde(default = "default::config::window::height")]
	pub height: u32,
}

impl Default for WindowProps {
	fn default() -> Self {
		Self {
			transparent: default::yes(),
			resizable: default::yes(),
			width: default::config::window::width(),
			height: default::config::window::height(),
		}
	}
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BoxPlacement {
	Inside,
	Outside,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScrollDirection {
	#[default]
	Up,
	Down,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
	pub speed: u32,

	#[serde(default)]
	pub direction: ScrollDirection,

	#[serde(default = "default::yes")]
	pub display_keys: bool,
	#[serde(default = "default::config::key_placement")]
	pub key_placement: BoxPlacement,

	#[serde(default = "default::yes")]
	pub display_counters: bool,
	#[serde(default = "default::config::counter_placement")]
	pub counter_placement: BoxPlacement,

	#[serde(default = "default::config::key_spacing")]
	pub key_spacing: u32,
	#[serde(default = "default::config::default_key_width")]
	pub default_key_width: u32,
	#[serde(default = "default::config::key_height")]
	pub key_height: u32,

	#[serde(default)]
	pub window: WindowProps,

	pub columns: Vec<ColumnProps>,
}

impl Default for Config {
	fn default() -> Self {
		Config {
			speed: 300,
			direction: ScrollDirection::default(),
			window: WindowProps::default(),
			display_keys: default::yes(),
			key_placement: default::config::key_placement(),
			display_counters: default::yes(),
			counter_placement: default::config::counter_placement(),
			key_spacing: default::config::key_spacing(),
			default_key_width: default::config::default_key_width(),
			key_height: default::config::key_height(),
			columns: vec![
				ColumnProps::new(None, [rdev::Key::KeyD].into()),
				ColumnProps::new(None, [rdev::Key::KeyF].into()),
				ColumnProps::new(None, [rdev::Key::KeyJ].into()),
				ColumnProps::new(None, [rdev::Key::KeyK].into()),
			],
		}
	}
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnProps {
	pub name: Option<String>,
	pub keys: Vec<rdev::Key>,
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
	fn new(name: Option<String>, keys: Vec<rdev::Key>) -> ColumnProps {
		ColumnProps {
			name,
			keys,
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
		use crate::config::BoxPlacement;

		pub mod window {
			pub fn width() -> u32 {
				420
			}

			pub fn height() -> u32 {
				690
			}
		}

		pub fn key_placement() -> BoxPlacement {
			BoxPlacement::Inside
		}

		pub fn counter_placement() -> BoxPlacement {
			BoxPlacement::Outside
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
