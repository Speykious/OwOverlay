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
use config::{BoxPlacement, ColumnProps, Config, ScrollDirection};
use layout::{Anchor, OwoRect};
use glam::{vec2, Vec2};
use key::display_key;
use loki_draw::drawer::{Drawer, RectBlueprint, TextBlueprint};
use loki_draw::font::Font;
use loki_draw::rect::Rect;
use winit::dpi::PhysicalSize;
use winit::window::WindowBuilder;

mod app;
mod app_frame;
mod config;
mod layout;
mod key;

const ROBOTO_FONT: &[u8] = include_bytes!("../assets/Roboto-Regular.ttf");

pub trait Scene {
	fn update(&mut self);
	fn draw(&self, viewport: Vec2, drawer: &mut impl Drawer);
}

#[derive(Clone)]
struct KeyColumn {
	pub name: String,
	pub count: u64,
	pub pressed: bool,
	pub pressed_keys: HashMap<rdev::Key, bool>,
	pub props: ColumnProps,
	pub times: VecDeque<SystemTime>,
}

impl fmt::Display for KeyColumn {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		let x = if self.pressed { "x" } else { " " };
		write!(f, "({}) [", x)?;

		for key in &self.props.keys {
			write!(f, "{:?}", key)?;
		}

		write!(f, "] {} (#T={})", self.count, self.times.len())
	}
}

impl KeyColumn {
	pub fn new(props: ColumnProps) -> Self {
		let name = match &props.name {
			Some(name) => name.clone(),
			None => {
				let mut s = String::new();

				for &key in &props.keys {
					s += display_key(key);
				}

				s
			}
		};

		let pressed_keys = props.keys.iter().copied().map(|key| (key, false)).collect();

		Self {
			name,
			count: 0,
			pressed: false,
			pressed_keys,
			props,
			times: VecDeque::with_capacity(1024),
		}
	}

	pub fn set_key_pressed(&mut self, event: KeyEvent) {
		let Some(pressed_key) = self.pressed_keys.get_mut(&event.key) else {
			return;
		};

		if *pressed_key == event.pressed {
			return;
		}

		*pressed_key = event.pressed;

		if event.pressed {
			self.count += 1;
		}

		let prev_pressed = self.pressed;
		self.pressed = self.pressed_keys.values().any(|&v| v);

		if prev_pressed == self.pressed {
			return;
		}

		if self.times.len() >= 1024 {
			self.times.pop_back();
		}

		self.times.push_front(event.time);
	}
}

#[derive(Debug, Clone)]
struct KeyEvent {
	pub key: rdev::Key,
	pub pressed: bool,
	pub time: SystemTime,
}

struct KeyOverlayScene {
	columns: Vec<KeyColumn>,
	key_column_map: HashMap<rdev::Key, usize>,
	default_font: Font<'static>,
	keyboard_rx: mpsc::Receiver<KeyEvent>,
	now: SystemTime,

	speed: f32,
	direction: ScrollDirection,
	display_keys: bool,
	key_placement: BoxPlacement,
	display_counters: bool,
	counter_placement: BoxPlacement,
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
		let mut key_column_map = HashMap::new();

		let columns = key_columns
			.into_iter()
			.enumerate()
			.map(|(i, kc)| {
				for &key in &kc.props.keys {
					key_column_map.insert(key, i);
				}

				kc
			})
			.collect();

		Self {
			columns,
			key_column_map,
			default_font: Font::from_data(ROBOTO_FONT),
			keyboard_rx,
			now: SystemTime::now(),

			speed: config.speed as f32,
			direction: config.direction,
			display_keys: config.display_keys,
			key_placement: config.key_placement,
			display_counters: config.display_counters,
			counter_placement: config.counter_placement,
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

	fn column_mut(&mut self, key: rdev::Key) -> Option<&mut KeyColumn> {
		self.columns.get_mut(*self.key_column_map.get(&key)?)
	}
}

impl Scene for KeyOverlayScene {
	fn update(&mut self) {
		while let Ok(key_event) = self.keyboard_rx.try_recv() {
			let column = self.column_mut(key_event.key).unwrap();
			column.set_key_pressed(key_event);
		}

		self.now = SystemTime::now();
	}

	fn draw(&self, viewport: Vec2, drawer: &mut impl Drawer) {
		drawer.clear();
		drawer.begin_frame();
		{
			let key_size = vec2(self.default_key_width, self.key_height);
			let spacing = self.key_spacing;
			let key_y = match self.direction {
				ScrollDirection::Up => viewport.y - 30.,
				ScrollDirection::Down => 30.,
			};

			let n_columns = self.columns.len() as f32;

			for (i, column) in self.columns.iter().enumerate() {
				let color = match column.pressed {
					true => column.props.hover_color,
					false => 0x111111,
				};

				let i = i as f32 + 0.5;
				let x_offset = (i - n_columns / 2.) * (key_size.x + spacing / 2.);

				let key_rect = OwoRect {
					pos: vec2(viewport.x / 2. + x_offset, key_y),
					size: key_size,
					origin: match self.direction {
						ScrollDirection::Up => Anchor::BC,
						ScrollDirection::Down => Anchor::TC,
					},
				};

				const KEY_BORDER_WIDTH: f32 = 8.;

				// key rectangle
				drawer.draw_rect(&RectBlueprint {
					rect: key_rect.to_rect(),
					color,
					border_color: column.props.border_color,
					border_width: KEY_BORDER_WIDTH,
					corner_radius: 2.,
					borders: [true, true, true, true],
					alpha: 1.,
				});

				// key and counter texts
				{
					const BIG_FONT_SIZE: f32 = 25.;
					const SMOL_FONT_SIZE: f32 = 20.;
					const BOTTOM_KEY_TEXT_GAP: f32 = 5.;
					const CENTER_TEXT_GAP: f32 = 2.;

					let mut key_text = TextBlueprint {
						text: &column.name,
						x: key_rect.pos.x,
						y: key_rect.pos.y,
						font: &self.default_font,
						size: 20.,
						col: 0xeeeeee,
						alpha: 1.,
					};

					let mut counter_text = TextBlueprint {
						text: &format!("{}", column.count),
						x: key_rect.pos.x,
						y: key_rect.pos.y,
						font: &self.default_font,
						size: 25.,
						col: 0xeeeeee,
						alpha: 1.,
					};

					let kt_rect;
					let ct_rect;

					match (self.key_placement, self.counter_placement) {
						(BoxPlacement::Inside, BoxPlacement::Inside) => {
							// key and counter inside
							// have key above and counter below with a gap

							key_text.size = BIG_FONT_SIZE;
							counter_text.size = SMOL_FONT_SIZE;

							kt_rect = OwoRect {
								pos: key_rect.center() - vec2(0., CENTER_TEXT_GAP),
								size: vec2(key_text.text_width(), key_text.text_height()),
								origin: Anchor::BC,
							};

							ct_rect = OwoRect {
								pos: key_rect.center() + vec2(0., CENTER_TEXT_GAP),
								size: vec2(counter_text.text_width(), counter_text.text_height()),
								origin: Anchor::TC,
							};
						}
						(BoxPlacement::Inside, BoxPlacement::Outside) => {
							// key inside, counter outside

							key_text.size = BIG_FONT_SIZE;
							counter_text.size = SMOL_FONT_SIZE;

							kt_rect = OwoRect {
								pos: key_rect.center(),
								size: vec2(key_text.text_width(), key_text.text_height()),
								origin: Anchor::CC,
							};

							ct_rect = match self.direction {
								ScrollDirection::Up => OwoRect {
									pos: key_rect.anchor(Anchor::BC) + vec2(0., BOTTOM_KEY_TEXT_GAP),
									size: vec2(counter_text.text_width(), counter_text.text_height()),
									origin: Anchor::TC,
								},
								ScrollDirection::Down => OwoRect {
									pos: key_rect.anchor(Anchor::TC) - vec2(0., BOTTOM_KEY_TEXT_GAP),
									size: vec2(counter_text.text_width(), counter_text.text_height()),
									origin: Anchor::BC,
								},
							};
						}
						(BoxPlacement::Outside, BoxPlacement::Inside) => {
							// key outside, counter inside

							key_text.size = SMOL_FONT_SIZE;
							counter_text.size = BIG_FONT_SIZE;

							kt_rect = match self.direction {
								ScrollDirection::Up => OwoRect {
									pos: key_rect.anchor(Anchor::BC) + vec2(0., BOTTOM_KEY_TEXT_GAP),
									size: vec2(key_text.text_width(), key_text.text_height()),
									origin: Anchor::TC,
								},
								ScrollDirection::Down => OwoRect {
									pos: key_rect.anchor(Anchor::TC) - vec2(0., BOTTOM_KEY_TEXT_GAP),
									size: vec2(key_text.text_width(), key_text.text_height()),
									origin: Anchor::BC,
								},
							};

							ct_rect = OwoRect {
								pos: key_rect.center(),
								size: vec2(counter_text.text_width(), counter_text.text_height()),
								origin: Anchor::CC,
							};
						}
						(BoxPlacement::Outside, BoxPlacement::Outside) => {
							// key and counter outside
							// have key on the left and counter on the right

							key_text.size = SMOL_FONT_SIZE;
							counter_text.size = SMOL_FONT_SIZE;

							kt_rect = match self.direction {
								ScrollDirection::Up => OwoRect {
									pos: key_rect.anchor(Anchor::BL) + vec2(KEY_BORDER_WIDTH, BOTTOM_KEY_TEXT_GAP),
									size: vec2(key_text.text_width(), key_text.text_height()),
									origin: Anchor::TL,
								},
								ScrollDirection::Down => OwoRect {
									pos: key_rect.anchor(Anchor::TL) + vec2(KEY_BORDER_WIDTH, -BOTTOM_KEY_TEXT_GAP),
									size: vec2(key_text.text_width(), key_text.text_height()),
									origin: Anchor::BL,
								},
							};

							ct_rect = match self.direction {
								ScrollDirection::Up => OwoRect {
									pos: key_rect.anchor(Anchor::BR) + vec2(-KEY_BORDER_WIDTH, BOTTOM_KEY_TEXT_GAP),
									size: vec2(counter_text.text_width(), counter_text.text_height()),
									origin: Anchor::TR,
								},
								ScrollDirection::Down => OwoRect {
									pos: key_rect.anchor(Anchor::TR) + vec2(-KEY_BORDER_WIDTH, -BOTTOM_KEY_TEXT_GAP),
									size: vec2(counter_text.text_width(), counter_text.text_height()),
									origin: Anchor::BR,
								},
							};
						}
					}

					let key_text_pos = kt_rect.top_left();
					key_text.x = key_text_pos.x;
					key_text.y = key_text_pos.y;

					let counter_text_pos = ct_rect.top_left();
					counter_text.x = counter_text_pos.x;
					counter_text.y = counter_text_pos.y;

					// // debug rectangles
					// {
					// 	drawer.draw_rect(&RectBlueprint {
					// 		rect: kt_rect.to_rect(),
					// 		color,
					// 		border_color: 0xffff00,
					// 		border_width: 1.,
					// 		corner_radius: 0.,
					// 		borders: [true, true, true, true],
					// 		alpha: 1.,
					// 	});

					// 	drawer.draw_rect(&RectBlueprint {
					// 		rect: ct_rect.to_rect(),
					// 		color,
					// 		border_color: 0xff00ff,
					// 		border_width: 1.,
					// 		corner_radius: 0.,
					// 		borders: [true, true, true, true],
					// 		alpha: 1.,
					// 	});
					// }

					if self.display_keys {
						drawer.draw_text(&key_text);
					}

					if self.display_counters {
						drawer.draw_text(&counter_text);
					}
				}

				// history rectangles
				let mut opt_prev_time: Option<SystemTime> = column.pressed.then_some(self.now);

				for time in column.times.iter().copied() {
					match opt_prev_time {
						Some(prev_time) => {
							let base_pos = key_rect.anchor(match self.direction {
								ScrollDirection::Up => Anchor::TL,
								ScrollDirection::Down => Anchor::BL,
							});

							let time_secs = self.time_to_secs(time);
							let prev_time_secs = self.time_to_secs(prev_time);

							let h = ((time_secs - prev_time_secs) * self.speed).min(viewport.y);
							let y = (prev_time_secs - self.time_to_secs(self.now)) * self.speed;
							let y = match self.direction {
									ScrollDirection::Up => base_pos.y + y,
									ScrollDirection::Down => base_pos.y - y - h,
								};

							// stop drawing rectangles once off-screen
							if y <= 0. {
								break;
							}

							drawer.draw_rect(&RectBlueprint {
								rect: Rect {
									x: base_pos.x,
									y,
									w: key_size.x,
									h,
								},
								color: column.props.color,
								border_color: 0x000000,
								border_width: 0.,
								corner_radius: 0.,
								borders: [false, false, false, false],
								alpha: column.props.alpha,
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
	config: Option<PathBuf>,
	#[arg(short, long, help = "Name of a config stored in the config directory")]
	preset: Option<PathBuf>,
}

fn main() -> Result<(), Box<dyn Error>> {
	let Cli {
		config: config_path,
		preset,
	} = Cli::parse();

	let config_dir = dirs::config_dir()
		.expect("You don't have a config directory???")
		.join("OwOverlay");

	fs::create_dir_all(&config_dir)?;

	let (config_path, do_default) = match (config_path, preset) {
		(Some(_), Some(_)) => panic!("Can't specify the config path and a preset at the same time!"),
		(Some(config_path), None) => (config_path, false),
		(None, Some(preset)) => (config_dir.join(preset).with_extension("toml"), false),
		(None, None) => (config_dir.join("cowonfig.toml"), true),
	};

	let config = match fs::read_to_string(&config_path) {
		Ok(c) => toml::from_str(&c)?,
		Err(e) if e.kind() == io::ErrorKind::NotFound => {
			if !do_default {
				panic!("ERROR: {} doesn't exist :(", config_path.display());
			}

			let config = Config::default();
			fs::write(&config_path, toml::to_string(&config)?)?;
			config
		}
		Err(e) => return Err(e.into()),
	};

	let mut keys = HashSet::new();

	let key_columns = (config.columns.iter().cloned())
		.inspect(|column| keys.extend(column.keys.iter().copied()))
		.map(KeyColumn::new)
		.collect::<Vec<_>>();

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

	let (width, height) = (config.window.width, config.window.height);

	let app_frame = AppFrame::init(
		WindowBuilder::new()
			.with_transparent(config.window.transparent)
			.with_resizable(config.window.resizable)
			.with_inner_size(PhysicalSize::new(width, height)),
	)?;

	app_frame.run(OwOverlayApp::new(width, height, scene))
}
