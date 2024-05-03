use rand::prelude::*;
use serde::{Deserialize, Serialize};

pub const CHUNK_SIZE: usize = 32;

#[derive(Default, Debug, PartialEq, Clone, Copy, Serialize, Deserialize)]
pub struct Cell(pub u16);

#[derive(Debug)]
pub struct Dish {
	pub chunk: Chunk,
	pub rules: Vec<Rule>,
	pub cell_groups: Vec<Vec<Option<Cell>>>,
}

#[derive(Debug)]
pub struct Chunk {
	pub contents: Box<[[Cell; CHUNK_SIZE]; CHUNK_SIZE]>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Rule {
	base: SubRule,
	#[serde(skip)]
	variants: Vec<SubRule>,
	pub enabled: bool,
	// probability: u8
	pub flip_h: bool,
	pub flip_v: bool,
	pub rotate: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct SubRule {
	width: usize,
	height: usize,
	contents: Vec<(RuleCellFrom, RuleCellTo)>,
}

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub enum RuleCellFrom {
	/// matches anything
	#[default]
	Any,
	/// matches one cell type
	One(Cell),
	/// matches anything defined in the group referenced by this index
	Group(usize),
}

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub enum RuleCellTo {
	/// don't modify this cell
	#[default]
	None,
	/// set to this cell
	One(Cell),
	/// randomly choose from the group
	GroupRandom(usize),
	/// copy the cell from the corresponding input position
	Copy(usize),
}

impl std::default::Default for SubRule {
	fn default() -> Self {
		Self::new()
	}
}

impl SubRule {
	fn new() -> Self {
		Self {
			width: 1,
			height: 1,
			contents: vec![Default::default()],
		}
	}

	fn get(&self, x: usize, y: usize) -> (RuleCellFrom, RuleCellTo) {
		if x >= self.width || y >= self.height {
			Default::default()
		} else {
			self.contents[x + self.width * y].clone()
		}
	}

	fn get_mut(&mut self, x: usize, y: usize) -> &mut (RuleCellFrom, RuleCellTo) {
		assert!(x < self.width || y < self.height);
		&mut self.contents[x + self.width * y]
	}

	fn set_both(&mut self, x: usize, y: usize, cells: (RuleCellFrom, RuleCellTo)) {
		if x < self.width && y < self.height {
			self.contents[x + self.width * y] = cells;
		}
	}

	fn set_from(&mut self, x: usize, y: usize, cell: RuleCellFrom) {
		if x < self.width && y < self.height {
			self.contents[x + self.width * y].0 = cell;
		}
	}

	fn set_to(&mut self, x: usize, y: usize, cell: RuleCellTo) {
		if x < self.width && y < self.height {
			self.contents[x + self.width * y].1 = cell;
		}
	}
}

type ResizeParam = (isize, isize, isize, isize);
impl Rule {
	pub const EXTEND_LEFT: ResizeParam = (1, 0, -1, 0);
	pub const EXTEND_RIGHT: ResizeParam = (1, 0, 0, 0);
	pub const EXTEND_UP: ResizeParam = (0, 1, 0, -1);
	pub const EXTEND_DOWN: ResizeParam = (0, 1, 0, 0);
	pub const SHRINK_LEFT: ResizeParam = (-1, 0, 1, 0);
	pub const SHRINK_RIGHT: ResizeParam = (-1, 0, 0, 0);
	pub const SHRINK_UP: ResizeParam = (0, -1, 0, 1);
	pub const SHRINK_DOWN: ResizeParam = (0, -1, 0, 0);

	pub fn get(&self, x: usize, y: usize) -> (RuleCellFrom, RuleCellTo) {
		self.base.get(x, y)
	}

	pub fn get_mut(&mut self, x: usize, y: usize) -> &mut (RuleCellFrom, RuleCellTo) {
		self.base.get_mut(x, y)
	}

	pub fn set_from(&mut self, x: usize, y: usize, cell: RuleCellFrom) {
		self.base.set_from(x, y, cell);
		self.generate_variants();
	}

	pub fn set_to(&mut self, x: usize, y: usize, cell: RuleCellTo) {
		self.base.set_to(x, y, cell);
		self.generate_variants();
	}

	pub fn height(&self) -> usize {
		self.base.height
	}

	pub fn width(&self) -> usize {
		self.base.width
	}

	pub fn resize(&mut self, params: ResizeParam) {
		let (dw, dh, dx, dy) = params;

		let new_width = self.base.width.saturating_add_signed(dw);
		let new_height = self.base.height.saturating_add_signed(dh);
		if new_width < 1 || new_height < 1 {
			return;
		}
		let mut new_contents = vec![Default::default(); new_width * new_height];

		for nx in 0..new_width {
			let oldx = nx.wrapping_add_signed(dx);
			for ny in 0..new_height {
				let oldy = ny.wrapping_add_signed(dy);
				new_contents[nx + new_width * ny] = self.get(oldx, oldy);
			}
		}

		self.base.contents = new_contents;
		self.base.height = new_height;
		self.base.width = new_width;
		self.generate_variants();
	}

	pub fn generate_variants(&mut self) {
		self.variants.clear();
		self.variants.push(self.base.clone());

		fn transform_variants(variants: &mut Vec<SubRule>, f: fn(&SubRule) -> SubRule) {
			let mut new = Vec::new();
			for v in variants.iter() {
				let new_variant = f(v);
				if !variants.contains(&new_variant) {
					new.push(new_variant);
				}
			}
			variants.extend_from_slice(&new);
		}

		if self.flip_h {
			transform_variants(&mut self.variants, |b| {
				let mut new = b.clone();
				for y in 0..new.height {
					for x in 0..new.width {
						let old = b.get(new.width - x - 1, y);
						new.set_both(x, y, old);
					}
				}
				new
			});
		}
		if self.flip_v {
			transform_variants(&mut self.variants, |b| {
				let mut new = b.clone();
				for y in 0..new.height {
					for x in 0..new.width {
						let old = b.get(x, new.height - y - 1);
						new.set_both(x, y, old);
					}
				}
				new
			});
		}
		if self.rotate {
			// 180° rotations (same as flipping x and y)
			transform_variants(&mut self.variants, |b| {
				let mut new = b.clone();
				for y in 0..new.height {
					for x in 0..new.width {
						let old = b.get(new.width - x - 1, new.height - y - 1);
						new.set_both(x, y, old);
					}
				}
				new
			});
			// 90° rotations
			transform_variants(&mut self.variants, |b| {
				let mut new = b.clone();
				new.height = b.width;
				new.width = b.height;
				for y in 0..new.height {
					for x in 0..new.width {
						let old = b.get(y, x);
						new.set_both(x, y, old);
					}
				}
				new
			})
		}
	}
}

impl Chunk {
	fn new() -> Self {
		Self {
			contents: vec![[Cell(0); CHUNK_SIZE]; CHUNK_SIZE]
				.into_boxed_slice()
				.try_into()
				.unwrap(),
		}
	}

	fn fill_random(mut self) -> Self {
		for col in self.contents.iter_mut() {
			for cell in col.iter_mut() {
				if random::<u8>() % 4 == 0 {
					*cell = Cell(1);
				}
			}
		}
		self
	}

	pub fn get_cell(&self, x: usize, y: usize) -> Cell {
		self.contents[x][y]
	}

	fn set_cell(&mut self, x: usize, y: usize, cell: Cell) {
		self.contents[x][y] = cell
	}
}

impl Dish {
	pub fn new() -> Self {
		let mut default_rules = vec![
			Rule {
				enabled: true,
				base: SubRule {
					width: 1,
					height: 2,
					contents: vec![
						(RuleCellFrom::One(Cell(1)), RuleCellTo::One(Cell(0))),
						(RuleCellFrom::One(Cell(0)), RuleCellTo::One(Cell(1))),
					],
				},
				..Rule::default()
			},
			Rule {
				enabled: true,
				base: SubRule {
					width: 2,
					height: 2,
					contents: vec![
						(RuleCellFrom::One(Cell(1)), RuleCellTo::One(Cell(0))),
						(RuleCellFrom::Any, RuleCellTo::None),
						(RuleCellFrom::One(Cell(1)), RuleCellTo::None),
						(RuleCellFrom::One(Cell(0)), RuleCellTo::One(Cell(1))),
					],
				},
				flip_h: true,
				..Rule::default()
			},
		];

		for rule in &mut default_rules {
			rule.generate_variants()
		}

		Self {
			chunk: Chunk::new().fill_random(),
			rules: default_rules,
			cell_groups: vec![vec![None, Some(Cell(1))]],
		}
	}

	pub fn update_rules(&mut self) {
		for rule in &mut self.rules {
			rule.generate_variants();
		}
	}

	pub fn fire_blindly(&mut self) {
		if self.rules.is_empty() {
			return;
		}
		let enabled_rules = self
			.rules
			.iter()
			.enumerate()
			.filter_map(|(i, r)| r.enabled.then_some(i))
			.collect::<Vec<_>>();
		if enabled_rules.is_empty() {
			return;
		}
		let rule = random::<usize>() % enabled_rules.len();
		let rule = enabled_rules[rule];
		self.fire_rule(rule);
	}

	fn fire_rule(&mut self, rule_index: usize) {
		let rule = &self.rules[rule_index];
		let variant_index = random::<usize>() % rule.variants.len();
		let variant = &rule.variants[variant_index].clone();
		let border_x = variant.width - 1;
		let border_y = variant.height - 1;
		let x = ((random::<usize>() % (CHUNK_SIZE + border_x)) as isize)
			.wrapping_sub_unsigned(border_x);
		let y = ((random::<usize>() % (CHUNK_SIZE + border_y)) as isize)
			.wrapping_sub_unsigned(border_y);

		if !self.subrule_matches(x, y, variant) {
			return;
		}

		let width = variant.width;
		let height = variant.height;
		let mut old_state = Vec::new();
		for dy in 0..height {
			for dx in 0..width {
				old_state.push(
					self.get_cell((x as usize).wrapping_add(dx), (y as usize).wrapping_add(dy)),
				);
			}
		}

		for dx in 0..width {
			for dy in 0..height {
				let px = x.wrapping_add_unsigned(dx) as usize;
				let py = y.wrapping_add_unsigned(dy) as usize;
				match variant.get(dx, dy).1 {
					RuleCellTo::One(rule_cell) => {
						self.set_cell(px, py, rule_cell);
					}
					RuleCellTo::GroupRandom(group_id) => {
						let group = &self.cell_groups[group_id];
						let i = random::<usize>() % group.len();
						let cell = group[i];
						if let Some(cell) = cell {
							self.set_cell(px, py, cell);
						}
					}
					RuleCellTo::Copy(index) => {
						let cell = old_state[index];
						if let Some(cell) = cell {
							// if the copy source is outside the world, do nothing
							self.set_cell(px, py, cell);
						}
					}
					RuleCellTo::None => (),
				}
			}
		}
	}

	fn subrule_matches(&self, x: isize, y: isize, subrule: &SubRule) -> bool {
		for dx in 0..subrule.width {
			for dy in 0..subrule.height {
				let x = x.wrapping_add_unsigned(dx) as usize;
				let y = y.wrapping_add_unsigned(dy) as usize;
				let cell = self.get_cell(x, y);
				match subrule.get(dx, dy).0 {
					RuleCellFrom::One(rule_cell) => {
						if cell != Some(rule_cell) {
							return false;
						}
					}
					RuleCellFrom::Group(group_id) => {
						if !self.cell_groups[group_id].contains(&cell) {
							return false;
						}
					}
					RuleCellFrom::Any => (),
				}
			}
		}
		true
	}

	//todo isize
	pub fn get_cell(&self, x: usize, y: usize) -> Option<Cell> {
		if x >= CHUNK_SIZE || y >= CHUNK_SIZE {
			None
		} else {
			Some(self.chunk.get_cell(x, y))
		}
	}

	//todo isize
	pub fn set_cell(&mut self, x: usize, y: usize, cell: Cell) {
		if x >= CHUNK_SIZE || y >= CHUNK_SIZE {
			return;
		}
		self.chunk.set_cell(x, y, cell)
	}
}

impl Cell {
	pub fn id(&self) -> usize {
		self.0 as usize
	}
}
