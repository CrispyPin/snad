use std::{
	fs::{self, File},
	io::Write,
};

use eframe::{
	egui::{
		CentralPanel, Color32, Painter, Pos2, Rect, ScrollArea, Sense, SidePanel, Slider, Ui, Vec2,
	},
	epaint::Hsva,
	NativeOptions,
};
use egui::PointerButton;
use native_dialog::FileDialog;
use rand::prelude::*;
use serde::{Deserialize, Serialize};

use petri::{Cell, Chunk, Dish, Rule, RuleCellFrom, RuleCellTo, CHUNK_SIZE};
use serde_json::{json, Value};

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
	speed: usize,
	show_grid: bool,
	cell_types: Vec<CellData>,
}

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct CellData {
	name: String,
	color: Color32,
}

impl UScope {
	fn new(_cc: &eframe::CreationContext<'_>) -> Self {
		Self {
			dish: Dish::new(),
			speed: 500,
			show_grid: false,
			brush: Cell(1),
			cell_types: vec![
				CellData::new("air", 0, 0, 0),
				CellData::new("pink_sand", 255, 147, 219),
			],
		}
	}

	fn save_universe(&self) -> Option<()> {
		if let Ok(Some(path)) = FileDialog::new()
			.set_filename("universe_1.json")
			.add_filter("JSON", &["json"])
			.show_save_single_file()
		{
			let out = json!({
				"cell_types": self.cell_types,
				"rules": self.dish.rules,
				"groups": self.dish.cell_groups,
			});
			let out = serde_json::to_string(&out).ok()?;
			let mut file = File::create(path).ok()?;
			file.write_all(out.as_bytes()).ok()?;
		}
		Some(())
	}

	fn open_universe(&mut self) -> Option<()> {
		if let Ok(Some(path)) = FileDialog::new()
			.set_filename("universe_1.json")
			.add_filter("JSON", &["json"])
			.show_open_single_file()
		{
			let s = fs::read_to_string(path).ok()?;
			let data: Value = serde_json::from_str(&s).ok()?;
			let cell_types = serde_json::from_value(data["cell_types"].clone()).ok()?;
			let groups = serde_json::from_value(data["groups"].clone()).ok()?;
			let rules = serde_json::from_value(data["rules"].clone()).ok()?;
			self.cell_types = cell_types;
			self.dish.rules = rules;
			self.dish.cell_groups = groups;
			self.dish.update_rules();
		}
		Some(())
	}
}

impl eframe::App for UScope {
	fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
		ctx.request_repaint();
		for _ in 0..self.speed {
			self.dish.fire_blindly();
		}
		SidePanel::left("left_panel")
			.min_width(100.)
			.show(ctx, |ui| {
				ui.heading("Simulation");
				ui.label("speed");
				ui.add(Slider::new(&mut self.speed, 0..=5000));
				ui.checkbox(&mut self.show_grid, "show grid");
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
					for (i, cell) in self.cell_types.iter_mut().enumerate() {
						ui.horizontal(|ui| {
							ui.set_width(120.);
							ui.radio_value(&mut self.brush.0, i as u16, "");
							ui.text_edit_singleline(&mut cell.name);
							ui.color_edit_button_srgba(&mut cell.color);
						});
					}

					if ui.button("add cell").clicked() {
						let h = random::<f32>();
						let s = random::<f32>() * 0.5 + 0.5;
						let v = random::<f32>() * 0.5 + 0.5;
						let color = Hsva::new(h, s, v, 1.).into();
						let name = format!("cell #{}", self.cell_types.len());
						self.cell_types.push(CellData { name, color })
					}
					if ui.button("fill").clicked() {
						self.dish.chunk.contents.fill([self.brush; CHUNK_SIZE]);
					}
					ui.separator();

					ui.heading("Groups");
					for group in &mut self.dish.cell_groups {
						let (rect, _response) =
							ui.allocate_exact_size(Vec2::splat(CSIZE), Sense::click());
						draw_group(ui, rect, group, &self.cell_types);
						ui.menu_button("edit", |ui| {
							let mut void = group.contains(&None);
							if ui.checkbox(&mut void, "void").changed() {
								if void {
									group.push(None);
								} else {
									group.retain(|c| c.is_some());
								}
							}
							for (i, celldata) in self.cell_types.iter().enumerate() {
								let mut included = group.contains(&Some(Cell(i as u16)));
								if ui.checkbox(&mut included, &celldata.name).changed() {
									if included {
										group.push(Some(Cell(i as u16)));
									} else {
										group.retain(|c| c != &Some(Cell(i as u16)));
									}
								}
							}
						});
					}
					if ui.button("add group").clicked() {
						self.dish.cell_groups.push(Vec::new());
					}

					ui.heading("Rules");

					let mut to_remove = None;
					for (i, rule) in self.dish.rules.iter_mut().enumerate() {
						ui.separator();
						rule_editor(ui, rule, &self.cell_types, &self.dish.cell_groups);
						if ui.button("delete").clicked() {
							to_remove = Some(i);
						}
					}
					if let Some(i) = to_remove {
						self.dish.rules.remove(i);
					}
					ui.separator();
					if ui.button("add rule").clicked() {
						self.dish.rules.push(Rule::default());
					}
				});
			});
		CentralPanel::default().show(ctx, |ui| {
			let bounds = ui.available_rect_before_wrap();
			let painter = ui.painter_at(bounds);
			paint_chunk(painter, &self.dish.chunk, &self.cell_types, self.show_grid);

			let rect = ui.allocate_rect(bounds, Sense::click_and_drag());
			if let Some(pos) = rect.interact_pointer_pos() {
				let p = ((pos - bounds.min) / GRID_SIZE).floor();
				let x = p.x as usize;
				let y = p.y as usize;
				self.dish.set_cell(x, y, self.brush);
			}
		});
	}
}

const GRID_SIZE: f32 = 16.;
fn paint_chunk(painter: Painter, chunk: &Chunk, cells: &[CellData], grid: bool) {
	let bounds = painter.clip_rect();
	for x in 0..CHUNK_SIZE {
		for y in 0..CHUNK_SIZE {
			let cell = &chunk.get_cell(x, y);
			let corner = bounds.min + (Vec2::from((x as f32, y as f32)) * GRID_SIZE);
			let rect = Rect::from_min_size(corner, Vec2::splat(GRID_SIZE));
			let color = cells[cell.id()].color;
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
fn rule_editor(ui: &mut Ui, rule: &mut Rule, cells: &[CellData], groups: &[Vec<Option<Cell>>]) {
	ui.checkbox(&mut rule.enabled, "enable rule");
	ui.horizontal(|ui| {
		ui.label("flip");
		if ui.checkbox(&mut rule.flip_h, "H").changed() {
			rule.generate_variants();
		}
		if ui.checkbox(&mut rule.flip_v, "V").changed() {
			rule.generate_variants();
		}
	});
	if ui.checkbox(&mut rule.rotate, "rotate").changed() {
		rule.generate_variants();
	}

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
				rule.generate_variants();
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

	for (a, b) in overlay_lines {
		ui.painter().line_segment([a, b], (2., Color32::WHITE));
	}
}

fn rule_cell_edit_from(
	ui: &mut Ui,
	origin: Pos2,
	rule: &mut RuleCellFrom,
	x: usize,
	y: usize,
	cells: &[CellData],
	groups: &[Vec<Option<Cell>>],
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
	groups: &[Vec<Option<Cell>>],
	(rule_width, rule_height): (usize, usize),
	overlay_lines: &mut Vec<(Pos2, Pos2)>,
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
		RuleCellTo::None => (),
		RuleCellTo::One(cell) => {
			let color = cells[cell.id()].color;
			ui.painter()
				.rect(rect.shrink(OUTLINE.0 / 2.), 0., color, OUTLINE);
		}
		RuleCellTo::GroupRandom(group_id) => {
			let group = &groups[*group_id];
			draw_group(ui, rect, group, cells);
		}
		RuleCellTo::Copy(index) => {
			let this = rect.center();
			let x = *index % rule_width;
			let y = *index / rule_width;
			let target = origin + Vec2::from((x as f32, y as f32)) * CSIZE
				- Vec2::X * (CSIZE * (rule_width as f32 + 1.) + RESIZE_BUTTON_WIDTH * 2.)
				+ Vec2::splat(CSIZE) * 0.5;
			overlay_lines.push((this, target));
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
			RuleCellTo::Copy(index) => {
				*index = (*index + 1) % (rule_width * rule_height);
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
				*rule = RuleCellTo::Copy(0);
			}
			RuleCellTo::Copy(_) => {
				*rule = RuleCellTo::None;
			}
		}
	}
	changed
}

fn draw_group(ui: &mut Ui, rect: Rect, group: &[Option<Cell>], cells: &[CellData]) {
	let mut group_size = group.len();
	let has_void = group.contains(&None);
	if has_void {
		group_size -= 1;
	}
	let radius_per_color = (CSIZE * 0.7) / (group_size as f32);
	for (i, cell) in group.iter().flatten().enumerate() {
		let color = cells[cell.id()].color;
		let radius = radius_per_color * ((group_size - i) as f32);
		ui.painter_at(rect)
			.circle_filled(rect.center(), radius, color);
	}
	if has_void {
		ui.painter_at(rect)
			.line_segment([rect.min, rect.max], (1., Color32::WHITE));
	}
}

impl CellData {
	fn new(name: &str, r: u8, g: u8, b: u8) -> Self {
		Self {
			name: name.to_owned(),
			color: Color32::from_rgb(r, g, b),
		}
	}
}
