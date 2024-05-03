use rand::prelude::*;

pub const CHUNK_SIZE: usize = 32;

#[derive(Default, Debug, PartialEq, Clone, Copy)]
pub struct Cell(pub u16);

#[derive(Debug)]
pub struct Dish {
	pub chunk: Chunk,
	pub rules: Vec<Rule>,
}

#[derive(Debug)]
pub struct Chunk {
	pub contents: Box<[[Cell; CHUNK_SIZE]; CHUNK_SIZE]>,
}

#[derive(Debug)]
pub struct Rule {
	base: SubRule,
	variants: Vec<SubRule>,
	pub enabled: bool,
	// probability: u8
	pub flip_h: bool,
	pub flip_v: bool,
	// rotate:
}

#[derive(Debug, Clone, PartialEq)]
struct SubRule {
	width: usize,
	height: usize,
	contents: Vec<(Option<Cell>, Option<Cell>)>,
}

impl SubRule {
	fn new() -> Self {
		Self {
			width: 1,
			height: 1,
			contents: vec![(None, None)],
		}
	}

	fn get(&self, x: usize, y: usize) -> (Option<Cell>, Option<Cell>) {
		if x >= self.width || y >= self.height {
			(None, None)
		} else {
			self.contents[x + self.width * y].clone()
		}
	}

	fn get_mut(&mut self, x: usize, y: usize) -> &mut (Option<Cell>, Option<Cell>) {
		assert!(x < self.width || y < self.height);
		&mut self.contents[x + self.width * y]
	}

	fn set_both(&mut self, x: usize, y: usize, cells: (Option<Cell>, Option<Cell>)) {
		if x < self.width && y < self.height {
			self.contents[x + self.width * y] = cells;
		}
	}

	fn set_from(&mut self, x: usize, y: usize, cell: Option<Cell>) {
		if x < self.width && y < self.height {
			self.contents[x + self.width * y].0 = cell;
		}
	}

	fn set_to(&mut self, x: usize, y: usize, cell: Option<Cell>) {
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

	pub fn new() -> Self {
		Self {
			enabled: false,
			base: SubRule::new(),
			variants: Vec::new(),
			flip_h: false,
			flip_v: false,
		}
	}

	pub fn get(&self, x: usize, y: usize) -> (Option<Cell>, Option<Cell>) {
		self.base.get(x, y)
	}

	pub fn get_mut(&mut self, x: usize, y: usize) -> &mut (Option<Cell>, Option<Cell>) {
		self.base.get_mut(x, y)
	}

	pub fn set_from(&mut self, x: usize, y: usize, cell: Option<Cell>) {
		self.base.set_from(x, y, cell);
		self.generate_variants();
	}

	pub fn set_to(&mut self, x: usize, y: usize, cell: Option<Cell>) {
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
		let mut new_contents = vec![(None, None); new_width * new_height];

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
			})
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
		self.contents[x][y].clone()
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
						(Some(Cell(1)), Some(Cell(0))),
						(Some(Cell(0)), Some(Cell(1))),
					],
				},
				..Rule::new()
			},
			Rule {
				enabled: true,
				base: SubRule {
					width: 2,
					height: 2,
					contents: vec![
						(Some(Cell(1)), Some(Cell(0))),
						(None, None),
						(Some(Cell(1)), None),
						(Some(Cell(0)), Some(Cell(1))),
					],
				},
				flip_h: true,
				..Rule::new()
			},
		];

		for rule in &mut default_rules {
			rule.generate_variants()
		}

		Self {
			chunk: Chunk::new().fill_random(),
			rules: default_rules,
		}
	}

	pub fn fire_blindly(&mut self) {
		if self.rules.is_empty() {
			return;
		}
		let x = random::<usize>() % CHUNK_SIZE;
		let y = random::<usize>() % CHUNK_SIZE;
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
		self.fire_rule(rule, x, y);
	}

	fn fire_rule(&mut self, rule_index: usize, x: usize, y: usize) {
		let rule = &self.rules[rule_index];
		// find matching variants
		let mut matching_variants = Vec::new();
		for (i, v) in rule.variants.iter().enumerate() {
			if self.subrule_matches(x, y, v) {
				matching_variants.push(i);
			}
		}
		if matching_variants.is_empty() {
			return;
		}

		let variant_index = random::<usize>() % matching_variants.len();
		let variant = rule.variants[matching_variants[variant_index]].clone();

		let width = variant.width;
		let height = variant.height;
		for dx in 0..width {
			for dy in 0..height {
				let x = x + dx;
				let y = y + dy;
				if let Some(rule_cell) = variant.get(dx, dy).1 {
					self.set_cell(x, y, rule_cell.clone());
				}
			}
		}
	}

	fn subrule_matches(&self, x: usize, y: usize, subrule: &SubRule) -> bool {
		for dx in 0..subrule.width {
			for dy in 0..subrule.height {
				let x = x + dx;
				let y = y + dy;
				if let Some(rule_cell) = subrule.get(dx, dy).0 {
					if self.get_cell(x, y) != Some(rule_cell) {
						return false;
					}
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
