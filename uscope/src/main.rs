use eframe::{
	egui::{CentralPanel, Color32, Painter, Rect, SidePanel, Ui, Vec2},
	NativeOptions,
};
use petri::{Chunk, Dish, Rule, CHUNK_SIZE};

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
}

impl UScope {
	fn new(_cc: &eframe::CreationContext<'_>) -> Self {
		Self { dish: Dish::new() }
	}
}

impl eframe::App for UScope {
	fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
		ctx.request_repaint();
		for _ in 0..100 {
			self.dish.fire_blindly();
		}
		SidePanel::left("left_panel").show(ctx, |ui| {
			ui.heading("Rules");
			ui.text_edit_singleline(&mut "dummy");
			if ui.button("aaa").clicked() {
				dbg!(&self.dish.rules);
			}
			for rule in &mut self.dish.rules {
				rule_editor(ui, rule);
			}
		});
		CentralPanel::default().show(ctx, |ui| {
			let bounds = ui.available_rect_before_wrap();
			let painter = ui.painter_at(bounds);
			paint_chunk(painter, &self.dish.chunk);
		});
	}
}

fn paint_chunk(painter: Painter, chunk: &Chunk) {
	let bounds = painter.clip_rect();
	let size = 16.;
	for x in 0..CHUNK_SIZE {
		for y in 0..CHUNK_SIZE {
			let cell = &chunk.get_cell(x, y);
			let corner = bounds.min + (Vec2::from((x as f32, y as f32)) * size);
			let rect = Rect::from_min_size(corner, Vec2::splat(size));
			let color = Color32::from_rgb(cell.0, cell.1, cell.2);
			painter.rect(rect, 0., color, (1., Color32::GRAY));
		}
	}
}

fn rule_editor(ui: &mut Ui, rule: &mut Rule) {
	let patt_height = rule.from.height();
	let patt_width = rule.from.height();

	const CSIZE: f32 = 24.;

	let (_, bounds) = ui.allocate_space(Vec2::new(
		CSIZE * (patt_width * 2 + 1) as f32,
		CSIZE * patt_height as f32,
	));
	for x in 0..patt_width {
		for y in 0..patt_height {
			let rect = Rect::from_min_size(
				bounds.min + Vec2::from((x as f32, y as f32)) * CSIZE,
				Vec2::splat(CSIZE),
			);
			if let Some(cell) = rule.from.get(x, y) {
				ui.painter().rect(
					rect,
					2.,
					Color32::from_rgb(cell.0, cell.1, cell.2),
					(1., Color32::GRAY),
				);
			}

			if let Some(cell) = rule.to.get(x, y) {
				let rect = rect.translate(Vec2::X * (patt_width as f32 + 1.) * CSIZE);

				ui.painter().rect(
					rect,
					2.,
					Color32::from_rgb(cell.0, cell.1, cell.2),
					(1., Color32::GRAY),
				);
			}
		}
	}
}
