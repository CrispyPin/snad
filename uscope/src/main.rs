use eframe::{
	egui::{CentralPanel, Color32, Painter, Pos2, Rect, Sense, SidePanel, Ui, Vec2},
	NativeOptions,
};
use petri::{Cell, Chunk, Dish, Rule, RulePattern, CHUNK_SIZE};

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
		for _ in 0..100 {
			self.dish.fire_blindly();
		}
		SidePanel::left("left_panel").show(ctx, |ui| {
			ui.heading("Cells");
			for cell in &mut self.celltypes {
				ui.horizontal(|ui| {
					ui.set_width(100.);
					ui.text_edit_singleline(&mut cell.name);
					ui.color_edit_button_srgba(&mut cell.color);
				});
			}
			ui.heading("Rules");
			for rule in &mut self.dish.rules {
				rule_editor(ui, rule, &self.celltypes);
			}
		});
		CentralPanel::default().show(ctx, |ui| {
			let bounds = ui.available_rect_before_wrap();
			let painter = ui.painter_at(bounds);
			paint_chunk(painter, &self.dish.chunk, &self.celltypes);
		});
	}
}

fn paint_chunk(painter: Painter, chunk: &Chunk, cells: &[CellData]) {
	let bounds = painter.clip_rect();
	let size = 16.;
	for x in 0..CHUNK_SIZE {
		for y in 0..CHUNK_SIZE {
			let cell = &chunk.get_cell(x, y);
			let corner = bounds.min + (Vec2::from((x as f32, y as f32)) * size);
			let rect = Rect::from_min_size(corner, Vec2::splat(size));
			let color = cells[cell.id()].color;
			painter.rect(rect, 0., color, (1., Color32::GRAY));
		}
	}
}

const CSIZE: f32 = 24.;
const OUTLINE: (f32, Color32) = (1., Color32::GRAY);
fn rule_editor(ui: &mut Ui, rule: &mut Rule, cells: &[CellData]) {
	let patt_height = rule.from.height();
	let patt_width = rule.from.height();

	let (_, bounds) = ui.allocate_space(Vec2::new(
		CSIZE * (patt_width * 2 + 1) as f32,
		CSIZE * patt_height as f32,
	));
	for x in 0..patt_width {
		for y in 0..patt_height {
			rule_cell_edit(ui, bounds.min, &mut rule.from, x, y, cells);
			let offset = Vec2::X * (patt_width as f32 + 1.) * CSIZE;
			rule_cell_edit(ui, bounds.min + offset, &mut rule.to, x, y, cells);
		}
	}
}

fn rule_cell_edit(
	ui: &mut Ui,
	origin: Pos2,
	rule: &mut RulePattern,
	x: usize,
	y: usize,
	cells: &[CellData],
) {
	let rect = Rect::from_min_size(
		origin + Vec2::from((x as f32, y as f32)) * CSIZE,
		Vec2::splat(CSIZE),
	);
	let aabb = ui.allocate_rect(rect, Sense::click());
	if let Some(cell) = rule.get_mut(x, y) {
		let color = cells[cell.id()].color;
		ui.painter().rect(rect, 2., color, OUTLINE);
		if aabb.clicked() {
			cell.0 += 1;
			if cell.0 as usize == cells.len() {
				rule.set(x, y, None);
			}
		}
	} else if aabb.clicked() {
		rule.set(x, y, Some(Cell(0)));
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
