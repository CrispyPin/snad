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
	pub from: RulePattern,
	pub to: RulePattern,
	pub enabled: bool,
	// probability: u8
	// flip:
	// rotate:
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
			from: RulePattern::new(),
			to: RulePattern::new(),
		}
	}

	pub fn resize(&mut self, params: ResizeParam) {
		let (dw, dh, dx, dy) = params;
		self.from.resize(dw, dh, dx, dy);
		self.to.resize(dw, dh, dx, dy);
	}
}

impl Chunk {
	fn new() -> Self {
		Self {
			contents: vec![[Cell::EMPTY; CHUNK_SIZE]; CHUNK_SIZE]
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
		Self {
			chunk: Chunk::new().fill_random(),

			rules: vec![
				Rule {
					enabled: true,
					from: RulePattern {
						width: 1,
						height: 2,
						contents: vec![Some(Cell(1)), Some(Cell(0))],
					},
					to: RulePattern {
						width: 1,
						height: 2,
						contents: vec![Some(Cell(0)), Some(Cell(1))],
					},
				},
				Rule {
					enabled: true,
					from: RulePattern {
						width: 2,
						height: 2,
						contents: vec![Some(Cell(1)), None, Some(Cell(1)), Some(Cell(0))],
					},
					to: RulePattern {
						width: 2,
						height: 2,
						contents: vec![Some(Cell(0)), None, Some(Cell(1)), Some(Cell(1))],
					},
				},
				Rule {
					enabled: true,
					from: RulePattern {
						width: 2,
						height: 2,
						contents: vec![None, Some(Cell(1)), Some(Cell(0)), Some(Cell(1))],
					},
					to: RulePattern {
						width: 2,
						height: 2,
						contents: vec![None, Some(Cell(0)), Some(Cell(1)), Some(Cell(1))],
					},
				},
			],
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
		let width = rule.to.width;
		let height = rule.to.height;
		// check is match
		for dx in 0..width {
			for dy in 0..height {
				let x = x + dx;
				let y = y + dy;
				if let Some(rule_cell) = rule.from.get(dx, dy) {
					if self.get_cell(x, y) != Some(rule_cell) {
						return;
					}
				}
			}
		}
		for dx in 0..width {
			for dy in 0..height {
				let x = x + dx;
				let y = y + dy;
				if let Some(rule_cell) = &self.rules[rule_index].to.get(dx, dy) {
					self.set_cell(x, y, rule_cell.clone());
				}
			}
		}
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

#[derive(Debug)]
pub struct RulePattern {
	width: usize,
	height: usize,
	contents: Vec<Option<Cell>>,
}

impl RulePattern {
	pub fn new() -> Self {
		Self {
			width: 1,
			height: 1,
			contents: vec![None],
		}
	}

	pub fn get(&self, x: usize, y: usize) -> Option<Cell> {
		if x >= self.width || y >= self.height {
			None
		} else {
			self.contents[x + self.width * y].clone()
		}
	}

	pub fn get_mut(&mut self, x: usize, y: usize) -> Option<&mut Cell> {
		if x >= self.width || y >= self.height {
			None
		} else {
			self.contents[x + self.width * y].as_mut()
		}
	}

	pub fn set(&mut self, x: usize, y: usize, cell: Option<Cell>) {
		if x < self.width && y < self.height {
			self.contents[x + self.width * y] = cell
		}
	}

	pub fn height(&self) -> usize {
		self.height
	}

	pub fn width(&self) -> usize {
		self.width
	}

	fn resize(&mut self, dw: isize, dh: isize, dx: isize, dy: isize) {
		let new_width = self.width.saturating_add_signed(dw);
		let new_height = self.height.saturating_add_signed(dh);
		if new_width < 1 || new_height < 1 {
			return;
		}
		let mut new_contents = vec![None; new_width * new_height];

		for nx in 0..new_width {
			let oldx = nx.wrapping_add_signed(dx);
			for ny in 0..new_height {
				let oldy = ny.wrapping_add_signed(dy);
				new_contents[nx + new_width * ny] = self.get(oldx, oldy);
			}
		}

		self.contents = new_contents;
		self.height = new_height;
		self.width = new_width;
	}
}

impl Cell {
	pub const EMPTY: Self = Cell(0);
	pub fn id(&self) -> usize {
		self.0 as usize
	}
}
