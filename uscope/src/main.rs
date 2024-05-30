use std::{
	fs::{self, File},
	io::Write,
	time::{Duration, Instant},
};

use eframe::{
	egui::{
		CentralPanel, Color32, Painter, Pos2, Rect, ScrollArea, Sense, SidePanel, Slider, Ui, Vec2,
	},
	epaint::Hsva,
	NativeOptions,
};
use egui::{collapsing_header::CollapsingState, DragValue, PointerButton};
use native_dialog::FileDialog;
use rand::prelude::*;

use petri::{Cell, CellData, CellGroup, Dish, Rule, RuleCellFrom, RuleCellTo, CHUNK_SIZE};

fn main() {
	eframe::run_native(
		"µscope",
		NativeOptions::default(),
		Box::new(|_cc| Box::new(UScope::new(_cc))),
	)
	.unwrap();
}

#[derive(Debug)]
struct UScope {
	dish: Dish,
	brush: Cell,
	speed: u32,
	show_grid: bool,
	sim_times: Vec<Duration>,
}

impl UScope {
	fn new(_cc: &eframe::CreationContext<'_>) -> Self {
		Self {
			dish: Dish::new(),
			speed: 50,
			show_grid: false,
			brush: Cell(1),
			// sim_times: vec![0],
			sim_times: vec![Duration::from_micros(1)],
		}
	}

	fn save_universe(&self) -> Option<()> {
		if let Ok(Some(path)) = FileDialog::new()
			.set_filename("universe_1.json")
			.add_filter("JSON", &["json"])
			.show_save_single_file()
		{
			let mut file = File::create(path).ok()?;
			let out = serde_json::to_string(&self.dish).ok()?;
			file.write_all(out.as_bytes()).ok()?;
		}
		Some(())
	}

	fn open_universe(&mut self) {
		if let Ok(Some(path)) = FileDialog::new()
			.set_filename("universe_1.json")
			.add_filter("JSON", &["json"])
			.show_open_single_file()
		{
			// TODO: show errors to user
			let s = fs::read_to_string(path).unwrap();
			self.dish = serde_json::from_str(&s).unwrap();
			self.dish.update_all_rules();
		}
	}
}

impl eframe::App for UScope {
	fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
		ctx.request_repaint();
		let sim_frame = Instant::now();
		for _ in 0..self.speed {
			self.dish.try_one_location();
			// self.dish.apply_one_match();
		}
		let sim_time = sim_frame.elapsed();
		// self.sim_times.push(sim_time.as_micros());
		self.sim_times.push(sim_time);
		if self.sim_times.len() > 60 {
			self.sim_times.remove(0);
		}
		SidePanel::left("left_panel")
			.min_width(100.)
			.show(ctx, |ui| {
				ui.heading("Simulation");
				ui.label("speed");
				ui.add(Slider::new(&mut self.speed, 0..=500).clamp_to_range(false));
				ui.label(format!("sim time: {sim_time:?}"));
				let avg_sim_time =
					self.sim_times.iter().sum::<Duration>() / self.sim_times.len() as u32;
				ui.label(format!("average sim time: {avg_sim_time:?}"));

				ui.checkbox(&mut self.show_grid, "show grid");
				if ui.button("regenerate rules and cache").clicked() {
					self.dish.update_all_rules();
				}
				if ui.button("debug cache").clicked() {
					self.dish.dbg_cache();
				}
				ui.horizontal(|ui| {
					if ui.button("Save").clicked() {
						self.save_universe();
					}
					if ui.button("Open").clicked() {
						self.open_universe();
					}
				});
				ui.separator();

				ScrollArea::vertical().show(ui, |ui| {
					ui.heading("Cells");
					for (i, cell) in self.dish.types.iter_mut().enumerate() {
						ui.horizontal(|ui| {
							ui.set_width(120.);
							ui.radio_value(&mut self.brush.0, i as u16, "");
							ui.text_edit_singleline(&mut cell.name);
							ui.color_edit_button_srgb(&mut cell.color);
						});
					}

					if ui.button("add cell").clicked() {
						let h = random::<f32>();
						let s = random::<f32>() * 0.5 + 0.5;
						let v = random::<f32>() * 0.5 + 0.5;
						let color = Hsva::new(h, s, v, 1.).to_srgb();
						let name = format!("cell #{}", self.dish.types.len());
						self.dish.types.push(CellData { name, color })
					}
					if ui.button("fill").clicked() {
						self.dish.fill(self.brush);
					}
					ui.separator();

					ui.heading("Groups");
					for group in &mut self.dish.groups {
						let (rect, _response) =
							ui.allocate_exact_size(Vec2::splat(CSIZE), Sense::click());
						draw_group(ui, rect, group, &self.dish.types);
						ui.horizontal(|ui| {
							ui.menu_button("edit", |ui| {
								ui.checkbox(&mut group.void, "void");
								for (i, celldata) in self.dish.types.iter().enumerate() {
									let mut included = group.cells.contains(&Cell(i as u16));
									if ui.checkbox(&mut included, &celldata.name).changed() {
										if included {
											group.cells.push(Cell(i as u16));
										} else {
											group.cells.retain(|c| c != &Cell(i as u16));
										}
									}
								}
							});
							ui.text_edit_singleline(&mut group.name);
						});
					}
					if ui.button("add group").clicked() {
						self.dish.groups.push(CellGroup::default());
					}

					ui.heading("Rules");

					let mut to_remove = None;
					let mut to_clone = None;
					let mut to_update = None;
					for (i, rule) in self.dish.rules.iter_mut().enumerate() {
						let changed = rule_editor(
							ui,
							rule,
							i,
							&self.dish.types,
							&self.dish.groups,
							&mut to_remove,
							&mut to_clone,
						);
						if changed {
							rule.generate_variants();
							to_update = Some(i);
						}
					}
					if let Some(i) = to_update {
						self.dish.update_cache_single_rule(i);
					}
					if let Some(i) = to_remove {
						self.dish.rules.remove(i);
						self.dish.rebuild_cache();
					}
					if let Some(i) = to_clone {
						let mut new_rule = self.dish.rules[i].clone();
						new_rule.enabled = false;
						self.dish.rules.push(new_rule);
						self.dish.cache_last_added_rule();
					}
					ui.separator();
					if ui.button("add rule").clicked() {
						self.dish.rules.push(Rule::new());
						self.dish.cache_last_added_rule()
					}
				});
			});
		CentralPanel::default().show(ctx, |ui| {
			let bounds = ui.available_rect_before_wrap();
			let painter = ui.painter_at(bounds);
			paint_world(painter, &self.dish, self.show_grid);

			let rect = ui.allocate_rect(bounds, Sense::click_and_drag());
			if let Some(pos) = rect.interact_pointer_pos() {
				let p = ((pos - bounds.min) / GRID_SIZE).floor();
				let x = p.x as usize;
				let y = p.y as usize;
				let pick = ui.input(|i| i.modifiers.shift);
				if pick {
					if let Some(clicked_cell) = self.dish.get_cell(x, y) {
						self.brush = clicked_cell;
					}
				} else {
					let old = self.dish.get_cell(x, y);
					if Some(self.brush) != old {
						self.dish.set_cell(x, y, self.brush);
						self.dish.update_cache(x as isize, y as isize, 1, 1);
					}
				}
			}
		});
	}
}

const GRID_SIZE: f32 = 16.;
fn paint_world(painter: Painter, world: &Dish, grid: bool) {
	let cells = &world.types;
	let bounds = painter.clip_rect();
	for x in 0..CHUNK_SIZE {
		for y in 0..CHUNK_SIZE {
			let cell = &world.get_cell(x, y).unwrap();
			let corner = bounds.min + (Vec2::from((x as f32, y as f32)) * GRID_SIZE);
			let rect = Rect::from_min_size(corner, Vec2::splat(GRID_SIZE));
			if cell.id() >= cells.len() {
				continue;
			}
			let color = cells[cell.id()].color;
			let color = Color32::from_rgb(color[0], color[1], color[2]);
			if grid {
				painter.rect(rect, 0., color, (1., Color32::GRAY));
			} else {
				painter.rect_filled(rect, 0., color);
			}
		}
	}
}

const CSIZE: f32 = 24.;
const RESIZE_BUTTON_WIDTH: f32 = 8.;

const OUTLINE: (f32, Color32) = (2., Color32::GRAY);
fn rule_editor(
	ui: &mut Ui,
	rule: &mut Rule,
	index: usize,
	cells: &[CellData],
	groups: &[CellGroup],
	to_remove: &mut Option<usize>,
	to_clone: &mut Option<usize>,
) -> bool {
	let mut changed = false;
	let id = ui.make_persistent_id(format!("rule {index}"));
	CollapsingState::load_with_default_open(ui.ctx(), id, true)
		.show_header(ui, |ui| {
			if ui.checkbox(&mut rule.enabled, &rule.name).changed() {
				changed = true;
			}
			if ui.button("delete").clicked() {
				*to_remove = Some(index);
			}
			if ui.button("copy").clicked() {
				*to_clone = Some(index);
			}
		})
		.body(|ui| {
			ui.text_edit_singleline(&mut rule.name);
			ui.horizontal(|ui| {
				if ui.checkbox(&mut rule.flip_x, "flip X").changed() {
					changed = true;
				}
				if ui.checkbox(&mut rule.flip_y, "flip Y").changed() {
					changed = true;
				}
				if ui.checkbox(&mut rule.rotate, "rotate").changed() {
					changed = true;
				}
			});
			ui.horizontal(|ui| {
				ui.label("fail rate:");
				ui.add(DragValue::new(&mut rule.failrate));
				ui.label(format!("variants: {}", rule.variant_count()));
				if ui.button("debug").clicked() {
					rule.dbg_variants();
				}
			});
			let cells_y = rule.height();
			let cells_x = rule.width();
			let patt_width = CSIZE * cells_x as f32;
			let patt_height = CSIZE * cells_y as f32;

			let (_, bounds) = ui.allocate_space(Vec2::new(
				patt_width * 2. + RESIZE_BUTTON_WIDTH * 4. + CSIZE,
				patt_height + RESIZE_BUTTON_WIDTH * 2.,
			));

			let from_cells_rect = Rect::from_min_size(
				bounds.min + Vec2::splat(RESIZE_BUTTON_WIDTH),
				Vec2::new(patt_width, patt_height),
			);
			let to_cells_rect = Rect::from_min_size(
				bounds.min
					+ Vec2::splat(RESIZE_BUTTON_WIDTH)
					+ Vec2::X * (patt_width + RESIZE_BUTTON_WIDTH * 2. + CSIZE),
				Vec2::new(patt_width, patt_height),
			);

			let mut overlay_lines = Vec::new();
			for x in 0..cells_x {
				for y in 0..cells_y {
					let (left, right) = rule.get_mut(x, y);
					let changed_left =
						rule_cell_edit_from(ui, from_cells_rect.min, left, x, y, cells, groups);
					let changed_right = rule_cell_edit_to(
						ui,
						to_cells_rect.min,
						right,
						(x, y),
						cells,
						groups,
						(cells_x, cells_y),
						&mut overlay_lines,
					);
					if changed_left || changed_right {
						changed = true;
					}
				}
			}

			let delete_mode = ui.input(|i| i.modifiers.shift);

			let mut resize_box = |x, y, w, h| {
				let rect_a = Rect::from_min_size(Pos2::new(x, y), Vec2::new(w, h));
				let a = ui.allocate_rect(rect_a, Sense::click());
				let rect_b = rect_a.translate(to_cells_rect.min - from_cells_rect.min);
				let b = ui.allocate_rect(rect_b, Sense::click());
				let result = a.union(b);
				let color = if result.hovered() {
					if delete_mode {
						Color32::RED
					} else {
						Color32::GRAY
					}
				} else {
					Color32::DARK_GRAY
				};
				ui.painter_at(bounds).rect_filled(rect_a, 0., color);
				ui.painter_at(bounds).rect_filled(rect_b, 0., color);

				result.clicked()
			};
			if resize_box(
				bounds.min.x,
				bounds.min.y + RESIZE_BUTTON_WIDTH,
				RESIZE_BUTTON_WIDTH,
				patt_height,
			) {
				if delete_mode {
					rule.resize(Rule::SHRINK_LEFT);
				} else {
					rule.resize(Rule::EXTEND_LEFT);
				}
			}
			if resize_box(
				from_cells_rect.max.x,
				bounds.min.y + RESIZE_BUTTON_WIDTH,
				RESIZE_BUTTON_WIDTH,
				patt_height,
			) {
				if delete_mode {
					rule.resize(Rule::SHRINK_RIGHT);
				} else {
					rule.resize(Rule::EXTEND_RIGHT);
				}
			}
			if resize_box(
				bounds.min.x + RESIZE_BUTTON_WIDTH,
				bounds.min.y,
				patt_width,
				RESIZE_BUTTON_WIDTH,
			) {
				if delete_mode {
					rule.resize(Rule::SHRINK_UP);
				} else {
					rule.resize(Rule::EXTEND_UP);
				}
			}
			if resize_box(
				bounds.min.x + RESIZE_BUTTON_WIDTH,
				bounds.max.y - RESIZE_BUTTON_WIDTH,
				patt_width,
				RESIZE_BUTTON_WIDTH,
			) {
				if delete_mode {
					rule.resize(Rule::SHRINK_DOWN);
				} else {
					rule.resize(Rule::EXTEND_DOWN);
				}
			}

			for (a, b, marked) in overlay_lines {
				let stroke = if marked {
					(6., Color32::RED)
				} else {
					(2., Color32::WHITE)
				};
				ui.painter().line_segment([a, b], stroke);
			}
		});
	changed
}

fn rule_cell_edit_from(
	ui: &mut Ui,
	origin: Pos2,
	rule: &mut RuleCellFrom,
	x: usize,
	y: usize,
	cells: &[CellData],
	groups: &[CellGroup],
) -> bool {
	let mut changed = false;
	let rect = Rect::from_min_size(
		origin + Vec2::from((x as f32, y as f32)) * CSIZE,
		Vec2::splat(CSIZE),
	);
	let aabb = ui.allocate_rect(rect, Sense::click());
	let cycle_colors = aabb.clicked_by(PointerButton::Primary);
	let switch_type = aabb.clicked_by(PointerButton::Secondary);

	// draw
	match rule {
		RuleCellFrom::Any => (),
		RuleCellFrom::One(cell) => {
			let color = cells[cell.id()].color;
			let color = Color32::from_rgb(color[0], color[1], color[2]);
			ui.painter()
				.rect(rect.shrink(OUTLINE.0 / 2.), 0., color, OUTLINE);
		}
		RuleCellFrom::Group(group_id) => {
			let group = &groups[*group_id];
			draw_group(ui, rect, group, cells);
		}
	}
	// update
	if cycle_colors {
		match rule {
			RuleCellFrom::Any => (),
			RuleCellFrom::One(cell) => {
				cell.0 += 1;
				cell.0 %= cells.len() as u16;
				changed = true;
			}
			RuleCellFrom::Group(group_id) => {
				*group_id += 1;
				*group_id %= groups.len();
				changed = true;
			}
		}
	}
	if switch_type {
		changed = true;
		match rule {
			RuleCellFrom::Any => {
				*rule = RuleCellFrom::One(Cell(0));
			}
			RuleCellFrom::One(_) => {
				*rule = RuleCellFrom::Group(0);
			}
			RuleCellFrom::Group(_) => {
				*rule = RuleCellFrom::Any;
			}
		}
	}
	changed
}

fn rule_cell_edit_to(
	ui: &mut Ui,
	origin: Pos2,
	rule: &mut RuleCellTo,
	(x, y): (usize, usize),
	cells: &[CellData],
	groups: &[CellGroup],
	(rule_width, rule_height): (usize, usize),
	overlay_lines: &mut Vec<(Pos2, Pos2, bool)>,
) -> bool {
	let mut changed = false;
	let rect = Rect::from_min_size(
		origin + Vec2::from((x as f32, y as f32)) * CSIZE,
		Vec2::splat(CSIZE),
	);
	let aabb = ui.allocate_rect(rect, Sense::click());
	let cycle_colors = aabb.clicked_by(PointerButton::Primary);
	let switch_type = aabb.clicked_by(PointerButton::Secondary);
	let hovered = aabb.hovered();

	// draw
	match rule {
		RuleCellTo::None => (),
		RuleCellTo::One(cell) => {
			let color = cells[cell.id()].color;
			let color = Color32::from_rgb(color[0], color[1], color[2]);
			ui.painter()
				.rect(rect.shrink(OUTLINE.0 / 2.), 0., color, OUTLINE);
		}
		RuleCellTo::GroupRandom(group_id) => {
			let group = &groups[*group_id];
			draw_group(ui, rect, group, cells);
		}
		RuleCellTo::Copy(x, y) => {
			let this = rect.center();
			let target = origin + Vec2::from((*x as f32, *y as f32)) * CSIZE
				- Vec2::X * (CSIZE * (rule_width as f32 + 1.) + RESIZE_BUTTON_WIDTH * 2.)
				+ Vec2::splat(CSIZE) * 0.5;
			overlay_lines.push((this, target, hovered));
		}
	}

	if cycle_colors {
		match rule {
			RuleCellTo::None => (),
			RuleCellTo::One(cell) => {
				cell.0 += 1;
				cell.0 %= cells.len() as u16;
				changed = true;
			}
			RuleCellTo::GroupRandom(group_id) => {
				*group_id += 1;
				*group_id %= groups.len();
				changed = true;
			}
			RuleCellTo::Copy(x, y) => {
				*x += 1;
				if *x >= rule_width {
					*x = 0;
					*y += 1;
					if *y >= rule_height {
						*y = 0;
					}
				}
				changed = true;
			}
		}
	}

	if switch_type {
		changed = true;
		match rule {
			RuleCellTo::None => {
				*rule = RuleCellTo::One(Cell(0));
			}
			RuleCellTo::One(_) => {
				*rule = RuleCellTo::GroupRandom(0);
			}
			RuleCellTo::GroupRandom(_) => {
				*rule = RuleCellTo::Copy(0, 0);
			}
			RuleCellTo::Copy(_, _) => {
				*rule = RuleCellTo::None;
			}
		}
	}
	changed
}

fn draw_group(ui: &mut Ui, rect: Rect, group: &CellGroup, cells: &[CellData]) {
	let group_size = group.cells.len();
	let radius_per_color = (CSIZE * 0.7) / (group_size as f32);
	for (i, cell) in group.cells.iter().enumerate() {
		let color = cells[cell.id()].color;
		let color = Color32::from_rgb(color[0], color[1], color[2]);
		let radius = radius_per_color * ((group_size - i) as f32);
		ui.painter_at(rect)
			.circle_filled(rect.center(), radius, color);
	}
	if group.void {
		ui.painter_at(rect)
			.line_segment([rect.min, rect.max], (1., Color32::WHITE));
	}
	ui.allocate_rect(rect, Sense::hover())
		.on_hover_text(&group.name);
}
