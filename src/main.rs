use core::fmt;
use std::collections::{HashMap, HashSet, VecDeque};
use std::error::Error;
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::{Duration, SystemTime};
use std::{fs, io, thread};

use app::OwOverlayApp;
use app_frame::AppFrame;
use clap::Parser;
use config::{BoxPlacement, ColumnProps, Config, ScrollDirection};
use glam::{vec2, Vec2};
use key::display_key;
use layout::{Anchor, OwoRect};
use loki_draw::drawer::{Drawer, RectBlueprint, TextBlueprint};
use loki_draw::font::Font;
use loki_draw::rect::Rect;
use winit::dpi::PhysicalSize;
use winit::event::ElementState;
use winit::keyboard::ModifiersState;
use winit::window::WindowBuilder;

mod app;
mod app_frame;
mod config;
mod key;
mod layout;

const ROBOTO_FONT: &[u8] = include_bytes!("../assets/Roboto-Regular.ttf");

pub trait Scene {
	fn update(&mut self);
	fn inapp_key_event(&mut self, event: winit::event::KeyEvent, modifiers: ModifiersState);
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

	debug_mode: bool,
	frame_count: u64,
	frame_deltas: VecDeque<Duration>,
	debug_texts: Vec<String>,

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

			debug_mode: false,
			frame_count: 0,
			frame_deltas: VecDeque::new(),
			debug_texts: Vec::new(),

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

	fn duration_since_now(&self, time: SystemTime) -> Duration {
		let time = time.duration_since(SystemTime::UNIX_EPOCH).unwrap();
		let now = self.now.duration_since(SystemTime::UNIX_EPOCH).unwrap();

		if time >= now {
			time - now
		} else {
			now - time
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

		if self.debug_mode {
			while self.frame_deltas.len() >= 60 {
				self.frame_deltas.pop_front();
			}
			self.frame_deltas.push_back(self.now.elapsed().unwrap());

			if self.frame_count % 100 == 0 {
				let avg_delta = self.frame_deltas.iter().sum::<Duration>() / self.frame_deltas.len().max(1) as u32;
				self.debug_texts = vec![
					format!(
						"SystemTime count: {}",
						(self.columns.iter())
							.map(|c| c.times.len().to_string())
							.collect::<Vec<_>>()
							.join(" | ")
					),
					format!("Frame performance: {:.2?}", avg_delta),
				];
			}
		}
		self.now = SystemTime::now();

		self.frame_count += 1;
	}

	fn inapp_key_event(&mut self, event: winit::event::KeyEvent, modifiers: ModifiersState) {
		if modifiers.control_key()
			&& event.state == ElementState::Released
			&& event.logical_key.as_ref() == winit::keyboard::Key::Character("d")
			&& !event.repeat
		{
			self.debug_mode = !self.debug_mode
		}
	}

	fn draw(&self, viewport: Vec2, drawer: &mut impl Drawer) {
		let mut drawn_rects = 0;
		let mut drawn_texts = 0;

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
				drawn_rects += 1;

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
						drawn_texts += 1;
					}

					if self.display_counters {
						drawer.draw_text(&counter_text);
						drawn_texts += 1;
					}
				}

				// history rectangles
				let mut opt_prev_time: Option<SystemTime> = column.pressed.then_some(self.now);

				for time in column.times.iter().copied() {
					match opt_prev_time {
						Some(prev_time) => {
							let rect = {
								let base_pos = key_rect.anchor(match self.direction {
									ScrollDirection::Up => Anchor::TL,
									ScrollDirection::Down => Anchor::BL,
								});

								let this_time = self.duration_since_now(time);
								let prev_time = self.duration_since_now(prev_time);
								let now_time = self.duration_since_now(self.now);

								let y_start = (this_time - now_time).as_secs_f32() * self.speed;
								let y_end = (prev_time - now_time).as_secs_f32() * self.speed;

								// stop drawing rectangles once off-screen
								if y_end >= viewport.y {
									break;
								}

								// clamp coordinates to avoid floating point glitches
								let y_start = y_start.clamp(0.0, viewport.y);
								let y_end = y_end.clamp(0.0, viewport.y);

								let (y, h) = match self.direction {
									ScrollDirection::Up => (-y_start, y_start - y_end),
									ScrollDirection::Down => (y_start, y_end - y_start),
								};

								Rect {
									x: base_pos.x,
									y: base_pos.y + y,
									w: key_size.x,
									h,
								}
							};

							drawer.draw_rect(&RectBlueprint {
								rect,
								color: column.props.color,
								border_color: 0x000000,
								border_width: 0.,
								corner_radius: 0.,
								borders: [false, false, false, false],
								alpha: column.props.alpha,
							});
							drawn_rects += 1;

							opt_prev_time = None;
						}
						None => opt_prev_time = Some(time),
					}
				}
			}

			if self.debug_mode {
				drawn_texts += 2;

				let line_spacing = 15.0;
				let total_text_height = line_spacing * (self.debug_texts.len() as f32 + 1.0);
				let start_y = match self.direction {
					ScrollDirection::Up => 5.0,
					ScrollDirection::Down => viewport.y - 5.0 - total_text_height,
				};

				drawer.draw_rect(&RectBlueprint {
					rect: Rect::new(0.0, start_y - 5.0, viewport.x, total_text_height + 10.0),
					color: 0x000000,
					border_color: 0x000000,
					border_width: 0.0,
					corner_radius: 0.0,
					borders: [false, false, false, false],
					alpha: 1.0,
				});

				let debug_text = format!("Drawn | Rectangles = {} | Texts = {}", drawn_rects, drawn_texts);
				drawer.draw_text(&TextBlueprint {
					text: &debug_text,
					x: 5.0,
					y: start_y,
					font: &self.default_font,
					size: 15.,
					col: 0x64ff64,
					alpha: 1.,
				});

				let debug_text_start_y = start_y + line_spacing;

				for (i, debug_text) in self.debug_texts.iter().enumerate() {
					drawer.draw_text(&TextBlueprint {
						text: debug_text,
						x: 5.0,
						y: debug_text_start_y + i as f32 * line_spacing,
						font: &self.default_font,
						size: 15.,
						col: 0x64ff64,
						alpha: 1.,
					});
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
			.with_title("OwOverlay")
			.with_transparent(config.window.transparent)
			.with_resizable(config.window.resizable)
			.with_inner_size(PhysicalSize::new(width, height)),
	)?;

	app_frame.run(OwOverlayApp::new(width, height, scene))
}
