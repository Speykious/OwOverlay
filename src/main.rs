use core::fmt;
use std::collections::{HashMap, HashSet, VecDeque};
use std::error::Error;
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::SystemTime;
use std::{fs, io, thread};

use app::OwOverlayApp;
use app_frame::AppFrame;
use clap::Parser;
use config::{default_config, ColumnProps, Config};
use draw::{center_from, rect, Anchor};
use glam::{vec2, Vec2};
use key::{display_key, OwoKey};
use loki_draw::drawer::{Drawer, RectBlueprint, TextBlueprint};
use loki_draw::font::Font;
use loki_draw::rect::Rect;
use winit::dpi::PhysicalSize;
use winit::window::WindowBuilder;

mod app;
mod app_frame;
mod config;
mod draw;
mod key;

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
	pub props: ColumnProps,
	pub times: VecDeque<SystemTime>,
}

impl fmt::Display for KeyColumn {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		let x = if self.pressed { "x" } else { " " };
		write!(f, "({}) [{:?}] {} (#T={})", x, self.key, self.count, self.times.len())
	}
}

impl KeyColumn {
	pub fn new(key: rdev::Key, props: ColumnProps) -> Self {
		Self {
			key,
			count: 0,
			pressed: false,
			props,
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
	display_keys: bool,
	display_counters: bool,
	key_spacing: f32,
	default_key_width: f32,
	key_height: f32,
}

impl KeyOverlayScene {
	fn new(
		keyboard_rx: mpsc::Receiver<KeyEvent>,
		config: &Config,
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

			speed: config.speed as f32,
			display_keys: config.display_keys,
			display_counters: config.display_counters,
			key_spacing: config.key_spacing as f32,
			default_key_width: config.default_key_width as f32,
			key_height: config.key_height as f32,
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
		}

		self.now = SystemTime::now();
	}

	fn draw(&self, viewport: Vec2, drawer: &mut impl Drawer) {
		drawer.clear();
		drawer.begin_frame();
		{
			let key_size = vec2(self.default_key_width, self.key_height);
			let spacing = self.key_spacing;
			let bottom_y = viewport.y - 30.;
			let n_columns = self.columns.len() as f32;

			for (i, key) in self.keys.iter().enumerate() {
				let column = self.columns.get(key).unwrap();

				let color = match column.pressed {
					true => column.props.hover_color,
					false => 0x111111,
				};

				let i = i as f32 + 0.5;
				let x_offset = (i - n_columns / 2.) * (key_size.x + spacing / 2.);
				let key_pos = vec2(viewport.x / 2. + x_offset, bottom_y);

				let center_pos = center_from(key_pos, key_size, Anchor::BC);

				// key rectangle
				drawer.draw_rect(&RectBlueprint {
					rect: rect(center_pos, key_size),
					color,
					border_color: column.props.border_color,
					border_width: 8.,
					corner_radius: 2.,
					borders: [true, true, true, true],
					alpha: 1.,
				});

				// key name
				if self.display_keys {
					drawer.draw_text(&TextBlueprint {
						text: display_key(column.key),
						x: key_pos.x - key_size.x / 2. + 5.,
						y: key_pos.y + 10.,
						font: &self.default_font,
						size: 20.,
						col: 0xeeeeee,
						alpha: 1.,
					});
				}

				// counter
				if self.display_counters {
					drawer.draw_text(&TextBlueprint {
						text: &format!("{}", column.count),
						x: key_pos.x - key_size.x / 2. + 15.,
						y: key_pos.y - key_size.y / 2. - 10.,
						font: &self.default_font,
						size: 25.,
						col: 0xeeeeee,
						alpha: 1.,
					});
				}

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
									w: key_size.x,
									h,
								},
								color: column.props.color,
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

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
	#[arg(short, long, help = "Path to the config")]
	config_path: Option<PathBuf>,
	#[arg(short, long, help = "Name of a config stored in the config directory")]
	preset: Option<PathBuf>,
}

fn main() -> Result<(), Box<dyn Error>> {
	let Cli { config_path, preset } = Cli::parse();

	let config_dir = dirs::config_dir()
		.expect("You don't have a config directory???")
		.join("OwOverlay");

	fs::create_dir_all(&config_dir)?;

	let config_path = match (config_path, preset) {
		(Some(_), Some(_)) => panic!("Can't specify the config path and a preset at the same time!"),
		(Some(config_path), None) => config_path,
		(None, Some(preset)) => config_dir.join(preset).with_extension("toml"),
		(None, None) => config_dir.join("cowonfig.toml"),
	};

	let config = match fs::read_to_string(&config_path) {
		Ok(c) => toml::from_str(&c)?,
		Err(e) if e.kind() == io::ErrorKind::NotFound => {
			let config = default_config();
			fs::write(&config_path, toml::to_string(&config)?)?;
			config
		}
		Err(e) => return Err(e.into()),
	};

	let mut keys = HashSet::new();
	let mut key_columns = Vec::new();
	for column in config.columns.iter().cloned() {
		let key: OwoKey = column.key.parse()?;
		keys.insert(key.0);
		key_columns.push(KeyColumn::new(key.0, column));
	}

	let (keyboard_tx, keyboard_rx) = mpsc::channel::<KeyEvent>();

	let scene = KeyOverlayScene::new(keyboard_rx, &config, key_columns);

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

	let (width, height) = (800, 500);

	let app_frame = AppFrame::init(
		WindowBuilder::new()
			.with_transparent(config.transparent_window)
			.with_resizable(true)
			.with_inner_size(PhysicalSize::new(width, height)),
	)?;

	app_frame.run(OwOverlayApp::new(width, height, scene))
}
