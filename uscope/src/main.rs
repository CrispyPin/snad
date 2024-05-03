use eframe::{
	egui::{CentralPanel, Color32, Painter, Pos2, Rect, Sense, SidePanel, Slider, Ui, Vec2},
	epaint::Hsva,
	NativeOptions,
};
use petri::{Cell, Chunk, Dish, Rule, CHUNK_SIZE};
use rand::prelude::*;

fn main() {
	eframe::run_native(
		"V3 World Editor",
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
	celltypes: Vec<CellData>,
}

#[derive(Default, Debug)]
pub struct CellData {
	name: String,
	color: Color32,
}

impl UScope {
	fn new(_cc: &eframe::CreationContext<'_>) -> Self {
		Self {
			dish: Dish::new(),
			speed: 250,
			brush: Cell(1),
			celltypes: vec![
				CellData::new("air", 0, 0, 0),
				CellData::new("pink_sand", 255, 147, 219),
			],
		}
	}
}

impl eframe::App for UScope {
	fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
		ctx.request_repaint();
		for _ in 0..self.speed {
			self.dish.fire_blindly();
		}
		SidePanel::left("left_panel").show(ctx, |ui| {
			ui.heading("Simulation");
			ui.label("speed");
			ui.add(Slider::new(&mut self.speed, 0..=5000));
			ui.separator();

			ui.heading("Cells");
			for (i, cell) in self.celltypes.iter_mut().enumerate() {
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
				let name = format!("cell #{}", self.celltypes.len());
				self.celltypes.push(CellData { name, color })
			}
			ui.separator();

			ui.heading("Rules");
			let mut to_remove = None;
			for (i, rule) in self.dish.rules.iter_mut().enumerate() {
				rule_editor(ui, rule, &self.celltypes);
				if ui.button("delete").clicked() {
					to_remove = Some(i);
				}
				ui.separator();
			}
			if let Some(i) = to_remove {
				self.dish.rules.remove(i);
			}
			if ui.button("add rule").clicked() {
				self.dish.rules.push(Rule::new());
			}
		});
		CentralPanel::default().show(ctx, |ui| {
			let bounds = ui.available_rect_before_wrap();
			let painter = ui.painter_at(bounds);
			paint_chunk(painter, &self.dish.chunk, &self.celltypes);

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
fn paint_chunk(painter: Painter, chunk: &Chunk, cells: &[CellData]) {
	let bounds = painter.clip_rect();
	for x in 0..CHUNK_SIZE {
		for y in 0..CHUNK_SIZE {
			let cell = &chunk.get_cell(x, y);
			let corner = bounds.min + (Vec2::from((x as f32, y as f32)) * GRID_SIZE);
			let rect = Rect::from_min_size(corner, Vec2::splat(GRID_SIZE));
			let color = cells[cell.id()].color;
			painter.rect(rect, 0., color, (1., Color32::GRAY));
		}
	}
}

const CSIZE: f32 = 24.;
const OUTLINE: (f32, Color32) = (2., Color32::GRAY);
fn rule_editor(ui: &mut Ui, rule: &mut Rule, cells: &[CellData]) {
	ui.checkbox(&mut rule.enabled, "enable rule");
	if ui.checkbox(&mut rule.flip_h, "flip H").changed() {
		rule.generate_variants();
	}
	if ui.checkbox(&mut rule.flip_v, "flip V").changed() {
		rule.generate_variants();
	}

	let cells_y = rule.height();
	let cells_x = rule.width();
	let margin = 8.;
	let patt_width = CSIZE * cells_x as f32;
	let patt_height = CSIZE * cells_y as f32;

	let (_, bounds) = ui.allocate_space(Vec2::new(
		patt_width * 2. + margin * 4. + CSIZE,
		patt_height + margin * 2.,
	));

	let from_cells_rect = Rect::from_min_size(
		bounds.min + Vec2::splat(margin),
		Vec2::new(patt_width, patt_height),
	);
	let to_cells_rect = Rect::from_min_size(
		bounds.min + Vec2::splat(margin) + Vec2::X * (patt_width + margin * 2. + CSIZE),
		Vec2::new(patt_width, patt_height),
	);

	for x in 0..cells_x {
		for y in 0..cells_y {
			let (left, right) = rule.get_mut(x, y);
			let changed_left = rule_cell_edit(ui, from_cells_rect.min, left, x, y, cells);
			let changed_right = rule_cell_edit(ui, to_cells_rect.min, right, x, y, cells);
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
	if resize_box(bounds.min.x, bounds.min.y + margin, margin, patt_height) {
		if delete_mode {
			rule.resize(Rule::SHRINK_LEFT);
		} else {
			rule.resize(Rule::EXTEND_LEFT);
		}
	}
	if resize_box(
		from_cells_rect.max.x,
		bounds.min.y + margin,
		margin,
		patt_height,
	) {
		if delete_mode {
			rule.resize(Rule::SHRINK_RIGHT);
		} else {
			rule.resize(Rule::EXTEND_RIGHT);
		}
	}
	if resize_box(bounds.min.x + margin, bounds.min.y, patt_width, margin) {
		if delete_mode {
			rule.resize(Rule::SHRINK_UP);
		} else {
			rule.resize(Rule::EXTEND_UP);
		}
	}
	if resize_box(
		bounds.min.x + margin,
		bounds.max.y - margin,
		patt_width,
		margin,
	) {
		if delete_mode {
			rule.resize(Rule::SHRINK_DOWN);
		} else {
			rule.resize(Rule::EXTEND_DOWN);
		}
	}
}

fn rule_cell_edit(
	ui: &mut Ui,
	origin: Pos2,
	rule: &mut Option<Cell>,
	x: usize,
	y: usize,
	cells: &[CellData],
) -> bool {
	let mut changed = false;
	let rect = Rect::from_min_size(
		origin + Vec2::from((x as f32, y as f32)) * CSIZE,
		Vec2::splat(CSIZE),
	);
	let aabb = ui.allocate_rect(rect, Sense::click());
	if let Some(cell) = rule {
		let color = cells[cell.id()].color;
		ui.painter()
			.rect(rect.shrink(OUTLINE.0 / 2.), 0., color, OUTLINE);
		if aabb.clicked() {
			changed = true;
			cell.0 += 1;
			if cell.0 as usize == cells.len() {
				*rule = None;
			}
		}
	} else if aabb.clicked() {
		*rule = Some(Cell(0));
		changed = true;
	}
	changed
}

impl CellData {
	fn new(name: &str, r: u8, g: u8, b: u8) -> Self {
		Self {
			name: name.to_owned(),
			color: Color32::from_rgb(r, g, b),
		}
	}
}
