use core::fmt;
use std::collections::{HashMap, HashSet, VecDeque};
use std::error::Error;
use std::sync::mpsc;
use std::time::SystemTime;
use std::{fs, thread};

use config::Config;
use draw::{center_from, square, Anchor};
use glam::{vec2, Vec2};
use key::{display_key, OwoKey};
use loki_draw::drawer::{Drawer, RectBlueprint, TextBlueprint};
use loki_draw::font::Font;
use loki_draw::rect::Rect;

use crate::window::spawn_window;

mod config;
mod draw;
mod key;
mod opengl;
mod window;

const ROBOTO_FONT: &[u8] = include_bytes!("../assets/Roboto-Regular.ttf");

pub trait Scene {
	fn update(&mut self);
	fn draw(&self, viewport: Vec2, drawer: &mut impl Drawer);
}

#[derive(Clone)]
struct KeyColumn {
	pub key: rdev::Key,
	pub count: u64,
	pub pressed: bool,
	pub times: VecDeque<SystemTime>,
}

impl fmt::Display for KeyColumn {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		let x = if self.pressed { "x" } else { " " };
		write!(f, "({}) [{:?}] {} (#T={})", x, self.key, self.count, self.times.len())
	}
}

impl KeyColumn {
	pub fn new(key: rdev::Key) -> Self {
		Self {
			key,
			count: 0,
			pressed: false,
			times: VecDeque::with_capacity(1024),
		}
	}

	pub fn toggle_key(&mut self, time: SystemTime) {
		self.pressed = !self.pressed;

		if self.pressed {
			self.count += 1;
		}

		if self.times.len() >= 1024 {
			self.times.pop_back();
		}

		self.times.push_front(time);
	}
}

#[derive(Debug, Clone)]
struct KeyEvent {
	pub key: rdev::Key,
	pub pressed: bool,
	pub time: SystemTime,
}

struct KeyOverlayScene {
	keys: Vec<rdev::Key>,
	columns: HashMap<rdev::Key, KeyColumn>,
	default_font: Font<'static>,
	keyboard_rx: mpsc::Receiver<KeyEvent>,
	now: SystemTime,
	speed: f32,
}

impl KeyOverlayScene {
	fn new(
		keyboard_rx: mpsc::Receiver<KeyEvent>,
		speed: f32,
		key_columns: impl IntoIterator<Item = KeyColumn>,
	) -> Self {
		let mut keys = Vec::new();

		let columns = key_columns
			.into_iter()
			.map(|kc| {
				keys.push(kc.key);
				(kc.key, kc)
			})
			.collect();

		Self {
			keys,
			columns,
			default_font: Font::from_data(ROBOTO_FONT),
			keyboard_rx,
			now: SystemTime::now(),
			speed,
		}
	}

	fn time_to_secs(&self, time: SystemTime) -> f32 {
		let time = time.duration_since(SystemTime::UNIX_EPOCH).unwrap().as_millis();
		let now = self.now.duration_since(SystemTime::UNIX_EPOCH).unwrap().as_millis();

		if time > now {
			(time - now) as f32 / 1000.
		} else {
			(now - time) as f32 / -1000.
		}
	}
}

impl Scene for KeyOverlayScene {
	fn update(&mut self) {
		while let Ok(key_event) = self.keyboard_rx.try_recv() {
			let column = self.columns.get_mut(&key_event.key).unwrap();

			if column.pressed != key_event.pressed {
				column.toggle_key(key_event.time);
			}

			println!("{}", column);
		}

		self.now = SystemTime::now();
	}

	fn draw(&self, viewport: Vec2, drawer: &mut impl Drawer) {
		drawer.clear();
		drawer.begin_frame();
		{
			let key_size = 100.;
			let spacing = 10.;
			let bottom_y = viewport.y - 30.;
			let n_columns = self.columns.len() as f32;
			let even_columns = self.columns.len() % 2 == 0;

			for (i, key) in self.keys.iter().enumerate() {
				let column = self.columns.get(key).unwrap();

				let color = match column.pressed {
					true => 0x555555,
					false => 0x111111,
				};

				let i = if even_columns { i as f32 + 0.5 } else { i as f32 };
				let x_offset = (i - n_columns / 2.) * (key_size + spacing / 2.);
				let key_pos = vec2(viewport.x / 2. + x_offset, bottom_y);

				let center_pos = center_from(key_pos, key_size, Anchor::BC);

				// key square
				drawer.draw_rect(&RectBlueprint {
					rect: square(center_pos, key_size),
					color,
					border_color: 0xeeeeee,
					border_width: 8.,
					corner_radius: 2.,
					borders: [true, true, true, true],
					alpha: 1.,
				});

				// key name
				drawer.draw_text(&TextBlueprint {
					text: display_key(column.key),
					x: key_pos.x - key_size / 2. + 5.,
					y: key_pos.y + 10.,
					font: &self.default_font,
					size: 20.,
					col: 0xeeeeee,
					alpha: 1.,
				});

				// counter
				drawer.draw_text(&TextBlueprint {
					text: &format!("{}", column.count),
					x: key_pos.x - key_size / 2. + 15.,
					y: key_pos.y - key_size / 2. - 10.,
					font: &self.default_font,
					size: 25.,
					col: 0xeeeeee,
					alpha: 1.,
				});

				// history rectangles
				let mut opt_prev_time: Option<SystemTime> = column.pressed.then_some(self.now);

				for time in column.times.iter().copied() {
					match opt_prev_time {
						Some(prev_time) => {
							let time_secs = self.time_to_secs(time);
							let prev_time_secs = self.time_to_secs(prev_time);
							let h = ((time_secs - prev_time_secs) * self.speed).min(viewport.y);
							let y = (prev_time_secs - self.time_to_secs(self.now)) * self.speed;
							let y = y + center_pos.y;

							// stop drawing rectangles once off-screen
							if y <= 0. {
								break;
							}

							drawer.draw_rect(&RectBlueprint {
								rect: Rect {
									x: center_pos.x,
									y,
									w: key_size,
									h,
								},
								color: 0x63ffec,
								border_color: 0x000000,
								border_width: 0.,
								corner_radius: 0.,
								borders: [false, false, false, false],
								alpha: 0.5,
							});

							opt_prev_time = None;
						}
						None => opt_prev_time = Some(time),
					}
				}
			}
		}
		drawer.end_frame();
	}
}

fn main() -> Result<(), Box<dyn Error>> {
	let cowonfig = fs::read_to_string("cowonfig.toml")?;
	let config: Config = toml::from_str(&cowonfig)?;

	let mut keys = HashSet::new();
	let mut key_columns = Vec::new();
	for column in &config.columns {
		let key: OwoKey = column.key.parse()?;
		keys.insert(key.0);
		key_columns.push(KeyColumn::new(key.0));
	}

	let (keyboard_tx, keyboard_rx) = mpsc::channel::<KeyEvent>();

	let mut scene = KeyOverlayScene::new(keyboard_rx, config.speed as f32, key_columns);

	thread::Builder::new()
		.name("Global Keyboard Listener".to_string())
		.spawn(move || {
			let result = rdev::listen(move |event| {
				let (key, pressed) = match event.event_type {
					rdev::EventType::KeyPress(k) => (k, true),
					rdev::EventType::KeyRelease(k) => (k, false),
					_ => return,
				};

				if !keys.contains(&key) {
					return;
				}

				let result = keyboard_tx.send(KeyEvent {
					key,
					time: event.time,
					pressed,
				});

				if let Err(e) = result {
					eprintln!("ERROR (tx.send): {}", e);
				}
			});

			if let Err(e) = result {
				eprintln!("ERROR (listen): {:?}", e)
			}
		})?;

	spawn_window(&mut scene)?;

	Ok(())
}
